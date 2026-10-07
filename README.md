# a-souvenir-of-sudokus

### ▶ [Play it in your browser](https://bdcbqa314159.github.io/a-souvenir-of-sudokus/) — nothing to install, works on phones too.

A sudoku game dedicated to *el abuelo*. The digits are an original generated
typeface derived from his handwriting; the paper is drawn from measurements
of his notebook. Classic mode, and a **phantom mode** where the puzzle flips
into its phantom twin if you stall too long.

## Download the desktop app

Grab the file for your system from the
[**latest release**](https://github.com/bdcbqa314159/a-souvenir-of-sudokus/releases/latest)
(ignore the "Source code" entries — those are for programmers):

| System | File | First-open note |
|---|---|---|
| **Windows** | `...WINDOWS-setup.exe` | if Windows says "protected your PC": *More info → Run anyway* (once) |
| **Mac** | `...MACOS.zip` | unzip, drag to Applications; first open: *right-click → Open* (once) |
| **Linux** | `...LINUX.AppImage` | `chmod +x`, then run |

The warnings appear because the app isn't code-signed — it's a small family
project. Full Windows walkthrough in three languages below.

---

Engine-first architecture: a C++20 engine with one JSON command surface
(`engine/include/souvenir/api.hpp`); every frontend — web, desktop, CLI —
is a thin client of it.

## Install on Windows · Installer sous Windows · Instalar en Windows

> One file, click and play. Windows 10/11 already include everything it needs
> (WebView2) — nothing else to install.
> Un seul fichier, cliquez et jouez. Windows 10/11 contient déjà tout le
> nécessaire (WebView2) — rien d'autre à installer.
> Un solo archivo, haz clic y juega. Windows 10/11 ya incluye todo lo
> necesario (WebView2) — no hay que instalar nada más.

**English**
1. Download **`a-souvenir-of-sudokus_<version>_x64-setup.exe`** from the Releases page.
2. Double-click it. If Windows shows *"Windows protected your PC"*, click **More info → Run anyway** — the app is safe, it just isn't code-signed.
3. It installs for your account only (no administrator rights needed) and adds a Start-menu and desktop shortcut.
4. Open it from the shortcut and play. To remove it later: *Settings → Apps → Installed apps*.

**Français**
1. Téléchargez **`a-souvenir-of-sudokus_<version>_x64-setup.exe`** depuis la page Releases.
2. Double-cliquez dessus. Si Windows affiche « *Windows a protégé votre ordinateur* », cliquez sur **Informations complémentaires → Exécuter quand même** — l'application est sûre, elle n'est simplement pas signée.
3. Elle s'installe uniquement pour votre compte (sans droits administrateur) et ajoute un raccourci dans le menu Démarrer et sur le bureau.
4. Ouvrez-la depuis le raccourci et jouez. Pour la désinstaller : *Paramètres → Applications → Applications installées*.

**Español**
1. Descarga **`a-souvenir-of-sudokus_<version>_x64-setup.exe`** desde la página de Releases.
2. Haz doble clic. Si Windows muestra «*Windows protegió su PC*», haz clic en **Más información → Ejecutar de todas formas** — la aplicación es segura, solo que no está firmada.
3. Se instala solo para tu usuario (sin permisos de administrador) y añade un acceso directo en el menú Inicio y en el escritorio.
4. Ábrela desde el acceso directo y juega. Para desinstalarla: *Configuración → Aplicaciones → Aplicaciones instaladas*.

## Play

Build the engine module once, then play:

```
cmake --preset release -S engine -DSOUVENIR_BUILD_PYTHON=ON
cmake --build --preset release engine

python3 tui.py [easy|medium|hard] [seed]   # vim-style grid: hjkl move, 1-9 put, x clear, u undo
python3 cli.py [easy|medium|hard] [seed]   # line-mode REPL
```

### Browser (Rust · Leptos → wasm)

```
./scripts/build-desktop.sh      # desktop app for this OS (also builds the wasm engine)
./scripts/dev-serve.sh          # dev server at https://localhost:8642 (append ?dev for solve button)
```

Same keys as the TUI (hjkl/arrows, 1-9, m pencil, x clear, u/r undo/redo,
H hint, c check, n new) plus mouse/touch. Digits render from the asset pack
in `web/assets/grandpere/` — the generated typeface, committed to the repo,
so a bare clone builds the complete game (see BUILDING.md and
ASSETS-LICENSE.md).

REPL commands: `put r c v` · `del r c` · `hint` · `check` · `solve` · `save` · `load` · `new` · `quit`

## Test

```
python3 test_sudoku.py     # the Python spec engine
python3 test_bindings.py   # compiled module + frontends + cross-engine save files
```

## C++ engine (`engine/`)

The backend: same API and JSON save-file contract as the Python draft
(`sudoku.py`, retired to executable spec), C++20, no runtime dependencies
(nlohmann/json vendored). Consumable via CMake `FetchContent` as
`souvenir::souvenir`. pybind11 bindings (`-DSOUVENIR_BUILD_PYTHON=ON`) build
the `souvenir` Python module the frontends run on — local build only for now.

```
cmake --preset release -S engine
cmake --build --preset release
ctest --preset release
```

Presets: `debug` · `release` · `asan` (non-Windows).

### Browser engine (wasm)

The whole engine compiles to WebAssembly behind one function —
`souvenir_cmd(requestJson) → responseJson` (command set in
`engine/include/souvenir/api.hpp`). Emscripten is pinned locally in the
gitignored `.emsdk/` (same version as CI):

```
git clone --depth 1 https://github.com/emscripten-core/emsdk.git .emsdk
./.emsdk/emsdk install 3.1.64 && ./.emsdk/emsdk activate 3.1.64
source .emsdk/emsdk_env.sh

emcmake cmake -S engine -B engine/build/wasm -DCMAKE_BUILD_TYPE=Release
cmake --build engine/build/wasm -j
node test_wasm.mjs
```

## Roadmap

- [x] Python engine draft (generator with guaranteed-unique solutions, solver, game state)
- [x] CLI frontend (REPL + vim-style curses TUI)
- [x] C++ engine (`engine/` — library + tests)
- [x] Technique-based difficulty grading (the "GM pass": generate → grade → regenerate until the label is true)
- [x] pybind11 bindings; frontends run on the compiled engine (local build; packaging when the project is final)
- [x] Engine in the browser: wasm build with the JSON command surface
- [x] Browser frontend: Rust (Leptos → wasm), asset-pack driven, placeholder pack first
- [x] Button-bar UX: handwritten digit palette, grouped rows, segmented difficulty
- [x] Phantom mode core: stall clock (difficulty-scaled), flips via `phantom_of`, 3 lives
- [x] Phantom grace window (correct placement wards off the flip)
- [x] Phantom overlay: incoming givens fade in, flip becomes a crossfade
- [x] Game rules: hints/checks capped (3 each), none in phantom, deliberate phantom exit, `?dev` uncaps
- [x] `atelier/`: photos of the handwritten grids → the `grandpere` asset pack (originals and pack stay out of git)
- [x] The built font: generated typeface blended from the handwriting (no glyph is a scan), curated digit by digit
- [x] Synthetic paper measured from the real notebook page; generated pack committed — clone and build anywhere
- [x] IP: MIT code + all-rights-reserved assets, embedded stamps, authenticated invisible watermark (audited)
- [x] Desktop app (Tauri) for macOS/Windows/Linux; browser play via GitHub Pages
- [x] Phone-fit layout, session persistence, one-request spritesheet, composed load reveal
- [ ] v0.1.0 release binaries · PWA (offline/installable) · Android APK
- [x] Renderer finale — open the game with `?pack=grandpere` and it is written in his hand, on his paper
