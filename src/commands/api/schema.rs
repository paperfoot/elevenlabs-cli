//! Bundled OpenAPI discovery and local structural validation. No network or auth.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde_json::{Value, json};

use crate::error::AppError;

pub const METHODS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "trace",
];

pub fn spec() -> &'static Value {
    static SPEC: OnceLock<Value> = OnceLock::new();
    SPEC.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../docs/reference/openapi.elevenlabs.json"
        ))
        .expect("bundled OpenAPI is validated by the API coverage tests")
    })
}

pub struct Operation {
    pub id: String,
    pub method: &'static str,
    pub path: &'static str,
    pub definition: &'static Value,
}

impl Operation {
    pub fn summary(&self) -> Value {
        json!({"id":self.id,"operation_id":self.definition["operationId"],
            "method":self.method.to_uppercase(),"path":self.path,
            "summary":self.definition["summary"].as_str().unwrap_or(""),
            "deprecated":self.definition["deprecated"].as_bool().unwrap_or(false)})
    }

    pub fn group(&self) -> &str {
        self.id
            .rsplit_once('.')
            .map_or(&self.id, |(group, _)| group)
    }

    pub fn requires_confirmation(&self) -> bool {
        self.method == "delete"
            || (self.method == "post"
                && ["/remove-rules", "/members/remove", "/bulk-delete"]
                    .iter()
                    .any(|suffix| self.path.ends_with(suffix)))
    }

    pub fn parameters(&self) -> Vec<&'static Value> {
        let mut params: BTreeMap<(String, String), &Value> = BTreeMap::new();
        for value in [&spec()["paths"][self.path], self.definition] {
            for parameter in value["parameters"].as_array().into_iter().flatten() {
                let parameter = resolve(parameter);
                params.insert(
                    (parameter["in"].to_string(), parameter["name"].to_string()),
                    parameter,
                );
            }
        }
        params.into_values().collect()
    }

    pub fn body(&self) -> Option<(&str, &'static Value)> {
        let body = resolve(&self.definition["requestBody"]);
        let content = body["content"].as_object()?;
        content
            .iter()
            .next()
            .map(|(media, value)| (media.as_str(), &value["schema"]))
    }

    pub fn needs_output(&self) -> bool {
        self.definition["responses"]
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(status, _)| status.starts_with('2'))
            .any(|(_, response)| {
                resolve(response)["content"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .any(|(media, _)| {
                        !is_json(media)
                            && !matches!(media.as_str(), "text/plain" | "text/html" | "text/csv")
                    })
            })
    }

    pub fn error(&self, message: impl Into<String>) -> AppError {
        AppError::bad_input_with(
            message,
            format!("Inspect inputs: elevenlabs api schema {}", self.id),
        )
    }

    pub fn schema(&self) -> Value {
        let mut result = json!({"id":self.id,"operation_id":self.definition["operationId"],
            "method":self.method.to_uppercase(),"path":self.path,
            "summary":self.definition["summary"],"description":self.definition["description"],
            "deprecated":self.definition["deprecated"].as_bool().unwrap_or(false),
            "parameters":self.parameters(),"request_body":self.definition["requestBody"],
            "responses":self.definition["responses"],"requires_output":self.needs_output(),
            "requires_confirmation":self.requires_confirmation()});
        let mut refs = BTreeSet::new();
        collect_refs(&result, &mut refs);
        let mut seen = BTreeSet::new();
        let mut components = serde_json::Map::from_iter([("schemas".into(), json!({}))]);
        while let Some(reference) = refs.pop_first() {
            if !seen.insert(reference.clone()) {
                continue;
            }
            if let Some(value) = reference.strip_prefix('#').and_then(|p| spec().pointer(p)) {
                collect_refs(value, &mut refs);
                let parts: Vec<_> = reference.split('/').collect();
                if parts.len() == 4 && parts[1] == "components" {
                    components
                        .entry(parts[2].to_string())
                        .or_insert_with(|| json!({}))[parts[3]] = value.clone();
                }
            }
        }
        result["components"] = Value::Object(components);
        result
    }
}

pub fn is_json(media: &str) -> bool {
    media == "application/json" || media.ends_with("+json")
}

fn collect_refs(value: &Value, refs: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                refs.insert(reference.into());
            }
            for value in map.values() {
                collect_refs(value, refs);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_refs(value, refs);
            }
        }
        _ => {}
    }
}

