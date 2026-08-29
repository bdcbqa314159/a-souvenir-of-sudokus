//! a-souvenir-of-sudokus — browser frontend (Leptos, CSR).
//!
//! All game logic lives in the C++ engine, loaded as wasm by index.html and
//! reached through one JS function: `souvenir_cmd(requestJson) -> responseJson`
//! (command set: engine/include/souvenir/api.hpp). This crate is UI only.
//! Rendering is asset-pack driven: digits are images picked from
//! assets/<pack>/manifest.json — the placeholder pack today, grand-père's
//! handwriting at the end.

use std::collections::HashMap;

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = souvenir_cmd, catch)]
    fn souvenir_cmd(request: &str) -> Result<String, JsValue>;
}

/// Asset pack resolution. ?pack=<name> in the URL always wins; otherwise the
/// loader probes grandpere first (so the packaged desktop app opens straight
/// into his handwriting) and falls back to the placeholder. The loader sets
/// the store before the first render reads it.
fn pack_store() -> &'static std::sync::OnceLock<String> {
    static PACK: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    &PACK
}

fn pack() -> &'static str {
    pack_store().get().map(String::as_str).unwrap_or("assets/placeholder")
}

fn url_pack() -> Option<String> {
    let search = window().location().search().unwrap_or_default();
    for kv in search.trim_start_matches('?').split('&') {
        if let Some(name) = kv.strip_prefix("pack=") {
            if !name.is_empty()
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Some(format!("assets/{name}"));
            }
        }
    }
    None
}
const DIFFICULTIES: [&str; 3] = ["easy", "medium", "hard"];
const LIVES: u32 = 3;
const HINTS: u32 = 3; // per game; check-undo would be a brute-force oracle if free
const CHECKS: u32 = 3;
const DIGIT_KEYS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Classic,
    Phantom,
}

/// Time without a correct placement before the haunting begins, per difficulty.
fn stall_ms(difficulty: &str) -> f64 {
    match difficulty {
        "easy" => 90_000.0,
        "hard" => 45_000.0,
        _ => 60_000.0,
    }
}

/// The grace window: once haunted, how long you have to place a correct number.
fn window_ms(difficulty: &str) -> f64 {
    match difficulty {
        "easy" => 20_000.0,
        "hard" => 12_000.0,
        _ => 15_000.0,
    }
}

type Manifest = HashMap<String, HashMap<String, Vec<String>>>; // role -> digit -> variants

fn cmd(req: Value) -> Value {
    match souvenir_cmd(&req.to_string()) {
        Ok(s) => serde_json::from_str(&s)
            .unwrap_or_else(|e| json!({"ok": false, "error": format!("bad response: {e}")})),
        Err(_) => json!({"ok": false, "error": "engine not loaded"}),
    }
}

// Always 81 entries (non-integers become 0, wrong lengths fall back wholesale),
// so indexing with 0..81 can never panic on a malformed response.
fn board_of(game: &Value, key: &str) -> Vec<i64> {
    game[key]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_i64().unwrap_or(0)).collect::<Vec<_>>())
        .filter(|v| v.len() == 81)
        .unwrap_or_else(|| vec![0; 81])
}

/// Solved means a real, fully matching board — two all-zero fallbacks don't count.
fn solved_game(game: &Value) -> bool {
    let board = board_of(game, "board");
    board.iter().any(|v| *v != 0) && board == board_of(game, "solution")
}

/// Deterministic variant per (cell, digit): the page looks written, not stamped.
fn digit_src(manifest: &Manifest, role: &str, digit: i64, cell: usize) -> Option<String> {
    let variants = manifest.get(role)?.get(&digit.to_string())?;
    if variants.is_empty() {
        return None; // a sparse pack must degrade, not divide by zero
    }
    let v = variants.get((cell * 31 + digit as usize) % variants.len())?;
    Some(format!("{}/{v}", pack()))
}

#[derive(Clone, Default)]
struct History {
    undo: Vec<Value>,
    redo: Vec<Value>,
}

