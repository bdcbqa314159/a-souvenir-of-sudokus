#!/usr/bin/env python3
"""atelier — photos of the handwritten grids -> the grandpere asset pack.

Stages (run in order; every output lands in originals/atelier-work/, gitignored):

  extract   all photos -> rectified grids -> ink split -> glyph crops + meta.csv
  sheet     contact sheets of unlabeled glyphs (for eyeball labeling)
  train     fit HOG+SVM on the labeled rows of meta.csv
  predict   label the rest with the SVM; write per-class sheets for verification
  emit      normalized RGBA glyphs -> web/assets/grandpere/ + manifest.json
  review    open a curation matrix: every accepted glyph per class, indexed by
            id; emitted variants outlined green, pinned ones blue
  pin       pipeline.py pin 0012 0034 ...  (unpin: pin -0012) — pinned glyphs
            always emit first

Labels live in meta.csv (column `label`, 1-9, empty = unlabeled; -1 = rejected).
Apply labels with:  pipeline.py label <id>=<digit> <id>=<digit> ...
"""
import csv
import json
import pathlib
import sys

import cv2
import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent.parent
ORIGINALS = ROOT / "originals"
WORK = ORIGINALS / "atelier-work"
PACK = ROOT / "web" / "assets" / "grandpere"
SIDE = 1080
CELL = SIDE // 9
GLYPH = 96  # emitted asset size
ROLES = {"given": "dark", "user": "red"}  # pack role -> ink

# ---------------------------------------------------------------- image basics


def red_mask(bgr):
    hsv = cv2.cvtColor(bgr, cv2.COLOR_BGR2HSV)
    lo = cv2.inRange(hsv, (0, 50, 60), (12, 255, 255))
    hi = cv2.inRange(hsv, (168, 50, 60), (180, 255, 255))
    return lo | hi


def dark_mask(bgr):
    """Black/grey pen, exposure-independent: adaptive threshold on luminance,
    minus anything the red mask claims. Fixed HSV bounds missed the fainter
    pages entirely (the old black-glyph yield was 175/14 pages; ~3x now)."""
    gray = cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY)
    ink = cv2.adaptiveThreshold(
        gray, 255, cv2.ADAPTIVE_THRESH_GAUSSIAN_C, cv2.THRESH_BINARY_INV, 35, 11
    )
    return ink & ~red_mask(bgr)


def grid_score(mask):
    """Does this warped quad look like a sudoku? Measured on the real photos:
    grids have red fraction 0.07-0.16 and ~10 ink-profile peaks per direction;
    skin (the knuckle that once won) is ~0.7 red and one solid band. Returns
    -1 for a rejected quad, else the total band count (higher = more grid)."""
    frac = np.count_nonzero(mask) / mask.size
    if not 0.02 < frac < 0.30:
        return -1
    fat = cv2.dilate(mask, cv2.getStructuringElement(cv2.MORPH_RECT, (5, 5)))

    def bands(axis):
        prof = (fat > 0).sum(axis=axis).astype(np.float32)
        prof = np.convolve(prof, np.ones(9) / 9, mode="same")
        above = prof > prof.max() * 0.35
        return int(np.count_nonzero(above[1:] & ~above[:-1]) + int(above[0]))

    return bands(1) + bands(0)


