mod commands;
mod ssh;

use commands::AppState;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            client: Arc::new(Mutex::new(None)),
            creds: Mutex::new(None),
            abort: Arc::new(AtomicBool::new(false)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::connect_device,
            commands::disconnect_device,
            commands::test_mirrors,
            commands::switch_mirror,
            commands::install_plugins,
            commands::abort_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenApk");
}
