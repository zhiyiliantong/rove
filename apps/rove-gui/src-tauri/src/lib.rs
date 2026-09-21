#[tauri::command]
async fn agent_call(
    client: tauri::State<'_, rove_sdk::LocalClient>,
    request: rove_protocol::Request,
) -> Result<rove_protocol::Response, String> {
    client.call(request).await.map_err(|e| e.to_string())
}

// Called by MainActivity before Tauri starts the embedded agent. Initialize the
// same verifier version used by reqwest, with the application class loader.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_app_rove_desktop_MainActivity_initializeTls<'local>(
    mut env: jni::EnvUnowned<'local>,
    _activity: jni::objects::JObject<'local>,
    context: jni::objects::JObject<'local>,
) {
    env.with_env(|env| rustls_platform_verifier::android::init_with_env(env, context))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}
#[tauri::command]
fn share_qr(url: String) -> Result<String, String> {
    rove_sdk::ShareQr::new(&url)
        .map(|qr| qr.svg())
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn agent_events(
    client: tauri::State<'_, rove_sdk::LocalClient>,
    run_id: uuid::Uuid,
    after_seq: i64,
    target: Option<rove_protocol::SocketTarget>,
) -> Result<serde_json::Value, String> {
    client
        .poll_events(run_id, after_seq, target)
        .await
        .map_err(|e| e.to_string())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .setup(|app| {
            use tauri::Manager;
            #[cfg(not(mobile))]
            let dir = rove_sdk::default_data_dir();
            #[cfg(mobile)]
            let dir = app.path().app_data_dir()?.join("rove");
            #[cfg(mobile)]
            {
                let embedded =
                    tauri::async_runtime::block_on(rove_agent::EmbeddedAgent::start(&dir))?;
                app.manage(embedded);
            }
            let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
            app.manage(rove_sdk::LocalClient::new(rove_sdk::socket_path(&dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![agent_call, share_qr, agent_events])
        .run(tauri::generate_context!())
        .expect("failed to run Rove GUI");
}

#[cfg(all(test, unix))]
mod tests;