def rectify(photo: pathlib.Path):
    """Warp the sudoku frame to SIDE². Candidate quads come from the largest red
    contours; the winner is the one whose warp actually contains ruled lines —
    skin is red in HSV, and one photo's knuckle used to beat the grid."""
    bgr = cv2.imread(str(photo))
    if bgr is None:
        raise SystemExit(f"unreadable: {photo}")
    bgr = cv2.rotate(bgr, cv2.ROTATE_90_COUNTERCLOCKWISE)  # this batch is sideways
    mask = red_mask(bgr)
    k = cv2.getStructuringElement(cv2.MORPH_RECT, (5, 5))
    closed = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, k, iterations=3)
    contours, _ = cv2.findContours(closed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    target = np.float32([[0, 0], [SIDE, 0], [SIDE, SIDE], [0, SIDE]])
    best_warp, best_score = None, -1
    for cont in sorted(contours, key=cv2.contourArea, reverse=True)[:8]:
        peri = cv2.arcLength(cont, True)
        approx = cv2.approxPolyDP(cont, 0.02 * peri, True)
        if len(approx) != 4:
            approx = cv2.boxPoints(cv2.minAreaRect(cont)).astype(np.int32).reshape(-1, 1, 2)
        q = approx.reshape(4, 2).astype(np.float32)
        s, d = q.sum(axis=1), np.diff(q, axis=1).ravel()
        quad = np.float32([q[np.argmin(s)], q[np.argmin(d)], q[np.argmax(s)], q[np.argmax(d)]])
        warp = cv2.warpPerspective(bgr, cv2.getPerspectiveTransform(quad, target), (SIDE, SIDE))
        score = grid_score(red_mask(warp))
        if score > best_score:
            best_warp, best_score = warp, score
    if best_score < 10:
        print(f"WARNING: no grid found in {photo.name} (best score {best_score}) — skipped")
        return None
    return best_warp


def strip_lines(mask):
    n = SIDE // 12
    horiz = cv2.morphologyEx(mask, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_RECT, (n, 1)))
    vert = cv2.morphologyEx(mask, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_RECT, (1, n)))
    lines = cv2.dilate(horiz | vert, cv2.getStructuringElement(cv2.MORPH_RECT, (5, 5)))
    return mask & ~lines


def components(mask):
    """Connected components with slightly-merged broken strokes."""
    fat = cv2.dilate(mask, cv2.getStructuringElement(cv2.MORPH_RECT, (3, 3)))
    n, labels = cv2.connectedComponents(fat)
    for lab in range(1, n):
        m = ((labels == lab) & (mask > 0)).astype(np.uint8) * 255
        if not m.any():
            continue
        ys, xs = np.nonzero(m)
        yield m, int(xs.min()), int(ys.min()), int(xs.max() - xs.min() + 1), int(ys.max() - ys.min() + 1)


def is_big_digit(w, h, area):
    """Shape triage: big pen digits vs pencil marks vs surviving line remnants."""
    if h > 1.45 * CELL or w > 1.45 * CELL:
        return False  # line remnant
    if h > 0.9 * CELL and w < 0.14 * CELL:
        return False  # thin vertical remnant (his 1s are wider than a ruled line)
    if h < 0.34 * CELL:
        return False  # pencil mark (harvest those another day)
    return area >= 0.02 * CELL * CELL


# --------------------------------------------------------------------- stages


def glyph_rgba(warp, mask, x, y, w, h, ink):
    """Crop with soft alpha: opacity follows the real ink strength, so pale
    strokes render as pen pressure, not holes ('a bit too white')."""
    crop_c = warp[y : y + h, x : x + w]
    crop_m = mask[y : y + h, x : x + w]
    if ink == "dark":
        strength = 255 - cv2.cvtColor(crop_c, cv2.COLOR_BGR2GRAY)
    else:
        strength = cv2.cvtColor(crop_c, cv2.COLOR_BGR2HSV)[:, :, 1]
    alpha = np.clip(strength.astype(np.float32) * 2.0, 0, 255).astype(np.uint8)
    alpha[crop_m == 0] = 0
    alpha = cv2.GaussianBlur(alpha, (3, 3), 0)
    return np.dstack([crop_c, alpha])


def iou(a, b):
    ax, ay, aw, ah = a
    bx, by, bw, bh = b
    ix = max(0, min(ax + aw, bx + bw) - max(ax, bx))
    iy = max(0, min(ay + ah, by + bh) - max(ay, by))
    inter = ix * iy
    return inter / (aw * ah + bw * bh - inter) if inter else 0.0


