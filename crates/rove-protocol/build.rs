use serde_json::Value;
use std::{env, fs, path::PathBuf};

fn pascal(value: &str) -> String {
    value
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

struct Generator {
    declarations: String,
}
impl Generator {
    fn ty(&mut self, name: &str, schema: &Value) -> String {
        if schema["$ref"] == "#/components/schemas/Id" || schema["format"] == "uuid" {
            for id in [
                "DeviceId",
                "NetworkId",
                "SessionId",
                "RunId",
                "ServiceId",
                "RequestId",
                "InstanceId",
            ] {
                if name.ends_with(id) {
                    return format!("rove_core::{id}");
                }
            }
        }
        if let Some(reference) = schema["$ref"].as_str() {
            return reference
                .strip_prefix("#/components/schemas/")
                .expect("local schema reference")
                .into();
        }
        if let Some(types) = schema["type"].as_array() {
            if !types.contains(&Value::from("null")) {
                let variants: Vec<Value> = types
                    .iter()
                    .map(|kind| serde_json::json!({"type":kind}))
                    .collect();
                return self.ty(name, &serde_json::json!({"oneOf":variants}));
            }
            assert_eq!(types.len(), 2, "unsupported type union: {name}");
            assert!(types.contains(&Value::from("null")));
            let mut inner = schema.clone();
            inner["type"] = types.iter().find(|v| **v != "null").unwrap().clone();
            return format!("Option<{}>", self.ty(name, &inner));
        }
        if let Some(types) = schema["anyOf"].as_array() {
            assert_eq!(types.len(), 2, "unsupported anyOf: {name}");
            assert!(types.iter().any(|s| s["type"] == "null"));
            return format!(
                "Option<{}>",
                self.ty(name, types.iter().find(|s| s["type"] != "null").unwrap())
            );
        }
        if schema["type"].is_null()
            && let Some(variants) = schema["oneOf"].as_array()
        {
            let mut fields = String::new();
            for (i, variant) in variants.iter().enumerate() {
                let ty = self.ty(&format!("{name}Variant{i}"), variant);
                fields.push_str(&format!("Variant{i}(Box<{ty}>),\n"));
            }
            self.declarations.push_str(&format!("#[derive(Clone, serde::Serialize, serde::Deserialize)]\n#[serde(untagged)]\npub enum {name} {{ {fields} }}\n"));
            return name.into();
        }
        match schema["type"].as_str() {
            Some("object") => {
                let Some(properties) = schema["properties"].as_object() else {
                    let value = if schema["additionalProperties"].is_object() {
                        self.ty(&format!("{name}Value"), &schema["additionalProperties"])
                    } else {
                        "serde_json::Value".into()
                    };
                    return format!("std::collections::BTreeMap<String, {value}>");
                };
                let mut fields = String::new();
                for (field, property) in properties {
                    let ty = self.ty(&format!("{name}{}", pascal(field)), property);
                    let required = schema["required"]
                        .as_array()
                        .is_some_and(|r| r.contains(&Value::from(field.clone())));
                    let attr = if !required {
                        "#[serde(default, skip_serializing_if = \"Option::is_none\", deserialize_with = \"super::present\")]"
                    } else if ty.starts_with("Option<") {
                        "#[serde(deserialize_with = \"super::required\")]"
                    } else {
                        ""
                    };
                    let ty = if required {
                        ty
                    } else {
                        format!("Option<{ty}>")
                    };
                    fields.push_str(&format!("{attr}\npub r#{field}: {ty},\n"));
                }
                assert_eq!(
                    schema["additionalProperties"], false,
                    "unhandled extensible object: {name}"
                );
                self.declarations.push_str(&format!("#[derive(Clone, serde::Serialize, serde::Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct {name} {{ {fields} }}\n"));
                name.into()
            }
            Some("array") => format!("Vec<{}>", self.ty(&format!("{name}Item"), &schema["items"])),
            Some("string") if schema["enum"].is_array() || schema["const"].is_string() => {
                let values = schema["enum"]
                    .as_array()
                    .cloned()
                    .unwrap_or_else(|| vec![schema["const"].clone()]);
                let fields: String = values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        let text = value.as_str().unwrap();
                        let variant =
                            if text.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                                && text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                            {
                                pascal(text)
                            } else {
                                format!("Value{index}")
                            };
                        format!("#[serde(rename = {value})] {},\n", variant)
                    })
                    .collect();
                self.declarations.push_str(&format!("#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]\npub enum {name} {{ {fields} }}\n"));
                name.into()
            }
            Some("string") if schema["format"] == "uuid" => "uuid::Uuid".into(),
            Some("string") => "String".into(),
            Some("integer") => "super::Integer".into(),
            Some("number") => "f64".into(),
            Some("boolean") => "bool".into(),
            Some("null") => "()".into(),
            None if schema.as_object().is_some_and(|o| o.is_empty()) => "serde_json::Value".into(),
            _ => panic!("unsupported schema: {name}: {schema}"),
        }
    }
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../api");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    for (module, filename, contract) in [
        ("agent", "rove-agent.openapi.json", "AGENT"),
        ("config_server", "rove-config-server.openapi.json", "BLOBS"),
    ] {
        let path = root.join(filename);
        println!("cargo:rerun-if-changed={}", path.display());
        let document: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let schemas = document["components"]["schemas"].as_object().unwrap();
        let mut generator = Generator {
            declarations: String::new(),
        };
        let mut checks = String::new();
        for (name, schema) in schemas {
            let ty = generator.ty(name, schema);
            if ty != *name {
                generator
                    .declarations
                    .push_str(&format!("pub type {name} = {ty};\n"));
            }
            // Trait implementations only apply to concrete generated models;
            // aliases share primitive implementations and cannot be duplicated.
            if ty == *name {
                generator.declarations.push_str(&format!("impl super::WireDto for {name} {{ const SCHEMA: &'static str = \"{name}\"; fn contract() -> &'static crate::contract::Contract {{ &crate::contract::{contract} }} }}\n"));
            }
            checks.push_str(&format!(
                "\"{name}\" => serde_json::to_value(serde_json::from_value::<{name}>(value)?),\n"
            ));
        }
        generator.declarations.push_str(&format!("#[cfg(test)] pub(super) fn roundtrip(name: &str, value: serde_json::Value) -> Result<serde_json::Value, serde_json::Error> {{ match name {{ {checks} _ => panic!(\"unknown schema\") }} }}\n"));
        fs::write(out.join(format!("{module}_dto.rs")), generator.declarations).unwrap();
    }
}
