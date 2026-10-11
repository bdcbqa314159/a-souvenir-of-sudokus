//! Session persistence: a refresh resumes the game in progress.
use leptos::prelude::window;
use serde_json::Value;

/// Session persistence: a refresh resumes the game in progress; only a new
/// game (or solving it) clears the slot. The engine is stateless — the game
/// JSON on the wire IS the save format, so restoring is just handing it back.
pub const SESSION_KEY: &str = "souvenir-session";

pub fn save_session(blob: &Value) {
    if let Ok(Some(store)) = window().local_storage() {
        let _ = store.set_item(SESSION_KEY, &blob.to_string());
    }
}

pub fn clear_session() {
    if let Ok(Some(store)) = window().local_storage() {
        let _ = store.remove_item(SESSION_KEY);
    }
}

pub fn load_session() -> Option<Value> {
    let store = window().local_storage().ok()??;
    let raw = store.get_item(SESSION_KEY).ok()??;
    serde_json::from_str(&raw).ok()
}

