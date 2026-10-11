"""a-souvenir-of-sudokus — Python engine draft, retired to readable spec.

The live backend is the C++ engine (engine/, compiled `souvenir` module) —
the frontends import that. This file stays as the executable specification:
same JSON save-file contract (test_bindings.py cross-checks both); the
in-memory API matches too, except randomness is passed as `rng` objects here
vs integer `seed`s in the engine, and the engine additionally grades puzzles.

Pure logic, no I/O. Board = list of 81 ints, 0 = empty, row-major.
"""
from __future__ import annotations

import json
import random

DIGITS = set(range(1, 10))

ROWS = [[r * 9 + c for c in range(9)] for r in range(9)]
COLS = [[r * 9 + c for r in range(9)] for c in range(9)]
BOXES = [
    [(br * 3 + r) * 9 + (bc * 3 + c) for r in range(3) for c in range(3)]
    for br in range(3)
    for bc in range(3)
]
PEERS = [set() for _ in range(81)]
for _unit in ROWS + COLS + BOXES:
    for _i in _unit:
        PEERS[_i] |= set(_unit) - {_i}


def candidates(board: list[int], i: int) -> set[int]:
    return DIGITS - {board[p] for p in PEERS[i]}


def consistent(board: list[int]) -> bool:
    """No digit repeated within any row/col/box."""
    for unit in ROWS + COLS + BOXES:
        seen = [board[i] for i in unit if board[i]]
        if len(seen) != len(set(seen)):
            return False
    return True


def _most_constrained(board: list[int]) -> tuple[int, set[int]] | None:
    """Empty cell with fewest candidates, or None if board is full."""
    best = None
    for i in range(81):
        if board[i] == 0:
            c = candidates(board, i)
            if best is None or len(c) < len(best[1]):
                best = (i, c)
                if len(c) <= 1:
                    break
    return best


def solve(board: list[int], rng: random.Random | None = None) -> list[int] | None:
    """A solution to `board`, or None. rng shuffles branching (used to generate)."""
    if not consistent(board):
        return None
    b = list(board)

    def bt() -> bool:
        cell = _most_constrained(b)
        if cell is None:
            return True
        i, cands = cell
        vals = list(cands)
        if rng:
            rng.shuffle(vals)
        for v in vals:
            b[i] = v
            if bt():
                return True
        b[i] = 0
        return False

    return b if bt() else None


def count_solutions(board: list[int], limit: int = 2) -> int:
    """Number of solutions, stopping at `limit` (uniqueness check = limit 2)."""
    if not consistent(board):
        return 0
    b = list(board)
    n = 0

    def bt() -> bool:  # True = limit reached, abort
        nonlocal n
        cell = _most_constrained(b)
        if cell is None:
            n += 1
            return n >= limit
        i, cands = cell
        for v in cands:
            b[i] = v
            if bt():
                return True
        b[i] = 0
        return False

    bt()
    return n


# Dig targets, kept == the C++ engine's (which additionally grades by human
# technique and regenerates until the grade matches — this spec engine doesn't).
CLUE_TARGET = {"easy": 40, "medium": 26, "hard": 24}


def generate(difficulty: str = "medium", seed: int | None = None) -> tuple[list[int], list[int]]:
    """(puzzle, solution). Puzzle always has exactly one solution."""
    if difficulty not in CLUE_TARGET:
        raise ValueError(f"difficulty must be one of {sorted(CLUE_TARGET)}")
    rng = random.Random(seed)
    full = solve([0] * 81, rng)
    assert full is not None
    puzzle = list(full)
    order = list(range(81))
    rng.shuffle(order)
    clues = 81
    for i in order:
        if clues <= CLUE_TARGET[difficulty]:
            break
        saved, puzzle[i] = puzzle[i], 0
        if count_solutions(puzzle) == 1:
            clues -= 1
        else:
            puzzle[i] = saved
    return puzzle, full


class Game:
    """One playing session. The JSON form is the frontend contract."""

    def __init__(self, puzzle: list[int], solution: list[int], difficulty: str = "medium"):
        self.puzzle = list(puzzle)  # givens, immutable during play
        self.solution = list(solution)
        self.board = list(puzzle)
        self.marks = [set() for _ in range(81)]  # pencil marks per cell
        self.difficulty = difficulty

    @classmethod
    def new(cls, difficulty: str = "medium", seed: int | None = None) -> "Game":
        return cls(*generate(difficulty, seed), difficulty)

    def is_given(self, i: int) -> bool:
        return self.puzzle[i] != 0

    def _apply(self, i: int, v: int) -> None:
        self.board[i] = v
        if v:  # a placed value erases the cell's marks and that digit from peer marks
            self.marks[i] = set()
            for p in PEERS[i]:
                self.marks[p].discard(v)

    def put(self, i: int, v: int) -> None:
        if self.is_given(i):
            raise ValueError("cell is a given")
        if v not in DIGITS and v != 0:
            raise ValueError("value must be 0-9")
        self._apply(i, v)

    def toggle_mark(self, i: int, v: int) -> None:
        if self.is_given(i) or self.board[i]:
            raise ValueError("cell not markable")
        if v not in DIGITS:
            raise ValueError("mark must be 1-9")
        self.marks[i] ^= {v}

    def wrong_cells(self) -> list[int]:
        return [i for i in range(81) if self.board[i] and self.board[i] != self.solution[i]]

    def is_solved(self) -> bool:
        return self.board == self.solution

    def hint(self, rng: random.Random | None = None) -> int | None:
        """Fill one empty-or-wrong cell with its solution value; returns the index."""
        todo = [i for i in range(81) if self.board[i] != self.solution[i]]
        if not todo:
            return None
        i = (rng or random).choice(todo)
        self._apply(i, self.solution[i])
        return i

    def to_json(self) -> str:
        return json.dumps(
            {
                "name": "a-souvenir-of-sudokus",
                "difficulty": self.difficulty,
                "puzzle": self.puzzle,
                "solution": self.solution,
                "board": self.board,
                "marks": [sorted(m) for m in self.marks],
            }
        )

    @classmethod
    def from_json(cls, s: str) -> "Game":
        d = json.loads(s)

        def grid(key: str) -> list[int]:
            v = d.get(key)
            if not (isinstance(v, list) and len(v) == 81 and all(isinstance(x, int) and 0 <= x <= 9 for x in v)):
                raise ValueError(f"bad save file: {key}")
            return v

        puzzle, solution, board = grid("puzzle"), grid("solution"), grid("board")
        if 0 in solution or not consistent(solution):
            raise ValueError("bad save file: solution is not a solved grid")
        if any(puzzle[i] not in (0, solution[i]) for i in range(81)):
            raise ValueError("bad save file: puzzle/solution mismatch")
        marks = d.get("marks", [[]] * 81)
        if not (isinstance(marks, list) and len(marks) == 81):
            raise ValueError("bad save file: marks")
        g = cls(puzzle, solution, d.get("difficulty", "medium"))
        g.board = list(board)
        g.marks = [set(m) for m in marks]
        return g
