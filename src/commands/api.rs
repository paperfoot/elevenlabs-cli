//! Complete, schema-backed HTTP access alongside the curated short commands.
mod schema;

use std::collections::BTreeMap;
use std::path::PathBuf;

use futures_util::StreamExt;
use serde_json::{Map, Value, json};
use tokio::io::AsyncWriteExt;

use crate::cli::{ApiAction, ApiCallArgs};
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::{
    config,
    output::{self, Ctx},
};
use schema::Operation;

fn emit(ctx: Ctx, data: &Value) -> Result<(), AppError> {
    output::print_success_or(ctx, data, |data| println!("{data:#}"))
}

/// Keep discovery and dry runs independent of config, credentials and Tokio.
pub fn local(ctx: Ctx, action: &ApiAction) -> Result<bool, AppError> {
    match action {
        ApiAction::List { group, search, all } => {
            let operations = schema::operations();
            let total = operations.len();
            if !all && group.is_none() && search.is_none() {
                let mut groups = BTreeMap::new();
                for operation in &operations {
                    *groups
                        .entry(operation.id.split('.').next().unwrap_or("other"))
                        .or_insert(0) += 1;
                }
                emit(
                    ctx,
                    &json!({"total_operations":total,"groups":groups.into_iter().map(|(name, operations)|json!({"name":name,"operations":operations})).collect::<Vec<_>>(),
                    "next":"elevenlabs api list --group workspace"}),
                )?;
            } else {
                let search = search.as_ref().map(|value| value.to_lowercase());
                let selected: Vec<_> = operations
                    .iter()
                    .filter(|op| {
                        group.as_ref().is_none_or(|g| {
                            op.group() == g || op.group().starts_with(&format!("{g}."))
                        }) && search.as_ref().is_none_or(|s| {
                            format!("{} {} {}", op.id, op.path, op.definition["summary"])
                                .to_lowercase()
                                .contains(s)
                        })
                    })
                    .map(Operation::summary)
                    .collect();
                emit(
                    ctx,
                    &json!({"total_operations":total,"operations":selected}),
                )?;
            }
        }
        ApiAction::Schema { operation } => emit(ctx, &schema::find(operation)?.schema())?,
        ApiAction::Call(args) if args.dry_run => {
            let prepared = prepare(args)?;
            emit(ctx, &prepared.preview())?;
        }
        ApiAction::Call(_) => return Ok(false),
    }
    Ok(true)
}

struct Prepared {
    operation: Operation,
    path: String,
    query: Map<String, Value>,
    headers: Map<String, Value>,
    body: Option<Value>,
    files: Vec<(String, PathBuf)>,
}

impl Prepared {
    fn preview(&self) -> Value {
        let mut value = json!({"operation":self.operation.id,"method":self.operation.method.to_uppercase(),
            "path":self.path,"query":self.query,"headers":self.headers,"body":self.body,
            "files":self.files.iter().map(|(field,path)|json!({"field":field,"path":path})).collect::<Vec<_>>(),
            "dry_run":true});
        redact(&mut value, None);
        value
    }

    fn query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        for (key, value) in &self.query {
            // OpenAPI query defaults are form + explode=true.
            if let Some(values) = value.as_array() {
                for value in values {
                    pairs.push((key.clone(), text_value(value)));
                }
            } else {
                pairs.push((key.clone(), text_value(value)));
            }
        }
        pairs
    }
}

fn assignment<'a>(input: &'a str, op: &Operation) -> Result<(&'a str, &'a str), AppError> {
    input
        .split_once('=')
        .filter(|(key, _)| !key.is_empty())
        .ok_or_else(|| op.error("Expected NAME=VALUE"))
}

