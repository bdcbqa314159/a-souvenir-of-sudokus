//! a-souvenir-of-sudokus — browser frontend (Leptos, CSR).
//!
//! All game logic lives in the C++ engine, loaded as wasm by index.html and
//! reached through one JS function: `souvenir_cmd(requestJson) -> responseJson`
//! (command set: engine/include/souvenir/api.hpp). This crate is UI only.
//! Rendering is asset-pack driven: digits are images picked from
//! assets/<pack>/manifest.json — the placeholder pack today, grand-père's
//! handwriting at the end.

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::{json, Value};

mod engine;
mod i18n;
mod pack;
mod phantom;
mod session;

use engine::{board_of, cmd, solved_game, Manifest};
use i18n::{detect_lang, diff_label, fill, save_lang, t, translate_engine_error, Lang, Mode};
use pack::{digit_style, load_pack, pack, paper_store, sprite_url, url_pack};
use phantom::{stall_ms, window_ms, Phantom, LIVES};
use session::{clear_session, load_session, save_session};

const DIFFICULTIES: [&str; 3] = ["easy", "medium", "hard"];
const HINTS: u32 = 3; // per game; check-undo would be a brute-force oracle if free
const CHECKS: u32 = 3;
const DIGIT_KEYS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

#[derive(Clone, Default)]
struct History {
    undo: Vec<Value>,
    redo: Vec<Value>,
}

