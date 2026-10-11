//! Asset-pack resolution, loading and glyph rendering: which pack is live,
//! its content revision, its spritesheet, and the CSS that paints a digit.
use std::collections::HashMap;

use crate::engine::Manifest;
use leptos::prelude::*;
use serde_json::Value;

/// Asset pack resolution. ?pack=<name> in the URL always wins; otherwise the
/// loader probes grandpere first (so the packaged desktop app opens straight
/// into his handwriting) and falls back to the placeholder. The loader sets
/// the store before the first render reads it.
pub fn pack_store() -> &'static std::sync::Mutex<Option<String>> {
    static PACK: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    &PACK
}

pub fn pack() -> String {
    pack_store().lock().unwrap().clone().unwrap_or_else(|| "assets/placeholder".into())
}

/// Content revision of the loaded pack (manifest "rev"). Appended as ?v= to
/// asset URLs: glyph FILENAMES never change across regenerations, so without
/// this a returning browser keeps serving stale cached glyphs after a deploy.
pub fn rev_store() -> &'static std::sync::Mutex<String> {
    static REV: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
    &REV
}

pub fn versioned(url: String) -> String {
    let rev = rev_store().lock().unwrap().clone();
    if rev.is_empty() { url } else { format!("{url}?v={rev}") }
}

/// Spritesheet of the loaded pack: (file, cols, rows, rel-path -> tile index).
/// One request paints the whole board; packs without one (placeholder) fall
/// back to per-file URLs.
pub type Sprite = (String, u32, u32, HashMap<String, u32>);

/// Versioned URL of the loaded pack's paper, for the splash preloader.
pub fn paper_store() -> &'static std::sync::Mutex<Option<String>> {
    static PAPER: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    &PAPER
}

pub fn sprite_url() -> Option<String> {
    let (file, ..) = sprite_store().lock().unwrap().clone()?;
    Some(versioned(format!("{}/{file}", pack())))
}

pub fn sprite_store() -> &'static std::sync::Mutex<Option<Sprite>> {
    static SPRITE: std::sync::Mutex<Option<Sprite>> = std::sync::Mutex::new(None);
    &SPRITE
}

/// Fetch a pack's manifest, install it (pack path, paper, digit manifest).
/// Used at startup and by the classic/souvenir toggle.
pub async fn load_pack(cand: String, manifest: RwSignal<Option<Manifest>>) -> bool {
    // cache-buster: a stale cached manifest points at pack files that
    // no longer exist after a re-emit ("broken images, wrong background")
    let url = format!("{cand}/manifest.json?t={}", js_sys::Date::now() as u64);
    let Ok(rsp) = gloo_net::http::Request::get(&url).send().await else {
        return false;
    };
    let Ok(m) = rsp.json::<Value>().await else { return false };
    let Ok(parsed) = serde_json::from_value::<Manifest>(m["digits"].clone()) else {
        return false;
    };
    *pack_store().lock().unwrap() = Some(cand.clone());
    *rev_store().lock().unwrap() = m["rev"].as_str().unwrap_or_default().to_string();
    *sprite_store().lock().unwrap() = (|| {
        let s = &m["sprite"];
        let index = s["index"]
            .as_object()?
            .iter()
            .filter_map(|(k, v)| Some((k.clone(), v.as_u64()? as u32)))
            .collect();
        Some((s["file"].as_str()?.to_string(), s["cols"].as_u64()? as u32, s["rows"].as_u64()? as u32, index))
    })();
    // a pack may bring its own paper — the page becomes his notebook;
    // a paperless pack (classic) falls back to the plain CSS cream
    if let Some(body) = document().body() {
        let style = body.style();
        if let Some(paper) = m["paper"].as_str() {
            let paper_url = versioned(format!("{cand}/{paper}"));
            *paper_store().lock().unwrap() = Some(paper_url.clone());
            let _ = style.set_property("background-image", &format!("url('{paper_url}')"));
            let _ = style.set_property("background-size", "540px");
        } else {
            *paper_store().lock().unwrap() = None;
            let _ = style.remove_property("background-image");
        }
    }
    manifest.set(Some(parsed));
    true
}

pub fn url_pack() -> Option<String> {
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

/// Inline background style for one glyph: spritesheet position when the pack
/// has one (one request paints everything), per-file URL otherwise.
pub fn digit_style(manifest: &Manifest, role: &str, digit: i64, cell: usize) -> Option<String> {
    let variants = manifest.get(role)?.get(&digit.to_string())?;
    if variants.is_empty() {
        return None; // a sparse pack must degrade, not divide by zero
    }
    let v = variants.get((cell * 31 + digit as usize) % variants.len())?;
    let sprite = sprite_store().lock().unwrap().clone();
    if let Some((file, cols, rows, index)) = sprite {
        if let Some(&n) = index.get(v) {
            let (c, r) = (n % cols, n / cols);
            let px = if cols > 1 { c as f64 * 100.0 / (cols - 1) as f64 } else { 0.0 };
            let py = if rows > 1 { r as f64 * 100.0 / (rows - 1) as f64 } else { 0.0 };
            let url = versioned(format!("{}/{file}", pack()));
            return Some(format!(
                "background-image:url('{url}');background-size:{}% {}%;background-position:{px:.4}% {py:.4}%",
                cols * 100,
                rows * 100
            ));
        }
    }
    let url = versioned(format!("{}/{v}", pack()));
    Some(format!("background-image:url('{url}');background-size:contain;background-position:center"))
}
