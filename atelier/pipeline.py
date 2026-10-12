#!/usr/bin/env python3
"""atelier — photos of the handwritten grids -> the souvenir asset pack.

Stages (run in order; every output lands in originals/atelier-work/, gitignored):

  extract   all photos -> rectified grids -> ink split -> glyph crops + meta.csv
  sheet     contact sheets of unlabeled glyphs (for eyeball labeling)
  train     fit HOG+SVM on the labeled rows of meta.csv
  predict   label the rest with the SVM; write per-class sheets for verification
  emit      normalized RGBA glyphs -> web/assets/souvenir/ + manifest.json
  review    open a curation matrix: every accepted glyph per class, indexed by
            id; emitted variants outlined green, pinned ones blue
  paper     pack paper.jpg (the game's background becomes his actual notebook
            paper). Optional arg: photo stem prefix. A rectified puzzle page
            gets its ink inpainted away; a stem matching only a raw photo in
            originals/ is taken as a blank quad-ruled page and turned into a
            seamless tile: lighting flattened, every ruling line remapped to
            an exact grid, whole squares cropped, base toned to the UI cream,
            ruling faded to PAPER_FADE (sharpest result — photograph the
            empty notebook frame-filling and straight-on for this).
  pin       pipeline.py pin 0012 0034 ...  (unpin: pin -0012) — pinned glyphs
            always emit first
  verify    pipeline.py verify <asset.png> — check the invisible provenance
            watermark (needs the secret wm.key from atelier-work)

Labels live in meta.csv (column `label`, 1-9, empty = unlabeled; -1 = rejected).
Apply labels with:  pipeline.py label <id>=<digit> <id>=<digit> ...
"""
import sys

# the stages live in modules now; pipeline.py is the CLI that dispatches to
# them, and re-exports the names tools and tests already import from here
from config import *  # noqa: F401,F403
from curation import *  # noqa: F401,F403
from extract import *  # noqa: F401,F403
from imaging import *  # noqa: F401,F403
from pack import *  # noqa: F401,F403
from paper import *  # noqa: F401,F403
from provenance import *  # noqa: F401,F403
from synth import *  # noqa: F401,F403

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
    elif stage == "synth":
        stage_synth(targets=sys.argv[2:] or None)
    elif stage == "verify":
        stage_verify(sys.argv[2] if len(sys.argv) > 2 else sys.exit("usage: pipeline.py verify <asset.png>"))
    elif stage == "genkey":
        stage_genkey()
    elif stage == "genpaper":
        stage_genpaper()
    elif stage == "paper":
        stage_paper(sys.argv[2] if len(sys.argv) > 2 else None)
    else:
        sys.exit(__doc__)
