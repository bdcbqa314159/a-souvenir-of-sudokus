# a-souvenir-of-sudokus

A sudoku game built engine-first. The engine is a pure library (`sudoku.py`)
with a JSON game-state contract; frontends are thin layers on top — the CLI
(`cli.py`) is the first, browser/desktop come later.

The final rendering will be built from photographs of the original handwritten
grids this game is named after.

## Play

Build the engine module once, then play:

```
cmake --preset release -S engine -DSOUVENIR_BUILD_PYTHON=ON
cmake --build --preset release engine

python3 tui.py [easy|medium|hard] [seed]   # vim-style grid: hjkl move, 1-9 put, x clear, u undo
python3 cli.py [easy|medium|hard] [seed]   # line-mode REPL
```

REPL commands: `put r c v` · `del r c` · `hint` · `check` · `solve` · `save` · `load` · `new` · `quit`

## Test

```
python3 test_sudoku.py     # the Python spec engine
python3 test_bindings.py   # compiled module + frontends + cross-engine save files
```

## C++ engine (`engine/`)

The backend: same API and JSON save-file contract as the Python draft
(`sudoku.py`, retired to executable spec), C++20, no runtime dependencies
(nlohmann/json vendored). Consumable via CMake `FetchContent` as
`souvenir::souvenir`. pybind11 bindings (`-DSOUVENIR_BUILD_PYTHON=ON`) build
the `souvenir` Python module the frontends run on — local build only for now.

```
cmake --preset release -S engine
cmake --build --preset release
ctest --preset release
```

Presets: `debug` · `release` · `asan` (non-Windows).

## Roadmap

- [x] Python engine draft (generator with guaranteed-unique solutions, solver, game state)
- [x] CLI frontend (REPL + vim-style curses TUI)
- [x] C++ engine (`engine/` — library + tests)
- [x] pybind11 bindings; frontends run on the compiled engine (local build; packaging when the project is final)
- [ ] Renderer(s) — browser first, from the handwritten-grid photographs
