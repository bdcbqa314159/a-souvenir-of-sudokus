# atelier

Photos of the original handwritten grids → the `grandpere` asset pack.

Everything personal stays out of git: `originals/` (the photos and all
intermediate work) and `web/assets/grandpere/` (the derived glyphs) are
gitignored; publishing the pack is a deliberate decision, not a default.

## Setup

```
python3 -m venv .venv && .venv/bin/pip install -r requirements.txt
```

## Pipeline

```
.venv/bin/python pipeline.py extract   # photos -> rectified grids -> glyph crops + meta.csv
.venv/bin/python pipeline.py sheet     # contact sheets for eyeball labeling
.venv/bin/python pipeline.py label 0012=6 0013=1 0017=-1 ...   # 1-9, -1 = reject
.venv/bin/python pipeline.py predict   # HOG-less SVM labels the rest; per-class sheets to verify
.venv/bin/python pipeline.py emit      # RGBA glyphs + manifest -> web/assets/grandpere/
```

Stages in detail:

- **extract** — rotate, find the red hand-ruled frame, perspective-rectify to
  1080², split red/black ink in HSV, strip ruled lines morphologically, then
  triage connected components by shape: big pen digits kept, pencil marks and
  line remnants dropped. Crops keep the real ink as RGBA (color + mask alpha).
- **label / predict** — label a seed batch from the contact sheets, let an SVM
  (raw 32×32 mask pixels) label the rest, then verify per-class sheets sorted
  least-confident-first and correct with more `label` calls.
- **emit** — per (role, digit): rank by typicality (distance from the class's
  median height demotes glyphs with pencil-mark residue fused on), take up to
  10 variants, pad square, soften the alpha, write 96×96 PNGs + manifest.json
  in the exact schema of `web/assets/placeholder/`.

Play in his handwriting: `trunk serve` then open with `?pack=grandpere`.
