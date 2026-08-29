#!/usr/bin/env python3
"""Feasibility check on one photo: rectify the grid, split the inks, crop cells.

Usage: .venv/bin/python feasibility.py <photo.jpg>
Writes diagnostics into originals/feasibility/ (gitignored with the originals).
"""
import pathlib
import sys

import cv2
import numpy as np

OUT = pathlib.Path(__file__).resolve().parent.parent / "originals" / "feasibility"
SIDE = 1080  # rectified grid size


def red_mask(bgr):
    """Red ink: hue wraps around 0 in HSV."""
    hsv = cv2.cvtColor(bgr, cv2.COLOR_BGR2HSV)
    lo = cv2.inRange(hsv, (0, 50, 60), (12, 255, 255))
    hi = cv2.inRange(hsv, (168, 50, 60), (180, 255, 255))
    return lo | hi


def dark_mask(bgr):
    """Black/grey ink: low value, low saturation (excludes the red pen)."""
    hsv = cv2.cvtColor(bgr, cv2.COLOR_BGR2HSV)
    return cv2.inRange(hsv, (0, 0, 0), (180, 70, 110))


def find_grid_quad(mask):
    """Largest 4-ish contour in the red mask = the hand-ruled outer frame."""
    k = cv2.getStructuringElement(cv2.MORPH_RECT, (5, 5))
    closed = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, k, iterations=3)
    contours, _ = cv2.findContours(closed, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    best = max(contours, key=cv2.contourArea)
    peri = cv2.arcLength(best, True)
    approx = cv2.approxPolyDP(best, 0.02 * peri, True)
    if len(approx) != 4:  # fall back to min-area rectangle
        approx = cv2.boxPoints(cv2.minAreaRect(best)).astype(np.int32).reshape(-1, 1, 2)
    return approx.reshape(4, 2).astype(np.float32)


def order_quad(q):
    s, d = q.sum(axis=1), np.diff(q, axis=1).ravel()
    return np.float32([q[np.argmin(s)], q[np.argmin(d)], q[np.argmax(s)], q[np.argmax(d)]])


def strip_lines(mask):
    """Remove long horizontal/vertical strokes (the ruled lines), keep compact digit blobs."""
    n = SIDE // 12  # anything spanning more than a cell-ish length is a line
    horiz = cv2.morphologyEx(mask, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_RECT, (n, 1)))
    vert = cv2.morphologyEx(mask, cv2.MORPH_OPEN, cv2.getStructuringElement(cv2.MORPH_RECT, (1, n)))
    lines = cv2.dilate(horiz | vert, cv2.getStructuringElement(cv2.MORPH_RECT, (5, 5)))
    return mask & ~lines


def main():
    photo = pathlib.Path(sys.argv[1])
    OUT.mkdir(parents=True, exist_ok=True)
    bgr = cv2.imread(str(photo))
    bgr = cv2.rotate(bgr, cv2.ROTATE_90_COUNTERCLOCKWISE)  # notebook photographed sideways

    quad = order_quad(find_grid_quad(red_mask(bgr)))
    target = np.float32([[0, 0], [SIDE, 0], [SIDE, SIDE], [0, SIDE]])
    warp = cv2.warpPerspective(bgr, cv2.getPerspectiveTransform(quad, target), (SIDE, SIDE))
    cv2.imwrite(str(OUT / "1_rectified.png"), warp)

    red = red_mask(warp)
    dark = dark_mask(warp)
    red_ink = strip_lines(red)
    dark_ink = strip_lines(dark)
    cv2.imwrite(str(OUT / "2_red_raw.png"), red)
    cv2.imwrite(str(OUT / "3_red_digits.png"), red_ink)
    cv2.imwrite(str(OUT / "4_dark_digits.png"), dark_ink)

    # composite: red digits in red, dark digits in black, on white
    comp = np.full_like(warp, 255)
    comp[dark_ink > 0] = (40, 40, 40)
    comp[red_ink > 0] = (30, 30, 200)
    cv2.imwrite(str(OUT / "5_ink_split.png"), comp)

    # a strip of sample cells (row 4 of 9, all columns) at each stage
    c = SIDE // 9
    row = 3
    strip = []
    for col in range(9):
        y, x = row * c, col * c
        cell = warp[y : y + c, x : x + c]
        cell_comp = comp[y : y + c, x : x + c]
        strip.append(np.vstack([cell, cell_comp]))
    cv2.imwrite(str(OUT / "6_cells_row4.png"), np.hstack(strip))
    print(f"wrote {OUT}/1..6*.png")
    print(f"red ink px: {int(np.count_nonzero(red_ink))}, dark ink px: {int(np.count_nonzero(dark_ink))}")


if __name__ == "__main__":
    main()
