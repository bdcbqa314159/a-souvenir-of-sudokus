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
- **A build is a regular build unless it is asked otherwise.** Holding the
  originals is not enough to get them into one: `SOUVENIR_FAMILY=1` (or
  `./scripts/build-desktop.sh --family`) is the only way, and it fails loudly
  if the pack is not there rather than quietly producing a public build. The
  trunk hook `web/scripts/family-pack.sh` enforces this for all three worlds at
  once — browser, desktop and Android all build through trunk. `index.html`
  names the public packs one by one; it used to copy all of `assets/`, which
  swept his handwriting into everything built here, Play artifacts included.
- Runtime pack priority is still abuelo > souvenir > placeholder — the real
  hand, the souvenir of it, then the stand-in. It simply never finds the first
  one unless the build was asked for it.
- Raw photos, curation data (`meta.csv`), and the watermark key live in
  `originals/` (gitignored). Without that machine they are not recoverable —
  do not assume they exist.

## Key commands

- Build desktop (any OS): `./scripts/build-desktop.sh` (see BUILDING.md)
- Web dev server: `cd web && trunk serve --port 8642`; open `/?dev` for
  uncapped assists + a solve button. Prefix `SOUVENIR_FAMILY=1` to develop
  against his real handwriting; without it you get the generated pack, which
  is what everyone else sees.
- Family build of the desktop app: `./scripts/build-desktop.sh --family`
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
  The key file is encrypted with that password. Both are gitignored and
  mirrored as the `TAURI_SIGNING_PRIVATE_KEY` / `..._PASSWORD` GitHub
  secrets, which is how CI signs releases.
  **Back them up — and back them up SEPARATELY**: the key with the photos and
  `wm.key`, the password in a password manager. Together they can sign
  updates for every installed copy; apart, neither is enough. Losing them
  means installed apps can never be updated again (they reject anything
  signed by a different key).
  Only CI needs to sign, so the local password file is optional: delete it
  and `build-desktop.sh` simply builds without update artifacts.
- The release workflow signs each bundle and publishes `latest.json`
  (version + per-platform URL and signature) to the release; the app polls
  `releases/latest/download/latest.json`.
- A clone without the key still builds — `build-desktop.sh` then disables
  update artifacts rather than failing.

## Security posture (what is defended, and what is not)

- `apply_command` is the only parser of input nobody trusts: the saved game
  lives in editable localStorage. It validates exactly 81 integer cells,
  range-checked on the wide type before narrowing, with a nesting-depth guard,
  and never throws. `fuzz_api.mjs` (30k hostile requests) runs in CI on every
  push — not on request.
- The frontend has no raw-HTML sink: no `inner_html`, no `eval`. Leptos escapes
  text nodes; the valvula page writes `textContent` only.
- `?dev` is debug-build-only (`cfg!(debug_assertions)` plus an exact key), so
  it cannot uncap assists in anything a player receives. CI runs the web tests
  in release as well, because only a release run can prove the gate holds.
- Stray paths and unknown query parameters go to the valvula page before the
  engine loads (`web/gate.js`). The game reads exactly two knobs: `pack`
  (whitelisted charset, 32 chars, only reorders the pack chain) and `dev`.
- CSP: both web pages carry one as a `<meta>`, since Pages cannot set headers.
  Neither forbids inline script, and `index.html` cannot: **trunk injects its
  own inline module to boot the Rust wasm**, so a strict `script-src` there
  stops the game from starting — silently, the way every CSP violation does.
  What the policy does buy is the outward direction: no foreign script or
  image, `connect-src 'self'`, `object-src`/`base-uri 'none'`. `404.html` is
  stricter still (`default-src 'none'`, `connect-src 'none'`) because it loads
  nothing and calls nobody; it keeps its script inline because it is served
  for URLs at any depth, where a relative `src` would miss.
- `tauri.conf.json` keeps `"csp": null` **deliberately**: the webview serves
  only bundled assets, updates are Ed25519-verified before they are ever
  unpacked, and there is no remote content to constrain — while a wrong policy
  would silently break the IPC the launch-time update check rides on. Revisit
  only with a desktop build in hand to test it.
- Not defended, and cannot be: the solution is client-side. Every engine
  response carries the game, and the game carries `solution`, because hangman
  judges each digit as it lands and the ledger stores finished grids. `check`
  reports how many cells are wrong, never which, so it is not a map — but a
  player with developer tools has the answer regardless. A serverless game
  cannot keep its answer from its own client.

## Known flakes & gotchas

- The Tauri build sometimes fails its first run after asset changes
  ("failed to read asset"): run the build script again — it retries once,
  occasionally needs a second invocation.
- The browser caches glyphs by filename: after regenerating assets, HARD
  refresh (Cmd+Shift+R; Ctrl+Shift+R on Windows).
- `trunk serve` watches `web/`, NOT `engine/build/`. Rebuild the engine wasm
  and the dev server keeps serving the previous one — the page then runs an
  engine one version behind, which looks like a frontend bug (a new command
  field silently ignored, for instance). Touch a file under `web/` after
  `cmake --build engine/build/wasm`, and confirm with
  `cmp engine/build/wasm/souvenir.wasm web/dist/souvenir/souvenir.wasm`.
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
