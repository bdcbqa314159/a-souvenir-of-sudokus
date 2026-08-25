"""Self-check: python3 test_sudoku.py"""
import random

from sudoku import PEERS, Game, count_solutions, generate, solve


def main() -> None:
    # solver produces a valid full grid
    full = solve([0] * 81, random.Random(1))
    assert full is not None and 0 not in full
    for unit in [[r * 9 + c for c in range(9)] for r in range(9)]:
        assert sorted(full[i] for i in unit) == list(range(1, 10))

    # generation: unique solution, clue target respected, reproducible by seed
    for diff, target in [("easy", 40), ("medium", 32), ("hard", 26)]:
        puzzle, sol = generate(diff, seed=42)
        assert count_solutions(puzzle) == 1
        assert sum(1 for v in puzzle if v) >= target
        assert solve(puzzle) == sol
    assert generate("medium", seed=7) == generate("medium", seed=7)

    # game flow: put/given/hint/JSON round-trip
    g = Game.new("easy", seed=3)
    i = next(j for j in range(81) if not g.is_given(j))
    g.put(i, g.solution[i])
    given = next(j for j in range(81) if g.is_given(j))
    try:
        g.put(given, 5)
        raise AssertionError("put on given must fail")
    except ValueError:
        pass
    # pencil marks: toggle, prune on put, given/filled cells rejected
    empties = [j for j in range(81) if g.board[j] == 0]
    a = empties[0]
    g.toggle_mark(a, 7)
    g.toggle_mark(a, 3)
    g.toggle_mark(a, 7)
    assert g.marks[a] == {3}
    peer = next(j for j in empties[1:] if a in PEERS[j])
    g.marks[peer] = {3, 5}
    g.put(a, 3)
    assert g.marks[a] == set() and g.marks[peer] == {5}
    g.put(a, 0)
    try:
        g.toggle_mark(next(j for j in range(81) if g.is_given(j)), 1)
        raise AssertionError("mark on given must fail")
    except ValueError:
        pass

    g.marks[a] = {2, 9}
    g2 = Game.from_json(g.to_json())
    assert g2.board == g.board and g2.puzzle == g.puzzle and g2.marks == g.marks
    while g.hint() is not None:
        pass
    assert g.is_solved()

    # inconsistent boards are rejected, not solved/hung
    bad = [0] * 81
    bad[0] = bad[1] = 5
    assert solve(bad) is None and count_solutions(bad) == 0

    # bad difficulty rejected
    try:
        generate("expert")
        raise AssertionError("unknown difficulty must fail")
    except ValueError:
        pass

    # malformed save files rejected
    import json

    for s in ["{}", json.dumps({"puzzle": [0] * 80, "solution": [1] * 81, "board": [0] * 81})]:
        try:
            Game.from_json(s)
            raise AssertionError("bad save must fail")
        except ValueError:
            pass
    d = json.loads(g.to_json())
    d["puzzle"][next(i for i in range(81) if d["puzzle"][i])] = 0  # still fine: fewer givens
    Game.from_json(json.dumps(d))
    d["puzzle"] = [10 - v if v else 0 for v in d["solution"]]  # puzzle contradicts solution
    try:
        Game.from_json(json.dumps(d))
        raise AssertionError("puzzle/solution mismatch must fail")
    except ValueError:
        pass

    # cli coordinate parsing bounds
    from cli import idx

    assert idx("1", "1") == 0 and idx("9", "9") == 80
    for r, c in [("0", "1"), ("1", "0"), ("10", "1"), ("1", "-1")]:
        try:
            idx(r, c)
            raise AssertionError("out-of-range coordinate must fail")
        except ValueError:
            pass

    # tui conflict checker (imports fine headless; curses only starts in wrapper)
    from tui import conflicts

    b = [0] * 81
    b[0] = b[8] = 5  # same row
    assert conflicts(b) == {0, 8}
    b[8] = 6
    assert conflicts(b) == set()

    print("all checks passed")


if __name__ == "__main__":
    main()
