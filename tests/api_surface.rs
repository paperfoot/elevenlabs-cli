//! Contract tests for the schema-backed `api` command group.
//!
//! Discovery and dry-run tests stay offline. Request tests use wiremock so
//! they verify exact HTTP construction without contacting ElevenLabs.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn bin() -> Command {
    Command::cargo_bin("elevenlabs").unwrap()
}

fn temp_config_with_key(api_key: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

fn offline(args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let malformed = dir.path().join("config.toml");
    std::fs::write(&malformed, b"not valid toml = [").unwrap();
    bin()
        .env("ELEVENLABS_CLI_CONFIG", malformed)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY")
        .args(args)
        .output()
        .unwrap()
}

fn output_json(output: &std::process::Output) -> Value {
    assert!(
        output.status.success(),
        "command failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).expect("stdout must contain one JSON value")
}

fn data(output: &std::process::Output) -> Value {
    let envelope = output_json(output);
    assert_eq!(envelope["status"], "success");
    envelope["data"].clone()
}

fn assert_invalid(args: &[&str], message_fragment: &str) {
    let (_dir, config) = temp_config_with_key("sk_test_api_surface_key");
    let output = bin()
        .env("ELEVENLABS_CLI_CONFIG", config)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args(args)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "{args:?} must fail as bad input: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "errors must not leak to stdout");
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr must be JSON");
    assert_eq!(error["status"], "error");
    assert_eq!(error["error"]["code"], "invalid_input");
    assert!(error["error"]["suggestion"].is_string());
    let message = error["error"]["message"].as_str().unwrap_or("");
    assert!(
        message.to_ascii_lowercase().contains(message_fragment),
        "error message {message:?} does not contain {message_fragment:?}"
    );
}

fn command_for(mock: &MockServer, config: &Path, args: &[&str]) -> std::process::Output {
    bin()
        .env("ELEVENLABS_CLI_CONFIG", config)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args(args)
        .output()
        .unwrap()
}

fn operation_alias(operation: &Value) -> String {
    let mut parts = match operation.get("x-fern-sdk-group-name") {
        Some(Value::String(group)) => vec![group.clone()],
        Some(Value::Array(groups)) => groups
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    if let Some(method_name) = operation
        .get("x-fern-sdk-method-name")
        .and_then(Value::as_str)
    {
        parts.push(method_name.to_owned());
    }
    if parts.is_empty() {
        operation["operationId"].as_str().unwrap().to_owned()
    } else {
        parts.join(".")
    }
}

fn vendored_operations() -> BTreeMap<String, (String, String, String)> {
    let spec: Value =
        serde_json::from_str(include_str!("../docs/reference/openapi.elevenlabs.json")).unwrap();
    let mut operations = BTreeMap::new();
    for (route, path_item) in spec["paths"].as_object().unwrap() {
        for (method_name, operation) in path_item.as_object().unwrap() {
            if !matches!(
                method_name.as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
            ) {
                continue;
            }
            let operation_id = operation["operationId"].as_str().unwrap().to_owned();
            operations.insert(
                operation_id.clone(),
                (
                    operation_alias(operation),
                    method_name.to_ascii_uppercase(),
                    route.clone(),
                ),
            );
        }
    }
    operations
}

fn collect_schema_refs(value: &Value, refs: &mut BTreeSet<String>) {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_schema_refs(value, refs);
            }
        }
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                if let Some(name) = reference.strip_prefix("#/components/schemas/") {
                    refs.insert(name.to_owned());
                }
            }
            for value in object.values() {
                collect_schema_refs(value, refs);
            }
        }
        _ => {}
    }
}

#[test]
fn api_list_group_summary_is_offline_and_complete() {
    let output = offline(&["api", "list"]);
    let data = data(&output);
    assert_eq!(data["total_operations"], 391);
    let groups = data["groups"].as_array().expect("groups must be an array");
    assert!(!groups.is_empty());
    assert!(groups.iter().all(|group| {
        group["name"].is_string() && group["operations"].as_u64().is_some_and(|count| count > 0)
    }));
    assert!(groups.iter().any(|group| group["name"] == "history"));
    assert!(groups.iter().any(|group| group["name"] == "workspace"));
}

