# Publishing to Google Play

Everything decided and everything still to do, so none of it has to be worked
out again when the moment comes.

## The decisions, and why

| | |
|---|---|
| **Account type** | **Individual** — "deLaPatada Software" is a name, not a registered company, and a D-U-N-S number needs a real entity. Registering a business to avoid a 14-day test would be a bad trade. |
| **Developer display name** | `deLaPatada Software` — allowed on an individual account; the legal name stays private with Google. No `Ltd`/`Inc` suffix, which would imply a company that does not exist. |
| **Price** | **Free, and free forever.** Play allows paid→free but **never free→paid**, so this is one-way. The tribute should not have a till on it, and the browser version is free anyway. |
| **Ads** | None, ever. An ad SDK would need `INTERNET`, collect the advertising ID, require a consent banner, and make "collects no data" untrue — for an income of approximately nothing. |
| **Trader status** | **Non-trader**, which keeps a home address off the public listing. Note it is **account-level**: the day anything is sold under this account, trader details appear on *every* listing, this one included. Use a business or forwarding address then. |
| **Family copies** | Promo codes (up to 500 a quarter) if a future app is ever paid. Not needed while this one is free. |

## What Play requires

- [ ] **$25** one-off registration, and **identity verification** (slow — start first)
- [ ] **Privacy policy URL** → `https://bdcbqa314159.github.io/a-souvenir-of-sudokus/privacy.html` (in this repo, `web/privacy.html`)
- [ ] **App icon** 512×512 — from `web/src-tauri/icons/icon.png`
- [ ] **Feature graphic** 1024×500 — to be made
- [ ] **At least 2 phone screenshots**
- [ ] **Content rating** questionnaire — a puzzle game, no violence, no user content
- [ ] **Data safety form** — *no data collected, no data shared.* Literally true: the release manifest asks for no permissions at all
- [ ] **Target audience** — all ages; no ads declaration
- [ ] **A signed AAB** — needs the keystore (below)
- [ ] **Production access** — a closed test with **12+ testers opted in for 14 continuous days**. Granted to the *account*, so it happens once and never again for later apps.

## The keystore

Yours to create, in Studio (Build → Generate Signed Bundle / APK) or with
`keytool`. Then put it where Gradle can find it and **never commit either file**:

```
web/src-tauri/gen/android/keystore.properties   (gitignored by the project already)
```

```properties
storeFile=/absolute/path/to/souvenir-upload.jks
storePassword=…
keyAlias=upload
keyPassword=…
```

Under **Play App Signing** (the default) this is only an *upload key*: Google
holds the real app signing key, and if you lose the upload key Google can reset
it. That makes it a far smaller risk than `originals/atelier-work/updater.key`,
which is genuinely unrecoverable. Back it up anyway.

## Each upload

- **versionCode must increase every time.** It derives from the version in
  `web/src-tauri/tauri.conf.json`, so a Play release means a version bump — the
  same four-file bump as v0.2.0 (`tauri.conf.json`, both `Cargo.toml` with their
  lockfiles, `engine/CMakeLists.txt`).
- Build with `./scripts/android.sh aab` — **never** with `--family`. A store
  artifact must carry the generated pack, never his real handwriting.

## Listing copy

### Short description (80 characters max)

- **EN** — `Sudoku written in my grandfather's own hand.` (43)
- **FR** — `Un sudoku écrit de la main de mon grand-père.` (45)
- **ES** — `Un sudoku escrito con la letra de mi abuelo.` (44)

### Full description

**EN**

> A sudoku game dedicated to my grandfather.
>
> Every digit you see is his. His handwriting was photographed, cut into
> thousands of glyphs, and used to generate a typeface — so the grids are
> written in his hand, on paper drawn from his own notebook. No two digits are
> quite alike, because his never were.
>
> Three ways to play:
>
> • **Classic** — sudoku as it should be, with pencil marks, hints you can aim
> at a square, and a check that tells you how many digits are wrong without
> telling you which.
> • **Phantom** — a pact with the clock. Stall too long and the grid flips into
> its phantom twin. Three lives, no help.
> • **Hangman** — every digit is interrogated the moment it lands. Right, and
> he stays as he is. Wrong, and he draws one more stroke of himself. Ten
> mistakes finish the drawing.
>
> A clock on every grid, and a ledger of the ones you have finished — you can
> open any of them again and see it exactly as you left it.
>
> English, French and Spanish, switchable mid-game.
>
> No account. No ads. No data collected — the app asks for no permissions at
> all and never touches the network. Your games are saved on your own device
> and nowhere else.
>
> Also playable free in any browser:
> https://bdcbqa314159.github.io/a-souvenir-of-sudokus/

