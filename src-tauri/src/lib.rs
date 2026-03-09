mod commands;
mod marshal;

use std::sync::Mutex;
use commands::audio::AudioHandle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_handle = AudioHandle::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Mutex::new(audio_handle))
        .invoke_handler(tauri::generate_handler![
            commands::load_data,
            commands::audio::preview_audio,
            commands::audio::stop_audio,
            commands::audio::is_audio_playing
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
