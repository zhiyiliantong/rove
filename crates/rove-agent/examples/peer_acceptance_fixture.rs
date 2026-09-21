//! Deterministic, loopback-only fixture for isolated two-host acceptance.
//! Not included in Rove packages and never an externally hosted model.
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde_json::{Value, json};

async fn completion(
    State(network): State<String>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let finished = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "tool");
    let delta = if finished {
        json!({"role":"assistant","content":"Controlled two-host deployment finished; verify the actual tool results."})
    } else {
        json!({"role":"assistant","tool_calls":[
            {"index":0,"id":"deploy","type":"function","function":{"name":"system_exec","arguments":json!({"command":"/opt/fixture app >/state/app.log 2>&1 & sleep 0.2","timeout_seconds":5}).to_string()}},
            {"index":1,"id":"publish","type":"function","function":{"name":"publish_service","arguments":json!({"network_id":network,"name":"Two-host fixture","target":{"host":"127.0.0.1","port":19081},"protocol":"http","access_info":"Bearer rove-test-only"}).to_string()}}
        ]})
    };
    let mut output = String::new();
    for (delta, finish) in [
        (delta, Value::Null),
        (
            json!({}),
            json!(if finished { "stop" } else { "tool_calls" }),
        ),
    ] {
        let chunk = json!({"id":"fixture","object":"chat.completion.chunk","created":1,"model":"fixture","choices":[{"index":0,"delta":delta,"finish_reason":finish}]});
        output.push_str(&format!("data: {chunk}\n\n"));
    }
    output.push_str("data: [DONE]\n\n");
    ([("content-type", "text/event-stream")], output)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    anyhow::ensure!(
        std::env::var("ROVE_DISPOSABLE_OVERLAY_TEST").as_deref() == Ok("1"),
        "Only run in the explicit disposable test environment"
    );
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("app") => {
            let router = Router::new().route(
                "/",
                get(|headers: HeaderMap| async move {
                    if headers.get("authorization").and_then(|v| v.to_str().ok())
                        == Some("Bearer rove-test-only")
                    {
                        (StatusCode::OK, "rove-two-host-service")
                    } else {
                        (StatusCode::UNAUTHORIZED, "unauthorized")
                    }
                }),
            );
            axum::serve(
                tokio::net::TcpListener::bind("127.0.0.1:19081").await?,
                router,
            )
            .await?;
        }
        Some("model") => {
            let network = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing network ID"))?
                .parse::<uuid::Uuid>()?
                .to_string();
            let router = Router::new()
                .route("/v1/chat/completions", post(completion))
                .with_state(network);
            axum::serve(
                tokio::net::TcpListener::bind("127.0.0.1:19080").await?,
                router,
            )
            .await?;
        }
        Some("fetch") => {
            let address = args.get(2).ok_or_else(|| anyhow::anyhow!("Missing URL"))?;
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(3))
                .build()?;
            let mut request = client.get(address);
            if args.get(3).is_some_and(|s| s == "authenticated") {
                request = request.bearer_auth("rove-test-only");
            }
            match request.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    println!("{}", json!({"status":status,"body":response.text().await?}));
                }
                Err(_) => println!("{}", json!({"status":0,"body":"unreachable"})),
            }
        }
        _ => anyhow::bail!("Expected app, model <network-id> or fetch <URL> [authenticated]"),
    }
    Ok(())
}
