#!/usr/bin/env python3
"""a-souvenir-of-sudokus — curses TUI frontend (vim-style).

Usage: python3 tui.py [easy|medium|hard] [seed]
Keys:  hjkl / arrows move · 1-9 put (toggle marks in pencil mode) · m pencil mode
       x or 0 clear · u undo · H hint · c check · s/L save/load · q quit

Colours follow the original handwritten grids: givens in black (terminal ink),
user entries and pencil marks in red, the central tic-tac-toe cross in black
with every other grid line in red. Conflicts flash yellow.
"""
import curses
import sys

from sudoku import CLUE_TARGET, Game, PEERS

SAVE = "game.json"
HELP = "hjkl move  1-9 put  m pencil  x clear  u undo  H hint  c check  s/L save/load  q quit"
MIN_LINES, MIN_COLS = 23, 40
MOVES = {
    "h": -1, "l": +1, "k": -9, "j": +9,
    curses.KEY_LEFT: -1, curses.KEY_RIGHT: +1, curses.KEY_UP: -9, curses.KEY_DOWN: +9,
}
RED, YELLOW = 1, 2


def conflicts(board):
    """Cells whose value repeats in a peer (row/col/box) — the live input checker."""
    return {i for i in range(81) if board[i] and any(board[p] == board[i] for p in PEERS[i])}


def snap(g):
    return g.board[:], [set(m) for m in g.marks]


def restore(g, s):
    g.board, g.marks = s[0][:], [set(m) for m in s[1]]


def draw(scr, g, cur, msg, pencil):
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
    for r in range(9):
        for c in range(9):
            i = r * 9 + c
            y, x = r * 2 + 1, c * 4 + 1
            v = g.board[i]
            if v:
                attr = curses.A_BOLD if g.is_given(i) else red
                if i in bad:
                    attr = curses.color_pair(YELLOW) | curses.A_BOLD
                if i == cur:
                    attr |= curses.A_REVERSE
                scr.addstr(y, x + 1, str(v), attr)
            else:
                m = sorted(g.marks[i])
                s = "".join(map(str, m)) if len(m) <= 3 else "".join(map(str, m[:2])) + "+"
                attr = red | curses.A_DIM
                if i == cur:
                    attr |= curses.A_REVERSE
                scr.addstr(y, x, f"{s:<3}", attr)

    mk = " ".join(map(str, sorted(g.marks[cur]))) or "-"
    w = cols - 1
    scr.addstr(19, 0, msg[:w])
    scr.addstr(20, 0, f"[{'PENCIL' if pencil else 'pen'}]  marks here: {mk}"[:w])
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
    cur, msg, undo, pencil = 40, f"a-souvenir-of-sudokus — {g.difficulty}", [], False
    while True:
        draw(scr, g, cur, msg, pencil)
        key = scr.getch()
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
        elif ch == "m":
            pencil = not pencil
        elif isinstance(ch, str) and ch in "0123456789x":
            if g.is_given(cur):
                msg = "given cell — locked"
            elif ch in ("x", "0"):
                undo.append((cur, snap(g)))
                if g.board[cur]:
                    g.put(cur, 0)
                else:
                    g.marks[cur] = set()
            elif pencil:
                if g.board[cur]:
                    msg = "cell has a value — x to clear first"
                else:
                    undo.append((cur, snap(g)))
                    g.toggle_mark(cur, int(ch))
            else:
                undo.append((cur, snap(g)))
                g.put(cur, int(ch))
        elif ch == "u":
            if undo:
                cur, s = undo.pop()
                restore(g, s)
            else:
                msg = "nothing to undo"
        elif ch == "H":
            undo.append((cur, snap(g)))
            i = g.hint()
            if i is None:
                undo.pop()
            else:
                cur = i
        elif ch == "c":
            wrong = g.wrong_cells()
            msg = "all good so far" if not wrong else f"{len(wrong)} wrong cell(s)"
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
                    g = Game.from_json(f.read())
                undo.clear()
                msg = f"loaded {SAVE}"
            except (OSError, ValueError) as e:
                msg = f"load failed: {e}"
        if g.is_solved():
            msg = "solved — grand-père would be proud.  (q to quit)"


def main():
    diff = sys.argv[1] if len(sys.argv) > 1 else "medium"
    if diff not in CLUE_TARGET:
        sys.exit(f"unknown difficulty {diff!r} — pick one of {sorted(CLUE_TARGET)}")
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else None
    curses.wrapper(run, Game.new(diff, seed))


if __name__ == "__main__":
    main()
