use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;

/// Ask GitHub whether a newer release exists, and offer it. Every update is
/// verified against the public key baked into this binary before it is
/// installed — an unsigned or foreign-signed payload is refused by the
/// plugin, so a tampered download cannot become a tampered game.
/// Failure is silent on purpose: offline players just keep playing.
async fn offer_update(app: tauri::AppHandle) {
    let updater = match app.updater() {
        Ok(u) => u,
        Err(e) => return log::warn!("updater unavailable: {e}"),
    };
    let update = match updater.check().await {
        Ok(Some(u)) => u,
        Ok(None) => return,
        Err(e) => return log::info!("no update check: {e}"),
    };
    // the game speaks three languages; so does the one dialog it ever shows
    let body = format!(
        "Version {} is ready.\n\nInstall it now?\nL'installer maintenant ?\n¿Instalarla ahora?",
        update.version
    );
    let accepted = app
        .dialog()
        .message(body)
        .title("a-souvenir-of-sudokus")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Update".into(),
            "Later".into(),
        ))
        .blocking_show();
    if !accepted {
        return;
    }
    match update.download_and_install(|_, _| {}, || {}).await {
        Ok(()) => app.restart(),
        Err(e) => {
            log::error!("update failed: {e}");
            app.dialog()
                .message(format!("The update could not be installed: {e}"))
                .title("a-souvenir-of-sudokus")
                .blocking_show();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
      // fit the window to the host screen: a fixed 760x900 overflowed short
      // or scaled displays, so the game needed scrolling to be seen whole
      if let Some(win) = app.get_webview_window("main") {
        if let Ok(Some(monitor)) = win.current_monitor() {
          let scale = monitor.scale_factor();
          let avail = monitor.size().to_logical::<f64>(scale);
          let w = 760.0_f64.min(avail.width * 0.92);
          let h = 900.0_f64.min(avail.height * 0.90);
          let _ = win.set_size(tauri::LogicalSize::new(w, h));
          let _ = win.center();
        }
      }
      // check for a newer release in the background, once per launch
      let handle = app.handle().clone();
      tauri::async_runtime::spawn(offer_update(handle));
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