def stage_extract():
    WORK.mkdir(parents=True, exist_ok=True)
    (WORK / "rectified").mkdir(exist_ok=True)
    # curation must survive re-extraction: carry labels and pins over by geometry
    old_by = {}
    if (WORK / "meta.csv").exists():
        for r in load_meta():
            old_by.setdefault((r["photo"], r["role"]), []).append(r)
        (WORK / "meta.csv").rename(WORK / "meta.prev.csv")
    glyphs_dir = WORK / "glyphs"
    if glyphs_dir.exists():
        for p in glyphs_dir.glob("*.png"):
            p.unlink()
    glyphs_dir.mkdir(exist_ok=True)

    rows, gid, carried = [], 0, 0
    photos = sorted(p for p in ORIGINALS.iterdir() if p.suffix.lower() in (".jpg", ".jpeg", ".png"))
    for photo in photos:
        warp = rectify(photo)
        if warp is None:
            continue
        cv2.imwrite(str(WORK / "rectified" / f"{photo.stem}.png"), warp)
        for role, ink in ROLES.items():
            mask = strip_lines(red_mask(warp) if ink == "red" else dark_mask(warp))
            for m, x, y, w, h in components(mask):
                if not is_big_digit(w, h, int(np.count_nonzero(m))):
                    continue
                cv2.imwrite(str(glyphs_dir / f"{gid:04d}.png"), glyph_rgba(warp, m, x, y, w, h, ink))
                row = {
                    "id": f"{gid:04d}",
                    "photo": photo.stem,
                    "row": y // CELL,
                    "col": x // CELL,
                    "role": role,
                    "x": x,
                    "y": y,
                    "w": w,
                    "h": h,
                    "label": "",
                    "conf": "",
                    "pin": "",
                }
                prev = old_by.get((photo.stem, role), [])
                match = max(
                    prev,
                    key=lambda o: iou((x, y, w, h), (int(o["x"]), int(o["y"]), int(o["w"]), int(o["h"]))),
                    default=None,
                )
                if match is not None and iou(
                    (x, y, w, h), (int(match["x"]), int(match["y"]), int(match["w"]), int(match["h"]))
                ) > 0.4:
                    row["label"] = match["label"]
                    row["pin"] = match.get("pin", "")
                    if match["label"]:
                        carried += 1
                rows.append(row)
                gid += 1
    save_meta(rows)
    by_role = {r: sum(1 for x in rows if x["role"] == r) for r in ROLES}
    print(f"{len(rows)} glyphs from {len(photos)} photos  {by_role}  (labels carried: {carried})")


def load_meta():
    with open(WORK / "meta.csv", newline="") as f:
        return list(csv.DictReader(f))


def save_meta(rows):
    with open(WORK / "meta.csv", "w", newline="") as f:
        wtr = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        wtr.writeheader()
        wtr.writerows(rows)


def tile(row, size=64, pad_bottom=18):
    rgba = cv2.imread(str(WORK / "glyphs" / f"{row['id']}.png"), cv2.IMREAD_UNCHANGED)
    mask = rgba[:, :, 3]
    color = (30, 30, 200) if row["role"] == "user" else (40, 40, 40)
    h, w = mask.shape
    scale = (size - 8) / max(h, w)
    m = cv2.resize(mask, (max(1, int(w * scale)), max(1, int(h * scale))))
    t = np.full((size + pad_bottom, size, 3), 255, np.uint8)
    y0, x0 = (size - m.shape[0]) // 2, (size - m.shape[1]) // 2
    t[y0 : y0 + m.shape[0], x0 : x0 + m.shape[1]][m > 0] = color
    cv2.putText(t, row["id"], (2, size + 13), cv2.FONT_HERSHEY_SIMPLEX, 0.42, (0, 0, 0), 1)
    return t


def montage(rows, path, cols=16):
    tiles = [tile(r) for r in rows]
    while len(tiles) % cols:
        tiles.append(np.full_like(tiles[0], 255))
    grid = np.vstack([np.hstack(tiles[i : i + cols]) for i in range(0, len(tiles), cols)])
    cv2.imwrite(str(path), grid)


def stage_sheet():
    rows = [r for r in load_meta() if r["label"] == ""]
    (WORK / "sheets").mkdir(exist_ok=True)
    per = 16 * 8
    for n, i in enumerate(range(0, len(rows), per)):
        montage(rows[i : i + per], WORK / "sheets" / f"unlabeled_{n:02d}.png")
    print(f"{(len(rows) + per - 1) // per} sheets for {len(rows)} unlabeled glyphs")


