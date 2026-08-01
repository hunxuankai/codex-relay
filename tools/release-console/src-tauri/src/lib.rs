#![allow(linker_messages)]

pub mod app_state;
pub mod commands;
pub mod infrastructure;
pub mod models;
pub mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(app_state::AppState::system())
        .invoke_handler(tauri::generate_handler![
            commands::test_release_connection,
            commands::inspect_release_repository,
            commands::push_release_repository,
            commands::prepare_release_plan,
            commands::start_release,
            commands::get_release_session,
            commands::resume_release,
            commands::cancel_release,
            commands::publish_release,
            commands::export_release_summary,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Codex Relay release console");
}
