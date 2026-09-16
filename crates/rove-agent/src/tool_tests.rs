use super::*;
use rove_protocol::Request;
use std::os::unix::fs::PermissionsExt;

async fn fixture() -> (tempfile::TempDir, Arc<Agent>, String) {
    let temp = tempfile::tempdir().unwrap();
    let agent = Agent::open(&temp.path().join("agent")).unwrap();
    agent.store.set("model_config", &json!({"provider":"openai_compatible","base_url":"http://127.0.0.1:1/v1","model":"unused","api_key":null})).unwrap();
    let session = agent
        .session_operation(&Request::new("create_session").with_body(json!({})))
        .unwrap()
        .1
        .unwrap();
    let run = agent
        .session_operation(
            &Request::new("submit_run")
                .with_path("session_id", session["session_id"].as_str().unwrap())
                .with_body(json!({"request_id":uuid::Uuid::new_v4(),"message":"tool fixture"})),
        )
        .unwrap()
        .1
        .unwrap();
    // Stop before the current-thread scheduler can contact a provider. These
    // tests exercise the tool against a real persistent run, not AI scheduling.
    agent.shutdown().await;
    (temp, agent, run["run_id"].as_str().unwrap().to_owned())
}
async fn execute(agent: &Arc<Agent>, run: &str, input: Value) -> Value {
    let output = tokio::time::timeout(
        Duration::from_secs(8),
        system_exec(
            agent.clone(),
            run,
            "test_call",
            &input,
            CancellationToken::new(),
        ),
    )
    .await
    .unwrap();
    AGENT.validate("SystemExecResult", &output).unwrap();
    output
}

#[tokio::test]
async fn unsupported_platform_never_starts_a_process_or_emits_output() {
    let (temp, agent, run) = fixture().await;
    let before = agent.run_snapshot(&run).unwrap();
    for os in ["android", "ios", "unknown"] {
        assert!(!supports_system_exec(os));
        let output = system_exec_on_platform(
            agent.clone(),
            &run,
            "unsupported_call",
            &json!({"command":"touch unsupported_executed", "working_directory":temp.path()}),
            CancellationToken::new(),
            os,
        )
        .await;
        AGENT.validate("SystemExecResult", &output).unwrap();
        assert_eq!(output["status"], "unsupported");
        assert_eq!(output["error"]["code"], "unsupported");
        assert!(output["exit_code"].is_null());
        assert_eq!(output["stdout"], "");
        assert_eq!(output["stderr"], "");
        assert!(!temp.path().join("unsupported_executed").exists());
        assert_eq!(agent.run_snapshot(&run).unwrap(), before);
    }
    for os in ["linux", "macos", "windows"] {
        assert!(supports_system_exec(os));
    }
    assert_eq!(
        crate::capabilities().contains(&"system_exec"),
        supports_system_exec(std::env::consts::OS)
    );
}

#[tokio::test]
async fn command_directory_identity_exit_output_and_limits() {
    let (temp, agent, run) = fixture().await;
    let out = execute(
        &agent,
        &run,
        json!({"command":"pwd; id -u; printf '错误' >&2; exit 7", "working_directory":temp.path()}),
    )
    .await;
    assert_eq!(out["status"], "failed");
    assert_eq!(out["exit_code"], 7);
    assert_eq!(out["stderr"], "错误");
    // SAFETY: getuid reads the identity of this test process without mutation.
    let uid = unsafe { libc::getuid() };
    let directory = temp.path().canonicalize().unwrap();
    assert_eq!(out["stdout"], format!("{}\n{uid}\n", directory.display()));
    let out = execute(&agent, &run, json!({"command":"pwd"})).await;
    if let Some(dir) = std::env::var_os("HOME") {
        assert_eq!(
            out["stdout"],
            format!(
                "{}\n",
                std::path::Path::new(&dir).canonicalize().unwrap().display()
            )
        );
    }
    let out = execute(
        &agent,
        &run,
        json!({"command":"head -c 80000 /dev/zero | tr '\\000' x"}),
    )
    .await;
    assert_eq!(out["status"], "succeeded");
    assert_eq!(out["stdout"].as_str().unwrap().len(), 65536);
    assert_eq!(out["output_truncated"], true);
    let snapshot = agent.run_snapshot(&run).unwrap();
    assert_eq!(
        snapshot["output_tail"].as_str().unwrap().chars().count(),
        65536
    );
    assert_eq!(snapshot["output_truncated"], true);
    let out = execute(
        &agent,
        &run,
        json!({"command":"true", "working_directory":temp.path().join("missing")}),
    )
    .await;
    assert_eq!(out["error"]["code"], "command_spawn_failed");
    for input in [
        json!({"command":"true","timeout_seconds":86401}),
        json!({"command":"true","timeout_seconds":0}),
        json!({"command":""}),
    ] {
        assert_eq!(execute(&agent, &run, input).await["status"], "failed");
    }
}

