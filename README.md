# a-souvenir-of-sudokus

A sudoku game built engine-first. The engine is a pure library (`sudoku.py`)
with a JSON game-state contract; frontends are thin layers on top — the CLI
(`cli.py`) is the first, browser/desktop come later.

The final rendering will be built from photographs of the original handwritten
grids this game is named after.

## Play

```
python3 cli.py [easy|medium|hard] [seed]
```

Commands: `put r c v` · `del r c` · `hint` · `check` · `solve` · `save` · `load` · `new` · `quit`

## Test

```
python3 test_sudoku.py
```

## Roadmap

- [x] Python engine draft (generator with guaranteed-unique solutions, solver, game state)
- [x] CLI frontend
- [ ] C++ engine rewrite once the design settles
- [ ] Renderer(s) — browser first, from the handwritten-grid photographs
