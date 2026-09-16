use crate::Agent;
use futures_util::StreamExt;
use rig::{
    client::CompletionClient,
    completion::{CompletionModel, Message, ToolDefinition},
    message::{AssistantContent, ToolResultContent, UserContent},
    providers::openai,
    streaming::StreamedAssistantContent,
};
use rove_protocol::{ApiError, contract::AGENT};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn provider_error() -> ApiError {
    ApiError::new(
        502,
        "model_provider_failed",
        "Model provider request failed; check model configuration and connectivity",
    )
}
fn cancelled() -> ApiError {
    ApiError::new(
        409,
        "run_cancelled",
        "Run cancelled; completed system changes are not rolled back",
    )
}
pub async fn execute(
    agent: Arc<Agent>,
    run: Value,
    config: Value,
    cancel: CancellationToken,
) -> Result<(), ApiError> {
    if !matches!(
        config["provider"].as_str(),
        Some("openai" | "openai_compatible")
    ) {
        return Err(ApiError::new(
            501,
            "unsupported",
            "This build supports openai and openai_compatible providers",
        ));
    }
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|_| provider_error())?;
    let client = openai::Client::builder()
        .api_key(config["api_key"].as_str().unwrap_or(""))
        .base_url(config["base_url"].as_str().unwrap())
        .http_client(http)
        .build()
        .map_err(|_| provider_error())?
        .completions_api();
    let model = client.completion_model(config["model"].as_str().unwrap());
    let id = run["run_id"].as_str().unwrap();
    let session = run["session_id"].as_str().unwrap();
    let mut history = agent.rig_history(session)?;
    let source: Value = serde_json::from_str(rove_protocol::AGENT_OPENAPI).unwrap();
    let tools=vec![ToolDefinition{name:"system_exec".into(),description:"Execute a shell command on this device, with its OS account. Commands may change the system. Use platform-appropriate commands and report actual results.".into(),parameters:source["components"]["schemas"]["SystemExecInput"].clone()},ToolDefinition{name:"publish_service".into(),description:"Publish an existing local TCP service into a joined network.".into(),parameters:expanded_service_schema(&source)}];
    for _step in 0..32 {
        if cancel.is_cancelled() {
            return Err(cancelled());
        }
        let prompt = history
            .pop()
            .ok_or_else(|| ApiError::new(500, "invalid_history", "Session history is empty"))?;
        let request=model.completion_request(prompt.clone()).messages(history.clone()).preamble(format!("You are rove-agent, a lightweight assembly agent on device {} ({} / {}). Execute only on this device. Network context: {}. You have system_exec and publish_service. Other devices are peers; do not assume a central controller. Never claim that a failed or unsupported operation succeeded. Do not include model credentials in output.",run["device_id"],std::env::consts::OS,std::env::consts::ARCH,run["network_id"])).tools(tools.clone());
        history.push(prompt);
        let mut stream = tokio::select! {_=cancel.cancelled()=>return Err(cancelled()),response=request.stream()=>response.map_err(|_|provider_error())?};
        let mut text = String::new();
        let mut emitted_calls = Vec::new();
        let mut failure = None;
        let mut tool_bytes = 0usize;
        loop {
            let next = tokio::select! {_=cancel.cancelled()=>{failure=Some(cancelled());break;},next=stream.next()=>next};
            match next {
                Some(Ok(StreamedAssistantContent::Text(delta))) => {
                    if text.len() + delta.text.len() > 1024 * 1024 {
                        failure = Some(ApiError::new(
                            413,
                            "model_output_too_large",
                            "Model output exceeded the per-turn limit",
                        ));
                        break;
                    }
                    text.push_str(&delta.text);
                    let chars: Vec<char> = delta.text.chars().collect();
                    for chunk in chars.chunks(16384) {
                        agent.append_event(
                            id,
                            "assistant_delta",
                            json!({"text":chunk.iter().collect::<String>()}),
                        )?;
                    }
                }
                Some(Ok(StreamedAssistantContent::ToolCall { tool_call, .. })) => {
                    if tool_call.function.arguments.to_string().len() > 1024 * 1024 {
                        failure = Some(ApiError::new(
                            413,
                            "tool_input_too_large",
                            "Tool arguments exceed 1 MiB",
                        ));
                        break;
                    }
                    emitted_calls.push(tool_call);
                    if emitted_calls.len() > 32 {
                        failure = Some(ApiError::new(
                            413,
                            "too_many_tools",
                            "Too many tools in one model turn",
                        ));
                        break;
                    }
                }
                Some(Ok(StreamedAssistantContent::ToolCallDelta { content, .. })) => {
                    let part = match content {
                        rig::streaming::ToolCallDeltaContent::Name(s)
                        | rig::streaming::ToolCallDeltaContent::Delta(s) => s,
                    };
                    tool_bytes = tool_bytes.saturating_add(part.len());
                    if tool_bytes > 4 * 1024 * 1024 {
                        failure = Some(ApiError::new(
                            413,
                            "tool_input_too_large",
                            "Streamed tool arguments exceed 4 MiB",
                        ));
                        break;
                    }
                }
                Some(Ok(_)) => {}
                Some(Err(_)) => {
                    failure = Some(provider_error());
                    break;
                }
                None => break,
            }
        }
        if failure.is_none() && stream.response.is_none() {
            failure = Some(provider_error());
        }
        if let Some(error) = failure {
            if !text.is_empty() {
                let message = Message::assistant(&text);
                agent.save_message(
                    &run,
                    "assistant",
                    json!([{"kind":"text","text":text}]),
                    Some(&message),
                )?;
            }
            return Err(error);
        }
        let mut parts = Vec::new();
        if !text.is_empty() {
            parts.push(json!({"kind":"text","text":text}));
        }
        for call in &emitted_calls {
            let wire = json!({"tool_call_id":call.id.to_string(),"tool_name":call.function.name,"arguments":call.function.arguments});
            AGENT.validate("ToolCall", &wire)?;
            parts.push(json!({"kind":"tool_call","call":wire}));
        }
        let assistant = Message::Assistant {
            id: stream.message_id.clone(),
            content: if stream.choice.is_empty() {
                vec![AssistantContent::text(&text)]
            } else {
                stream.choice.clone()
            },
        };
        agent.save_message(&run, "assistant", json!(parts), Some(&assistant))?;
        history.push(assistant);
        if emitted_calls.is_empty() {
            return Ok(());
        }
        // Intentionally no join_all: tools in one run execute in model order.
        for call in emitted_calls {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            let call_id = call.id.to_string();
            let name = &call.function.name;
            agent.append_event(id,"tool_started",json!({"tool_call_id":call_id,"tool_name":name,"arguments":call.function.arguments}))?;
            let result = if name == "system_exec" {
                json!({"tool_call_id":call_id,"tool_name":name,"result":crate::tools::system_exec(agent.clone(),id,&call_id,&call.function.arguments,cancel.clone()).await})
            } else {
                let response = agent
                    .handle(
                        &rove_protocol::Request::new("publish_service")
                            .with_body(call.function.arguments.clone()),
                    )
                    .await;
                if response.status_code < 400 {
                    json!({"tool_call_id":call_id,"tool_name":name,"result":response.body.unwrap(),"error":null})
                } else {
                    json!({"tool_call_id":call_id,"tool_name":name,"result":null,"error":response.body.unwrap()["error"]})
                }
            };
            AGENT.validate("ToolResult", &result)?;
            agent.append_event(id, "tool_finished", result.clone())?;
            let message = Message::User {
                content: vec![UserContent::tool_result_for(
                    call.id,
                    call.provider,
                    name,
                    vec![ToolResultContent::text(result.to_string())],
                )],
            };
            agent.save_message(
                &run,
                "tool",
                json!([{"kind":"tool_result","result":result}]),
                Some(&message),
            )?;
            history.push(message);
        }
    }
    Err(ApiError::new(
        422,
        "step_limit_reached",
        "Run reached 32 model turns; inspect results and continue with a new message",
    ))
}
fn expanded_service_schema(source: &Value) -> Value {
    let mut schema = source["components"]["schemas"]["ServiceWrite"].clone();
    schema["properties"]["target"] = source["components"]["schemas"]["ServiceTarget"].clone();
    schema
}