#[test]
fn api_list_all_matches_every_vendored_operation() {
    let expected = vendored_operations();
    assert_eq!(expected.len(), 391, "vendored operation count changed");

    let output = offline(&["api", "list", "--all"]);
    let data = data(&output);
    assert_eq!(data["total_operations"], 391);
    let listed = data["operations"]
        .as_array()
        .expect("--all must return operations");
    assert_eq!(listed.len(), expected.len());

    let actual = listed
        .iter()
        .map(|operation| {
            for field in [
                "id",
                "operation_id",
                "method",
                "path",
                "summary",
                "deprecated",
            ] {
                assert!(
                    operation.get(field).is_some(),
                    "catalog operation missing {field}: {operation}"
                );
            }
            assert!(operation["deprecated"].is_boolean());
            (
                operation["operation_id"].as_str().unwrap().to_owned(),
                (
                    operation["id"].as_str().unwrap().to_owned(),
                    operation["method"].as_str().unwrap().to_owned(),
                    operation["path"].as_str().unwrap().to_owned(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected);
}

#[test]
fn api_list_group_and_search_filters_return_operations() {
    let grouped = data(&offline(&["api", "list", "--group", "workspace"]));
    let operations = grouped["operations"].as_array().unwrap();
    assert!(!operations.is_empty());
    assert!(
        operations
            .iter()
            .all(|operation| { operation["id"].as_str().unwrap().starts_with("workspace.") })
    );

    let searched = data(&offline(&["api", "list", "--search", "history"]));
    let operations = searched["operations"].as_array().unwrap();
    assert!(
        operations
            .iter()
            .any(|operation| operation["id"] == "history.list")
    );
}

#[test]
fn api_schema_accepts_alias_and_operation_id_and_closes_refs() {
    let by_alias = data(&offline(&["api", "schema", "history.list"]));
    let by_operation_id = data(&offline(&["api", "schema", "get_speech_history"]));
    assert_eq!(by_alias, by_operation_id);
    assert_eq!(by_alias["id"], "history.list");
    assert_eq!(by_alias["method"], "GET");
    assert_eq!(by_alias["path"], "/v1/history");
    assert!(by_alias["parameters"].is_array());
    assert!(by_alias["request_body"].is_null());
    assert!(by_alias["responses"].is_object());
    assert!(by_alias["components"]["schemas"].is_object());

    let schema = data(&offline(&[
        "api",
        "schema",
        "conversational_ai.tests.update",
    ]));
    let mut refs = BTreeSet::new();
    collect_schema_refs(&schema, &mut refs);
    let components = schema["components"]["schemas"].as_object().unwrap();
    for reference in refs {
        assert!(
            components.contains_key(&reference),
            "scoped schema omitted referenced component {reference}"
        );
    }
    assert!(
        components.len() < 100,
        "scoped schema unexpectedly contains the full component catalog"
    );
}

#[test]
fn every_catalog_operation_has_a_resolvable_offline_schema() {
    let catalog = data(&offline(&["api", "list", "--all"]));
    for operation in catalog["operations"].as_array().unwrap() {
        let operation_id = operation["operation_id"].as_str().unwrap();
        let output = offline(&["api", "schema", operation_id]);
        assert!(
            output.status.success(),
            "schema failed for {operation_id}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let schema = data(&output);
        assert!(schema["id"].is_string());
        assert!(schema["method"].is_string());
        assert!(schema["path"].is_string());
        let mut refs = BTreeSet::new();
        collect_schema_refs(&schema, &mut refs);
        let components = schema["components"]["schemas"]
            .as_object()
            .unwrap_or_else(|| {
                panic!("{operation_id} schema must contain components.schemas object")
            });
        for reference in refs {
            assert!(
                components.contains_key(&reference),
                "{operation_id} omitted referenced component {reference}"
            );
        }
    }
}

#[test]
fn dry_run_is_config_free_and_builds_typed_request_preview() {
    let output = offline(&[
        "api",
        "call",
        "history.list",
        "--query",
        "page_size=2",
        "--query",
        "sort_direction=asc",
        "--dry-run",
    ]);
    let preview = data(&output);
    assert_eq!(preview["operation"], "history.list");
    assert_eq!(preview["method"], "GET");
    assert_eq!(preview["path"], "/v1/history");
    assert_eq!(
        preview["query"],
        json!({"page_size": 2, "sort_direction": "asc"})
    );
    assert!(preview["body"].is_null());
    assert_eq!(preview["files"], json!([]));
    assert_eq!(preview["headers"], json!({}));
}

#[test]
fn dry_run_fields_parse_json_but_preserve_schema_strings() {
    let preview = data(&offline(&[
        "api",
        "call",
        "pronunciation_dictionaries.update",
        "--path",
        "pronunciation_dictionary_id=dict-1",
        "--field",
        "archived=true",
        "--field",
        "name=123",
        "--dry-run",
    ]));
    assert_eq!(preview["body"], json!({"archived": true, "name": "123"}));
}

#[test]
fn dry_run_reads_body_from_file_and_supports_allowed_header() {
    let temp = tempfile::tempdir().unwrap();
    let body_path = temp.path().join("podcast.json");
    std::fs::write(
        &body_path,
        br#"{"model_id":"eleven_multilingual_v2","mode":{"type":"bulletin","bulletin":{"host_voice_id":"voice-1"}},"source":{"type":"text","text":"Test source"}}"#,
    )
    .unwrap();
    let body_arg = format!("@{}", body_path.display());
    let preview = data(&offline(&[
        "api",
        "call",
        "studio.create_podcast",
        "--header",
        "safety-identifier=test-suite",
        "--body",
        &body_arg,
        "--dry-run",
    ]));
    assert_eq!(preview["headers"]["safety-identifier"], "test-suite");
    assert_eq!(preview["body"]["source"]["text"], "Test source");
}

#[tokio::test(flavor = "multi_thread")]
async fn get_call_sends_typed_and_repeated_query_values() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/voices"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"voices": []})))
        .mount(&mock)
        .await;
    let (_dir, config) = temp_config_with_key("sk_test_api_surface_key");
    let output = command_for(
        &mock,
        &config,
        &[
            "api",
            "call",
            "voices.search",
            "--query",
            "page_size=2",
            "--query",
            "language=en",
            "--query",
            "language=es",
        ],
    );
    let response = data(&output);
    assert_eq!(response, json!({"voices": []}));

    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let pairs = requests[0]
        .url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    assert!(pairs.contains(&("page_size".to_owned(), "2".to_owned())));
    assert!(pairs.contains(&("language".to_owned(), "en".to_owned())));
    assert!(pairs.contains(&("language".to_owned(), "es".to_owned())));
}

#[tokio::test(flavor = "multi_thread")]
async fn post_json_call_sends_validated_body_and_path() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/workspace/groups/group-1/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .mount(&mock)
        .await;
    let (_dir, config) = temp_config_with_key("sk_test_api_surface_key");
    let output = command_for(
        &mock,
        &config,
        &[
            "api",
            "call",
            "workspace.groups.members.add",
            "--path",
            "group_id=group-1",
            "--body",
            r#"{"email":"person@example.com"}"#,
        ],
    );
    assert_eq!(data(&output), json!({"ok": true}));
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
        json!({"email": "person@example.com"})
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn patch_put_and_confirmed_delete_use_schema_methods() {
    let mock = MockServer::start().await;
    for (verb, route) in [
        ("PATCH", "/v1/pronunciation-dictionaries/dict-1"),
        ("PUT", "/v1/convai/agent-testing/test-1"),
        ("DELETE", "/v1/history/item-1"),
    ] {
        Mock::given(method(verb))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"method": verb})))
            .mount(&mock)
            .await;
    }
    let (_dir, config) = temp_config_with_key("sk_test_api_surface_key");
    let cases: &[&[&str]] = &[
        &[
            "api",
            "call",
            "pronunciation_dictionaries.update",
            "--path",
            "pronunciation_dictionary_id=dict-1",
            "--body",
            r#"{"name":"Updated"}"#,
        ],
        &[
            "api",
            "call",
            "conversational_ai.tests.update",
            "--path",
            "test_id=test-1",
            "--body",
            r#"{"name":"Updated test"}"#,
        ],
        &[
            "api",
            "call",
            "history.delete",
            "--path",
            "history_item_id=item-1",
            "--confirm",
        ],
    ];
    for args in cases {
        let output = command_for(&mock, &config, args);
        output_json(&output);
    }
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    let methods = requests
        .iter()
        .map(|request| request.method.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        methods,
        BTreeSet::from(["DELETE".into(), "PATCH".into(), "PUT".into()])
    );
}

