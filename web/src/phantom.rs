//! Phantom mode: the stall clock, the haunt window, and the bundle of
//! signals that drive them.
use leptos::prelude::*;
use serde_json::Value;

pub const LIVES: u32 = 3;
/// Time without a correct placement before the haunting begins, per difficulty.
pub fn stall_ms(difficulty: &str) -> f64 {
    match difficulty {
        "easy" => 90_000.0,
        "hard" => 45_000.0,
        _ => 60_000.0,
    }
}

/// The grace window: once haunted, how long you have to place a correct number.
pub fn window_ms(difficulty: &str) -> f64 {
    match difficulty {
        "easy" => 20_000.0,
        "hard" => 12_000.0,
        _ => 15_000.0,
    }
}

// Every phantom-mode signal in one bundle with one reset path, so the call
// sites can never drift apart again.
#[derive(Clone, Copy)]
pub struct Phantom {
    pub lives: RwSignal<u32>,
    pub over: RwSignal<bool>,
    pub haunt_start: RwSignal<Option<f64>>,
    pub incoming: RwSignal<Option<Value>>, // ghost preview shown during the haunt
    pub last_progress: RwSignal<f64>,
}

impl Phantom {
    pub fn new() -> Self {
        Self {
            lives: RwSignal::new(LIVES),
            over: RwSignal::new(false),
            haunt_start: RwSignal::new(None),
            incoming: RwSignal::new(None),
            last_progress: RwSignal::new(js_sys::Date::now()),
        }
    }
    pub fn clear_haunt(self) {
        self.haunt_start.set(None);
        self.incoming.set(None);
    }
    /// Full reset — only a NEW GAME may call this. Lives are per game;
    /// resetting them on mode toggles was an escape from "the phantoms won".
    pub fn reset(self) {
        self.lives.set(LIVES);
        self.over.set(false);
        self.clear_haunt();
        self.last_progress.set(js_sys::Date::now());
    }
}