#[tokio::test]
async fn invalid_limits_never_execute_and_maximum_timeout_is_accepted() {
    let (temp, agent, run) = fixture().await;
    for timeout in [json!(0), json!(-1), json!(86401), json!(1.5), json!("300")] {
        let out = execute(
            &agent,
            &run,
            json!({"command":"touch invalid_limit_executed", "working_directory":temp.path(), "timeout_seconds":timeout}),
        )
        .await;
        assert_eq!(out["status"], "failed");
        assert!(out["exit_code"].is_null());
        assert!(!temp.path().join("invalid_limit_executed").exists());
    }
    let out = execute(
        &agent,
        &run,
        json!({"command":format!("touch invalid_limit_executed #{}", "x".repeat(65536)), "working_directory":temp.path()}),
    )
    .await;
    assert_eq!(out["status"], "failed");
    assert!(!temp.path().join("invalid_limit_executed").exists());
    let out = execute(
        &agent,
        &run,
        json!({"command":"printf accepted", "timeout_seconds":86400}),
    )
    .await;
    assert_eq!(out["status"], "succeeded");
    assert_eq!(out["stdout"], "accepted");
}

#[tokio::test]
async fn timeout_precancel_and_permission_error_are_real_results() {
    let (temp, agent, run) = fixture().await;
    let out = execute(
        &agent,
        &run,
        json!({"command":"printf started; sleep 30", "timeout_seconds":1}),
    )
    .await;
    assert_eq!(out["status"], "timed_out");
    assert_eq!(out["stdout"], "started");
    assert!(out["residual_processes"].as_array().unwrap().is_empty());
    let cancel = CancellationToken::new();
    cancel.cancel();
    let out = system_exec(
        agent.clone(),
        &run,
        "cancelled",
        &json!({"command":"touch should_not_exist", "working_directory":temp.path()}),
        cancel,
    )
    .await;
    assert_eq!(out["status"], "cancelled");
    assert!(!temp.path().join("should_not_exist").exists());
    // Even root cannot execute a file with no execute bits. Test-owned fixture.
    let executable = temp.path().join("not_executable");
    std::fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o600)).unwrap();
    let out = execute(
        &agent,
        &run,
        json!({"command":"./not_executable", "working_directory":temp.path()}),
    )
    .await;
    assert_eq!(out["status"], "failed");
    assert_eq!(out["exit_code"], 126);
    assert!(!out["stderr"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn utf8_pipe_boundaries_and_invalid_bytes_are_bounded() {
    use tokio::io::AsyncWriteExt;
    let (mut writer, reader) = tokio::io::duplex(1);
    let (tx, mut rx) = mpsc::channel(8);
    let task = tokio::spawn(read_pipe(reader, "stdout", tx));
    let producer = tokio::spawn(async move {
        for byte in "漫游者".as_bytes().iter().copied().chain([0xff, 0xe4]) {
            writer.write_all(&[byte]).await.unwrap();
        }
    });
    let mut output = String::new();
    while let Some((stream, text)) = rx.recv().await {
        assert_eq!(stream, "stdout");
        output.push_str(&text);
    }
    producer.await.unwrap();
    task.await.unwrap().unwrap();
    assert_eq!(output, "漫游者��");
}

// Invoked only as a test-owned child by resource_conflicts; never starts an
// external service and requires no Python, package manager or root privileges.
#[test]
fn resource_probe() {
    if let Ok(path) = std::env::var("ROVE_TEST_LOCK") {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .unwrap();
        if fs2::FileExt::try_lock_exclusive(&file).is_err() {
            eprintln!("package lock is held");
            std::process::exit(73);
        }
    }
    if let Ok(port) = std::env::var("ROVE_TEST_PORT")
        && std::net::TcpListener::bind(format!("127.0.0.1:{port}")).is_err()
    {
        eprintln!("listen port is occupied");
        std::process::exit(74);
    }
}

#[tokio::test]
async fn resource_conflicts_do_not_become_false_success() {
    let (temp, agent, run) = fixture().await;
    let lock_path = temp.path().join("package.lock");
    let lock = std::fs::File::create(&lock_path).unwrap();
    fs2::FileExt::lock_exclusive(&lock).unwrap();
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let test_bin = quote(std::env::current_exe().unwrap().to_str().unwrap());
    let out = execute(&agent, &run, json!({"command":format!("ROVE_TEST_LOCK={} {test_bin} --exact tools::tests::resource_probe --nocapture", quote(lock_path.to_str().unwrap()))})).await;
    assert_eq!(out["exit_code"], 73);
    assert!(
        out["stderr"]
            .as_str()
            .unwrap()
            .contains("package lock is held")
    );
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let out = execute(&agent, &run, json!({"command":format!("ROVE_TEST_PORT={} {test_bin} --exact tools::tests::resource_probe --nocapture", occupied.local_addr().unwrap().port())})).await;
    assert_eq!(out["exit_code"], 74);
    assert_eq!(out["status"], "failed");
    assert!(
        out["stderr"]
            .as_str()
            .unwrap()
            .contains("listen port is occupied")
    );
}

// Test-only child server, not a host service. A hard lifetime bound also cleans
// up a descendant if the supervising test fails before its explicit cleanup.
#[test]
fn independent_service_probe() {
    use std::io::{Read, Write};
    let Some(ready) = std::env::var_os("ROVE_TEST_SERVICE_READY") else {
        return;
    };
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    std::fs::write(ready, listener.local_addr().unwrap().to_string()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while std::time::Instant::now() < deadline {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(1)))
                    .unwrap();
                let mut input = [0; 4];
                if stream.read_exact(&mut input).is_ok() && &input == b"ping" {
                    let _ = stream.write_all(b"pong");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("test service accept: {error}"),
        }
    }
}

async fn service_ready(path: &std::path::Path) -> std::net::SocketAddr {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(value) = std::fs::read_to_string(path)
                && let Ok(address) = value.parse()
            {
                return address;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

async fn ping_service(address: std::net::SocketAddr) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
        stream.write_all(b"ping").await.unwrap();
        let mut output = [0; 4];
        stream.read_exact(&mut output).await.unwrap();
        assert_eq!(&output, b"pong");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancellation_and_timeout_stop_managed_descendants_but_preserve_independent_service() {
    let (temp, agent, run) = fixture().await;
    let test_bin = std::env::current_exe().unwrap();
    let ready = temp.path().join("independent.ready");
    let application_data = temp.path().join("music.data");
    std::fs::write(&application_data, "preserve application data").unwrap();
    let mut independent = Command::new(&test_bin)
        .args([
            "--exact",
            "tools::tests::independent_service_probe",
            "--nocapture",
        ])
        .env("ROVE_TEST_SERVICE_READY", &ready)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let address = service_ready(&ready).await;
    ping_service(address).await;
    let quote =
        |value: &std::path::Path| format!("'{}'", value.to_str().unwrap().replace('\'', "'\\''"));
    for expected in ["cancelled", "timed_out"] {
        let managed_ready = temp.path().join(format!("{expected}.ready"));
        // Keep a real shell parent so the test proves descendant-group stopping,
        // not just termination of the directly spawned process.
        let input = json!({"command":format!("ROVE_TEST_SERVICE_READY={} {} --exact tools::tests::independent_service_probe --nocapture & wait", quote(&managed_ready), quote(&test_bin)),"timeout_seconds":if expected=="timed_out" {1}else{10}});
        let cancel = CancellationToken::new();
        let execution = system_exec(agent.clone(), &run, expected, &input, cancel.clone());
        let observer = async {
            let managed = service_ready(&managed_ready).await;
            ping_service(managed).await;
            if expected == "cancelled" {
                cancel.cancel();
            }
            managed
        };
        let (output, managed) = tokio::time::timeout(Duration::from_secs(8), async {
            tokio::join!(execution, observer)
        })
        .await
        .unwrap();
        AGENT.validate("SystemExecResult", &output).unwrap();
        assert_eq!(output["status"], expected);
        assert_eq!(output["residual_processes"], json!([]));
        assert!(tokio::net::TcpStream::connect(managed).await.is_err());
        assert!(independent.try_wait().unwrap().is_none());
        ping_service(address).await;
        assert_eq!(
            std::fs::read_to_string(&application_data).unwrap(),
            "preserve application data"
        );
    }
    agent.shutdown().await;
    ping_service(address).await;
    independent.kill().await.unwrap();
    independent.wait().await.unwrap();
}