// Every phantom-mode signal in one bundle with one reset path, so the call
// sites can never drift apart again.
#[derive(Clone, Copy)]
struct Phantom {
    lives: RwSignal<u32>,
    over: RwSignal<bool>,
    haunt_start: RwSignal<Option<f64>>,
    incoming: RwSignal<Option<Value>>, // ghost preview shown during the haunt
    last_progress: RwSignal<f64>,
}

impl Phantom {
    fn new() -> Self {
        Self {
            lives: RwSignal::new(LIVES),
            over: RwSignal::new(false),
            haunt_start: RwSignal::new(None),
            incoming: RwSignal::new(None),
            last_progress: RwSignal::new(js_sys::Date::now()),
        }
    }
    fn clear_haunt(self) {
        self.haunt_start.set(None);
        self.incoming.set(None);
    }
    /// Full reset — only a NEW GAME may call this. Lives are per game;
    /// resetting them on mode toggles was an escape from "the phantoms won".
    fn reset(self) {
        self.lives.set(LIVES);
        self.over.set(false);
        self.clear_haunt();
        self.last_progress.set(js_sys::Date::now());
    }
}

#[component]
fn App() -> impl IntoView {
    let game: RwSignal<Option<Value>> = RwSignal::new(None);
    let manifest: RwSignal<Option<Manifest>> = RwSignal::new(None);
    let selected = RwSignal::new(40usize);
    let pencil = RwSignal::new(false);
    let msg = RwSignal::new(String::new());
    let history = RwSignal::new(History::default());
    let mode = RwSignal::new(Mode::Classic);
    let ph = Phantom::new();
    let now = RwSignal::new(js_sys::Date::now());
    let hints_left = RwSignal::new(HINTS);
    let checks_left = RwSignal::new(CHECKS);
    let leave_arm: RwSignal<Option<f64>> = RwSignal::new(None); // phantom exit confirm
    // dev mode (?dev in the URL): assists uncapped — the debugging use survives
    let dev = window().location().search().unwrap_or_default().contains("dev");
    let flip_anim = RwSignal::new(false);

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
            None => vec!["assets/grandpere".into(), "assets/placeholder".into()],
        };
        let mut loaded = false;
        for cand in candidates {
            let Ok(rsp) = gloo_net::http::Request::get(&format!("{cand}/manifest.json")).send().await
            else {
                continue;
            };
            let Ok(m) = rsp.json::<Value>().await else { continue };
            let Ok(parsed) = serde_json::from_value::<Manifest>(m["digits"].clone()) else {
                continue;
            };
            let _ = pack_store().set(cand);
            // a pack may bring its own paper — the page becomes his notebook
            if let Some(paper) = m["paper"].as_str() {
                if let Some(body) = document().body() {
                    let style = body.style();
                    let _ = style.set_property(
                        "background-image",
                        &format!("url('{}/{}')", pack(), paper),
                    );
                    let _ = style.set_property("background-size", "540px");
                }
            }
            manifest.set(Some(parsed));
            loaded = true;
            break;
        }
        if !loaded {
            msg.set("no asset pack could be loaded".into());
        }
        let rsp = cmd(json!({"cmd": "new", "difficulty": "medium"}));
        if rsp["ok"].as_bool() == Some(true) {
            game.set(Some(rsp["game"].clone()));
        } else {
            msg.set(rsp["error"].as_str().unwrap_or("engine error").to_string());
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
            let Some(g) = game.get_untracked() else { continue };
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
                            msg.set(format!(
                                "the phantom is coming — place a correct number within {}s to hold it off",
                                (window_ms(&diff) / 1000.0) as u64
                            ));
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
                    msg.set("the phantoms won — n for a new game".into());
                } else {
                    msg.set(format!(
                        "the sudoku phantomed — {} live(s) left",
                        ph.lives.get_untracked()
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
            msg.set("the engine is still loading…".into());
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
                msg.set("solved — grand-père would be proud.".into());
            }
            true
        } else {
            msg.set(rsp["error"].as_str().unwrap_or("engine error").to_string());
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
            msg.set(format!("new {difficulty} game"));
        } else {
            msg.set(rsp["error"].as_str().unwrap_or("engine error").to_string());
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
                msg.set("nothing to undo".into());
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
                msg.set("nothing to redo".into());
            }
        });
    };

    // shared gate for capped assists: false = refused (with the reason in msg).
    // Charging happens at the call site, only after the operation succeeds.
    let assist_allowed = move |left: RwSignal<u32>, what: &str| -> bool {
        if dev {
            return true;
        }
        if mode.get_untracked() == Mode::Phantom {
            msg.set(format!("no {what} in phantom mode — trust your hand"));
            return false;
        }
        if left.get_untracked() == 0 {
            msg.set(format!("no {what} left"));
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
                if !assist_allowed(hints_left, "hints") {
                    return;
                }
                // charge only when the hint actually happened
                if play(json!({"cmd": "hint"})) && !dev {
                    hints_left.update(|h| *h -= 1);
                }
            }
            "c" => {
                if !assist_allowed(checks_left, "checks") {
                    return;
                }
                let Some(g) = game.get_untracked() else {
                    msg.set("the engine is still loading…".into());
                    return;
                };
                let mut req = json!({"cmd": "check"});
                req["game"] = g;
                let rsp = cmd(req);
                if rsp["ok"].as_bool() == Some(true) {
                    if !dev {
                        checks_left.update(|c| *c -= 1);
                    }
                    let n = rsp["wrong"].as_array().map(Vec::len).unwrap_or(0);
                    msg.set(if n == 0 { "all good so far".into() } else { format!("{n} wrong cell(s)") });
                } else {
                    msg.set(rsp["error"].as_str().unwrap_or("engine error").to_string());
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
                let accepted = play(json!({"cmd": if mark { "mark" } else { "put" }, "i": i, "v": v}));
                // progress = the engine ACCEPTED a NEW, correct placement.
                // A rejected keystroke on a given, or retyping an already-correct
                // digit, must never ward the phantom off.
                if accepted && !mark && before != v {
                    if let Some(g) = game.get_untracked() {
                        if board_of(&g, "board")[i] == v && board_of(&g, "solution")[i] == v {
                            ph.last_progress.set(js_sys::Date::now());
                            if ph.haunt_start.get_untracked().is_some() {
                                ph.clear_haunt();
                                msg.set("the phantom recedes…".into());
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
            if matches!(key.as_str(), "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown" | "Backspace") {
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
                    .map(|c| c.as_array().map(|d| d.iter().filter_map(Value::as_i64).collect()).unwrap_or_default())
                    .collect()
            })
            .unwrap_or_else(|| vec![vec![]; 81]);
        let sel = selected.get();
        let sel_value = board[sel];
        // the ghost: while haunted, the incoming puzzle's givens materialize over
        // the grid, solidifying as the grace window runs out
        let ghost: Option<(Vec<i64>, f64)> = ph.haunt_start.get().zip(ph.incoming.get()).map(|(t0, inc)| {
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
                    .then(|| digit_src(&man, role, v, i))
                    .flatten()
                    .map(|src| view! { <img class="value" src=src /> });
                let ghost_img = ghost.as_ref().and_then(|(inc_puzzle, opacity)| {
                    let gv = inc_puzzle[i];
                    (gv != 0)
                        .then(|| digit_src(&man, "given", gv, i))
                        .flatten()
                        .map(|src| {
                            view! { <img class="ghost" src=src style=format!("opacity:{opacity:.2}") /> }
                        })
                });
                let mark_imgs = (v == 0)
                    .then(|| {
                        marks[i]
                            .iter()
                            .filter_map(|d| digit_src(&man, "user", *d, i))
                            .map(|src| view! { <img src=src /> })
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
        game.get()
            .map(|g| {
                let board = board_of(&g, "board");
                let filled = board.iter().filter(|v| **v != 0).count();
                let diff = g["difficulty"].as_str().unwrap_or("?").to_string();
                let mut s =
                    format!("{diff} · {filled}/81 · {}", if pencil.get() { "pencil" } else { "pen" });
                if dev {
                    s.push_str(" · dev");
                }
                if mode.get() == Mode::Phantom {
                    let hearts = "♥".repeat(ph.lives.get() as usize);
                    match ph.haunt_start.get() {
                        Some(t0) => {
                            let remain = ((window_ms(&diff) - (now.get() - t0)) / 1000.0).max(0.0);
                            s.push_str(&format!(
                                " · {hearts} · ⚠ HAUNTED — {}s to place a number",
                                remain as u64
                            ));
                        }
                        None => {
                            let remain = ((stall_ms(&diff) - (now.get() - ph.last_progress.get()))
                                / 1000.0)
                                .max(0.0);
                            s.push_str(&format!(" · {hearts} · phantom in {}s", remain as u64));
                        }
                    }
                }
                s
            })
            .unwrap_or_else(|| "loading engine…".into())
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
                    let src = digit_src(&man, "user", d as i64, d * 7);
                    view! {
                        <button class="digit" class:done=move || done
                            on:click=move |_| key_action(DIGIT_KEYS[d - 1])>
                            {src.map(|s| view! { <img src=s /> })}
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
        if ph.over.get_untracked() {
            msg.set("the phantoms won — start a new game (n)".into());
            return;
        }
        if mode.get_untracked() == Mode::Phantom {
            let armed_recently =
                leave_arm.get_untracked().is_some_and(|t| js_sys::Date::now() - t < 3_000.0);
            if !armed_recently {
                leave_arm.set(Some(js_sys::Date::now()));
                msg.set("leave phantom mode? click phantom again to confirm".into());
                return;
            }
            mode.set(Mode::Classic);
            msg.set("back to classic".into());
        } else {
            mode.set(Mode::Phantom);
            msg.set("phantom mode — keep placing right, or the sudoku flips".into());
        }
        leave_arm.set(None);
        ph.clear_haunt();
        ph.last_progress.set(js_sys::Date::now());
    };
    let current_diff =
        move || game.get().and_then(|g| g["difficulty"].as_str().map(String::from)).unwrap_or_default();

    view! {
        <h1>"a-souvenir-of-sudokus"</h1>
        <div class="grid" class:flipping=move || flip_anim.get()>{cells}</div>
        <div class="bar palette">
            {palette}
            <button class="digit erase" title="erase" on:click=move |_| key_action("x")>"⌫"</button>
        </div>
        <div class="bar">
            <button class:on=move || pencil.get() on:click=move |_| key_action("m")>"pencil"</button>
            <button
                disabled=move || !dev && (mode.get() == Mode::Phantom || hints_left.get() == 0)
                on:click=move |_| key_action("H")
            >
                {move || if dev { "hint".into() } else { format!("hint ({})", hints_left.get()) }}
            </button>
            <button
                disabled=move || !dev && (mode.get() == Mode::Phantom || checks_left.get() == 0)
                on:click=move |_| key_action("c")
            >
                {move || if dev { "check".into() } else { format!("check ({})", checks_left.get()) }}
            </button>
            <button on:click=move |_| key_action("u")>"undo"</button>
            <button on:click=move |_| key_action("r")>"redo"</button>
        </div>
        <div class="bar setup">
            <button on:click=move |_| key_action("n")>"new game"</button>
            <span class="seg">
                {DIFFICULTIES
                    .iter()
                    .map(|d| {
                        let d = d.to_string();
                        let label = d.clone();
                        let is_current = {
                            let d = d.clone();
                            move || current_diff() == d
                        };
                        view! {
                            <button
                                class:on=is_current
                                disabled=move || mode.get() == Mode::Phantom
                                title="leave phantom mode to change difficulty"
                                on:click=move |_| new_game(d.clone())
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect::<Vec<_>>()}
            </span>
            <button class:on=move || mode.get() == Mode::Phantom on:click=toggle_phantom>
                "phantom"
            </button>
        </div>
        <div class="msg" class:solved=solved>{move || msg.get()}</div>
        <div class="status">{status}</div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