#[test]
fn dry_run_percent_encodes_safe_path_values() {
    let preview = data(&offline(&[
        "api",
        "call",
        "history.get_audio",
        "--path",
        "history_item_id=item with spaces",
        "--output",
        "/tmp/offline-audio.mp3",
        "--dry-run",
    ]));
    assert_eq!(preview["path"], "/v1/history/item%20with%20spaces/audio");
}

#[tokio::test(flavor = "multi_thread")]
async fn multipart_call_sends_file_and_typed_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/speech-to-text"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"text": "hello"})))
        .mount(&mock)
        .await;
    let temp = tempfile::tempdir().unwrap();
    let audio = temp.path().join("sample.wav");
    std::fs::write(&audio, b"RIFF-test-audio").unwrap();
    let file_arg = format!("file={}", audio.display());
    let (_dir, config) = temp_config_with_key("sk_test_api_surface_key");
    let output = command_for(
        &mock,
        &config,
        &[
            "api",
            "call",
            "speech_to_text.convert",
            "--field",
            "model_id=scribe_v2",
            "--field",
            "diarize=true",
            "--file",
            &file_arg,
        ],
    );
    assert_eq!(data(&output)["text"], "hello");
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = String::from_utf8_lossy(&requests[0].body);
    assert!(body.contains("name=\"file\""));
    assert!(body.contains("RIFF-test-audio"));
    assert!(body.contains("name=\"model_id\""));
    assert!(body.contains("scribe_v2"));
    assert!(body.contains("name=\"diarize\""));
    assert!(body.contains("true"));
}

