"""The built font: generated glyphs blended from his real digits."""
import json
import sys

import cv2
import numpy as np

from config import GLYPH, PACK, ROLES, STAMP
from curation import features, fit_svm, load_meta
from pack import harmonize_ink, pack_rev, write_manifest, write_sprite
from provenance import save_png

SYNTH_TUNE = {
    ("user", "2"): {"alpha": 8.0, "amp": 2.0},
    ("user", "5"): {"alpha": 8.0, "amp": 2.0, "reroll": 1},
    # holes: required enclosed-background regions — an open-hook 6 passes
    # every intensity check but encloses nothing
    ("user", "6"): {"alpha": 8.0, "amp": 2.0, "reroll": 1, "holes": 1},
}



def stage_synth(per_class=10, seed=7, targets=None):
    """The built font: each glyph is a blend of 3-4 of his real digits (SDF
    space) plus elastic warp — his style, no actual scan. Writes INTO the
    souvenir pack (web/assets/souvenir), replacing the pictured glyphs; run
    `emit` instead to restore the real scans. Same stamp + watermark, and the
    same finishing touch (harmonize_ink) as the real pack, so the approved
    ink look carries over by construction.
    `targets` (e.g. ["user5", "given3"]) regenerates ONLY those digits and
    leaves every other row's files untouched — approved rows stay frozen.
    Each digit gets its own rng stream, so re-rolling one digit can never
    reshuffle another."""
    rows = [r for r in load_meta() if r["label"] not in ("", "-1")]
    # the same SVM that labels his real glyphs gates the generated ones:
    # coverage checks pass chunky-but-malformed blends, a classifier doesn't
    clf = fit_svm(rows)

    def digit_proba(a8, d):
        m = (a8 > 40).astype(np.uint8) * 255
        ys, xs = np.nonzero(m)
        if len(ys) == 0:
            return 0.0
        crop = m[ys.min() : ys.max() + 1, xs.min() : xs.max() + 1]
        h, w = crop.shape
        side = max(h, w)
        sq = np.zeros((side, side), np.uint8)
        sq[(side - h) // 2 : (side - h) // 2 + h, (side - w) // 2 : (side - w) // 2 + w] = crop
        x = (cv2.resize(sq, (32, 32)).ravel() > 0).astype(np.float32)
        p = clf.predict_proba(x[None])[0]
        return float(p[list(clf.classes_).index(d)]) if d in clf.classes_ else 0.0

    out_pack = PACK
    manifest_path = out_pack / "manifest.json"
    old_man = json.loads(manifest_path.read_text()) if manifest_path.exists() else None
    if targets is None:
        for old in out_pack.glob("digits/*/*.png"):
            old.unlink()
    S = 128  # working canvas

    def norm_mask(r):
        rgba = cv2.imread(str(WORK / "glyphs" / f"{r['id']}.png"), cv2.IMREAD_UNCHANGED)
        a = rgba[:, :, 3]
        # adaptive threshold: user-role ink is softer — a fixed cut shreds it
        on = a[a > 8]
        if on.size == 0:
            return None
        thr = max(16, int(np.percentile(on, 50) * 0.45))
        m = ((a > thr).astype(np.uint8)) * 255
        m = cv2.morphologyEx(m, cv2.MORPH_CLOSE, np.ones((3, 3), np.uint8))
        ys, xs = np.nonzero(m)
        if len(ys) == 0:
            return None
        crop = m[ys.min() : ys.max() + 1, xs.min() : xs.max() + 1]
        h, w = crop.shape
        s = (S - 36) / max(h, w)
        crop = cv2.resize(crop, (max(1, int(w * s)), max(1, int(h * s))))
        c = np.zeros((S, S), np.uint8)
        y0, x0 = (S - crop.shape[0]) // 2, (S - crop.shape[1]) // 2
        c[y0 : y0 + crop.shape[0], x0 : x0 + crop.shape[1]] = crop
        return c

    def sdf(mask):
        m = (mask > 64).astype(np.uint8)
        return cv2.distanceTransform(1 - m, cv2.DIST_L2, 5) - cv2.distanceTransform(m, cv2.DIST_L2, 5)

    def elastic(field_shape, amp, sigma):
        d = rng.standard_normal(field_shape).astype(np.float32)
        return cv2.GaussianBlur(d, (0, 0), sigma) * amp

    manifest = {"name": "souvenir", "digits": {"given": {}, "user": {}}}
    for role in ROLES:
        ink = np.float32(INK_TARGET[role])
        for d in range(1, 10):
            if targets is not None and f"{role}{d}" not in targets:
                # frozen row: keep its existing files and manifest entry
                if old_man and str(d) in old_man["digits"].get(role, {}):
                    manifest["digits"][role][str(d)] = old_man["digits"][role][str(d)]
                continue
            tune = SYNTH_TUNE.get((role, str(d)), {})
            alpha_c, amp = tune.get("alpha", 4.0), tune.get("amp", 3.0)
            # own rng stream per digit: re-rolling one digit never reshuffles
            # another; "reroll" in SYNTH_TUNE bumps a digit onto a luckier stream
            rng = np.random.default_rng([seed + tune.get("reroll", 0), {"given": 0, "user": 1}[role], d])
            pool, pinned_idx = [], []
            for r in rows:
                if r["role"] == role and r["label"] == str(d) and (m := norm_mask(r)) is not None:
                    if r.get("pin"):
                        pinned_idx.append(len(pool))
                    pool.append(m)
            if len(pool) < 2:
                # a thin pool must not silently drop the row (and orphan its
                # files): keep the previous entry on targeted runs, warn loud
                if targets is not None and old_man and str(d) in old_man["digits"].get(role, {}):
                    manifest["digits"][role][str(d)] = old_man["digits"][role][str(d)]
                    print(f"WARNING: {role} {d}: pool < 2 exemplars — kept the previous row")
                else:
                    print(f"WARNING: {role} {d}: pool < 2 exemplars — row DROPPED from the pack")
                continue
            # runt gate: a blend of fragmented masks can come out shrunken or
            # broken; anything under 60% of the pool's median ink coverage is
            # rejected and redrawn
            pool_cov = float(np.median([(m > 64).sum() for m in pool]))
            # bold exemplars anchor more often: faint thin ones (his light-pen
            # 9s) dragged whole blends down when they led
            covs = np.array([(m > 64).sum() for m in pool], np.float64)
            anchor_p = covs / covs.sum()
            # pins steer the font: when a digit has pinned exemplars, only
            # they anchor — the curation knob for "this 9, not that 9"
            if pinned_idx:
                anchor_p = np.zeros(len(pool))
                anchor_p[pinned_idx] = covs[pinned_idx]
                anchor_p /= anchor_p.sum()
            sdfs = [sdf(m) for m in pool]
            # blend 3-4 exemplars, not a near-copy pair: measured on the pack,
            # pair-morphs sat at 0.79 IoU to their nearest scan while his own
            # digits sit at 0.60 to each other — multi-blends land at his
            # natural variation, so no output is close to any single scan.
            # anchored around the 3 most-alike exemplars so the blend stays a
            # coherent digit, not a mongrel of divergent shapes
            bin_pool = [(m > 64) for m in pool]

            def iou_m(a, b):
                return (a & b).sum() / max(1, (a | b).sum())

            paths = []
            for k in range(per_class):
                a8, best, best_score = None, None, -1.0
                for _ in range(12):
                    i = int(rng.choice(len(pool), p=anchor_p))
                    near = sorted(
                        (j for j in range(len(pool)) if j != i), key=lambda j: -iou_m(bin_pool[i], bin_pool[j])
                    )
                    circle = near[: min(6, len(near))]
                    picks = [i] + list(rng.choice(circle, min(3, len(circle)), replace=False))
                    # concentrated dirichlet: flat draws land near corners and one
                    # exemplar dominates — that's a near-copy again. Largest
                    # weight goes to the anchor: thin features (his 9-tails)
                    # smear away when the anchor doesn't lead its own blend.
                    w = np.sort(rng.dirichlet(np.ones(len(picks)) * alpha_c))[::-1]
                    s = sum(wi * sdfs[p] for wi, p in zip(w, picks))
                    gx, gy = np.meshgrid(np.arange(S, dtype=np.float32), np.arange(S, dtype=np.float32))
                    s = cv2.remap(
                        s, gx + elastic((S, S), amp, 10), gy + elastic((S, S), amp, 10), cv2.INTER_LINEAR
                    )
                    s += rng.uniform(-0.7, 0.7)  # stroke-width jitter
                    alpha = np.clip((0.75 - s) / 1.5, 0, 1)  # soft pen edge
                    ripple = cv2.GaussianBlur(rng.standard_normal((S, S)).astype(np.float32), (0, 0), 1.5)
                    alpha = np.clip(alpha * (1 + 0.25 * ripple), 0, 1)
                    cand = (alpha * 255).astype(np.uint8)
                    n_comp = cv2.connectedComponents((cand > 96).astype(np.uint8))[0] - 1
                    cov = (cand > 96).sum()
                    proba = digit_proba(cand, d)
                    # pale-streak integrity: where blended exemplars disagree
                    # mid-stroke, alpha dips and the glyph looks broken even
                    # though it's connected at low threshold
                    solid = (cand > 170).sum() >= 0.6 * max(1, cov)
                    # a pale streak splits the glyph at high threshold even
                    # when it's connected at low threshold — catch that too
                    n_hi = cv2.connectedComponents((cand > 170).astype(np.uint8))[0] - 1
                    holes_ok = count_holes(cand > 96) >= tune.get("holes", 0)
                    if cov >= 0.7 * pool_cov and n_comp <= 2 and n_hi <= 2 and proba >= 0.5 and solid and holes_ok:
                        a8 = cand
                        break
                    # remember the least-bad attempt, judged mostly by the SVM
                    score = proba + 0.3 * min(1.0, cov / max(1.0, pool_cov)) - 0.5 * max(0, n_comp - 2)
                    if score > best_score:
                        best, best_score = cand, score
                if a8 is None:
                    a8 = best  # 12 tries: least-bad beats a hole in the pack
                shade = cv2.GaussianBlur(rng.standard_normal((S, S)).astype(np.float32), (0, 0), 3)
                rgb = np.clip(ink[None, None, :] * (1 + 0.18 * shade[..., None]), 0, 255).astype(np.uint8)
                out = cv2.resize(np.dstack([rgb, a8]), (GLYPH, GLYPH), interpolation=cv2.INTER_AREA)
                out = harmonize_ink(out, role)  # the approved finishing touch, same as emit
                rel = f"digits/{role}/{d}_s{k:02d}.png"
                dst = out_pack / rel
                dst.parent.mkdir(parents=True, exist_ok=True)
                save_png(dst, out)  # ink stacked as BGR + alpha = BGRA, save_png's contract
                paths.append(rel)
            manifest["digits"][role][str(d)] = paths
    if (out_pack / "paper.png").exists():
        manifest["paper"] = "paper.png"
    manifest["copyright"] = STAMP
    write_sprite(out_pack, manifest)
    pack_rev(out_pack, manifest)
    write_manifest(out_pack, manifest)
    total = sum(len(v) for role in manifest["digits"].values() for v in role.values())
    print(f"synthesized {total} glyphs -> {out_pack}/manifest.json")


