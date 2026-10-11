use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
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
