mod auth;
mod commands;
mod connectors;
mod error;

use std::sync::Arc;

use tokio::sync::RwLock;

use connectors::obsidian::{ObsidianClient, ObsidianConfig};

pub struct AppState {
    pub http: reqwest::Client,
    pub auth: auth::state::SharedAuthState,
    pub obsidian: Arc<RwLock<Option<ObsidianClientState>>>,
}

pub struct ObsidianClientState {
    pub config: ObsidianConfig,
    pub client: ObsidianClient,
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
        obsidian: Arc::new(RwLock::new(None)),
    };

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    // Windows + Linux: when the OS launches a second instance to deliver a
    // deep-link URL, detect it, focus the original window, and let the
    // single-instance plugin's `deep-link` integration forward the URL to
    // the already-running deep-link listener.
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use tauri::Manager;
        builder = builder.plugin(tauri_plugin_single_instance::init(
            |app, _argv, _cwd| {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            },
        ));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::begin_login,
            commands::handle_callback,
            commands::current_session,
            commands::logout,
            commands::obsidian_configure,
            commands::obsidian_status,
            commands::obsidian_list_root,
            commands::obsidian_get_note,
            commands::obsidian_disconnect,
        ])
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                // On Windows and Linux, register the URL scheme with the OS
                // at runtime. On macOS this is unsupported at runtime — the
                // Info.plist handles registration at install time — so the
                // warning log is expected there.
                if let Err(e) = app.deep_link().register_all() {
                    tracing::debug!(error = %e, "deep-link register_all (expected on macOS)");
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
