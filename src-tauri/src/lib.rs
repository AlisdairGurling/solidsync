mod auth;
mod commands;
mod error;

use std::sync::Arc;

use tokio::sync::RwLock;

pub struct AppState {
    pub http: reqwest::Client,
    pub auth: auth::state::SharedAuthState,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("solidsync=info,warn")),
        )
        .try_init()
        .ok();

    let http = reqwest::Client::builder()
        .user_agent(concat!("SolidSync/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client");

    let app_state = AppState {
        http,
        auth: Arc::new(RwLock::new(auth::state::AuthState::default())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::begin_login,
            commands::handle_callback,
            commands::current_session,
            commands::logout,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                // Ensure the solidsync:// scheme is registered even in dev builds
                // where the Info.plist registration isn't applied.
                if let Err(e) = app.deep_link().register_all() {
                    tracing::warn!(error = %e, "deep-link register_all failed");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
