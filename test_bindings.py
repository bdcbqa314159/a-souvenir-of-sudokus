"""Checks for the compiled souvenir module and the frontends on top of it.

Requires the bindings built:
  cmake --preset release -S engine -DSOUVENIR_BUILD_PYTHON=ON && cmake --build --preset release engine
"""
import json

import souvenir
import sudoku  # the retired Python draft — kept as the spec and cross-check


def main() -> None:
    # generation contract, via C++
    puzzle, solution = souvenir.generate("medium", seed=42)
    assert souvenir.count_solutions(puzzle) == 1
    assert souvenir.solve(puzzle) == solution
    assert souvenir.generate("medium", seed=7) == souvenir.generate("medium", seed=7)
    try:
        souvenir.generate("expert")
        raise AssertionError("unknown difficulty must fail")
    except ValueError:
        pass

    # module constants agree with the Python spec
    assert souvenir.PEERS == sudoku.PEERS
    assert souvenir.CLUE_TARGET == sudoku.CLUE_TARGET

    # game flow mirrors sudoku.Game
    g = souvenir.Game.new("easy", 3)
    empty = next(i for i in range(81) if not g.is_given(i))
    given = next(i for i in range(81) if g.is_given(i))
    try:
        g.put(given, 5)
        raise AssertionError("put on given must fail")
    except ValueError:
        pass
    g.toggle_mark(empty, 7)
    g.toggle_mark(empty, 3)
    g.toggle_mark(empty, 7)
    assert g.marks[empty] == {3}
    g.clear_marks(empty)
    assert g.marks[empty] == set()

    # undo-style snapshot/restore through the board/marks properties
    g.toggle_mark(empty, 5)
    board, marks = g.board[:], [set(m) for m in g.marks]
    g.put(empty, g.solution[empty])
    g.board, g.marks = board, marks
    assert g.board[empty] == 0 and g.marks[empty] == {5}

    while g.hint(seed=1) is not None:
        pass
    assert g.is_solved() and not g.wrong_cells()

    # save files interchange between the two engines, both directions
    cpp = souvenir.Game.new("hard", 11)
    cpp.toggle_mark(next(i for i in range(81) if not cpp.is_given(i)), 9)
    py = sudoku.Game.from_json(cpp.to_json())
    assert py.board == cpp.board and [sorted(m) for m in py.marks] == [sorted(m) for m in cpp.marks]
    back = souvenir.Game.from_json(py.to_json())
    assert back.board == cpp.board and back.difficulty == "hard"
    try:
        souvenir.Game.from_json(json.dumps({}))
        raise AssertionError("bad save must fail")
    except ValueError:
        pass
    wrapped = json.loads(cpp.to_json())
    wrapped["board"][0] = 2**32 + 5  # would wrap to 5 if narrowed before range check
    for engine in (souvenir, sudoku):
        try:
            engine.Game.from_json(json.dumps(wrapped))
            raise AssertionError("wrapped 64-bit value must fail in both engines")
        except ValueError:
            pass
    try:
        cpp.is_given(-1)
        raise AssertionError("out-of-range is_given must raise, not crash")
    except ValueError:
        pass

    # frontends import against the compiled module
    from cli import idx

    assert idx("1", "1") == 0 and idx("9", "9") == 80
    for r, c in [("0", "1"), ("1", "0"), ("10", "1"), ("1", "-1")]:
        try:
            idx(r, c)
            raise AssertionError("out-of-range coordinate must fail")
        except ValueError:
            pass

    from tui import conflicts

    b = [0] * 81
    b[0] = b[8] = 5
    assert conflicts(b) == {0, 8}
    b[8] = 6
    assert conflicts(b) == set()

    print("all binding checks passed")


if __name__ == "__main__":
    main()