pub fn operations() -> Vec<Operation> {
    let mut result = Vec::new();
    for (path, item) in spec()["paths"].as_object().expect("OpenAPI paths") {
        for &method in METHODS {
            let Some(definition) = item.get(method) else {
                continue;
            };
            let mut names: Vec<&str> = match &definition["x-fern-sdk-group-name"] {
                Value::String(name) => vec![name],
                Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
                _ => Vec::new(),
            };
            names.push(
                definition["x-fern-sdk-method-name"]
                    .as_str()
                    .or_else(|| definition["operationId"].as_str())
                    .expect("operation identifier"),
            );
            result.push(Operation {
                id: names.join("."),
                method,
                path,
                definition,
            });
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

pub fn find(name: &str) -> Result<Operation, AppError> {
    operations()
        .into_iter()
        .find(|op| op.id == name || op.definition["operationId"].as_str() == Some(name))
        .ok_or_else(|| {
            AppError::bad_input_with(
                "Unknown API operation",
                "Find an operation: elevenlabs api list --all",
            )
        })
}

pub fn resolve(mut schema: &Value) -> &Value {
    for _ in 0..64 {
        let Some(reference) = schema["$ref"].as_str() else {
            break;
        };
        let Some(target) = reference.strip_prefix('#').and_then(|p| spec().pointer(p)) else {
            break;
        };
        schema = target;
    }
    schema
}

pub fn array_items(schema: &Value) -> Option<&Value> {
    let schema = resolve(schema);
    if schema["type"] == "array" {
        return schema.get("items");
    }
    for union in ["anyOf", "oneOf", "allOf"] {
        for branch in schema[union].as_array().into_iter().flatten() {
            if let Some(items) = array_items(branch) {
                return Some(items);
            }
        }
    }
    None
}

pub fn binary(schema: &Value) -> bool {
    let schema = resolve(schema);
    schema["format"] == "binary"
        || array_items(schema).is_some_and(binary)
        || ["anyOf", "oneOf"]
            .iter()
            .any(|union| schema[union].as_array().into_iter().flatten().any(binary))
}

pub fn parse_value(raw: &str, schema: &Value) -> Value {
    let schema = resolve(schema);
    // String enums such as numeric IDs must stay strings. For nullable strings,
    // JSON null is explicit; other unquoted text remains a string.
    if schema["type"] == "string" {
        return serde_json::from_str::<Value>(raw)
            .ok()
            .filter(Value::is_string)
            .unwrap_or_else(|| Value::String(raw.into()));
    }
    let parsed = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.into()));
    if validate(&parsed, schema, "value", 0).is_ok() {
        return parsed;
    }
    Value::String(raw.into())
}

/// Check structural constraints before sending. The provider remains authoritative
/// for formats, patterns, business rules and cross-field constraints.
pub fn validate(value: &Value, schema: &Value, at: &str, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err("Request nesting exceeds 64 levels".into());
    }
    let schema = resolve(schema);
    for union in ["anyOf", "oneOf"] {
        if let Some(branches) = schema[union].as_array() {
            let valid = branches
                .iter()
                .filter(|s| validate(value, s, at, depth + 1).is_ok())
                .count();
            if valid == 0 || (union == "oneOf" && valid != 1) {
                return Err(format!("{at} does not match {union} schema"));
            }
        }
    }
    for branch in schema["allOf"].as_array().into_iter().flatten() {
        validate(value, branch, at, depth + 1)?;
    }
    if let Some(choices) = schema["enum"].as_array() {
        if !choices.contains(value) {
            return Err(format!("{at} is not an allowed enum value"));
        }
    }
    if let Some(constant) = schema.get("const") {
        if value != constant {
            return Err(format!("{at} must match its schema constant"));
        }
    }
    let valid_type = match schema["type"].as_str() {
        Some("string") => value.is_string(),
        Some("integer") => value.is_i64() || value.is_u64(),
        Some("number") => value.is_number(),
        Some("boolean") => value.is_boolean(),
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        Some("null") => value.is_null(),
        _ => true,
    };
    if !valid_type {
        return Err(format!("{at} must have type {}", schema["type"]));
    }
    if let Some(map) = value.as_object() {
        for required in schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !map.contains_key(required) {
                return Err(format!("{at}.{required} is required"));
            }
        }
        for (key, value) in map {
            if let Some(property) = schema["properties"].get(key) {
                validate(value, property, &format!("{at}.{key}"), depth + 1)?;
            } else if schema["additionalProperties"] == false {
                return Err(format!("{at} contains an unknown property"));
            } else if schema["additionalProperties"].is_object() {
                validate(value, &schema["additionalProperties"], at, depth + 1)?;
            }
        }
    }
    if let Some(values) = value.as_array() {
        for value in values {
            validate(value, &schema["items"], at, depth + 1)?;
        }
        if schema["minItems"]
            .as_u64()
            .is_some_and(|n| (values.len() as u64) < n)
            || schema["maxItems"]
                .as_u64()
                .is_some_and(|n| (values.len() as u64) > n)
        {
            return Err(format!("{at} has an invalid number of items"));
        }
    }
    if let Some(text) = value.as_str() {
        let len = text.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|n| len < n)
            || schema["maxLength"].as_u64().is_some_and(|n| len > n)
        {
            return Err(format!("{at} has an invalid length"));
        }
    }
    if let Some(number) = value.as_f64() {
        if schema["minimum"].as_f64().is_some_and(|n| number < n)
            || schema["maximum"].as_f64().is_some_and(|n| number > n)
            || schema["exclusiveMinimum"]
                .as_f64()
                .is_some_and(|n| number <= n)
            || schema["exclusiveMaximum"]
                .as_f64()
                .is_some_and(|n| number >= n)
        {
            return Err(format!("{at} is outside the allowed range"));
        }
    }
    Ok(())
}
