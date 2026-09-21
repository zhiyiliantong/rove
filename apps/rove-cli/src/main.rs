use clap::{Parser, Subcommand};
use rove_protocol::Request;
use serde_json::{Value, json};
use std::path::PathBuf;
use uuid::Uuid;
mod commands;
mod interactive;

#[derive(Parser)]
#[command(name = "rove", version, about = "Rove — peer device roaming")]
struct Args {
    #[arg(long, env = "ROVE_DATA_DIR", global = true)]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long = "network", global = true, requires = "target_device")]
    target_network: Option<Uuid>,
    #[arg(long = "device", global = true, requires = "target_network")]
    target_device: Option<Uuid>,
}
#[derive(Subcommand)]
enum Command {
    /// Interactive menu; also the default when no command is provided.
    Interactive,
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    Network {
        #[command(subcommand)]
        command: commands::Network,
    },
    Device {
        #[command(subcommand)]
        command: commands::Device,
    },
    Model {
        #[command(subcommand)]
        command: commands::Model,
    },
    Config {
        #[command(subcommand)]
        command: commands::Config,
    },
    Service {
        #[command(subcommand)]
        command: commands::Service,
    },
    /// Query the running local agent.
    Status,
    /// List every OpenAPI operation and its HTTP mapping.
    Operations,
    /// Origin-owned conversations, optionally linked to a remote execution peer.
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    /// Inspect, cancel or follow an accepted run without resubmitting it.
    Run {
        #[command(subcommand)]
        command: RunCommand,
    },
    /// Invoke an OpenAPI operation. Read sensitive JSON from stdin with --body -.
    Call {
        operation_id: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long="path", value_parser=parse_pair)]
        path: Vec<(String, String)>,
        #[arg(long="query", value_parser=parse_pair)]
        query: Vec<(String, String)>,
    },
}
#[derive(Subcommand)]
enum AgentCommand {
    /// Query the agent runtime; this does not start or stop a system service.
    Status,
}
#[derive(Subcommand)]
enum SessionCommand {
    List {
        #[arg(long)]
        archived: Option<bool>,
        /// Sort by creation time, newest first by default.
        #[arg(long, default_value = "desc", value_parser = ["asc", "desc"])]
        order: String,
        #[command(flatten)]
        page: commands::Page,
    },
    Archive {
        session_id: Uuid,
    },
    Restore {
        session_id: Uuid,
    },
    /// Permanently delete an archived, inactive conversation (not deployed files).
    Delete {
        session_id: Uuid,
        #[arg(long, required = true)]
        yes: bool,
    },
    Create {
        #[arg(long)]
        title: Option<String>,
        /// Treat the initial title as a placeholder until the first accepted message.
        #[arg(long)]
        auto_title: bool,
        #[arg(long, requires = "execution_device")]
        execution_network: Option<Uuid>,
        #[arg(long, requires = "execution_network")]
        execution_device: Option<Uuid>,
        /// Stable creation ID for safely retrying a lost response.
        #[arg(long)]
        session_id: Option<Uuid>,
    },
    /// List persisted, unconfirmed remote requests; never resubmits them.
    Pending {
        session_id: Uuid,
        #[command(flatten)]
        page: commands::Page,
    },
    Show {
        session_id: Uuid,
    },
    Messages {
        session_id: Uuid,
        #[command(flatten)]
        page: commands::Page,
    },
    Rename {
        session_id: Uuid,
        title: String,
    },
    /// Persist a model choice for this session; omit model_id to follow device default.
    Model {
        session_id: Uuid,
        model_id: Option<Uuid>,
    },
    Send {
        session_id: Uuid,
        message: String,
        #[arg(long)]
        request_id: Option<Uuid>,
        #[arg(long)]
        model_id: Option<Uuid>,
    },
}
#[derive(Subcommand)]
enum RunCommand {
    List {
        #[arg(long)]
        session_id: Option<Uuid>,
        #[arg(long)]
        status: Option<String>,
        #[command(flatten)]
        page: commands::Page,
    },
    Show {
        run_id: Uuid,
    },
    Cancel {
        run_id: Uuid,
    },
    Watch {
        run_id: Uuid,
        #[arg(long, default_value_t = 0)]
        after_seq: i64,
    },
}
fn parse_pair(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.into(), v.into()))
        .ok_or_else(|| "Expected name=value".into())
}
#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("rove: {error}");
        std::process::exit(1);
    }
}
async fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let dir = args.data_dir.unwrap_or_else(rove_sdk::default_data_dir);
    let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
    let client = rove_sdk::LocalClient::new(rove_sdk::socket_path(&dir));
    let target = args
        .target_network
        .zip(args.target_device)
        .map(|(network_id, device_id)| rove_protocol::SocketTarget {
            network_id,
            device_id,
        });
    let mut show_qr = false;
    let mut open_endpoint = None;
    let mut request = match args.command.unwrap_or(Command::Interactive) {
        Command::Interactive => return interactive::run(&client, target, args.json).await,
        Command::Network { command } => {
            let (request, qr) = command.request()?;
            show_qr = qr;
            request
        }
        Command::Device { command } => command.request(),
        Command::Model { command } => command.request()?,
        Command::Config { command } => command.request()?,
        Command::Service { command } => {
            if let commands::Service::Open { endpoint, .. } = &command {
                open_endpoint = Some(*endpoint);
            }
            command.request()?
        }
        Command::Operations => {
            let values: Vec<_> = rove_protocol::contract::AGENT.operations.iter().map(|(id,op)| serde_json::json!({"operation_id":id,"method":op.method,"path":op.path})).collect();
            println!("{}", serde_json::to_string_pretty(&values)?);
            return Ok(());
        }
        Command::Status
        | Command::Agent {
            command: AgentCommand::Status,
        } => Request::new("get_device"),
        Command::Session { command } => match command {
            SessionCommand::List {
                page,
                archived,
                order,
            } => {
                let mut request = page.apply(Request::new("list_sessions"));
                request
                    .query_parameters
                    .insert("order".into(), json!(order));
                if let Some(value) = archived {
                    request
                        .query_parameters
                        .insert("archived".into(), json!(value));
                }
                request
            }
            SessionCommand::Archive { session_id } => {
                Request::new("archive_session").with_path("session_id", session_id.to_string())
            }
            SessionCommand::Restore { session_id } => {
                Request::new("restore_session").with_path("session_id", session_id.to_string())
            }
            SessionCommand::Delete { session_id, yes: _ } => {
                Request::new("delete_session").with_path("session_id", session_id.to_string())
            }
            SessionCommand::Create {
                title,
                auto_title,
                execution_network,
                execution_device,
                session_id,
            } => {
                let mut body = title.map_or(json!({}), |title| json!({"title":title}));
                if auto_title {
                    body["auto_title"] = json!(true);
                }
                if let Some((network_id, device_id)) = execution_network.zip(execution_device) {
                    body["execution_target"] =
                        json!({"network_id":network_id,"device_id":device_id});
                }
                if let Some(id) = session_id {
                    body["session_id"] = json!(id);
                }
                Request::new("create_session").with_body(body)
            }
            SessionCommand::Pending { session_id, page } => page.apply(
                Request::new("list_session_submissions").with_path("session_id", session_id),
            ),
            SessionCommand::Show { session_id } => {
                Request::new("get_session").with_path("session_id", session_id)
            }
            SessionCommand::Messages { session_id, page } => {
                page.apply(Request::new("list_messages").with_path("session_id", session_id))
            }
            SessionCommand::Rename { session_id, title } => Request::new("update_session")
                .with_path("session_id", session_id)
                .with_body(json!({"title":title})),
            SessionCommand::Model {
                session_id,
                model_id,
            } => Request::new("update_session")
                .with_path("session_id", session_id)
                .with_body(json!({"model_id":model_id})),
            SessionCommand::Send {
                session_id,
                message,
                request_id,
                model_id,
            } => {
                let mut body = json!({"request_id":request_id.unwrap_or_else(Uuid::new_v4),"message":commands::input(&message)?});
                if let Some(model_id) = model_id {
                    body["model_id"] = json!(model_id);
                }
                Request::new("submit_run")
                    .with_path("session_id", session_id)
                    .with_body(body)
            }
        },
        Command::Run { command } => match command {
            RunCommand::List {
                session_id,
                status,
                page,
            } => {
                let mut request = page.apply(Request::new("list_runs"));
                if let Some(id) = session_id {
                    request
                        .query_parameters
                        .insert("session_id".into(), json!(id));
                }
                if let Some(status) = status {
                    request
                        .query_parameters
                        .insert("status".into(), json!(status));
                }
                request
            }
            RunCommand::Show { run_id } => Request::new("get_run").with_path("run_id", run_id),
            RunCommand::Cancel { run_id } => Request::new("cancel_run").with_path("run_id", run_id),
            RunCommand::Watch { run_id, after_seq } => {
                if !args.json {
                    show_target(target.as_ref());
                }
                return watch(&client, run_id, after_seq, target, args.json).await;
            }
        },
        Command::Call {
            operation_id,
            body,
            path,
            query,
        } => {
            let mut request = Request::new(&operation_id);
            request.path_parameters.extend(path);
            for (key, value) in query {
                request.query_parameters.insert(
                    key,
                    serde_json::from_str(&value).unwrap_or(Value::String(value)),
                );
            }
            if let Some(body) = body {
                request.body = Some(commands::json_input(&body)?);
            }
            request
        }
    };
    request.target = target;
    if !args.json {
        show_target(request.target.as_ref());
        if request.operation_id == "submit_run" {
            eprintln!(
                "request_id: {}",
                request.body.as_ref().unwrap()["request_id"]
                    .as_str()
                    .unwrap()
            );
        }
    }
    let mut body = execute(&client, request).await?;
    if let Some(index) = open_endpoint {
        let service = body
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing service response"))?;
        let address = commands::browser_endpoint(service, index)?;
        #[cfg(target_os = "linux")]
        anyhow::ensure!(
            std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some(),
            "No graphical session; use service show and copy the endpoint to a browser device"
        );
        open::that_detached(&address).map_err(|_| {
            anyhow::anyhow!(
                "Could not launch the default browser; use service show and copy the endpoint"
            )
        })?;
        // OS handoff is not proof that a browser connected to the application.
        body =
            Some(json!({"service_id":service["service_id"],"open_requested":true,"url":address}));
    }
    if show_qr && let Some(url) = body.as_ref().and_then(|v| v["url"].as_str()) {
        eprintln!(
            "Share QR includes the network key; do not publish it.\n{}",
            rove_sdk::ShareQr::new(url)?.terminal()
        );
    }
    if args.json {
        println!("{}", serde_json::to_string(&body)?);
    } else if let Some(body) = body {
        println!("{}", serde_json::to_string_pretty(&body)?);
    }
    Ok(())
}
fn show_target(target: Option<&rove_protocol::SocketTarget>) {
    if let Some(target) = target {
        eprintln!(
            "Target: device {} on network {}",
            target.device_id, target.network_id
        );
    } else {
        eprintln!("Target: local rove-agent");
    }
}
async fn execute(
    client: &rove_sdk::LocalClient,
    request: Request,
) -> anyhow::Result<Option<Value>> {
    let request_id = if request.operation_id == "submit_run" {
        request
            .body
            .as_ref()
            .and_then(|body| body["request_id"].as_str())
            .map(str::to_owned)
    } else {
        None
    };
    let response = client.call(request).await.map_err(|error| {
        if let Some(id) = request_id {
            anyhow::anyhow!(
                "{error}; if retrying this submission, reuse request_id={id} and the same message"
            )
        } else {
            error
        }
    })?;
    if response.status_code >= 400 {
        let error = response.body.unwrap_or(Value::Null);
        anyhow::bail!(
            "{}: {}",
            error["error"]["code"].as_str().unwrap_or("request_failed"),
            error["error"]["message"]
                .as_str()
                .unwrap_or("Request failed")
        );
    }
    Ok(response.body)
}
async fn watch(
    client: &rove_sdk::LocalClient,
    run_id: Uuid,
    mut after: i64,
    target: Option<rove_protocol::SocketTarget>,
    json_output: bool,
) -> anyhow::Result<()> {
    let mut failures = 0;
    loop {
        let mut subscription = match client.subscribe_target(run_id, after, target.clone()).await {
            Ok(stream) => stream,
            Err(error) if error.to_string().contains("event_history_expired") => {
                let mut request = Request::new("get_run").with_path("run_id", run_id);
                request.target = target.clone();
                let snapshot = client.call(request).await?;
                anyhow::ensure!(
                    snapshot.status_code == 200,
                    "Could not restore run snapshot"
                );
                let snapshot = snapshot.body.unwrap();
                after = snapshot["snapshot_seq"].as_i64().unwrap();
                if json_output {
                    println!("{}", json!({"kind":"snapshot","snapshot":snapshot}));
                } else {
                    println!("{}", snapshot["output_tail"].as_str().unwrap());
                    eprintln!("Restored retained output; older events expired");
                }
                continue;
            }
            Err(error) => return Err(error),
        };
        loop {
            tokio::select! {
                // Leaving watch only releases this observer; never cancel run.
                _=tokio::signal::ctrl_c()=>return Ok(()),
                next=subscription.next()=>match next {
                    Ok(Some(event))=>{
                        failures=0;after=subscription.last_seq;
                        if json_output{println!("{event}");}else if matches!(event["kind"].as_str(),Some("assistant_delta"|"tool_output")){print!("{}",event["data"]["text"].as_str().unwrap());use std::io::Write;std::io::stdout().flush()?;}else if event["kind"]=="status"{eprintln!("run {run_id}: {}",event["data"]["status"].as_str().unwrap());}
                    },
                    Ok(None)=>return Ok(()),
                    Err(error)=>{failures+=1;if failures>=3{return Err(error);}eprintln!("Event connection interrupted; reconnecting from seq {after}");tokio::time::sleep(std::time::Duration::from_millis(300)).await;break;},
                },
            }
        }
    }
}
