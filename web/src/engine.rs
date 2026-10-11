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
        .map(|a| {
            a.iter()
                .map(|v| v.as_i64().unwrap_or(0))
                .collect::<Vec<_>>()
        })
        .filter(|v| v.len() == 81)
        .unwrap_or_else(|| vec![0; 81])
}

/// Solved means a real, fully matching board — two all-zero fallbacks don't count.
pub fn solved_game(game: &Value) -> bool {
    let board = board_of(game, "board");
    board.iter().any(|v| *v != 0) && board == board_of(game, "solution")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn game(board: Vec<i64>, solution: Vec<i64>) -> Value {
        json!({"board": board, "solution": solution})
    }

    #[test]
    fn board_of_reads_a_well_formed_grid() {
        let g = game((0..81).map(|i| i % 10).collect(), vec![1; 81]);
        assert_eq!(board_of(&g, "board").len(), 81);
        assert_eq!(board_of(&g, "board")[11], 1);
    }

    #[test]
    fn board_of_defends_against_malformed_input() {
        // the restore path feeds this user-editable localStorage
        for bad in [
            json!({"board": [1, 2, 3]}),
            json!({"board": "nope"}),
            json!({}),
        ] {
            assert_eq!(
                board_of(&bad, "board"),
                vec![0; 81],
                "bad shape must degrade to zeros"
            );
        }
        let mixed = json!({"board": (0..81).map(|i| if i == 5 { json!("x") } else { json!(1) }).collect::<Vec<_>>()});
        assert_eq!(
            board_of(&mixed, "board")[5],
            0,
            "non-integer cell reads as empty"
        );
    }

    #[test]
    fn solved_game_needs_a_real_match() {
        let sol: Vec<i64> = (0..81).map(|i| (i % 9) + 1).collect();
        assert!(solved_game(&game(sol.clone(), sol.clone())));
        let mut wrong = sol.clone();
        wrong[0] = if sol[0] == 9 { 1 } else { sol[0] + 1 };
        assert!(!solved_game(&game(wrong, sol.clone())));
        assert!(
            !solved_game(&game(vec![0; 81], vec![0; 81])),
            "two empty boards are not a win"
        );
    }
}
