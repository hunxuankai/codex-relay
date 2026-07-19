#![allow(linker_messages)]

pub mod error;
pub mod infrastructure;
pub mod models;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run Codex Relay");
}
