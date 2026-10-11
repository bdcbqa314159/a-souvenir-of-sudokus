//! The wire to the C++ engine (one JSON command surface) and the small
//! helpers that read a game object.
use std::collections::HashMap;

use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = souvenir_cmd, catch)]
    fn souvenir_cmd(request: &str) -> Result<String, JsValue>;
}

pub type Manifest = HashMap<String, HashMap<String, Vec<String>>>; // role -> digit -> variants

pub fn cmd(req: Value) -> Value {
    match souvenir_cmd(&req.to_string()) {
        Ok(s) => serde_json::from_str(&s)
            .unwrap_or_else(|e| json!({"ok": false, "error": format!("bad response: {e}")})),
        Err(_) => json!({"ok": false, "error": "engine not loaded"}),
    }
}

// Always 81 entries (non-integers become 0, wrong lengths fall back wholesale),
// so indexing with 0..81 can never panic on a malformed response.
pub fn board_of(game: &Value, key: &str) -> Vec<i64> {
    game[key]
        .as_array()
        .map(|a| a.iter().map(|v| v.as_i64().unwrap_or(0)).collect::<Vec<_>>())
        .filter(|v| v.len() == 81)
        .unwrap_or_else(|| vec![0; 81])
}

/// Solved means a real, fully matching board — two all-zero fallbacks don't count.
pub fn solved_game(game: &Value) -> bool {
    let board = board_of(game, "board");
    board.iter().any(|v| *v != 0) && board == board_of(game, "solution")
}
