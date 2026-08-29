#!/usr/bin/env python3
"""Generate the placeholder asset pack (run once; outputs are committed).

Exercises everything the final pack from atelier/ will provide: digits 1-9 in
two pen roles (given = black ink, user = red ink), several variants per digit
so the renderer's per-cell variant picking shows, and a manifest.json that is
the contract atelier/ must emit for assets/grandpere/.
"""
import json
import pathlib
import random

ROOT = pathlib.Path(__file__).parent / "placeholder"
ROLES = {"given": "#1a1a1a", "user": "#b5271d"}
VARIANTS = ["a", "b", "c"]

SVG = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
<text x="32" y="46" font-size="46" text-anchor="middle"
 font-family="'Bradley Hand','Comic Sans MS',cursive" fill="{color}"
 transform="rotate({rot} 32 32) skewX({skew})">{digit}</text>
</svg>
"""


def main() -> None:
    rng = random.Random(20260828)
    manifest = {"name": "placeholder", "digits": {}}
    for role, color in ROLES.items():
        manifest["digits"][role] = {}
        for digit in range(1, 10):
            paths = []
            for v in VARIANTS:
                rel = f"digits/{role}/{digit}_{v}.svg"
                out = ROOT / rel
                out.parent.mkdir(parents=True, exist_ok=True)
                out.write_text(
                    SVG.format(
                        color=color,
                        digit=digit,
                        rot=round(rng.uniform(-7, 7), 1),
                        skew=round(rng.uniform(-6, 6), 1),
                    )
                )
                paths.append(rel)
            manifest["digits"][role][str(digit)] = paths
    (ROOT / "manifest.json").write_text(json.dumps(manifest, indent=1))
    print(f"wrote {ROOT}/manifest.json + {2 * 9 * len(VARIANTS)} svgs")


if __name__ == "__main__":
    main()
