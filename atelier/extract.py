"""Photos -> rectified grids -> ink split -> glyph crops + meta.csv."""
import numpy as np
import cv2

from config import CELL, ORIGINALS, ROLES, WORK
from curation import load_meta, save_meta
from imaging import components, dark_mask, glyph_rgba, iou, is_big_digit, red_mask, rectify, strip_lines

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
            comps = list(components(mask))
            bigs = [c for c in comps if is_big_digit(c[3], c[4], int(np.count_nonzero(c[0])))]
            # rescue detached top bars (his 5s): a small wide pen stroke hovering
            # just above a digit belongs to it — extraction used to drop it as
            # pencil-mark-sized, leaving bar-less fives
            for m_s, x_s, y_s, w_s, h_s in comps:
                if is_big_digit(w_s, h_s, int(np.count_nonzero(m_s))) or w_s <= h_s or w_s < 0.2 * CELL:
                    continue
                for i, (m, x, y, w, h) in enumerate(bigs):
                    gap = y - (y_s + h_s)
                    if -0.1 * CELL < gap < 0.15 * CELL and x - 0.1 * CELL < x_s + w_s / 2 < x + w + 0.1 * CELL:
                        m = m | m_s
                        x0, y0 = min(x, x_s), min(y, y_s)
                        bigs[i] = (m, x0, y0, max(x + w, x_s + w_s) - x0, max(y + h, y_s + h_s) - y0)
                        break
            for m, x, y, w, h in bigs:
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