**FR**

> Un jeu de sudoku dédié à mon grand-père.
>
> Chaque chiffre que vous voyez est le sien. Son écriture a été photographiée,
> découpée en milliers de signes, puis utilisée pour générer une typographie —
> les grilles sont donc écrites de sa main, sur un papier tiré de son propre
> cahier. Aucun chiffre n'est tout à fait identique à un autre, parce que les
> siens ne l'étaient jamais.
>
> Trois façons de jouer :
>
> • **Classique** — le sudoku comme il doit être : crayon pour les notes,
> indices que l'on peut viser sur une case, et une vérification qui dit combien
> de chiffres sont faux sans dire lesquels.
> • **Fantôme** — un pacte avec l'horloge. Tardez trop et la grille bascule
> dans son double fantôme. Trois vies, aucune aide.
> • **Pendu** — chaque chiffre est interrogé dès qu'il tombe. Juste, il ne
> bouge pas. Faux, il dessine un trait de plus de lui-même. Dix erreurs
> achèvent le dessin.
>
> Une horloge sur chaque grille, et un registre de celles que vous avez
> terminées — vous pouvez en rouvrir n'importe laquelle et la revoir telle que
> vous l'avez laissée.
>
> Anglais, français et espagnol, au choix en cours de partie.
>
> Aucun compte. Aucune publicité. Aucune donnée collectée — l'application ne
> demande aucune autorisation et ne touche jamais au réseau. Vos parties sont
> enregistrées sur votre appareil, et nulle part ailleurs.
>
> Jouable aussi gratuitement dans un navigateur :
> https://bdcbqa314159.github.io/a-souvenir-of-sudokus/

**ES**

> Un juego de sudoku dedicado a mi abuelo.
>
> Cada dígito que ves es suyo. Su letra fue fotografiada, recortada en miles de
> signos y usada para generar una tipografía — así que las cuadrículas están
> escritas con su mano, sobre un papel tomado de su propio cuaderno. Ningún
> dígito es exactamente igual a otro, porque los suyos nunca lo fueron.
>
> Tres maneras de jugar:
>
> • **Clásico** — el sudoku como debe ser: lápiz para las notas, pistas que
> puedes señalar en una casilla, y una comprobación que dice cuántos dígitos
> están mal sin decir cuáles.
> • **Fantasma** — un pacto con el reloj. Si tardas demasiado, la cuadrícula se
> vuelca en su gemela fantasma. Tres vidas, sin ayuda.
> • **Ahorcado** — cada dígito se interroga en cuanto cae. Bien, y él sigue
> igual. Mal, y dibuja un trazo más de sí mismo. Diez errores terminan el
> dibujo.
>
> Un reloj en cada cuadrícula, y un registro de las que has terminado — puedes
> volver a abrir cualquiera y verla tal como la dejaste.
>
> Inglés, francés y español, cambiables en mitad de la partida.
>
> Sin cuenta. Sin publicidad. Sin datos recogidos — la aplicación no pide
> ningún permiso y nunca toca la red. Tus partidas se guardan en tu propio
> dispositivo y en ningún otro sitio.
>
> También se puede jugar gratis en cualquier navegador:
> https://bdcbqa314159.github.io/a-souvenir-of-sudokus/

## Still to do in the repo

- [ ] Feature graphic, 1024×500
- [ ] Two phone screenshots
- [ ] Gradle signing config reading `keystore.properties`
- [ ] A CI job building the AAB from a clean clone, so a store artifact can
      never be a local build (and so never his handwriting)
- [ ] Replace `CONTACT_EMAIL` in `web/privacy.html` with the support address
