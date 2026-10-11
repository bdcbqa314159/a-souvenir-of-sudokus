"""Writing a pack: ink harmonisation, spritesheet, content revision,
manifest, and the real-scan emit."""
import json
import sys

import cv2
import numpy as np

from config import ABUELO, GLYPH, PACK, ROLES, STAMP, WORK
from curation import load_meta
from provenance import save_png

INK_TARGET = {"given": (0x1A, 0x1A, 0x1A), "user": (0x1D, 0x27, 0xB5)}  # BGR --ink / --red
INK_BLEND = {"given": 0.5, "user": 0.75}  # 0 = photo color, 1 = flat UI ink


def harmonize_ink(rgba, role):
    """Pull glyph color toward the UI ink so digits read consistently on the
    pack paper (the raw photo reds ranged washed-pink to maroon), keeping the
    per-pixel stroke variation as a brightness ripple so they stay handwritten."""
    ink = np.float32(INK_TARGET[role])
    rgb = rgba[:, :, :3].astype(np.float32)
    gray = cv2.cvtColor(rgb.astype(np.uint8), cv2.COLOR_BGR2GRAY).astype(np.float32)
    ripple = (gray[..., None] - gray.mean()) * 0.35
    k = INK_BLEND[role]
    out = np.clip(rgb * (1 - k) + (ink + ripple) * k, 0, 255)
    # equal darkness for every glyph: gain each glyph's stroke luminance onto
    # the target ink's, so no digit reads lighter than its neighbours
    core = rgba[:, :, 3] > 128
    if core.any():
        lum = np.float32([0.114, 0.587, 0.299])
        gain = float(np.dot(ink, lum) / max(out[core].mean(axis=0) @ lum, 1.0))
        out = out * np.clip(gain, 0.6, 1.4)
    rgba[:, :, :3] = np.clip(out, 0, 255).astype(np.uint8)
    # same intensity for every glyph: faint pen pressure varied per photo, so
    # scale each glyph's alpha until its stroke core hits a common strength
    a = rgba[:, :, 3]
    if (a > 0).any():
        core = float(np.percentile(a[a > 0], 90))
        if core > 0:
            af = np.clip(a.astype(np.float32) * (245.0 / core), 0, 255) / 255.0
            # gamma < 1 solidifies the stroke body ("pen a bit clearer"):
            # mid-alpha mush becomes ink while the soft edge tail survives
            rgba[:, :, 3] = (np.power(af, 0.55) * 255).astype(np.uint8)
    return rgba


def write_sprite(pack_dir, manifest):
    """All glyphs composed into one spritesheet: the board paints from ONE
    request instead of 180 (the per-file pack stays, as fallback and for
    anything that wants single glyphs). Indexed by the same rel paths the
    digits lists use; stamped + watermarked like any asset."""
    refs = [p for role in manifest["digits"].values() for v in role.values() for p in v]
    if not refs:
        return
    cols = 15
    rows = (len(refs) + cols - 1) // cols
    sheet = np.zeros((rows * GLYPH, cols * GLYPH, 4), np.uint8)
    index = {}
    for i, rel in enumerate(refs):
        img = cv2.imread(str(pack_dir / rel), cv2.IMREAD_UNCHANGED)
        if img is None or img.shape != (GLYPH, GLYPH, 4):
            sys.exit(
                f"sprite: missing or malformed glyph '{rel}' — the pack is inconsistent "
                "(a frozen manifest row references a file that is gone; run a full synth/emit)"
            )
        r, c = divmod(i, cols)
        sheet[r * GLYPH : (r + 1) * GLYPH, c * GLYPH : (c + 1) * GLYPH] = img
        index[rel] = i
    save_png(pack_dir / "sprite.png", sheet)
    manifest["sprite"] = {"file": "sprite.png", "cols": cols, "rows": rows, "index": index}


def pack_rev(pack_dir, manifest):
    """Content hash over every referenced asset -> manifest["rev"]. The web
    appends ?v=<rev> to asset URLs, so browsers drop stale cached glyphs the
    moment a regenerated pack deploys (filenames alone don't change)."""
    import hashlib

    h = hashlib.sha256()
    refs = sorted(p for role in manifest["digits"].values() for v in role.values() for p in v)
    if manifest.get("paper"):
        refs.append(manifest["paper"])
    if manifest.get("sprite"):
        refs.append(manifest["sprite"]["file"])
    for rel in refs:
        data = (pack_dir / rel).read_bytes()
        # path + length delimiters: concatenation alone is boundary-ambiguous
        h.update(rel.encode())
        h.update(len(data).to_bytes(8, "big"))
        h.update(data)
    manifest["rev"] = h.hexdigest()[:10]


def write_manifest(pack_dir, manifest):
    """Atomic manifest write (tmp + rename): a crash between asset writes and
    the manifest left packs serving a stale sprite + stale rev, silently."""
    import os

    p = pack_dir / "manifest.json"
    tmp = p.with_suffix(".json.tmp")
    tmp.write_text(json.dumps(manifest, indent=1))
    os.replace(tmp, p)


def stage_emit(max_variants=10):
    """Real-scan family pack -> web/assets/abuelo/ (gitignored; NEVER commit).
    The public repo carries only the generated pack (synth/genpaper -> PACK)."""
    rows = [r for r in load_meta() if r["label"] not in ("", "-1")]
    manifest = {"name": "abuelo", "digits": {"given": {}, "user": {}}}
    # start clean: stale glyphs from earlier emits otherwise linger in the pack
    # (and break the Tauri bundle when a cached build references a gone file)
    for old in ABUELO.glob("digits/*/*.png"):
        old.unlink()
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
                rgba = harmonize_ink(rgba, role)
                h, w = rgba.shape[:2]
                side = int(max(h, w) * 1.15)
                sq = np.zeros((side, side, 4), np.uint8)
                y0, x0 = (side - h) // 2, (side - w) // 2
                sq[y0 : y0 + h, x0 : x0 + w] = rgba
                # no second blur: glyph_rgba already softened once, and the
                # extra pass is what made the pen read washed instead of inked
                out = cv2.resize(sq, (GLYPH, GLYPH), interpolation=cv2.INTER_AREA)
                rel = f"digits/{role}/{d}_{r['id']}.png"
                dst = ABUELO / rel
                dst.parent.mkdir(parents=True, exist_ok=True)
                save_png(dst, out)
                paths.append(rel)
            if paths:
                manifest["digits"][role][str(d)] = paths
    # the family pack keeps his real notebook page as paper
    real_paper = WORK / "paper_real.png"
    if real_paper.exists():
        import shutil

        shutil.copy(real_paper, ABUELO / "paper.png")
        manifest["paper"] = "paper.png"
    manifest["copyright"] = STAMP
    write_sprite(ABUELO, manifest)
    pack_rev(ABUELO, manifest)
    write_manifest(ABUELO, manifest)
    total = sum(len(v) for role in manifest["digits"].values() for v in role.values())
    print(f"emitted {total} glyphs -> {ABUELO}/manifest.json")


