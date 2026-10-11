#!/usr/bin/env python3
"""a-souvenir-of-sudokus — curses TUI frontend (vim-style).

Usage: python3 tui.py [easy|medium|hard] [seed]
Keys:  hjkl / arrows move · 1-9 put (toggle marks in pencil mode) · m pencil mode
       x or 0 clear · u undo · r redo · p auto pencil-marks · H hint · c check
       n new game · N new game, next difficulty · s/L save/load · q quit

Colours follow the original handwritten grids: givens in black (terminal ink),
user entries and pencil marks in red, the central tic-tac-toe cross in black
with every other grid line in red. Conflicts flash yellow. Cells holding the
cursor's digit are underlined; the cursor's row/column/box shows a · crosshair.
"""
import curses
import sys
import time

try:
    from souvenir import CLUE_TARGET, Game, PEERS
except ImportError:
    sys.exit(
        "souvenir engine module not built — run:\n"
        "  cmake --preset release -S engine -DSOUVENIR_BUILD_PYTHON=ON && cmake --build --preset release engine"
    )

SAVE = "game.json"
DIGITS = set(range(1, 10))
DIFFICULTIES = ["easy", "medium", "hard"]
HELP = "hjkl 1-9  m pencil  x clear  u/r undo/redo  p auto-marks  H hint  c check  n/N new  s/L save/load  q quit"
MOVES = {
    "h": -1, "l": +1, "k": -9, "j": +9,
    curses.KEY_LEFT: -1, curses.KEY_RIGHT: +1, curses.KEY_UP: -9, curses.KEY_DOWN: +9,
}
RED, YELLOW = 1, 2
MIN_LINES, MIN_COLS = 23, 40


def conflicts(board):
    """Cells whose value repeats in a peer (row/col/box) — the live input checker."""
    return {i for i in range(81) if board[i] and any(board[p] == board[i] for p in PEERS[i])}


def auto_candidates(board):
    """Pencil marks a careful player would write: per empty cell, the digits its peers allow."""
    return [
        DIGITS - {board[p] for p in PEERS[i]} if board[i] == 0 else set()
        for i in range(81)
    ]


def snap(g):
    return g.board[:], [set(m) for m in g.marks]


def restore(g, s):
    g.board, g.marks = s[0][:], [set(m) for m in s[1]]


def draw(scr, g, cur, msg, pencil, start, mistakes):
    scr.erase()
    lines, cols = scr.getmaxyx()
    if lines < MIN_LINES or cols < MIN_COLS:
        try:
            scr.addstr(0, 0, f"terminal too small — need {MIN_COLS}x{MIN_LINES}"[: cols - 1])
        except curses.error:
            pass
        scr.refresh()
        return
    red = curses.color_pair(RED)
    hline = ("+" + "-" * 3) * 9 + "+"
    # red minor lines first, black tic-tac-toe cross drawn on top — like the original
    for k in range(10):
        if k not in (3, 6):
            scr.addstr(k * 2, 0, hline, red)
    for r in range(9):
        for k in range(10):
            if k not in (3, 6):
                scr.addstr(r * 2 + 1, k * 4, "|", red)
    for k in (3, 6):
        scr.addstr(k * 2, 0, hline, curses.A_BOLD)
        for y in range(19):
            scr.addstr(y, k * 4, "+" if y % 2 == 0 else "|", curses.A_BOLD)

    bad = conflicts(g.board)
    board = g.board
    cursor_value = board[cur]
    crosshair = PEERS[cur]
    for r in range(9):
        for c in range(9):
            i = r * 9 + c
            y, x = r * 2 + 1, c * 4 + 1
            v = board[i]
            if v:
                attr = curses.A_BOLD if g.is_given(i) else red
                if i in bad:
                    attr = curses.color_pair(YELLOW) | curses.A_BOLD
                if cursor_value and v == cursor_value and i != cur:
                    attr |= curses.A_UNDERLINE
                if i == cur:
                    attr |= curses.A_REVERSE
                scr.addstr(y, x + 1, str(v), attr)
            else:
                m = sorted(g.marks[i])
                s = "".join(map(str, m)) if len(m) <= 3 else "".join(map(str, m[:2])) + "+"
                if not s and i in crosshair:
                    scr.addstr(y, x, " · ", curses.A_DIM)
                    continue
                attr = red | curses.A_DIM
                if i == cur:
                    attr |= curses.A_REVERSE
                scr.addstr(y, x, f"{s:<3}", attr)

    elapsed = int(time.monotonic() - start)
    filled = sum(1 for v in board if v)
    mk = " ".join(map(str, sorted(g.marks[cur]))) or "-"
    w = cols - 1
    scr.addstr(19, 0, msg[:w])
    status = (
        f"[{'PENCIL' if pencil else 'pen'}]  marks: {mk}   "
        f"{elapsed // 60:02d}:{elapsed % 60:02d}   {filled}/81   mistakes: {mistakes}"
    )
    scr.addstr(20, 0, status[:w])
    scr.addstr(21, 0, HELP[:w], curses.A_DIM)
    scr.refresh()


