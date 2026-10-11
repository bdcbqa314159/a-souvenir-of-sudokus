"""Paths, sizes and the look constants every stage shares."""
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
ORIGINALS = ROOT / "originals"
WORK = ORIGINALS / "atelier-work"
PACK = ROOT / "web" / "assets" / "grandpere"  # generated pack — COMMITTED to the public repo
ABUELO = ROOT / "web" / "assets" / "abuelo"  # real-scan family pack — gitignored, this machine only
SIDE = 1080
CELL = SIDE // 9
GLYPH = 96  # emitted asset size
ROLES = {"given": "dark", "user": "red"}  # pack role -> ink


PAPER_FADE = 0.35  # ruling contrast: 0 = full ink, 1 = flat cream
PAPER_CREAM = (0xE8, 0xF2, 0xF7)  # BGR of the web UI's --paper
STAMP = (
    "(c) 2026 Bernardo Cohen / deLaPatada Software - a-souvenir-of-sudokus. "
    "Handwriting of el abuelo. All rights reserved; not licensed for reuse."
)


