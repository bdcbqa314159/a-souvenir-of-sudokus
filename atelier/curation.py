"""meta.csv and the human loop: labels, pins, contact sheets, the SVM that
pre-labels, and the review matrix."""
import csv
import subprocess
import sys

import cv2
import numpy as np

from config import PACK, ROLES, WORK

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