#[tokio::test(flavor = "multi_thread")]
async fn binary_call_requires_and_streams_to_output_file() {
    assert_invalid(
        &[
            "api",
            "call",
            "history.get_audio",
            "--path",
            "history_item_id=item-1",
        ],
        "output",
    );

    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/history/item-1/audio"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "audio/mpeg")
                .set_body_bytes(b"FAKE-AUDIO-BYTES".to_vec()),
        )
        .mount(&mock)
        .await;
    let temp = tempfile::tempdir().unwrap();
    let destination = temp.path().join("audio.mp3");
    let destination_arg = destination.to_str().unwrap();
    let (_dir, config) = temp_config_with_key("sk_test_api_surface_key");
    let output = command_for(
        &mock,
        &config,
        &[
            "api",
            "call",
            "history.get_audio",
            "--path",
            "history_item_id=item-1",
            "--output",
            destination_arg,
        ],
    );
    let result = data(&output);
    assert_eq!(std::fs::read(&destination).unwrap(), b"FAKE-AUDIO-BYTES");
    assert_eq!(result["output"], destination_arg);
    assert_eq!(result["bytes"], 16);
}

#[test]
fn request_validation_rejects_bad_parameters_before_network() {
    let cases: &[(&[&str], &str)] = &[
        (
            &["api", "call", "history.download", "--body", "{}"],
            "history_item_ids",
        ),
        (
            &["api", "call", "history.list", "--query", "page_size=two"],
            "page_size",
        ),
        (
            &[
                "api",
                "call",
                "history.list",
                "--query",
                "sort_direction=sideways",
            ],
            "sort_direction",
        ),
        (
            &[
                "api",
                "call",
                "history.list",
                "--query",
                "not_a_parameter=value",
            ],
            "not_a_parameter",
        ),
        (
            &[
                "api",
                "call",
                "history.list",
                "--query",
                "page_size=1",
                "--query",
                "page_size=2",
            ],
            "page_size",
        ),
    ];
    for (args, fragment) in cases {
        assert_invalid(args, fragment);
    }
}

#[test]
fn delete_requires_confirmation_except_for_dry_run() {
    assert_invalid(
        &[
            "api",
            "call",
            "history.delete",
            "--path",
            "history_item_id=item-1",
        ],
        "confirm",
    );
    let preview = data(&offline(&[
        "api",
        "call",
        "history.delete",
        "--path",
        "history_item_id=item-1",
        "--dry-run",
    ]));
    assert_eq!(preview["method"], "DELETE");
}

#[test]
fn path_traversal_and_url_injection_are_rejected() {
    for value in ["..", "../models", "item/../../models", "//evil.test"] {
        let assignment = format!("history_item_id={value}");
        assert_invalid(
            &[
                "api",
                "call",
                "history.get",
                "--path",
                &assignment,
                "--dry-run",
            ],
            "path",
        );
    }

    let assignment = "history_item_id=item?admin=true";
    let preview = data(&offline(&[
        "api",
        "call",
        "history.get",
        "--path",
        assignment,
        "--dry-run",
    ]));
    assert_eq!(preview["path"], "/v1/history/item%3Fadmin%3Dtrue");
}

#[test]
fn dry_run_redacts_secret_values() {
    let output = offline(&[
        "api",
        "call",
        "conversational_ai.secrets.create",
        "--body",
        r#"{"type":"new","name":"provider-key","value":"sk_super_secret_value"}"#,
        "--dry-run",
    ]);
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    assert!(!stdout.contains("sk_super_secret_value"));
    let preview = data(&output);
    assert_ne!(preview["body"]["value"], "sk_super_secret_value");
    assert!(preview["body"]["value"].as_str().is_some());
}

