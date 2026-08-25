#!/usr/bin/env python3
"""a-souvenir-of-sudokus — curses TUI frontend (vim-style).

Usage: python3 tui.py [easy|medium|hard] [seed]
Keys:  hjkl / arrows move · 1-9 put · x or 0 clear · u undo
       H hint · c check · s/l save/load · q quit
Given cells are bold and locked; rule conflicts show in red as you type.
"""
import curses
import sys

from sudoku import Game, PEERS

SAVE = "game.json"
HELP = "hjkl move  1-9 put  x clear  u undo  H hint  c check  s/l save/load  q quit"
MOVES = {
    "h": -1, "l": +1, "k": -9, "j": +9,
    curses.KEY_LEFT: -1, curses.KEY_RIGHT: +1, curses.KEY_UP: -9, curses.KEY_DOWN: +9,
}


def conflicts(board):
    """Cells whose value repeats in a peer (row/col/box) — the live input checker."""
    return {i for i in range(81) if board[i] and any(board[p] == board[i] for p in PEERS[i])}


def draw(scr, g, cur, msg):
    scr.erase()
    bad = conflicts(g.board)
    for y in (0, 4, 8, 12):
        scr.addstr(y, 1, "+-------+-------+-------+")
    for r in range(9):
        y = 1 + r + r // 3
        for x in (1, 9, 17, 25):
            scr.addstr(y, x, "|")
        for c in range(9):
            i = r * 9 + c
            v = g.board[i]
            attr = curses.A_BOLD if g.is_given(i) else curses.A_NORMAL
            if i in bad:
                attr |= curses.color_pair(1)
            if i == cur:
                attr |= curses.A_REVERSE
            scr.addstr(y, 3 + 2 * c + 2 * (c // 3), str(v) if v else ".", attr)
    scr.addstr(13, 1, msg[:70])
    scr.addstr(14, 1, HELP, curses.A_DIM)
    scr.refresh()


def run(scr, g):
    if curses.LINES < 16 or curses.COLS < 78:
        raise SystemExit("terminal too small — need at least 78x16")
    curses.curs_set(0)
    curses.use_default_colors()
    curses.init_pair(1, curses.COLOR_RED, -1)
    cur, msg, undo = 40, f"a-souvenir-of-sudokus — {g.difficulty}", []
    while True:
        draw(scr, g, cur, msg)
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
        elif isinstance(ch, str) and (ch.isdigit() or ch == "x"):
            if g.is_given(cur):
                msg = "given cell — locked"
            else:
                undo.append((cur, g.board[cur]))
                g.put(cur, 0 if ch == "x" else int(ch))
        elif ch == "u":
            if undo:
                i, v = undo.pop()
                g.board[i] = v
                cur = i
            else:
                msg = "nothing to undo"
        elif ch == "H":
            i = g.hint()
            if i is not None:
                undo.append((i, 0))
                cur = i
        elif ch == "c":
            wrong = g.wrong_cells()
            msg = "all good so far" if not wrong else f"{len(wrong)} wrong cell(s)"
        elif ch == "s":
            with open(SAVE, "w") as f:
                f.write(g.to_json())
            msg = f"saved to {SAVE}"
        elif ch == "l":
            try:
                with open(SAVE) as f:
                    g = Game.from_json(f.read())
                undo.clear()
                msg = f"loaded {SAVE}"
            except OSError as e:
                msg = f"load failed: {e}"
        if g.is_solved():
            msg = "solved — grand-père would be proud.  (q to quit)"


def main():
    diff = sys.argv[1] if len(sys.argv) > 1 else "medium"
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else None
    curses.wrapper(run, Game.new(diff, seed))


if __name__ == "__main__":
    main()