#[component]
fn App() -> impl IntoView {
    let game: RwSignal<Option<Value>> = RwSignal::new(None);
    let manifest: RwSignal<Option<Manifest>> = RwSignal::new(None);
    let selected = RwSignal::new(40usize);
    let pencil = RwSignal::new(false);
    let msg = RwSignal::new(String::new());
    let history = RwSignal::new(History::default());
    let lang = RwSignal::new(detect_lang());
    let mode = RwSignal::new(Mode::Classic);
    let ph = Phantom::new();
    let now = RwSignal::new(js_sys::Date::now());
    let hints_left = RwSignal::new(HINTS);
    let checks_left = RwSignal::new(CHECKS);
    let leave_arm: RwSignal<Option<f64>> = RwSignal::new(None); // phantom exit confirm
                                                                // dev mode (?dev in the URL): assists uncapped — the debugging use survives
    let dev = window()
        .location()
        .search()
        .unwrap_or_default()
        .contains("dev");
    let flip_anim = RwSignal::new(false);
    // classic/souvenir toggle: shown only when a souvenir pack exists;
    // remembers WHICH one loaded (abuelo or souvenir) as the toggle target
    let has_souvenir = RwSignal::new(false);
    let souvenir_pack: RwSignal<String> = RwSignal::new("assets/souvenir".into());
    // autosave arms only after restore has had its chance (see Effect below)
    let session_ready = RwSignal::new(false);
    let show_rules = RwSignal::new(false);
    // composed reveal: a splash covers the page until the engine, the game,
    // the paper AND the sprite are all ready — then one fade shows the
    // finished board. Loading never performs its scatter in public.
    let paper_ok = RwSignal::new(false);
    let sprite_ok = RwSignal::new(false);
    let preload: RwSignal<(Option<String>, Option<String>)> = RwSignal::new((None, None));

    // engine ready (index.html sets window.souvenir_cmd) + manifest fetched -> first game
    spawn_local(async move {
        loop {
            if cmd(json!({"cmd": "load"}))["error"] != "engine not loaded" {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(50).await;
        }
        let candidates = match url_pack() {
            Some(p) => vec![p],
            // abuelo (real scans, family builds from the main machine) wins
            // over souvenir (generated pack, committed to the public repo)
            None => vec![
                "assets/abuelo".into(),
                "assets/souvenir".into(),
                "assets/placeholder".into(),
            ],
        };
        let mut loaded = false;
        for cand in candidates {
            if load_pack(cand, manifest).await {
                loaded = true;
                break;
            }
        }
        if !loaded {
            msg.set(t(lang.get_untracked()).no_pack.into());
        }
        if pack() != "assets/placeholder" {
            souvenir_pack.set(pack());
            has_souvenir.set(true);
        }
        let (p_url, s_url) = (paper_store().lock().unwrap().clone(), sprite_url());
        if p_url.is_none() {
            paper_ok.set(true); // nothing to wait for
        }
        if s_url.is_none() {
            sprite_ok.set(true);
        }
        preload.set((p_url, s_url));
        // refresh resumes: restore the saved session if the engine accepts
        // its game state (localStorage is user-editable — garbage falls
        // through to a fresh game, silently)
        let restored = load_session().is_some_and(|s| {
            let g = s["game"].clone();
            if cmd(json!({"cmd": "check", "game": g}))["ok"].as_bool() != Some(true) {
                return false;
            }
            game.set(Some(s["game"].clone()));
            hints_left.set(
                s["hints"]
                    .as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(HINTS)
                    .min(HINTS),
            );
            checks_left.set(
                s["checks"]
                    .as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(CHECKS)
                    .min(CHECKS),
            );
            ph.lives.set(
                s["lives"]
                    .as_u64()
                    .map(|v| v as u32)
                    .unwrap_or(LIVES)
                    .min(LIVES),
            );
            ph.over.set(s["over"].as_bool().unwrap_or(false));
            if s["mode"] == "phantom" {
                mode.set(Mode::Phantom);
                // re-anchor the stall clock: the pact resumes from NOW, a
                // refresh must never hand the phantom an expired timer
                ph.last_progress.set(js_sys::Date::now());
                ph.clear_haunt();
            }
            true
        });
        if !restored {
            let rsp = cmd(json!({"cmd": "new", "difficulty": "medium"}));
            if rsp["ok"].as_bool() == Some(true) {
                game.set(Some(rsp["game"].clone()));
            } else {
                msg.set(rsp["error"].as_str().unwrap_or("engine error").to_string());
            }
        }
        session_ready.set(true);
    });

    // autosave: any change to the session-relevant signals writes the slot.
    // Gated on session_ready so the pre-restore defaults never clobber a
    // saved session during startup.
    Effect::new(move |_| {
        let Some(g) = game.get() else { return };
        let m = mode.get();
        let blob = json!({
            "game": g,
            "mode": if m == Mode::Phantom { "phantom" } else { "classic" },
            "lives": ph.lives.get(),
            "over": ph.over.get(),
            "hints": hints_left.get(),
            "checks": checks_left.get(),
        });
        if session_ready.get_untracked() {
            save_session(&blob);
        }
    });

    // phantom clock: every 500ms refresh `now`; on stall expiry, the sudoku flips
    spawn_local(async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(500).await;
            now.set(js_sys::Date::now());
            if mode.get_untracked() != Mode::Phantom || ph.over.get_untracked() {
                continue;
            }
            let Some(g) = game.get_untracked() else {
                continue;
            };
            if solved_game(&g) {
                continue; // solved — the phantoms lost
            }
            let diff = g["difficulty"].as_str().unwrap_or("medium").to_string();
            match ph.haunt_start.get_untracked() {
                None => {
                    // calm: has the stall clock run out? then the haunting begins —
                    // and a preview of the incoming puzzle is drawn as the ghost
                    if js_sys::Date::now() - ph.last_progress.get_untracked() >= stall_ms(&diff) {
                        let mut req = json!({"cmd": "phantom"});
                        req["game"] = g;
                        let rsp = cmd(req);
                        if rsp["ok"].as_bool() == Some(true) {
                            ph.incoming.set(Some(rsp["game"].clone()));
                            ph.haunt_start.set(Some(js_sys::Date::now()));
                            let secs = ((window_ms(&diff) / 1000.0) as u64).to_string();
                            msg.set(fill(t(lang.get_untracked()).coming, &secs));
                        }
                    }
                    continue;
                }
                Some(t0) => {
                    // haunting: still inside the grace window?
                    if js_sys::Date::now() - t0 < window_ms(&diff) {
                        continue;
                    }
                    ph.clear_haunt(); // window expired — the flip goes through
                }
            }
            // recompute from the LIVE board (not the haunt-start preview): undo
            // during the haunt changes the correct-cell count, and the flip must
            // preserve the player's real coverage
            let mut req = json!({"cmd": "phantom"});
            req["game"] = g;
            let rsp = cmd(req);
            if rsp["ok"].as_bool() == Some(true) {
                game.set(Some(rsp["game"].clone()));
                flip_anim.set(true);
                spawn_local(async move {
                    gloo_timers::future::TimeoutFuture::new(700).await;
                    flip_anim.set(false);
                });
                history.set(History::default()); // no undoing your way out of a phantom
                ph.last_progress.set(js_sys::Date::now());
                ph.lives.update(|l| *l = l.saturating_sub(1));
                if ph.lives.get_untracked() == 0 {
                    ph.over.set(true);
                    msg.set(t(lang.get_untracked()).won.into());
                } else {
                    msg.set(fill(
                        t(lang.get_untracked()).phantomed,
                        &ph.lives.get_untracked().to_string(),
                    ));
                }
            }
        }
    });

    // run an engine command against the current game; push undo on success.
    // Returns whether the engine accepted it — callers must not treat a
    // rejected command as if something happened.
    let play = move |req: Value| -> bool {
        let Some(g) = game.get_untracked() else {
            msg.set(t(lang.get_untracked()).engine_loading.into());
            return false;
        };
        let mut full = req;
        full["game"] = g.clone();
        let rsp = cmd(full);
        if rsp["ok"].as_bool() == Some(true) {
            history.update(|h| {
                h.undo.push(g);
                h.redo.clear();
            });
            game.set(Some(rsp["game"].clone()));
            if rsp["solved"].as_bool() == Some(true) {
                msg.set(t(lang.get_untracked()).solved.into());
                clear_session(); // a solved game is not worth resuming
            }
            true
        } else {
            let err = rsp["error"].as_str().unwrap_or("engine error");
            msg.set(translate_engine_error(lang.get_untracked(), err));
            false
        }
    };

    let new_game = move |difficulty: String| {
        let rsp = cmd(json!({"cmd": "new", "difficulty": difficulty}));
        if rsp["ok"].as_bool() == Some(true) {
            game.set(Some(rsp["game"].clone()));
            history.set(History::default());
            ph.reset();
            hints_left.set(HINTS);
            checks_left.set(CHECKS);
            let l = lang.get_untracked();
            msg.set(fill(t(l).new_game_msg, diff_label(l, &difficulty)));
        } else {
            let err = rsp["error"].as_str().unwrap_or("engine error");
            msg.set(translate_engine_error(lang.get_untracked(), err));
        }
    };

    let undo = move || {
        history.update(|h| {
            if let Some(prev) = h.undo.pop() {
                if let Some(cur) = game.get_untracked() {
                    h.redo.push(cur);
                }
                game.set(Some(prev));
            } else {
                msg.set(t(lang.get_untracked()).nothing_undo.into());
            }
        });
    };
    let redo = move || {
        history.update(|h| {
            if let Some(next) = h.redo.pop() {
                if let Some(cur) = game.get_untracked() {
                    h.undo.push(cur);
                }
                game.set(Some(next));
            } else {
                msg.set(t(lang.get_untracked()).nothing_redo.into());
            }
        });
    };

    // shared gate for capped assists: false = refused (with the reason in msg).
    // Charging happens at the call site, only after the operation succeeds.
    let assist_allowed = move |left: RwSignal<u32>, hints: bool| -> bool {
        if dev {
            return true;
        }
        let tr = t(lang.get_untracked());
        let noun = if hints { tr.hints_noun } else { tr.checks_noun };
        if mode.get_untracked() == Mode::Phantom {
            msg.set(fill(tr.no_assist_phantom, noun));
            return false;
        }
        if left.get_untracked() == 0 {
            msg.set(fill(tr.none_left, noun));
            return false;
        }
        true
    };

    let key_action = move |key: &str| {
        let i = selected.get_untracked();
        if ph.over.get_untracked() && key != "n" {
            return; // only a new game raises the dead
        }
        match key {
            "h" | "ArrowLeft" if i % 9 > 0 => selected.set(i - 1),
            "l" | "ArrowRight" if i % 9 < 8 => selected.set(i + 1),
            "k" | "ArrowUp" if i > 8 => selected.set(i - 9),
            "j" | "ArrowDown" if i < 72 => selected.set(i + 9),
            "m" => pencil.update(|p| *p = !*p),
            "x" | "0" | "Backspace" | "Delete" => {
                let clearing_value = game
                    .get_untracked()
                    .map(|g| board_of(&g, "board")[i] != 0)
                    .unwrap_or(false);
                if clearing_value {
                    play(json!({"cmd": "put", "i": i, "v": 0}));
                } else {
                    play(json!({"cmd": "clear_marks", "i": i}));
                }
            }
            "u" => undo(),
            "r" => redo(),
            "H" => {
                if !assist_allowed(hints_left, true) {
                    return;
                }
                // charge only when the hint actually happened
                if play(json!({"cmd": "hint"})) && !dev {
                    hints_left.update(|h| *h -= 1);
                }
            }
            // dev only: fill the whole board via hints, to see the glyphs in action
            "S" if dev => {
                let empties = game
                    .get_untracked()
                    .map(|g| board_of(&g, "board").iter().filter(|v| **v == 0).count())
                    .unwrap_or(0);
                for _ in 0..empties {
                    if !play(json!({"cmd": "hint"})) {
                        break;
                    }
                }
            }
            "c" => {
                if !assist_allowed(checks_left, false) {
                    return;
                }
                let Some(g) = game.get_untracked() else {
                    msg.set(t(lang.get_untracked()).engine_loading.into());
                    return;
                };
                let mut req = json!({"cmd": "check"});
                req["game"] = g;
                let rsp = cmd(req);
                let tr = t(lang.get_untracked());
                if rsp["ok"].as_bool() == Some(true) {
                    if !dev {
                        checks_left.update(|c| *c -= 1);
                    }
                    let n = rsp["wrong"].as_array().map(Vec::len).unwrap_or(0);
                    msg.set(if n == 0 {
                        tr.all_good.into()
                    } else {
                        fill(tr.wrong_cells, &n.to_string())
                    });
                } else {
                    let err = rsp["error"].as_str().unwrap_or("engine error");
                    msg.set(translate_engine_error(lang.get_untracked(), err));
                }
            }
            "n" => {
                let d = game
                    .get_untracked()
                    .and_then(|g| g["difficulty"].as_str().map(String::from))
                    .unwrap_or_else(|| "medium".into());
                new_game(d);
            }
            d @ ("1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") => {
                let v: i64 = d.parse().unwrap();
                let mark = pencil.get_untracked();
                let before = game
                    .get_untracked()
                    .map(|g| board_of(&g, "board")[i])
                    .unwrap_or(0);
                let accepted =
                    play(json!({"cmd": if mark { "mark" } else { "put" }, "i": i, "v": v}));
                // progress = the engine ACCEPTED a NEW, correct placement.
                // A rejected keystroke on a given, or retyping an already-correct
                // digit, must never ward the phantom off.
                if accepted && !mark && before != v {
                    if let Some(g) = game.get_untracked() {
                        if board_of(&g, "board")[i] == v && board_of(&g, "solution")[i] == v {
                            ph.last_progress.set(js_sys::Date::now());
                            if ph.haunt_start.get_untracked().is_some() {
                                ph.clear_haunt();
                                msg.set(t(lang.get_untracked()).recedes.into());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    };

    window_event_listener(leptos::ev::keydown, move |ev| {
        let key = ev.key();
        if !ev.ctrl_key() && !ev.meta_key() && !ev.alt_key() {
            if matches!(
                key.as_str(),
                "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Backspace"
            ) {
                ev.prevent_default();
            }
            key_action(&key);
            msg.track(); // keep message reactive scope alive
        }
    });

    let cells = move || {
        let (Some(g), Some(man)) = (game.get(), manifest.get()) else {
            return Vec::new();
        };
        let board = board_of(&g, "board");
        let puzzle = board_of(&g, "puzzle");
        let marks: Vec<Vec<i64>> = g["marks"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| {
                        c.as_array()
                            .map(|d| d.iter().filter_map(Value::as_i64).collect())
                            .unwrap_or_default()
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![vec![]; 81]);
        let sel = selected.get();
        let sel_value = board[sel];
        // the ghost: while haunted, the incoming puzzle's givens materialize over
        // the grid, solidifying as the grace window runs out
        let ghost: Option<(Vec<i64>, f64)> =
            ph.haunt_start
                .get()
                .zip(ph.incoming.get())
                .map(|(t0, inc)| {
                    let diff = g["difficulty"].as_str().unwrap_or("medium");
                    let progress = ((now.get() - t0) / window_ms(diff)).clamp(0.0, 1.0);
                    (board_of(&inc, "puzzle"), 0.08 + 0.45 * progress)
                });
        (0..81usize)
            .map(|i| {
                let v = board[i];
                let role = if puzzle[i] != 0 { "given" } else { "user" };
                let mut class = String::from("cell");
                if i % 9 == 2 || i % 9 == 5 {
                    class.push_str(" cross-r");
                }
                if i / 9 == 2 || i / 9 == 5 {
                    class.push_str(" cross-b");
                }
                if i == sel {
                    class.push_str(" selected");
                } else if v != 0 && v == sel_value {
                    class.push_str(" same");
                }
                let value_img = (v != 0)
                    .then(|| digit_style(&man, role, v, i))
                    .flatten()
                    .map(|style| view! { <div class="value glyph" style=style></div> });
                let ghost_img = ghost.as_ref().and_then(|(inc_puzzle, opacity)| {
                    let gv = inc_puzzle[i];
                    (gv != 0)
                        .then(|| digit_style(&man, "given", gv, i))
                        .flatten()
                        .map(|style| {
                            view! { <div class="ghost glyph" style=format!("{style};opacity:{opacity:.2}")></div> }
                        })
                });
                let mark_imgs = (v == 0)
                    .then(|| {
                        marks[i]
                            .iter()
                            .filter_map(|d| digit_style(&man, "user", *d, i))
                            .map(|style| view! { <div class="glyph" style=style></div> })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                view! {
                    <div class=class on:click=move |_| selected.set(i)>
                        {value_img}
                        {ghost_img}
                        <div class="marks">{mark_imgs}</div>
                    </div>
                }
            })
            .collect::<Vec<_>>()
    };

    let status = move || {
        let tr = t(lang.get());
        game.get()
            .map(|g| {
                let board = board_of(&g, "board");
                let filled = board.iter().filter(|v| **v != 0).count();
                let diff = g["difficulty"].as_str().unwrap_or("medium").to_string();
                let mut s = format!(
                    "{} · {filled}/81 · {}",
                    diff_label(lang.get(), &diff),
                    if pencil.get() { tr.pencil_mode } else { tr.pen }
                );
                if dev {
                    s.push_str(" · dev");
                }
                if mode.get() == Mode::Phantom {
                    let hearts = "♥".repeat(ph.lives.get() as usize);
                    match ph.haunt_start.get() {
                        Some(t0) => {
                            let remain = ((window_ms(&diff) - (now.get() - t0)) / 1000.0).max(0.0);
                            s.push_str(&format!(
                                " · {hearts} · {}",
                                fill(tr.haunted, &(remain as u64).to_string())
                            ));
                        }
                        None => {
                            let remain = ((stall_ms(&diff) - (now.get() - ph.last_progress.get()))
                                / 1000.0)
                                .max(0.0);
                            s.push_str(&format!(
                                " · {hearts} · {}",
                                fill(tr.phantom_in, &(remain as u64).to_string())
                            ));
                        }
                    }
                }
                s
            })
            .unwrap_or_else(|| tr.loading.into())
    };
    let solved = move || game.get().map(|g| solved_game(&g)).unwrap_or(false);

    // the palette: 1-9 in his (placeholder, for now) red handwriting — click to
    // write, exactly like typing the key. A digit fully placed on the board dims.
    let palette = move || {
        let man = manifest.get()?;
        let placed: Vec<usize> = game
            .get()
            .map(|g| {
                let board = board_of(&g, "board");
                (1..=9)
                    .map(|d| board.iter().filter(|v| **v == d).count())
                    .collect()
            })
            .unwrap_or_else(|| vec![0; 9]);
        Some(
            (1..=9usize)
                .map(|d| {
                    let done = placed[d - 1] >= 9;
                    let style = digit_style(&man, "user", d as i64, d * 7);
                    view! {
                        <button class="digit" class:done=move || done
                            on:click=move |_| key_action(DIGIT_KEYS[d - 1])>
                            {style.map(|s| view! { <div class="glyph" style=s></div> })}
                        </button>
                    }
                })
                .collect::<Vec<_>>(),
        )
    };

    // entering phantom is one click; leaving is deliberate — click twice within 3s.
    // Lives and game-over persist for the whole game: toggling modes must never
    // resurrect a lost run (only `n` does).
    let toggle_phantom = move |_| {
        let tr = t(lang.get_untracked());
        if ph.over.get_untracked() {
            msg.set(tr.won.into());
            return;
        }
        if mode.get_untracked() == Mode::Phantom {
            let armed_recently = leave_arm
                .get_untracked()
                .is_some_and(|t| js_sys::Date::now() - t < 3_000.0);
            if !armed_recently {
                leave_arm.set(Some(js_sys::Date::now()));
                msg.set(tr.leave_confirm.into());
                return;
            }
            mode.set(Mode::Classic);
            msg.set(tr.back_classic.into());
        } else {
            mode.set(Mode::Phantom);
            msg.set(tr.phantom_on.into());
        }
        leave_arm.set(None);
        ph.clear_haunt();
        ph.last_progress.set(js_sys::Date::now());
    };
    let current_diff = move || {
        game.get()
            .and_then(|g| g["difficulty"].as_str().map(String::from))
            .unwrap_or_default()
    };

    let ready = move || game.get().is_some() && paper_ok.get() && sprite_ok.get();

    view! {
        <div class="splash" class:hidden=ready>
            <h1>"a-souvenir-of-sudokus"</h1>
            <div class="dots"><span>"●"</span><span>"●"</span><span>"●"</span></div>
        </div>
        {move || {
            let (p, s) = preload.get();
            view! {
                {p.map(|u| view! { <img class="preload" src=u
                    on:load=move |_| paper_ok.set(true)
                    on:error=move |_| paper_ok.set(true) /> })}
                {s.map(|u| view! { <img class="preload" src=u
                    on:load=move |_| sprite_ok.set(true)
                    on:error=move |_| sprite_ok.set(true) /> })}
            }
        }}
        <div class="app" class:ready=ready>
        <h1>"a-souvenir-of-sudokus"</h1>
        <div class="grid" class:flipping=move || flip_anim.get()>{cells}</div>
        <div class="bar palette">
            {palette}
            <button class="digit erase" title=move || t(lang.get()).erase on:click=move |_| key_action("x")>"⌫"</button>
        </div>
        <div class="bar">
            <button class:on=move || pencil.get() on:click=move |_| key_action("m")>
                {move || t(lang.get()).pencil}
            </button>
            <button
                disabled=move || !dev && (mode.get() == Mode::Phantom || hints_left.get() == 0)
                on:click=move |_| key_action("H")
            >
                {move || {
                    let tr = t(lang.get());
                    if dev { tr.hint.into() } else { format!("{} ({})", tr.hint, hints_left.get()) }
                }}
            </button>
            <button
                disabled=move || !dev && (mode.get() == Mode::Phantom || checks_left.get() == 0)
                on:click=move |_| key_action("c")
            >
                {move || {
                    let tr = t(lang.get());
                    if dev { tr.check.into() } else { format!("{} ({})", tr.check, checks_left.get()) }
                }}
            </button>
            <button on:click=move |_| key_action("u")>{move || t(lang.get()).undo}</button>
            <button on:click=move |_| key_action("r")>{move || t(lang.get()).redo}</button>
            {dev.then(|| view! { <button on:click=move |_| key_action("S")>"solve"</button> })}
        </div>
        <div class="bar setup">
            <button on:click=move |_| key_action("n")>{move || t(lang.get()).new_game}</button>
            <span class="seg">
                {DIFFICULTIES
                    .iter()
                    .map(|d| {
                        let d = d.to_string();
                        let label = {
                            let d = d.clone();
                            move || diff_label(lang.get(), &d)
                        };
                        let is_current = {
                            let d = d.clone();
                            move || current_diff() == d
                        };
                        view! {
                            <button
                                class:on=is_current
                                disabled=move || mode.get() == Mode::Phantom
                                title=move || t(lang.get()).diff_hint
                                on:click=move |_| new_game(d.clone())
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </span>
            <button class:on=move || mode.get() == Mode::Phantom on:click=toggle_phantom>
                {move || t(lang.get()).phantom}
            </button>
            {move || has_souvenir.get().then(|| {
                let on_souvenir = move || pack() != "assets/placeholder";
                view! {
                    <button on:click=move |_| {
                        let target = if on_souvenir() {
                            "assets/placeholder".to_string()
                        } else {
                            souvenir_pack.get_untracked()
                        };
                        spawn_local(async move {
                            load_pack(target, manifest).await;
                        });
                    }>
                        {move || {
                            // read manifest so the label re-renders on switch
                            manifest.track();
                            if on_souvenir() { t(lang.get()).classic.to_string() } else { "souvenir".to_string() }
                        }}
                    </button>
                }
            })}
            <span class="seg">
                {[(Lang::Fr, "FR"), (Lang::En, "EN"), (Lang::Es, "ES")]
                    .into_iter()
                    .map(|(l, label)| {
                        view! {
                            <button
                                class:on=move || lang.get() == l
                                on:click=move |_| {
                                    lang.set(l);
                                    save_lang(l);
                                }
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </span>
            <button title=move || t(lang.get()).rules_btn on:click=move |_| show_rules.set(true)>"?"</button>
        </div>
        <div class="msg" class:solved=solved>{move || msg.get()}</div>
        <div class="status">{status}</div>
        </div>
        {move || show_rules.get().then(|| view! {
            <div class="rules-overlay" on:click=move |_| show_rules.set(false)>
                <div class="rules-card" on:click=|e| e.stop_propagation()>
                    <h1>"a-souvenir-of-sudokus"</h1>
                    <h2>{move || t(lang.get()).classic}</h2>
                    <p>{move || t(lang.get()).rules_classic}</p>
                    <h2>{move || t(lang.get()).phantom}</h2>
                    <p>{move || t(lang.get()).rules_phantom}</p>
                    <button on:click=move |_| show_rules.set(false)>{move || t(lang.get()).close}</button>
                </div>
            </div>
        })}
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