#[test]
fn unknown_operation_is_a_bad_input_error() {
    assert_invalid(&["api", "schema", "not.a.real.operation"], "operation");
}

#[tokio::test(flavor = "multi_thread")]
async fn redirects_do_not_forward_credentials() {
    let source = MockServer::start().await;
    let destination = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs"))
        .respond_with(ResponseTemplate::new(302).insert_header("location", destination.uri()))
        .mount(&source)
        .await;
    let (_dir, config) = temp_config_with_key("credential-without-sk-prefix");
    let result = command_for(&source, &config, &["api", "call", "redirect_to_mintlify"]);
    let response = data(&result);
    assert_eq!(response["http_status"], 302);
    assert_eq!(response["redirect_followed"], false);
    assert!(destination.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn api_errors_keep_semantic_codes_and_redact_authentication() {
    let mock = MockServer::start().await;
    let key = "credential-without-sk-prefix";
    let (_dir, config) = temp_config_with_key(key);
    for (status, code) in [(401, 2), (403, 2), (429, 4), (422, 3), (500, 1)] {
        mock.reset().await;
        Mock::given(method("GET"))
            .and(path("/v1/history"))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_json(json!({"detail":{"message":format!("Echoed {key}")}})),
            )
            .mount(&mock)
            .await;
        let result = command_for(&mock, &config, &["api", "call", "history.list"]);
        assert_eq!(result.status.code(), Some(code));
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains(key));
        let error: Value = serde_json::from_slice(&result.stderr).unwrap();
        assert!(error["error"]["suggestion"].is_string());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn downloads_preserve_existing_files_and_remove_failed_outputs() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/history/item/audio"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock)
        .await;
    let (_dir, config) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let files = tempfile::tempdir().unwrap();
    let output = files.path().join("output.mp3");
    std::fs::write(&output, b"existing content").unwrap();
    let args = [
        "api",
        "call",
        "history.get_audio",
        "--path",
        "history_item_id=item",
        "--output",
        output.to_str().unwrap(),
    ];
    let result = command_for(&mock, &config, &args);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(std::fs::read(&output).unwrap(), b"existing content");
    assert!(mock.received_requests().await.unwrap().is_empty());
    std::fs::remove_file(&output).unwrap();
    let result = command_for(&mock, &config, &args);
    assert_eq!(result.status.code(), Some(1));
    assert!(!output.exists());
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
}

#[test]
fn post_bulk_deletion_also_requires_confirmation() {
    let args = [
        "api",
        "call",
        "conversational_ai.knowledge_base.documents.bulk_delete",
        "--body",
        r#"{"document_ids":["document-1"]}"#,
    ];
    assert_invalid(&args, "confirm");
    let mut preview = args.to_vec();
    preview.push("--dry-run");
    assert_eq!(data(&offline(&preview))["method"], "POST");
}

#[test]
fn dry_run_redacts_provider_auth_tokens() {
    let output = offline(&[
        "api",
        "call",
        "conversational_ai.phone_numbers.create",
        "--body",
        r#"{"phone_number":"+15555551234","label":"test","provider":"twilio","sid":"account","token":"plain-token-value","account_auth_token":"another-credential"}"#,
        "--dry-run",
    ]);
    // Token fields in this body are provider credentials even without sk_ prefixes.
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("plain-token-value"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("another-credential"));
}

#[tokio::test(flavor = "multi_thread")]
async fn provider_errors_do_not_echo_submitted_credentials() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/convai/phone-numbers"))
        .respond_with(ResponseTemplate::new(422).set_body_json(
            json!({"detail":{"message":"Bad plain-token-value or another-credential"}}),
        ))
        .mount(&mock)
        .await;
    let (_dir, config) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let result = command_for(
        &mock,
        &config,
        &[
            "api",
            "call",
            "conversational_ai.phone_numbers.create",
            "--body",
            r#"{"phone_number":"+15555551234","label":"test","provider":"twilio","sid":"account","token":"plain-token-value","account_auth_token":"another-credential"}"#,
        ],
    );
    assert_eq!(result.status.code(), Some(3));
    assert!(result.stdout.is_empty());
    for secret in ["plain-token-value", "another-credential"] {
        assert!(!String::from_utf8_lossy(&result.stderr).contains(secret));
    }
}
