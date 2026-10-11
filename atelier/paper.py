"""The game's background: the real page tile, and the synthetic one."""
import json
import sys

import cv2
import numpy as np

from config import PACK, PAPER_CREAM, PAPER_FADE, STAMP, WORK
from imaging import dark_mask, red_mask
from pack import pack_rev, write_manifest
from provenance import save_png

def blank_page_tile(bgr, out_w=1080):
    """A blank quad-ruled page photo -> seamless background tile.

    The web app tiles paper.jpg (background-size 540px), so the tile must
    repeat invisibly. Three defects in a raw photo break that: the lighting
    gradient, the perspective/lens skew of the ruling, and a crop that cuts
    mid-square. So: flatten lighting, track every ruling line's (slightly
    curved) path and remap each onto an exact grid, crop whole squares,
    then tone the paper base to the UI cream and fade the ruling.
    ponytail: peak-finding constants assume ~4300px close-up photos with
    ~230px ruling; re-tune if the source photos change scale."""
    from scipy.signal import find_peaks

    h, w = bgr.shape[:2]
    s = min(h, w)
    crop = bgr[(h - s) // 2 : (h + s) // 2, (w - s) // 2 : (w + s) // 2].astype(np.float32)
    base = cv2.GaussianBlur(crop, (0, 0), 80)
    flat = np.clip(crop / base * base.mean(axis=(0, 1)), 0, 255)
    g = cv2.cvtColor(flat.astype(np.uint8), cv2.COLOR_BGR2GRAY).astype(np.float32)

    def peaks_of(prof):
        prof = -np.convolve(prof, np.ones(9) / 9, mode="same")
        p, _ = find_peaks(prof, distance=150, prominence=prof.std() * 0.5)
        return p

    def track(axis):
        # fit each line's path across 10 strips: vertical lines as x(y), deg-2
        # to capture the lens bow (a single homography left interior lines soft)
        ref = peaks_of(g.mean(axis=axis))
        size = s // 10
        fits = []
        for r in ref:
            pts = []
            for i in range(10):
                strip = g[i * size : (i + 1) * size] if axis == 0 else g[:, i * size : (i + 1) * size]
                pk = peaks_of(strip.mean(axis=axis))
                if len(pk):
                    near = pk[np.argmin(np.abs(pk - r))]
                    if abs(near - r) < 60:
                        pts.append((i * size + size / 2, near))
            if len(pts) >= 4:
                t, v = np.array(pts).T
                fits.append(np.poly1d(np.polyfit(t, v, 2)))
        return fits, ref

    vfits, vref = track(0)
    hfits, _ = track(1)
    nv, nh = len(vfits), len(hfits)
    if nv < 4 or nh < 4:
        sys.exit(f"could not track the ruling ({nv}x{nh} lines) — is the page quad-ruled and frame-filling?")
    per = float(np.median(np.diff(vref)))
    dw, dh = int((nv - 1) * per), int((nh - 1) * per)
    knots_x, knots_y = np.arange(nv) * per, np.arange(nh) * per
    cx = np.array([f(s / 2) for f in vfits])
    cy = np.array([f(s / 2) for f in hfits])
    xs, ys = np.arange(dw, dtype=np.float32), np.arange(dh, dtype=np.float32)
    x_in0, y_in0 = np.interp(xs, knots_x, cx), np.interp(ys, knots_y, cy)
    map_x = np.empty((dh, dw), np.float32)
    map_y = np.empty((dh, dw), np.float32)
    for j in range(dh):
        map_x[j] = np.interp(xs, knots_x, [f(y_in0[j]) for f in vfits])
    h_mat = np.array([f(x_in0) for f in hfits])
    for i in range(dw):
        map_y[:, i] = np.interp(ys, knots_y, h_mat[:, i])
    tile = cv2.remap(flat, map_x, map_y, cv2.INTER_CUBIC)

    cream = np.float32(PAPER_CREAM)
    tile = tile * (cream / np.median(tile.reshape(-1, 3), axis=0))
    tile = np.clip(tile * (1 - PAPER_FADE) + cream * PAPER_FADE, 0, 255)
    # the white-balance leaves the ruling minty; tilt line pixels back to blue
    dark = np.clip((cream.sum() - tile.sum(axis=2)) / 60.0, 0, 1)[..., None]
    tile = np.clip(tile * (1 + (np.float32([1.06, 1.0, 0.94]) - 1) * dark), 0, 255).astype(np.uint8)
    return cv2.resize(tile, (out_w, round(out_w * dh / dw)), interpolation=cv2.INTER_AREA)


def stage_paper(stem=None):
    rect = sorted((WORK / "rectified").glob("*.png"))
    photo = None
    if stem:
        # a blank-page photo wins over any rectified file of the same stem:
        # a stale bogus warp in rectified/ once shadowed IMG_7807 and shipped
        # inpainted wood-table blur as the pack paper
        # blank pages live in originals/blank/ so extract never scans them
        # (the wood table under a blank page can pass the red grid detector)
        cand = sorted(ORIGINALS.glob("blank/*")) + sorted(ORIGINALS.iterdir())
        photo = next(
            (p for p in cand if p.stem.startswith(stem) and p.suffix.lower() in (".jpg", ".jpeg", ".png")),
            None,
        )
    src = None
    if photo is None:
        src = next((p for p in rect if stem and p.stem.startswith(stem)), None)
    if src is None and photo is None:
        src = rect[0] if rect else None
        if src is None:
            sys.exit("run extract first, or drop a blank-page photo in originals/")
    if photo is not None:
        clean = blank_page_tile(cv2.imread(str(photo)))
        name = photo.stem
    else:
        bgr = cv2.imread(str(src))
        # ink away, paper stays: the faint quad ruling survives both masks, so the
        # inpainted page keeps its graph-paper soul
        mask = cv2.dilate(red_mask(bgr) | dark_mask(bgr), np.ones((7, 7), np.uint8))
        clean = cv2.medianBlur(cv2.inpaint(bgr, mask, 5, cv2.INPAINT_TELEA), 3)
        name = src.stem
    # PNG, not JPEG: the watermark lives in pixel LSBs and needs lossless.
    # The real-page tile is family-pack material now — the public pack's
    # paper comes from `genpaper` (fully synthetic).
    save_png(WORK / "paper_real.png", clean)
    print(f"paper from {name} -> {WORK}/paper_real.png (re-run emit to place it in the abuelo pack)")


def stage_genpaper(seed=2026):
    """Fully synthetic quad-ruled paper for the public pack: parameters
    (period, tones, grain) measured from his real page, pixels 100% drawn.
    Wrap-around wobble keeps the tile seamless under the web's 540px tiling.
    Tones tuned by eye 2026-09-24: cooler cream, bluer stronger ruling."""
    H = W = 1080
    rng = np.random.default_rng(seed)
    base = np.float32([238, 243, 245])
    line_col = np.float32([238, 210, 178])
    period, grain = 63.5, 3.73

    paper = np.ones((H, W, 3), np.float32) * base
    mottle = cv2.GaussianBlur(rng.standard_normal((H, W)).astype(np.float32), (0, 0), 45)
    mottle /= np.abs(mottle).max() + 1e-6
    paper *= 1 + 0.012 * mottle[..., None]
    g = cv2.GaussianBlur(rng.standard_normal((H, W)).astype(np.float32), (0, 0), 0.7)
    paper += g[..., None] * grain * 0.9

    n = round(H / period)
    step = H / n
    yy, xx = np.meshgrid(np.arange(H, dtype=np.float32), np.arange(W, dtype=np.float32), indexing="ij")
    ink = np.zeros((H, W), np.float32)

    def wobble(amp):
        k = int(rng.integers(2, 5))
        ph = rng.uniform(0, 2 * np.pi, k)
        a = rng.uniform(0.3, 1.0, k)
        a *= amp / a.sum()
        t = np.arange(H) * 2 * np.pi / H
        return sum(ai * np.sin((i + 1) * t + p) for i, (ai, p) in enumerate(zip(a, ph))).astype(np.float32)

    for i in range(n):
        c = i * step + step / 2
        for axis in (0, 1):
            w = wobble(rng.uniform(1.2, 2.6))
            width = rng.uniform(1.5, 2.2)
            strength = rng.uniform(0.75, 1.0)
            coord = (yy - c - w[None, :]) if axis == 0 else (xx - c - w[:, None])
            d = np.abs(((coord + H / 2) % H) - H / 2)
            ink += strength * np.exp(-((d / width) ** 2))
    pv = cv2.GaussianBlur(rng.standard_normal((H, W)).astype(np.float32), (0, 0), 25)
    ink = np.clip(ink * np.clip(1 + 0.35 * pv, 0.4, 1.6), 0, 1)[..., None]
    out = np.clip(paper * (1 - ink * 0.9) + line_col * ink * 0.9, 0, 255).astype(np.uint8)
    PACK.mkdir(parents=True, exist_ok=True)
    save_png(PACK / "paper.png", out)
    # a new paper must bump the rev or returning browsers keep the cached
    # old one forever (audit finding: this was the 'hard refresh' gotcha)
    mp = PACK / "manifest.json"
    if mp.exists():
        m = json.loads(mp.read_text())
        m["paper"] = "paper.png"
        m["copyright"] = STAMP
        pack_rev(PACK, m)
        write_manifest(PACK, m)
        print(f"synthetic paper -> {PACK}/paper.png (stamped + watermarked, rev {m['rev']})")
    else:
        print(f"synthetic paper -> {PACK}/paper.png (no manifest yet — run synth to create it)")


