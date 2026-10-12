# CLAUDE.md — a-souvenir-of-sudokus

Sudoku game dedicated to "el abuelo". The digits are a generated typeface
derived from his handwriting; the tribute is the point — treat the assets
with respect.

## Architecture (three worlds, one contract)

- `engine/` — C++20, ns `souvenir`. Solver, unique generator, technique
  grader, phantom mode. THE contract: `engine/include/souvenir/api.hpp` —
  one JSON command surface (`apply_command`). Everything else is a client.
- `web/` — Rust/Leptos → wasm frontend (UI only, zero game logic) +
  `src-tauri/` desktop wrap. The engine reaches the browser as wasm
  (`souvenir_cmd`), built by emscripten, loaded in `index.html`.
- `atelier/` — Python (cv2/sklearn/scipy) asset pipeline: photos → glyphs →
  generated typeface. See the `pipeline.py` docstring for the stages.

## The pack model

- `web/assets/souvenir/` = the GENERATED pack (the keepsake derived from his hand) (typeface + synthetic paper).
  Committed to the repo. 100% generated pixels; style derived from real
  handwriting. Stamped + invisibly watermarked. Covered by ASSETS-LICENSE.md
  (all rights reserved — separate from the MIT code).
- `web/assets/abuelo/` = the REAL-scan family pack — his actual handwriting. Gitignored; exists only
  on the maintainer's machine. NEVER commit anything from it.
- Runtime pack priority: abuelo > souvenir > placeholder — the real hand,
  the souvenir of it, then the stand-in. A build made
  where abuelo exists shows the real handwriting; every clone builds the
  public generated version. `scripts/build-desktop.sh` prints which one it
  bakes and fails gracefully if no valid pack exists.
- Raw photos, curation data (`meta.csv`), and the watermark key live in
  `originals/` (gitignored). Without that machine they are not recoverable —
  do not assume they exist.

## Key commands

- Build desktop (any OS): `./scripts/build-desktop.sh` (see BUILDING.md)
- Web dev server: `cd web && trunk serve --port 8642`; open `/?dev` for
  uncapped assists + a solve button
- Engine tests: `cd engine && cmake --preset release && cmake --build
  --preset release -j && ctest --preset release`
- Regenerate typeface: `cd atelier && .venv/bin/python pipeline.py synth`
  (`synth user5` → only that digit; approved rows stay frozen)
- Synthetic paper: `pipeline.py genpaper`
- Family pack (real scans): `pipeline.py emit` (maintainer machine only)
- Verify an asset's provenance watermark: `pipeline.py verify <file>`
  (needs the secret key in `originals/atelier-work/`; `pipeline.py genkey`
  creates it explicitly — nothing mints a key as a side effect)
- Watermark regression suite: `.venv/bin/python test_watermark.py` (key
  holder's machine only) — run after ANY change to the mark/verify code

## Updates (desktop)

The app checks GitHub for a newer release on launch and offers it in a
dialog; the user chooses. Every payload is verified against the public key
baked into the binary, so an unsigned or foreign-signed update is refused.

- `originals/atelier-work/updater.key` (+ `.password`) is the **signing key**.
  Gitignored, and mirrored as the `TAURI_SIGNING_PRIVATE_KEY` /
  `..._PASSWORD` GitHub secrets so CI can sign releases. **Back it up with
  the photos and `wm.key`: lose it and installed apps can never be updated
  again** (they will reject anything signed by a different key).
- The release workflow signs each bundle and publishes `latest.json`
  (version + per-platform URL and signature) to the release; the app polls
  `releases/latest/download/latest.json`.
- A clone without the key still builds — `build-desktop.sh` then disables
  update artifacts rather than failing.

## Known flakes & gotchas

- The Tauri build sometimes fails its first run after asset changes
  ("failed to read asset"): run the build script again — it retries once,
  occasionally needs a second invocation.
- The browser caches glyphs by filename: after regenerating assets, HARD
  refresh (Cmd+Shift+R; Ctrl+Shift+R on Windows).
- Windows: run `build-desktop.sh` in Git Bash. The script self-configures PATH
  (rustup's cargo and MSVC's `link.exe` ahead of the standalone Rust and Git's
  coreutils `link`) and re-activates a half-cloned `.emsdk`. Two prereqs it
  does NOT fix: the VS "Desktop C++" env must be loaded (LIB/INCLUDE set), and
  `python3` must run real Python — if it's the Microsoft Store alias the build
  stops early with the fix (copy `python.exe`→`python3.exe`, or disable the
  alias). Atelier venv path is `.venv/Scripts/python` (not `bin/`).
- `meta.csv` semantics: label `-1` = human-rejected exemplar; `pin` marks
  blend anchors — pins steer the generator (pinned exemplars are the only
  anchors for their digit). `SYNTH_TUNE` in pipeline.py holds per-digit
  blend overrides.
- Repo rules: `main` is PR-only, signed commits, linear history. No
  attribution footers in commits or PRs.