def stage_label(args):
    rows = load_meta()
    by_id = {r["id"]: r for r in rows}
    n = 0
    for a in args:
        gid, _, digit = a.partition("=")
        if gid in by_id and digit:
            by_id[gid]["label"] = digit
            n += 1
    save_meta(rows)
    done = sum(1 for r in rows if r["label"] not in ("",))
    print(f"applied {n}; {done}/{len(rows)} labeled")


def features(row):
    # centered, size-normalized 32x32 mask, raw pixels — plenty for 9 classes
    # (this cv2 wheel ships without HOGDescriptor; raw + rbf-SVM grades the same)
    rgba = cv2.imread(str(WORK / "glyphs" / f"{row['id']}.png"), cv2.IMREAD_UNCHANGED)
    mask = rgba[:, :, 3]
    h, w = mask.shape
    side = max(h, w)
    sq = np.zeros((side, side), np.uint8)
    sq[(side - h) // 2 : (side - h) // 2 + h, (side - w) // 2 : (side - w) // 2 + w] = mask
    return (cv2.resize(sq, (32, 32)).ravel() > 0).astype(np.float32)


def fit_svm(rows):
    from sklearn.svm import SVC

    train = [r for r in rows if r["label"] not in ("", "-1")]
    x = np.array([features(r) for r in train])
    y = np.array([int(r["label"]) for r in train])
    clf = SVC(kernel="rbf", probability=True, C=10).fit(x, y)
    return clf


def stage_train_predict():
    rows = load_meta()
    clf = fit_svm(rows)
    todo = [r for r in rows if r["label"] == ""]
    if todo:
        probs = clf.predict_proba(np.array([features(r) for r in todo]))
        for r, p in zip(todo, probs):
            r["label"] = str(clf.classes_[int(np.argmax(p))])
            r["conf"] = f"{float(np.max(p)):.2f}" if "conf" in r else r.get("conf", "")
    for r in rows:
        r.setdefault("conf", "")
    save_meta(rows)
    # per-class verification sheets, least confident first
    (WORK / "sheets").mkdir(exist_ok=True)
    for d in range(1, 10):
        cls = [r for r in rows if r["label"] == str(d)]
        cls.sort(key=lambda r: r.get("conf") or "1.0")
        if cls:
            montage(cls, WORK / "sheets" / f"class_{d}.png")
    counts = {d: sum(1 for r in rows if r["label"] == str(d)) for d in range(1, 10)}
    print(f"labeled all; per digit: {counts}")


def stage_emit(max_variants=10):
    rows = [r for r in load_meta() if r["label"] not in ("", "-1")]
    manifest = {"name": "grandpere", "digits": {"given": {}, "user": {}}}
    for role in ROLES:
        for d in range(1, 10):
            cand = [r for r in rows if r["role"] == role and r["label"] == str(d)]
            if not cand:
                continue
            # rank by typicality: pencil-mark residue fused above a digit inflates
            # its height, so distance from the class median height demotes it.
            # Curated pins always come first.
            med = float(np.median([int(r["h"]) for r in cand]))
            cand.sort(
                key=lambda r: (
                    0 if r.get("pin") else 1,
                    abs(int(r["h"]) - med) / med - 0.3 * float(r.get("conf") or 1.0),
                )
            )
            paths = []
            for r in cand[:max_variants]:
                rgba = cv2.imread(str(WORK / "glyphs" / f"{r['id']}.png"), cv2.IMREAD_UNCHANGED)
                h, w = rgba.shape[:2]
                side = int(max(h, w) * 1.15)
                sq = np.zeros((side, side, 4), np.uint8)
                y0, x0 = (side - h) // 2, (side - w) // 2
                sq[y0 : y0 + h, x0 : x0 + w] = rgba
                sq[:, :, 3] = cv2.GaussianBlur(sq[:, :, 3], (3, 3), 0)  # soften edges
                out = cv2.resize(sq, (GLYPH, GLYPH), interpolation=cv2.INTER_AREA)
                rel = f"digits/{role}/{d}_{r['id']}.png"
                dst = PACK / rel
                dst.parent.mkdir(parents=True, exist_ok=True)
                cv2.imwrite(str(dst), out)
                paths.append(rel)
            if paths:
                manifest["digits"][role][str(d)] = paths
    (PACK / "manifest.json").write_text(json.dumps(manifest, indent=1))
    total = sum(len(v) for role in manifest["digits"].values() for v in role.values())
    print(f"emitted {total} glyphs -> {PACK}/manifest.json")


def stage_pin(args):
    rows = load_meta()
    for r in rows:
        r.setdefault("pin", "")
    by_id = {r["id"]: r for r in rows}
    for a in args:
        if a.startswith("-"):
            gid = a[1:]
            if gid in by_id:
                by_id[gid]["pin"] = ""
        elif a in by_id:
            by_id[a]["pin"] = "1"
    save_meta(rows)
    print(f"pinned: {[r['id'] for r in rows if r.get('pin')]}")


def stage_review():
    """The curation matrix: tell me the ids — worst get `label <id>=-1`,
    keepers get `pin <id>` — then re-`emit`."""
    import json as _json
    import subprocess

    rows = load_meta()
    emitted = set()
    manifest = PACK / "manifest.json"
    if manifest.exists():
        man = _json.loads(manifest.read_text())
        for role in man["digits"].values():
            for paths in role.values():
                for p in paths:
                    emitted.add(pathlib.Path(p).stem.split("_", 1)[1])

    size, pad = 76, 20
    paper = (232, 242, 247)  # BGR cream, close to the game's paper

    def glyph_tile(r):
        rgba = cv2.imread(str(WORK / "glyphs" / f"{r['id']}.png"), cv2.IMREAD_UNCHANGED)
        h, w = rgba.shape[:2]
        s = (size - 10) / max(h, w)
        rgba = cv2.resize(rgba, (max(1, int(w * s)), max(1, int(h * s))))
        t = np.full((size + pad, size, 3), 255, np.uint8)
        t[:size] = paper
        y0, x0 = (size - rgba.shape[0]) // 2, (size - rgba.shape[1]) // 2
        a = rgba[:, :, 3:4].astype(np.float32) / 255
        region = t[y0 : y0 + rgba.shape[0], x0 : x0 + rgba.shape[1]]
        region[:] = (region * (1 - a) + rgba[:, :, :3] * a).astype(np.uint8)
        if r["id"] in emitted:
            cv2.rectangle(t, (0, 0), (size - 1, size - 1), (60, 160, 60), 2)
        if r.get("pin"):
            cv2.rectangle(t, (3, 3), (size - 4, size - 4), (200, 120, 30), 2)
        cv2.putText(t, r["id"], (14, size + 15), cv2.FONT_HERSHEY_SIMPLEX, 0.45, (0, 0, 0), 1)
        return t

    cols, blocks = 14, []
    blank = np.full((size + pad, size, 3), 255, np.uint8)
    for role in ROLES:
        for d in range(1, 10):
            cls = [r for r in rows if r["role"] == role and r["label"] == str(d)]
            if not cls:
                continue
            header = np.full((26, cols * size), 255, np.uint8)
            header = cv2.cvtColor(header, cv2.COLOR_GRAY2BGR)
            cv2.putText(header, f"{role} {d}  ({len(cls)})", (4, 19),
                        cv2.FONT_HERSHEY_SIMPLEX, 0.55, (0, 0, 0), 1)
            blocks.append(header)
            tiles = [glyph_tile(r) for r in cls]
            while len(tiles) % cols:
                tiles.append(blank.copy())
            for i in range(0, len(tiles), cols):
                blocks.append(np.hstack(tiles[i : i + cols]))
    out = WORK / "pack_review.png"
    cv2.imwrite(str(out), np.vstack(blocks))
    print(f"wrote {out}  (green = in the pack, blue = pinned)")
    subprocess.run(["open", str(out)], check=False)


if __name__ == "__main__":
    stage = sys.argv[1] if len(sys.argv) > 1 else ""
    if stage == "extract":
        stage_extract()
    elif stage == "sheet":
        stage_sheet()
    elif stage == "label":
        stage_label(sys.argv[2:])
    elif stage == "predict":
        stage_train_predict()
    elif stage == "emit":
        stage_emit()
    elif stage == "review":
        stage_review()
    elif stage == "pin":
        stage_pin(sys.argv[2:])
    else:
        sys.exit(__doc__)