def run(scr, g):
    try:
        curses.curs_set(0)
    except curses.error:
        pass
    if curses.has_colors():
        curses.use_default_colors()
        curses.init_pair(RED, curses.COLOR_RED, -1)
        curses.init_pair(YELLOW, curses.COLOR_YELLOW, -1)
    scr.timeout(1000)  # wake up every second so the clock ticks

    cur, pencil, mistakes = 40, False, 0
    undo, redo = [], []
    # ponytail: the clock restarts on load — persisting elapsed time would mean
    # a TUI-only field in the shared save schema; add if anyone ever cares
    start = time.monotonic()
    msg = f"a-souvenir-of-sudokus — {g.difficulty}"

    def fresh(new_game, note):
        nonlocal g, mistakes, start, msg
        g = new_game
        undo.clear()
        redo.clear()
        mistakes = 0
        start = time.monotonic()
        msg = note

    while True:
        draw(scr, g, cur, msg, pencil, start, mistakes)
        key = scr.getch()
        if key == -1:  # timeout tick: just refresh the clock
            continue
        ch = chr(key) if 0 <= key < 256 else key
        msg = ""
        if ch == "q":
            break
        elif ch in MOVES or key in MOVES:
            d = MOVES.get(ch, MOVES.get(key))
            if d in (-1, 1) and (cur % 9) + d in range(9):
                cur += d
            elif d in (-9, 9) and 0 <= cur + d < 81:
                cur += d
        elif ch == "n":
            fresh(Game.new(g.difficulty), f"new {g.difficulty} game")
        elif ch == "N":
            nxt = DIFFICULTIES[(DIFFICULTIES.index(g.difficulty) + 1) % len(DIFFICULTIES)]
            fresh(Game.new(nxt), f"new {nxt} game")
        elif ch == "s":
            try:
                with open(SAVE, "w") as f:
                    f.write(g.to_json())
                msg = f"saved to {SAVE}"
            except OSError as e:
                msg = f"save failed: {e}"
        elif ch == "L":  # capital: lowercase l is vim's move-right
            try:
                with open(SAVE) as f:
                    fresh(Game.from_json(f.read()), f"loaded {SAVE}")
            except (OSError, ValueError) as e:
                msg = f"load failed: {e}"
        elif g.is_solved():
            msg = "solved — n for a new game, q to quit"
        elif ch == "m":
            pencil = not pencil
        elif isinstance(ch, str) and ch in "0123456789x":
            if g.is_given(cur):
                msg = "given cell — locked"
            elif ch in ("x", "0"):
                undo.append((cur, snap(g)))
                redo.clear()
                if g.board[cur]:
                    g.put(cur, 0)
                else:
                    g.clear_marks(cur)
            elif pencil:
                if g.board[cur]:
                    msg = "cell has a value — x to clear first"
                else:
                    undo.append((cur, snap(g)))
                    redo.clear()
                    g.toggle_mark(cur, int(ch))
            else:
                undo.append((cur, snap(g)))
                redo.clear()
                g.put(cur, int(ch))
                if g.board[cur] != g.solution[cur]:
                    mistakes += 1
        elif ch == "u":
            if undo:
                redo.append((cur, snap(g)))
                cur, s = undo.pop()
                restore(g, s)
            else:
                msg = "nothing to undo"
        elif ch == "r":
            if redo:
                undo.append((cur, snap(g)))
                cur, s = redo.pop()
                restore(g, s)
            else:
                msg = "nothing to redo"
        elif ch == "p":
            undo.append((cur, snap(g)))
            redo.clear()
            g.marks = auto_candidates(g.board)
            msg = "pencil marks filled from candidates"
        elif ch == "H":
            undo.append((cur, snap(g)))
            redo.clear()
            i = g.hint()
            if i is None:
                undo.pop()
            else:
                cur = i
        elif ch == "c":
            wrong = g.wrong_cells()
            msg = "all good so far" if not wrong else f"{len(wrong)} wrong cell(s)"
        if g.is_solved():
            elapsed = int(time.monotonic() - start)
            msg = (
                f"solved in {elapsed // 60:02d}:{elapsed % 60:02d} with {mistakes} mistake(s) "
                "— el abuelo would be proud.  (n new game, q quit)"
            )


def main():
    diff = sys.argv[1] if len(sys.argv) > 1 else "medium"
    if diff not in CLUE_TARGET:
        sys.exit(f"unknown difficulty {diff!r} — pick one of {sorted(CLUE_TARGET)}")
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else None
    curses.wrapper(run, Game.new(diff, seed))


if __name__ == "__main__":
    main()