fn parameters(
    op: &Operation,
    location: &str,
    inputs: &[String],
) -> Result<Map<String, Value>, AppError> {
    let parameters = op.parameters();
    let mut result = Map::new();
    for input in inputs {
        let (key, raw) = assignment(input, op)?;
        let parameter = parameters
            .iter()
            .find(|p| p["in"] == location && p["name"] == key)
            .ok_or_else(|| {
                op.error(format!(
                    "Unknown {location} parameter {}; inspect the operation schema",
                    crate::client::redact_secrets(key)
                ))
            })?;
        if location == "header" && key.eq_ignore_ascii_case("xi-api-key") {
            return Err(AppError::bad_input_with(
                "Use the configured API key, not --header",
                "Configure authentication: elevenlabs config init --help",
            ));
        }
        let schema = &parameter["schema"];
        if let Some(items) = schema::array_items(schema) {
            let value = schema::parse_value(raw, items);
            result
                .entry(key.to_string())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .expect("array")
                .push(value);
        } else {
            if result.contains_key(key) {
                return Err(op.error(format!("{location} parameter {key} cannot be repeated")));
            }
            result.insert(key.into(), schema::parse_value(raw, schema));
        }
    }
    for parameter in parameters {
        if parameter["in"] != location || parameter["name"] == "xi-api-key" {
            continue;
        }
        let name = parameter["name"].as_str().expect("parameter name");
        if let Some(value) = result.get(name) {
            schema::validate(value, &parameter["schema"], name, 0).map_err(|e| op.error(e))?;
        } else if parameter["required"] == true {
            return Err(op.error(format!("Missing required {location} parameter {name}")));
        }
    }
    Ok(result)
}

fn encode_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn prepare(args: &ApiCallArgs) -> Result<Prepared, AppError> {
    let operation = schema::find(&args.operation)?;
    let path_params = parameters(&operation, "path", &args.path)?;
    let query = parameters(&operation, "query", &args.query)?;
    let headers = parameters(&operation, "header", &args.header)?;
    let mut path = operation.path.to_string();
    for (key, value) in path_params {
        let value = text_value(&value);
        if value.is_empty() || value == "." || value == ".." || value.contains(['/', '\\']) {
            return Err(operation.error("Path parameters must be nonempty single segments, without slashes or dot traversal"));
        }
        path = path.replace(&format!("{{{key}}}"), &encode_segment(&value));
    }
    let mut body = None;
    let mut files = Vec::new();
    if let Some((media, body_schema)) = operation.body() {
        if let Some(input) = &args.body {
            if media != "application/json" {
                return Err(operation.error("Multipart requests use --field and --file"));
            }
            let contents = match input.strip_prefix('@') {
                Some(path) => std::fs::read_to_string(path)?,
                None => input.clone(),
            };
            body = Some(
                serde_json::from_str(&contents)
                    .map_err(|_| operation.error("Body must be valid JSON or @file.json"))?,
            );
        } else if !args.field.is_empty() || !args.file.is_empty() {
            let mut fields = Map::new();
            let properties = &schema::resolve(body_schema)["properties"];
            for input in &args.field {
                let (key, raw) = assignment(input, &operation)?;
                if fields.contains_key(key) {
                    return Err(
                        operation.error("Body field cannot be repeated; pass arrays as JSON")
                    );
                }
                if media == "multipart/form-data" && properties.get(key).is_none() {
                    return Err(
                        operation.error("Unknown multipart field; inspect the operation schema")
                    );
                }
                if schema::binary(&properties[key]) {
                    return Err(operation.error("Binary fields require --file FIELD=PATH"));
                }
                fields.insert(key.into(), schema::parse_value(raw, &properties[key]));
            }
            for input in &args.file {
                if media != "multipart/form-data" {
                    return Err(operation.error("This operation does not accept file uploads"));
                }
                let (key, raw) = assignment(input, &operation)?;
                if !schema::binary(&properties[key]) {
                    return Err(operation
                        .error("File field is not declared as binary in the operation schema"));
                }
                let file = PathBuf::from(raw);
                if !file.is_file() {
                    return Err(operation.error("Upload path must be a readable regular file"));
                }
                std::fs::File::open(&file)?;
                if schema::array_items(&properties[key]).is_some() {
                    fields
                        .entry(key.to_string())
                        .or_insert_with(|| json!([]))
                        .as_array_mut()
                        .expect("file array")
                        .push(json!("<file>"));
                } else if fields.insert(key.into(), json!("<file>")).is_some() {
                    return Err(operation.error("This file field cannot be repeated"));
                }
                files.push((key.into(), file));
            }
            body = Some(Value::Object(fields));
        }
        if let Some(value) = &body {
            schema::validate(value, body_schema, "body", 0).map_err(|e| operation.error(e))?;
        } else if schema::resolve(&operation.definition["requestBody"])["required"] == true {
            return Err(operation
                .error("This operation requires a request body; use --body, --field or --file"));
        }
    } else if args.body.is_some() || !args.field.is_empty() || !args.file.is_empty() {
        return Err(operation.error("This operation does not accept a request body"));
    }
    if !args.dry_run {
        if operation.requires_confirmation() && !args.confirm {
            return Err(AppError::bad_input_with(
                "Deletion or removal requires --confirm",
                "Preview first, then add --confirm to your call: elevenlabs api call --help",
            ));
        }
        if operation.needs_output() && args.output.is_none() {
            return Err(AppError::bad_input_with(
                "This response requires an output file",
                "Add --output PATH: elevenlabs api call --help",
            ));
        }
    }
    if args.output.as_ref().is_some_and(|p| p.as_os_str() == "-") {
        return Err(
            operation.error("Use a file path for --output; API call stdout always contains JSON")
        );
    }
    Ok(Prepared {
        operation,
        path,
        query,
        headers,
        body,
        files,
    })
}

