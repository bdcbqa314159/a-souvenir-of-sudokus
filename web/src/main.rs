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

const PACK: &str = "assets/placeholder";
const DIFFICULTIES: [&str; 3] = ["easy", "medium", "hard"];

type Manifest = HashMap<String, HashMap<String, Vec<String>>>; // role -> digit -> variants

fn cmd(req: Value) -> Value {
    match souvenir_cmd(&req.to_string()) {
        Ok(s) => serde_json::from_str(&s)
            .unwrap_or_else(|e| json!({"ok": false, "error": format!("bad response: {e}")})),
        Err(_) => json!({"ok": false, "error": "engine not loaded"}),
    }
}

fn board_of(game: &Value, key: &str) -> Vec<i64> {
    game[key]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_i64).collect())
        .unwrap_or_else(|| vec![0; 81])
}

/// Deterministic variant per (cell, digit): the page looks written, not stamped.
fn digit_src(manifest: &Manifest, role: &str, digit: i64, cell: usize) -> Option<String> {
    let variants = manifest.get(role)?.get(&digit.to_string())?;
    let v = variants.get((cell * 31 + digit as usize) % variants.len())?;
    Some(format!("{PACK}/{v}"))
}

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

    // engine ready (index.html sets window.souvenir_cmd) + manifest fetched -> first game
    spawn_local(async move {
        loop {
            if cmd(json!({"cmd": "load"}))["error"] != "engine not loaded" {
                break;
            }
            gloo_timers::future::TimeoutFuture::new(50).await;
        }
        match gloo_net::http::Request::get(&format!("{PACK}/manifest.json")).send().await {
            Ok(rsp) => match rsp.json::<Value>().await {
                Ok(m) => {
                    let parsed = serde_json::from_value::<Manifest>(m["digits"].clone()).ok();
                    manifest.set(parsed);
                }
                Err(e) => msg.set(format!("bad manifest: {e}")),
            },
            Err(e) => msg.set(format!("manifest fetch failed: {e}")),
        }
        let rsp = cmd(json!({"cmd": "new", "difficulty": "medium"}));
        if rsp["ok"].as_bool() == Some(true) {
            game.set(Some(rsp["game"].clone()));
        } else {
            msg.set(rsp["error"].as_str().unwrap_or("engine error").to_string());
        }
    });

    // run an engine command against the current game; push undo on success
    let play = move |req: Value| {
        let Some(g) = game.get_untracked() else { return };
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
        } else {
            msg.set(rsp["error"].as_str().unwrap_or("engine error").to_string());
        }
    };

    let new_game = move |difficulty: String| {
        let rsp = cmd(json!({"cmd": "new", "difficulty": difficulty}));
        if rsp["ok"].as_bool() == Some(true) {
            game.set(Some(rsp["game"].clone()));
            history.set(History::default());
            msg.set(format!("new {difficulty} game"));
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

    let key_action = move |key: &str| {
        let i = selected.get_untracked();
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
            "H" => play(json!({"cmd": "hint"})),
            "c" => {
                if let Some(g) = game.get_untracked() {
                    let mut req = json!({"cmd": "check"});
                    req["game"] = g;
                    let rsp = cmd(req);
                    let n = rsp["wrong"].as_array().map(Vec::len).unwrap_or(0);
                    msg.set(if n == 0 { "all good so far".into() } else { format!("{n} wrong cell(s)") });
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
                play(json!({"cmd": if mark { "mark" } else { "put" }, "i": i, "v": v}));
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
                format!("{diff} · {filled}/81 · {}", if pencil.get() { "pencil" } else { "pen" })
            })
            .unwrap_or_else(|| "loading engine…".into())
    };
    let solved = move || {
        game.get()
            .map(|g| board_of(&g, "board") == board_of(&g, "solution"))
            .unwrap_or(false)
    };

    view! {
        <h1>"a-souvenir-of-sudokus"</h1>
        <div class="grid">{cells}</div>
        <div class="bar">
            <button on:click=move |_| key_action("n")>"new"</button>
            {DIFFICULTIES
                .iter()
                .map(|d| {
                    let d = d.to_string();
                    let label = d.clone();
                    view! { <button on:click=move |_| new_game(d.clone())>{label}</button> }
                })
                .collect::<Vec<_>>()}
            <button class:on=move || pencil.get() on:click=move |_| key_action("m")>"pencil"</button>
            <button on:click=move |_| key_action("H")>"hint"</button>
            <button on:click=move |_| key_action("c")>"check"</button>
            <button on:click=move |_| key_action("u")>"undo"</button>
            <button on:click=move |_| key_action("r")>"redo"</button>
        </div>
        <div class="msg" class:solved=solved>{move || msg.get()}</div>
        <div class="status">{status}</div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
