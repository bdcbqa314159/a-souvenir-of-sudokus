//! Asset-pack resolution, loading and glyph rendering: which pack is live,
//! its content revision, its spritesheet, and the CSS that paints a digit.
use std::collections::HashMap;

use crate::engine::Manifest;
use leptos::prelude::*;
use serde_json::Value;

/// Asset pack resolution. ?pack=<name> in the URL always wins; otherwise the
/// loader probes souvenir first (so the packaged desktop app opens straight
/// into his handwriting) and falls back to the placeholder. The loader sets
/// the store before the first render reads it.
pub fn pack_store() -> &'static std::sync::Mutex<Option<String>> {
    static PACK: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    &PACK
}

pub fn pack() -> String {
    pack_store()
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "assets/placeholder".into())
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
    if rev.is_empty() {
        url
    } else {
        format!("{url}?v={rev}")
    }
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
    // No cache-busting query here: a unique URL per load defeated the service
    // worker's cache (an exact-URL match), so the installed app could not load
    // a pack offline at all. `cache: no-cache` revalidates instead, which keeps
    // a re-emitted pack fresh AND leaves one cacheable URL for offline play.
    let url = format!("{cand}/manifest.json");
    let Ok(rsp) = gloo_net::http::Request::get(&url)
        .cache(web_sys::RequestCache::NoCache)
        .send()
        .await
    else {
        return false;
    };
    let Ok(m) = rsp.json::<Value>().await else {
        return false;
    };
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
        Some((
            s["file"].as_str()?.to_string(),
            s["cols"].as_u64()? as u32,
            s["rows"].as_u64()? as u32,
            index,
        ))
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
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
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
        if let (Some(&n), true) = (index.get(v), cols > 0 && rows > 0) {
            let (c, r) = (n % cols, n / cols);
            let px = if cols > 1 {
                c as f64 * 100.0 / (cols - 1) as f64
            } else {
                0.0
            };
            let py = if rows > 1 {
                r as f64 * 100.0 / (rows - 1) as f64
            } else {
                0.0
            };
            let url = versioned(format!("{}/{file}", pack()));
            return Some(format!(
                "background-image:url('{url}');background-size:{}% {}%;background-position:{px:.4}% {py:.4}%",
                cols * 100,
                rows * 100
            ));
        }
    }
    let url = versioned(format!("{}/{v}", pack()));
    Some(format!(
        "background-image:url('{url}');background-size:contain;background-position:center"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        let mut given = HashMap::new();
        given.insert(
            "1".to_string(),
            vec![
                "digits/given/1_s00.png".into(),
                "digits/given/1_s01.png".into(),
            ],
        );
        given.insert("2".to_string(), vec![]); // a sparse pack must degrade, not panic
        let mut m = HashMap::new();
        m.insert("given".to_string(), given);
        m
    }

    /// These tests share the process-wide pack/rev/sprite stores, so they run
    /// as one test rather than racing each other.
    #[test]
    fn glyph_styling_covers_sprite_and_per_file_packs() {
        let man = manifest();
        *pack_store().lock().unwrap() = Some("assets/souvenir".into());
        *rev_store().lock().unwrap() = String::new();
        *sprite_store().lock().unwrap() = None;

        // per-file fallback (no sprite): a plain background image
        let s = digit_style(&man, "given", 1, 0).expect("style for a present digit");
        assert!(s.contains("url('assets/souvenir/digits/given/1_s"), "{s}");
        assert!(s.contains("background-size:contain"), "{s}");
        assert!(!s.contains('?'), "no rev means no cache-busting query: {s}");

        // a content revision reaches the URL
        *rev_store().lock().unwrap() = "abc123".into();
        assert!(digit_style(&man, "given", 1, 0)
            .unwrap()
            .contains("?v=abc123"));

        // sprite packs paint by position, from ONE image
        let mut index = HashMap::new();
        index.insert("digits/given/1_s00.png".to_string(), 0u32);
        index.insert("digits/given/1_s01.png".to_string(), 16u32); // row 1, col 1 of a 15-wide sheet
        *sprite_store().lock().unwrap() = Some(("sprite.png".into(), 15, 12, index));
        let a = digit_style(&man, "given", 1, 0).unwrap();
        assert!(a.contains("sprite.png?v=abc123"), "{a}");
        assert!(a.contains("background-size:1500% 1200%"), "{a}");

        // the chosen variant is deterministic per (cell, digit) — the page
        // looks written, not stamped, and never reshuffles on re-render
        let again = digit_style(&man, "given", 1, 0).unwrap();
        assert_eq!(a, again);
        let other_cell = digit_style(&man, "given", 1, 1).unwrap();
        assert_ne!(a, other_cell, "neighbouring cells pick different variants");

        // sparse and missing entries degrade to None, never a panic
        assert!(
            digit_style(&man, "given", 2, 0).is_none(),
            "empty variant list"
        );
        assert!(digit_style(&man, "given", 9, 0).is_none(), "absent digit");
        assert!(digit_style(&man, "user", 1, 0).is_none(), "absent role");

        // a malformed pack must degrade, not divide by zero (audit #14)
        let mut idx = HashMap::new();
        idx.insert("digits/given/1_s00.png".to_string(), 0u32);
        *sprite_store().lock().unwrap() = Some(("sprite.png".into(), 0, 0, idx));
        assert!(
            digit_style(&man, "given", 1, 0).is_some(),
            "a zero-column sprite must fall back to the per-file path, not panic"
        );

        *sprite_store().lock().unwrap() = None;
        *rev_store().lock().unwrap() = String::new();
    }
}
