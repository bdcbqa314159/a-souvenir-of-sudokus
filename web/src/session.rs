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

// ---------------------------------------------------------------- history

/// Every finished grid, newest first. A record is tiny — difficulty, mode,
/// the time it took, when it was finished — so a lifetime of sudokus costs
/// a fraction of the storage a single photograph would.
pub const HISTORY_KEY: &str = "souvenir-history";
const HISTORY_MAX: usize = 500;

pub fn load_history() -> Vec<Value> {
    let Ok(Some(store)) = window().local_storage() else {
        return Vec::new();
    };
    store
        .get_item(HISTORY_KEY)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<Value>>(&raw).ok())
        .unwrap_or_default()
}

pub fn save_history(entries: &[Value]) {
    if let Ok(Some(store)) = window().local_storage() {
        let _ = store.set_item(
            HISTORY_KEY,
            &serde_json::to_string(entries).unwrap_or_default(),
        );
    }
}

/// Record a finished grid. Returns the new history (newest first), capped so
/// a very long-lived browser cannot fill its storage quota.
pub fn remember(entry: Value) -> Vec<Value> {
    let mut all = load_history();
    all.insert(0, entry);
    all.truncate(HISTORY_MAX);
    save_history(&all);
    all
}

pub fn clear_history() {
    if let Ok(Some(store)) = window().local_storage() {
        let _ = store.remove_item(HISTORY_KEY);
    }
}
