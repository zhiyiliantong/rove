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
    let response = agent.handle(&request).await;
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
    async fn management_uses_real_operations_and_rejects_unregistered_or_remote_requests() {
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
            json!({"operation_id":"update_settings","body":{"max_active_runs":0}}),
        ] {
            assert!(call(&agent, &input).await.is_err());
        }
        assert_eq!(call(&agent,&json!({"operation_id":"get_service","path_parameters":{"service_id":uuid::Uuid::new_v4()}})).await.unwrap_err().status,404);
        agent.shutdown().await;
    }
}
