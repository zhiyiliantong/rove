#[cfg(target_os = "android")]
mod android_vpn;

#[tauri::command]
async fn agent_call(
    client: tauri::State<'_, rove_sdk::LocalClient>,
    request: rove_protocol::Request,
) -> Result<rove_protocol::Response, String> {
    #[cfg(target_os = "android")]
    if request.operation_id == "start_network" && request.target.is_none() {
        // Ask the agent to validate the saved network first. Retry only the
        // explicit pre-acceptance permission refusal, never an unknown outcome.
        let response = client
            .call(request.clone())
            .await
            .map_err(|e| e.to_string())?;
        if response
            .body
            .as_ref()
            .and_then(|body| body["error"]["code"].as_str())
            != Some("overlay_permission_required")
        {
            return Ok(response);
        }
        if let Err(error) = android_vpn::request_permission().await {
            return Ok(rove_protocol::Response::error(
                &request,
                rove_protocol::ApiError::new(
                    503,
                    "overlay_permission_required",
                    &error.to_string(),
                ),
            ));
        }
    }
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
fn network_config_qr(config: String) -> Result<String, String> {
    rove_sdk::ShareQr::from_text(&config)
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
                #[cfg(target_os = "android")]
                let embedded = tauri::async_runtime::block_on(
                    rove_agent::EmbeddedAgent::start_with_vpn(&dir, android_vpn::host()?),
                )?;
                #[cfg(not(target_os = "android"))]
                let embedded =
                    tauri::async_runtime::block_on(rove_agent::EmbeddedAgent::start(&dir))?;
                app.manage(embedded);
            }
            let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
            let client = rove_sdk::LocalClient::new(rove_sdk::socket_path(&dir));
            #[cfg(windows)]
            tauri::async_runtime::block_on(ensure_windows_agent(&client, &dir))?;
            app.manage(client);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            agent_call,
            share_qr,
            network_config_qr,
            agent_events
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Rove GUI");
}

// Windows process management only: business state stays in the same local
// agent. Never elevate this child or launch a binary found through PATH.
#[cfg(windows)]
async fn ensure_windows_agent(
    client: &rove_sdk::LocalClient,
    directory: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::{os::windows::process::CommandExt, process::Stdio, time::Duration};
    if client
        .call(rove_protocol::Request::new("get_device"))
        .await
        .is_ok()
    {
        return Ok(());
    }
    let binary = std::env::current_exe()?.with_file_name("rove-agent.exe");
    let _child = std::process::Command::new(binary)
        .arg("--data-dir")
        .arg(directory)
        .creation_flags(0x08000000 | 0x00000008)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if client
                .call(rove_protocol::Request::new("get_device"))
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| "The bundled Rove agent did not become ready")?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
