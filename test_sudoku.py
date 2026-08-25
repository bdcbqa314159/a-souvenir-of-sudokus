"""Self-check: python3 test_sudoku.py"""
import random

from sudoku import Game, count_solutions, generate, solve


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
    g2 = Game.from_json(g.to_json())
    assert g2.board == g.board and g2.puzzle == g.puzzle
    while g.hint() is not None:
        pass
    assert g.is_solved()

    print("all checks passed")


if __name__ == "__main__":
    main()
