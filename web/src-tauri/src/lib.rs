#[cfg(desktop)]
use tauri::Manager;
#[cfg(desktop)]
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
#[cfg(desktop)]
use tauri_plugin_updater::UpdaterExt;

/// Ask GitHub whether a newer release exists, and offer it. Every update is
/// verified against the public key baked into this binary before it is
/// installed — an unsigned or foreign-signed payload is refused by the
/// plugin, so a tampered download cannot become a tampered game.
/// Failure is silent on purpose: offline players just keep playing.
///
/// Desktop only. On a phone the store (or whoever handed you the APK) does
/// this job, the plugin cannot install an APK in any case, and leaving it out
/// keeps a network stack — and the permission to use one — out of the Android
/// build of a game that is otherwise entirely offline.
#[cfg(desktop)]
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
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .setup(|app| {
            // a phone has one window, the size of the phone; this is the
            // desktop's problem of fitting a fixed 760x900 onto whatever
            // screen is there, which overflowed short or scaled displays
            #[cfg(desktop)]
            {
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
            }
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