fn text_value(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn redact(value: &mut Value, api_key: Option<&str>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                if crate::client::credential_field(key) && !value.is_null() {
                    *value = json!("[REDACTED]");
                } else {
                    redact(value, api_key);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                redact(value, api_key);
            }
        }
        Value::String(value) => {
            *value = crate::client::redact_secrets(value);
            if let Some(key) = api_key.filter(|key| !key.is_empty()) {
                *value = value.replace(key, "[REDACTED]");
            }
        }
        _ => {}
    }
}

async fn multipart(prepared: &Prepared) -> Result<reqwest::multipart::Form, AppError> {
    let mut form = reqwest::multipart::Form::new();
    for (key, value) in prepared
        .body
        .as_ref()
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if prepared.files.iter().any(|(field, _)| field == key) {
            continue;
        }
        if let Some(values) = value.as_array() {
            for value in values {
                form = form.text(key.clone(), text_value(value));
            }
        } else {
            form = form.text(key.clone(), text_value(value));
        }
    }
    for (field, path) in &prepared.files {
        let file = tokio::fs::File::open(path).await?;
        let length = file.metadata().await?.len();
        // reqwest's streaming file part keeps uploads bounded in memory.
        let part = reqwest::multipart::Part::stream_with_length(file, length)
            .file_name(
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            )
            .mime_str(mime_guess::from_path(path).first_or_octet_stream().as_ref())?;
        form = form.part(field.clone(), part);
    }
    Ok(form)
}

