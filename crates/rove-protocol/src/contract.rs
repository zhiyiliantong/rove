use crate::{AGENT_OPENAPI, ApiError, BLOB_OPENAPI, Request, Response};
use jsonschema::Validator;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::LazyLock};

pub static AGENT: LazyLock<Contract> = LazyLock::new(|| Contract::new(AGENT_OPENAPI));
pub static BLOBS: LazyLock<Contract> = LazyLock::new(|| Contract::new(BLOB_OPENAPI));

pub struct Operation {
    pub method: String,
    pub path: String,
    pub definition: Value,
    path_validator: Validator,
    query_validator: Validator,
    body_validator: Option<Validator>,
    body_required: bool,
    responses: BTreeMap<String, Option<Validator>>,
}
pub struct Contract {
    pub schemas: BTreeMap<String, Validator>,
    pub operations: BTreeMap<String, Operation>,
}

fn resolve<'a>(root: &'a Value, node: &'a Value) -> &'a Value {
    match node.get("$ref").and_then(Value::as_str) {
        Some(reference) => resolve(
            root,
            root.pointer(reference.strip_prefix('#').expect("local reference"))
                .expect("valid reference"),
        ),
        None => node,
    }
}
fn compile(root: &Value, schema: &Value) -> Validator {
    let mut schema = schema.clone();
    schema["components"] = root["components"].clone();
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(true)
        .build(&schema)
        .expect("valid OpenAPI schema")
}
impl Contract {
    pub fn new(source: &str) -> Self {
        let root: Value = serde_json::from_str(source).expect("valid OpenAPI JSON");
        let schemas = root["components"]["schemas"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, schema)| (name.clone(), compile(&root, schema)))
            .collect();
        let mut operations = BTreeMap::new();
        for (path, item) in root["paths"].as_object().unwrap() {
            for (method, def) in item.as_object().unwrap() {
                let Some(id) = def["operationId"].as_str() else {
                    continue;
                };
                let parameters: Vec<_> = item["parameters"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .chain(def["parameters"].as_array().into_iter().flatten())
                    .map(|p| resolve(&root, p))
                    .collect();
                let parameter_schema = |location: &str| {
                    let mut value = json!({"type":"object","properties":{},"required":[],"additionalProperties":false});
                    for p in parameters.iter().filter(|p| p["in"] == location) {
                        let name = p["name"].as_str().unwrap();
                        value["properties"][name] = p["schema"].clone();
                        if p["required"] == true {
                            value["required"].as_array_mut().unwrap().push(json!(name));
                        }
                    }
                    compile(&root, &value)
                };
                let body = resolve(&root, &def["requestBody"]);
                let body_schema = &body["content"]["application/json"]["schema"];
                let mut responses = BTreeMap::new();
                for (status, response) in def["responses"].as_object().unwrap() {
                    let response = resolve(&root, response);
                    let schema = &response["content"]["application/json"]["schema"];
                    responses.insert(
                        status.clone(),
                        (!schema.is_null()).then(|| compile(&root, schema)),
                    );
                }
                let operation = Operation {
                    method: method.to_uppercase(),
                    path: path.clone(),
                    definition: def.clone(),
                    path_validator: parameter_schema("path"),
                    query_validator: parameter_schema("query"),
                    body_validator: (!body_schema.is_null()).then(|| compile(&root, body_schema)),
                    body_required: body["required"] == true,
                    responses,
                };
                assert!(
                    operations.insert(id.to_string(), operation).is_none(),
                    "duplicate operationId"
                );
            }
        }
        Self {
            schemas,
            operations,
        }
    }
    pub fn validate(&self, name: &str, value: &Value) -> Result<(), ApiError> {
        if self.schemas.get(name).is_some_and(|v| v.is_valid(value)) {
            Ok(())
        } else {
            Err(ApiError::invalid(&format!("Value does not match {name}")))
        }
    }
    pub fn request(&self, request: &Request) -> Result<&Operation, ApiError> {
        self.validate("SocketRequest", &serde_json::to_value(request).unwrap())?;
        let op = self
            .operations
            .get(&request.operation_id)
            .ok_or_else(|| ApiError::new(404, "operation_not_found", "Unknown operation_id"))?;
        if !op.path_validator.is_valid(&json!(request.path_parameters))
            || !op
                .query_validator
                .is_valid(&json!(request.query_parameters))
        {
            return Err(ApiError::invalid(
                "Path or query parameters do not match the operation",
            ));
        }
        match (&request.body, &op.body_validator) {
            (Some(body), Some(schema)) if schema.is_valid(body) => {}
            (None, _) if !op.body_required => {}
            _ => return Err(ApiError::invalid("Body does not match the operation")),
        }
        Ok(op)
    }
    pub fn response(&self, op_id: &str, response: &Response) -> Result<(), ApiError> {
        self.validate("SocketResponse", &json!(response))?;
        if response.status_code == 204 {
            return if response.body.is_none() {
                Ok(())
            } else {
                Err(ApiError::invalid("204 must not have a body"))
            };
        }
        if op_id == "subscribe_run_events" && response.status_code == 200 {
            return self.validate(
                "SocketStreamOpened",
                response.body.as_ref().unwrap_or(&Value::Null),
            );
        }
        let op = self
            .operations
            .get(op_id)
            .ok_or_else(|| ApiError::invalid("Unknown operation response"))?;
        match op
            .responses
            .get(&response.status_code.to_string())
            .or_else(|| op.responses.get("default"))
        {
            Some(Some(validator))
                if response
                    .body
                    .as_ref()
                    .is_some_and(|b| validator.is_valid(b)) =>
            {
                Ok(())
            }
            Some(None) if response.body.is_none() => Ok(()),
            _ => Err(ApiError::invalid("Response does not match the operation")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_contract_examples() {
        assert_eq!(AGENT.operations.len(), 49);
        assert_eq!(BLOBS.operations.len(), 3);
        for (source, contract) in [(AGENT_OPENAPI, &*AGENT), (BLOB_OPENAPI, &*BLOBS)] {
            let root: Value = serde_json::from_str(source).unwrap();
            for (name, schema) in root["components"]["schemas"].as_object().unwrap() {
                for example in schema["examples"].as_array().into_iter().flatten() {
                    contract.validate(name, example).unwrap();
                }
            }
        }
        let cases: Value =
            serde_json::from_str(include_str!("../../../api/contract-examples.json")).unwrap();
        for case in cases["cases"].as_array().unwrap() {
            let contract = if case["api"] == "rove-agent.openapi.json" {
                &*AGENT
            } else {
                &*BLOBS
            };
            assert_eq!(
                contract
                    .validate(case["schema"].as_str().unwrap(), &case["value"])
                    .is_ok(),
                case["valid"].as_bool().unwrap(),
                "{}",
                case["schema"]
            );
        }
    }
    #[test]
    fn operation_level_validation_and_version() {
        assert!(AGENT.request(&Request::new("create_session")).is_err());
        assert!(
            AGENT
                .request(&Request::new("create_session").with_body(json!({})))
                .is_ok()
        );
        assert!(
            AGENT
                .request(&Request::new("update_settings").with_body(json!({"max_active_runs":0})))
                .is_err()
        );
        assert!(
            AGENT
                .request(&Request::new("get_device").with_body(Value::Null))
                .is_err()
        );
        assert!(crate::check_protocol(2, 3).is_err());
        assert!(crate::check_protocol(1, 1).is_ok());
    }
}
