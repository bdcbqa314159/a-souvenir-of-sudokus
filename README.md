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

### Browser (Rust · Leptos → wasm)

```
source .emsdk/emsdk_env.sh
emcmake cmake -S engine -B engine/build/wasm -DCMAKE_BUILD_TYPE=Release
cmake --build engine/build/wasm -j
cd web && trunk serve --open
```

Same keys as the TUI (hjkl/arrows, 1-9, m pencil, x clear, u/r undo/redo,
H hint, c check, n new) plus mouse. Digits render from the asset pack in
`web/assets/` — the placeholder pack today; `assets/grandpere/`, cut from the
original handwritten grids, at the end.

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

### Browser engine (wasm)

The whole engine compiles to WebAssembly behind one function —
`souvenir_cmd(requestJson) → responseJson` (command set in
`engine/include/souvenir/api.hpp`). Emscripten is pinned locally in the
gitignored `.emsdk/` (same version as CI):

```
git clone --depth 1 https://github.com/emscripten-core/emsdk.git .emsdk
./.emsdk/emsdk install 3.1.64 && ./.emsdk/emsdk activate 3.1.64
source .emsdk/emsdk_env.sh

emcmake cmake -S engine -B engine/build/wasm -DCMAKE_BUILD_TYPE=Release
cmake --build engine/build/wasm -j
node test_wasm.mjs
```

## Roadmap

- [x] Python engine draft (generator with guaranteed-unique solutions, solver, game state)
- [x] CLI frontend (REPL + vim-style curses TUI)
- [x] C++ engine (`engine/` — library + tests)
- [x] Technique-based difficulty grading (the "GM pass": generate → grade → regenerate until the label is true)
- [x] pybind11 bindings; frontends run on the compiled engine (local build; packaging when the project is final)
- [x] Engine in the browser: wasm build with the JSON command surface
- [x] Browser frontend: Rust (Leptos → wasm), asset-pack driven, placeholder pack first
- [x] Button-bar UX: handwritten digit palette, grouped rows, segmented difficulty
- [x] Phantom mode core: stall clock (difficulty-scaled), flips via `phantom_of`, 3 lives
- [x] Phantom grace window (correct placement wards off the flip)
- [x] Phantom overlay: incoming givens fade in, flip becomes a crossfade
- [x] Game rules: hints/checks capped (3 each), none in phantom, deliberate phantom exit, `?dev` uncaps
- [x] `atelier/`: photos of the handwritten grids → the `grandpere` asset pack (originals and pack stay out of git)
- [x] Renderer finale — open the game with `?pack=grandpere` and it is written in his hand