pub async fn run(ctx: Ctx, action: ApiAction) -> Result<(), AppError> {
    let ApiAction::Call(args) = action else {
        return local(ctx, &action).map(|_| ());
    };
    let prepared = prepare(&args)?;
    let client = ElevenLabsClient::for_api(&config::load()?)?;
    // Reserve a new file before any paid request; never truncate existing output.
    let mut output_file = if let Some(path) = &args.output {
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        Some(options.open(path).await?)
    } else {
        None
    };
    let result = async {
        let form = if prepared.operation.body().is_some_and(|(media, _)| media == "multipart/form-data") {
            Some(multipart(&prepared).await?)
        } else { None };
        let headers: Vec<_> = prepared.headers.iter().map(|(key,value)| (key.clone(), text_value(value))).collect();
        let response = client.send_api(prepared.operation.method, &prepared.path,
            &prepared.query_pairs(), &headers, prepared.body.as_ref(), form).await?;
        let status = response.status().as_u16();
        if response.status().is_redirection() {
            if args.output.is_some() {
                return Err(prepared.operation.error("Server redirected instead of returning a file; inspect the redirect with an API call without --output"));
            }
            return Ok(json!({"http_status":status,"location":response.headers().get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok()),"redirect_followed":false}));
        }
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
        let data = if let Some(file) = &mut output_file {
            let mut stream = response.bytes_stream();
            let mut bytes = 0u64;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| AppError::Http(e.without_url().to_string()))?;
                file.write_all(&chunk).await?;
                bytes += chunk.len() as u64;
            }
            file.flush().await?;
            json!({"operation":prepared.operation.id,"output":args.output,"bytes":bytes,"http_status":status,"content_type":content_type})
        } else if status == 204 || response.content_length() == Some(0) {
            Value::Null
        } else {
            let media = content_type.split(';').next().unwrap_or("");
            if !schema::is_json(media) && !matches!(media, "text/plain" | "text/html" | "text/csv" | "") {
                return Err(prepared.operation.error("Server returned a non-JSON response; repeat with --output PATH"));
            }
            let bytes = response.bytes().await.map_err(|e| AppError::Http(e.without_url().to_string()))?;
            if bytes.is_empty() { Value::Null }
            else if schema::is_json(media) || media.is_empty() {
                serde_json::from_slice(&bytes).map_err(|_| AppError::Transient("Server returned invalid JSON; inspect with --output PATH".into()))?
            } else { json!(String::from_utf8_lossy(&bytes)) }
        };
        Ok::<_, AppError>(data)
    }.await;
    // Close handles before removing incomplete files, including on Windows.
    drop(output_file);
    match result {
        Ok(mut data) => {
            redact(&mut data, Some(&client.api_key));
            emit(ctx, &data)
        }
        Err(error) => {
            if let Some(path) = args.output {
                let _ = tokio::fs::remove_file(path).await;
            }
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example(schema: &Value, depth: usize) -> Value {
        assert!(
            depth < 64,
            "recursive required schema has no finite fixture"
        );
        let schema = schema::resolve(schema);
        if let Some(constant) = schema.get("const") {
            return constant.clone();
        }
        if let Some(value) = schema["enum"].as_array().and_then(|a| a.first()) {
            return value.clone();
        }
        for union in ["anyOf", "oneOf"] {
            if let Some(branches) = schema[union].as_array() {
                return example(
                    branches
                        .iter()
                        .find(|s| schema::resolve(s)["type"] != "null")
                        .unwrap(),
                    depth + 1,
                );
            }
        }
        if let Some(branches) = schema["allOf"].as_array() {
            let mut combined = json!({});
            for branch in branches {
                let value = example(branch, depth + 1);
                if let Some(map) = value.as_object() {
                    combined.as_object_mut().unwrap().extend(map.clone());
                } else {
                    return value;
                }
            }
            return combined;
        }
        match schema["type"].as_str() {
            Some("object") => {
                let mut map = Map::new();
                for name in schema["required"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    map.insert(
                        name.to_string(),
                        example(&schema["properties"][name], depth + 1),
                    );
                }
                for (name, property) in schema["properties"].as_object().into_iter().flatten() {
                    if let Some(constant) = schema::resolve(property).get("const") {
                        map.insert(name.clone(), constant.clone());
                    }
                }
                Value::Object(map)
            }
            Some("array") => json!(
                (0..schema["minItems"].as_u64().unwrap_or(0))
                    .map(|_| example(&schema["items"], depth + 1))
                    .collect::<Vec<_>>()
            ),
            Some("integer") => json!(schema["minimum"].as_f64().unwrap_or(0.0).ceil() as i64),
            Some("number") => json!(schema["minimum"].as_f64().unwrap_or(0.0)),
            Some("boolean") => json!(false),
            Some("null") => Value::Null,
            _ => json!("x".repeat(schema["minLength"].as_u64().unwrap_or(1) as usize)),
        }
    }

    #[test]
    fn every_official_operation_can_prepare_a_request() {
        let upload = tempfile::NamedTempFile::new().unwrap();
        for op in schema::operations() {
            let mut args = ApiCallArgs {
                operation: op.id.clone(),
                path: vec![],
                query: vec![],
                header: vec![],
                body: None,
                field: vec![],
                file: vec![],
                output: None,
                dry_run: true,
                confirm: false,
            };
            for param in op
                .parameters()
                .iter()
                .filter(|p| p["required"] == true && p["name"] != "xi-api-key")
            {
                let value = example(&param["schema"], 0);
                let input = format!("{}={}", param["name"].as_str().unwrap(), text_value(&value));
                match param["in"].as_str().unwrap() {
                    "path" => args.path.push(input),
                    "query" => args.query.push(input),
                    "header" => args.header.push(input),
                    _ => unreachable!(),
                }
            }
            if let Some((media, body_schema)) = op.body() {
                let body = example(body_schema, 0);
                if media == "application/json" {
                    args.body = Some(body.to_string());
                } else {
                    for (name, value) in body.as_object().unwrap() {
                        if schema::binary(&schema::resolve(body_schema)["properties"][name]) {
                            args.file
                                .push(format!("{name}={}", upload.path().display()));
                        } else {
                            args.field.push(format!("{name}={}", text_value(value)));
                        }
                    }
                    // A required empty multipart object still needs an explicit
                    // body representation; current upstream has no such cases.
                }
            }
            let prepared = prepare(&args).unwrap_or_else(|error| panic!("{}: {error}", op.id));
            assert_eq!(prepared.operation.method, op.method);
            assert!(!prepared.path.contains('{'), "{} has unfilled path", op.id);
        }
    }
}
