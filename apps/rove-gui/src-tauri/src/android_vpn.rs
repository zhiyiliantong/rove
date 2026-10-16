use jni::{JavaVM, jni_sig, jni_str, objects::JObject, refs::Global};
use rove_agent::overlay::{
    mobile_vpn::{NativeTun, NativeVpnHost},
    vpn_plan::VpnPlan,
};
use std::{
    os::fd::{FromRawFd, OwnedFd},
    sync::{Arc, OnceLock},
};

static HOST: OnceLock<Arc<Host>> = OnceLock::new();
static PERMISSION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub struct Host {
    vm: JavaVM,
    controller: Global<JObject<'static>>,
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_app_rove_desktop_MainActivity_initializeVpn<'local>(
    mut env: jni::EnvUnowned<'local>,
    _activity: JObject<'local>,
    controller: JObject<'local>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let host = Arc::new(Host {
            vm: env.get_java_vm()?,
            controller: env.new_global_ref(controller)?,
        });
        // The controller is a Kotlin singleton holding only a weak Activity;
        // recreating the Activity does not retain a destroyed window.
        let _ = HOST.set(host);
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

pub fn host() -> anyhow::Result<Arc<dyn NativeVpnHost>> {
    HOST.get()
        .cloned()
        .map(|h| h as Arc<dyn NativeVpnHost>)
        .ok_or_else(|| anyhow::anyhow!("Native VPN bridge has not initialized"))
}

impl Host {
    fn status(&self) -> anyhow::Result<i32> {
        self.vm.attach_current_thread(|env| -> anyhow::Result<i32> {
            Ok(env
                .call_method(
                    &self.controller,
                    jni_str!("permissionStatus"),
                    jni_sig!("()I"),
                    &[],
                )?
                .i()?)
        })
    }
}

pub async fn request_permission() -> anyhow::Result<()> {
    let _guard = PERMISSION.lock().await;
    let host = HOST
        .get()
        .ok_or_else(|| anyhow::anyhow!("Native VPN bridge has not initialized"))?;
    if host.ready() {
        return Ok(());
    }
    host.vm.attach_current_thread(|env| -> anyhow::Result<()> {
        env.call_method(
            &host.controller,
            jni_str!("requestPermission"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    })?;
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        loop {
            match host.status()? {
                1 => return Ok(()),
                0 => tokio::time::sleep(std::time::Duration::from_millis(100)).await,
                _ => anyhow::bail!(
                    "Android VPN permission was denied or the foreground service is unavailable"
                ),
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Android VPN permission request timed out"))?
}

impl NativeVpnHost for Host {
    fn ready(&self) -> bool {
        self.status().is_ok_and(|s| s == 1)
    }
    fn establish(&self, plan: &VpnPlan) -> anyhow::Result<NativeTun> {
        let plan = serde_json::to_string(&plan.networks().iter().map(|n| serde_json::json!({
            "address": n.address.to_string(), "network": n.subnet.network().to_string(), "prefix": n.subnet.prefix_len()
        })).collect::<Vec<_>>())?;
        let fd = self
            .vm
            .attach_current_thread(|env| -> anyhow::Result<i32> {
                let value = env.new_string(plan)?;
                Ok(env
                    .call_method(
                        &self.controller,
                        jni_str!("establish"),
                        jni_sig!("(Ljava/lang/String;)I"),
                        &[jni::JValue::Object(value.as_ref())],
                    )?
                    .i()?)
            })?;
        anyhow::ensure!(fd >= 0, "Native VPN returned an invalid descriptor");
        // SAFETY: Kotlin detached a new duplicate and transferred sole ownership.
        NativeTun::from_descriptor(unsafe { OwnedFd::from_raw_fd(fd) })
    }
    fn close(&self) -> anyhow::Result<()> {
        self.vm.attach_current_thread(|env| -> anyhow::Result<()> {
            env.call_method(&self.controller, jni_str!("close"), jni_sig!("()V"), &[])?;
            Ok(())
        })
    }
}
