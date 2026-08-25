#!/usr/bin/env python3
"""a-souvenir-of-sudokus — CLI frontend (the first renderer).

Usage: python3 cli.py [easy|medium|hard] [seed]
Commands: put r c v | del r c | hint | check | solve | save [f] | load [f] | new [diff] | quit
"""
import sys

from sudoku import Game

SAVE = "game.json"


def show(g: Game) -> None:
    print("    1 2 3   4 5 6   7 8 9")
    print("  +-------+-------+-------+")
    for r in range(9):
        cells = []
        for c in range(9):
            v = g.board[r * 9 + c]
            cells.append(str(v) if v else ".")
        row = " ".join(cells[0:3]) + " | " + " ".join(cells[3:6]) + " | " + " ".join(cells[6:9])
        print(f"{r + 1} | {row} |")
        if r in (2, 5):
            print("  +-------+-------+-------+")
    print("  +-------+-------+-------+")


def idx(r: str, c: str) -> int:
    return (int(r) - 1) * 9 + (int(c) - 1)


def main() -> None:
    diff = sys.argv[1] if len(sys.argv) > 1 else "medium"
    seed = int(sys.argv[2]) if len(sys.argv) > 2 else None
    g = Game.new(diff, seed)
    print(f"a-souvenir-of-sudokus — {diff}")
    show(g)
    while True:
        try:
            parts = input("> ").split()
        except EOFError:
            break
        if not parts:
            continue
        cmd, args = parts[0], parts[1:]
        try:
            if cmd == "put":
                g.put(idx(args[0], args[1]), int(args[2]))
            elif cmd == "del":
                g.put(idx(args[0], args[1]), 0)
            elif cmd == "hint":
                g.hint()
            elif cmd == "check":
                bad = g.wrong_cells()
                print("all good" if not bad else "wrong: " + ", ".join(f"r{i // 9 + 1}c{i % 9 + 1}" for i in bad))
                continue
            elif cmd == "solve":
                g.board = list(g.solution)
            elif cmd == "save":
                path = args[0] if args else SAVE
                with open(path, "w") as f:
                    f.write(g.to_json())
                print(f"saved to {path}")
                continue
            elif cmd == "load":
                path = args[0] if args else SAVE
                with open(path) as f:
                    g = Game.from_json(f.read())
            elif cmd == "new":
                g = Game.new(args[0] if args else "medium")
            elif cmd in ("quit", "q", "exit"):
                break
            else:
                print(__doc__.strip().splitlines()[-1])
                continue
        except (ValueError, IndexError, OSError) as e:
            print(f"error: {e}")
            continue
        show(g)
        if g.is_solved():
            print("solved — grand-père would be proud.")
            break


if __name__ == "__main__":
    main()
