//! Structured wire models generated from OpenAPI during the build.
//! Use `WireDto::from_value` / `to_value` at boundaries: JSON Schema remains
//! authoritative for numeric limits, conditional constraints and formats.
//! Models deliberately omit Debug because some fields contain credentials.
use crate::{ApiError, contract::Contract};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

/// JSON Schema integers include integral JSON numbers such as `1.0`. Preserve
/// their representation rather than narrowing every wire integer to an i64.
#[derive(Clone, Serialize)]
#[serde(transparent)]
pub struct Integer(serde_json::Number);
impl<'de> Deserialize<'de> for Integer {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let number = serde_json::Number::deserialize(d)?;
        if number.is_i64() || number.is_u64() || number.as_f64().is_some_and(|n| n.fract() == 0.0) {
            Ok(Self(number))
        } else {
            Err(serde::de::Error::custom("Expected an integer"))
        }
    }
}
impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Self(value.into())
    }
}
impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Self(value.into())
    }
}
impl Integer {
    pub fn as_u64(&self) -> Option<u64> {
        self.0.as_u64().or_else(|| {
            self.0
                .as_f64()
                .filter(|n| *n >= 0.0 && *n < 18446744073709551616.0)
                .map(|n| n as u64)
        })
    }
    pub fn as_i64(&self) -> Option<i64> {
        self.0.as_i64().or_else(|| {
            self.0
                .as_f64()
                .filter(|n| *n >= -9223372036854775808.0 && *n < 9223372036854775808.0)
                .map(|n| n as i64)
        })
    }
}

pub trait WireDto: Serialize + DeserializeOwned {
    const SCHEMA: &'static str;
    fn contract() -> &'static Contract;
    fn from_value(value: Value) -> Result<Self, ApiError> {
        Self::contract().validate(Self::SCHEMA, &value)?;
        serde_json::from_value(value)
            .map_err(|_| ApiError::invalid("Wire model could not be decoded"))
    }
    fn to_value(&self) -> Result<Value, ApiError> {
        let value = serde_json::to_value(self)
            .map_err(|_| ApiError::invalid("Wire model could not be encoded"))?;
        Self::contract().validate(Self::SCHEMA, &value)?;
        Ok(value)
    }
}

fn present<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
fn required<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(d: D) -> Result<T, D::Error> {
    T::deserialize(d)
}

pub mod agent {
    include!(concat!(env!("OUT_DIR"), "/agent_dto.rs"));
}
pub mod config_server {
    include!(concat!(env!("OUT_DIR"), "/config_server_dto.rs"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn presence_limits_and_secrets() {
        assert!(serde_json::from_value::<agent::ModelConfigWrite>(json!({"provider":"openai_compatible","base_url":"http://localhost/v1","model":"test"})).is_err());
        let explicit_null: agent::ModelConfigWrite = serde_json::from_value(json!({"provider":"openai_compatible","base_url":"http://localhost/v1","model":"test","api_key":null})).unwrap();
        assert!(serde_json::to_value(explicit_null).unwrap()["api_key"].is_null());
        for body in [None, Some(Value::Null)] {
            let mut request = json!({"kind":"request","correlation_id":uuid::Uuid::new_v4(),"operation_id":"get_device"});
            if let Some(body) = body {
                request["body"] = body;
            }
            let parsed: agent::SocketRequest = serde_json::from_value(request.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed).unwrap(), request);
        }
        assert!(
            agent::SystemExecInput::from_value(json!({"command":"true","timeout_seconds":0}))
                .is_err()
        );
        assert!(
            agent::SystemExecInput::from_value(json!({"command":"true","timeout_seconds":null}))
                .is_err()
        );
        assert!(agent::SettingsPatch::from_value(json!({})).is_err());
        let input = json!({"command":"true", "timeout_seconds":1.0});
        let parsed = agent::SystemExecInput::from_value(input.clone()).unwrap();
        assert_eq!(parsed.timeout_seconds.as_ref().unwrap().as_u64(), Some(1));
        assert_eq!(parsed.to_value().unwrap(), input);
        assert!(serde_json::from_value::<Integer>(json!(1.5)).is_err());
        let unsigned: Integer = serde_json::from_value(json!(u64::MAX)).unwrap();
        assert_eq!(unsigned.as_u64(), Some(u64::MAX));
        assert!(unsigned.as_i64().is_none());
    }

    #[test]
    fn shared_examples_roundtrip_without_losing_fields() {
        for (source, is_agent) in [(crate::AGENT_OPENAPI, true), (crate::BLOB_OPENAPI, false)] {
            let root: Value = serde_json::from_str(source).unwrap();
            for (name, schema) in root["components"]["schemas"].as_object().unwrap() {
                for example in schema["examples"].as_array().into_iter().flatten() {
                    let actual = if is_agent {
                        agent::roundtrip(name, example.clone())
                    } else {
                        config_server::roundtrip(name, example.clone())
                    }
                    .unwrap();
                    assert_eq!(&actual, example, "schema {name}");
                }
            }
        }
        let fixtures: Value = serde_json::from_str(include_str!(
            "../../../openspec/changes/bootstrap-rove/api/contract-examples.json"
        ))
        .unwrap();
        for case in fixtures["cases"].as_array().unwrap() {
            let name = case["schema"].as_str().unwrap();
            let value = case["value"].clone();
            let is_agent = case["api"] == "rove-agent.openapi.json";
            let contract = if is_agent {
                &crate::contract::AGENT
            } else {
                &crate::contract::BLOBS
            };
            let validation = contract.validate(name, &value);
            if case["valid"] == false {
                assert!(validation.is_err());
                continue;
            }
            validation.unwrap();
            let actual = if is_agent {
                agent::roundtrip(name, value.clone())
            } else {
                config_server::roundtrip(name, value.clone())
            }
            .unwrap();
            assert_eq!(actual, value, "schema {name}");
        }
    }
}
