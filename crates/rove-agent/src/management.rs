use crate::Agent;
use rove_protocol::{ApiError, Request, contract::AGENT};
use serde_json::Value;
use std::sync::Arc;

/// Tool calls share the same contract, validation and implementation as CLI/API.
pub(crate) async fn call(agent: &Arc<Agent>, input: &Value) -> Result<Option<Value>, ApiError> {
    AGENT.validate("RoveApiInput", input)?;
    let mut request = Request::new(input["operation_id"].as_str().unwrap());
    if let Some(path) = input.get("path_parameters").and_then(Value::as_object) {
        for (key, value) in path {
            request
                .path_parameters
                .insert(key.clone(), value.as_str().unwrap().to_string());
        }
    }
    if let Some(query) = input.get("query_parameters").and_then(Value::as_object) {
        request.query_parameters.extend(
            query
                .iter()
                .map(|(key, value)| (key.clone(), value.clone())),
        );
    }
    request.body = input.get("body").cloned();
    let target: Option<rove_protocol::SocketTarget> = input
        .get("target")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .map_err(|_| ApiError::invalid("Invalid target"))?;
    let remote_session = matches!(
        request.operation_id.as_str(),
        "create_session" | "get_session" | "submit_run" | "list_runs" | "get_run" | "cancel_run"
    );
    if crate::device_aliases::is_local_operation(&request.operation_id) && target.is_some() {
        return Err(ApiError::invalid(
            "Device aliases must be resolved on this device without target",
        ));
    }
    if remote_session
        && target
            .as_ref()
            .is_none_or(|t| t.device_id == agent.store.device_id.0)
    {
        return Err(ApiError::invalid(
            "Session tools require an explicit remote device; keep the current conversation local",
        ));
    }
    if request.operation_id == "create_session" {
        let body = request.body.as_ref().unwrap_or(&Value::Null);
        if body.get("execution_target").is_some() || body.get("session_id").is_none() {
            return Err(ApiError::invalid(
                "Remote creation requires a stable session_id and no execution_target",
            ));
        }
    }
    AGENT.request(&request)?;
    let response = if let Some(target) = target {
        if target.device_id == agent.store.device_id.0 {
            return Err(ApiError::invalid(
                "Omit target for operations on this device",
            ));
        }
        // Reuse the discovered overlay client; never substitute local execution.
        agent.linked_client(&target).await?.call(request).await.map_err(|_| {
            ApiError::new(503, "target_unreachable", "Remote outcome is unknown; query the same IDs before retrying; no local execution occurred")
        })?
    } else {
        agent.handle(&request).await
    };
    if response.status_code >= 400 {
        let mut error: ApiError =
            serde_json::from_value(response.body.unwrap()["error"].clone())
                .map_err(|_| ApiError::new(500, "invalid_error", "Invalid operation error"))?;
        error.status = response.status_code;
        return Err(error);
    }
    Ok(response.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[tokio::test]
    async fn management_uses_real_operations_and_rejects_unsafe_session_requests() {
        let tmp = tempfile::tempdir().unwrap();
        let agent = Agent::open(&tmp.path().join("agent")).unwrap();
        assert_eq!(
            call(&agent, &json!({"operation_id":"get_device"}))
                .await
                .unwrap()
                .unwrap()["device_id"],
            agent.store.device_id.to_string()
        );
        call(
            &agent,
            &json!({"operation_id":"update_settings","body":{"max_active_runs":2}}),
        )
        .await
        .unwrap();
        assert_eq!(agent.store.settings().unwrap()["max_active_runs"], 2);
        for input in [
            json!({"operation_id":"submit_run"}),
            json!({"operation_id":"get_device","target":{}}),
            json!({"operation_id":"create_session","body":{"session_id":uuid::Uuid::new_v4()}}),
            json!({"operation_id":"create_session","target":{"network_id":uuid::Uuid::new_v4(),"device_id":agent.store.device_id},"body":{"session_id":uuid::Uuid::new_v4()}}),
            json!({"operation_id":"create_session","target":{"network_id":uuid::Uuid::new_v4(),"device_id":uuid::Uuid::new_v4()},"body":{}}),
            json!({"operation_id":"create_session","target":{"network_id":uuid::Uuid::new_v4(),"device_id":uuid::Uuid::new_v4()},"body":{"session_id":uuid::Uuid::new_v4(),"execution_target":{}}}),
            json!({"operation_id":"update_settings","body":{"max_active_runs":0}}),
        ] {
            assert!(call(&agent, &input).await.is_err());
        }
        assert_eq!(call(&agent,&json!({"operation_id":"get_service","path_parameters":{"service_id":uuid::Uuid::new_v4()}})).await.unwrap_err().status,404);
        agent.shutdown().await;
    }
}
