"""cv2 primitives: ink masks, page rectification, component triage."""
import pathlib

import cv2
import numpy as np

from config import CELL, GLYPH, ROLES, SIDE

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


def count_holes(mask_bool):
    """Enclosed background regions (loop interiors): background components
    that never touch the canvas border."""
    inv = (~mask_bool).astype(np.uint8)
    n, labels = cv2.connectedComponents(inv)
    border = set(np.unique(np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])))
    return sum(1 for lab in range(1, n) if lab not in border and (labels == lab).sum() >= 12)
