# legacy — the Python draft

This project was prototyped in Python before the C++ engine existed. These
files are kept as **executable history**, not as part of the shipped game:

- `sudoku.py` — the original engine (generator, solver, game state). The C++
  engine in `engine/` implements the same API and the same JSON save
  contract; this file is the spec it was written against.
- `test_sudoku.py` — that engine's test suite. Still run in CI, because it
  pins the behaviour the C++ engine promises to match.
- `cli.py`, `tui.py` — the first two frontends (REPL and a vim-style curses
  grid). Superseded by the web/desktop app; they still run on the compiled
  `souvenir` module when the pybind11 bindings are built.

Nothing here is used by the browser game, the desktop app, or the asset
pipeline. `test_bindings.py` stays at the repo root: it tests the *current*
engine through its Python bindings.

```
cd legacy
python3 test_sudoku.py                      # the spec suite
python3 tui.py [easy|medium|hard] [seed]    # needs the built bindings
```
