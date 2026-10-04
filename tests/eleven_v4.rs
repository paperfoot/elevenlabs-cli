//! Eleven v4 routing, payload, compatibility, and validation contracts.

use assert_cmd::Command as AssertCmd;
use base64::Engine as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const TEST_KEY: &str = "sk_test_keyyyyyyyyy";

fn bin() -> AssertCmd {
    AssertCmd::cargo_bin("elevenlabs").unwrap()
}

fn temp_config(default_model: Option<&str>) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "api_key = \"{TEST_KEY}\"").unwrap();
    if let Some(model) = default_model {
        writeln!(file, "[defaults]").unwrap();
        writeln!(file, "voice_id = \"v_test\"").unwrap();
        writeln!(file, "model_id = \"{model}\"").unwrap();
    }
    (dir, path)
}

fn configured_bin(config: &Path, server: &MockServer) -> AssertCmd {
    let mut command = bin();
    command
        .env("ELEVENLABS_CLI_CONFIG", config)
        .env("ELEVENLABS_API_BASE_URL", server.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .env_remove("ELEVENLABS_CLI_API_KEY");
    command
}

fn assert_success(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["status"], "success");
    envelope
}

fn assert_invalid_input(output: &std::process::Output) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(3),
        "expected exit 3; stdout={}; stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "errors must not leak to stdout");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["status"], "error");
    assert_eq!(envelope["error"]["code"], "invalid_input");
    envelope
}

fn json_body(request: &Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("request body must be JSON")
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn assert_close(value: &serde_json::Value, expected: f64) {
    let actual = value.as_f64().expect("setting must be numeric");
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn v4_tts_uses_dialogue_payload_and_preserves_result_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"V4AUDIO".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config(None);
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("v4.mp3");

    let output = configured_bin(&config, &server)
        .args([
            "tts",
            "Main line",
            "--model",
            "eleven_v4",
            "--voice-id",
            "v_test",
            "--stability",
            "0.4",
            "--similarity",
            "0.8",
            "--previous-text",
            "Previous line",
            "--next-text",
            "Future line",
            "--previous-request-id",
            "prev_1",
            "--previous-request-id",
            "prev_2",
            "--next-request-id",
            "next_1",
            "--use-pvc-as-ivc",
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let envelope = assert_success(&output);
    assert_eq!(envelope["data"]["voice_id"], "v_test");
    assert_eq!(envelope["data"]["model_id"], "eleven_v4");
    assert_eq!(envelope["data"]["bytes_written"], 7);
    assert_eq!(std::fs::read(&output_path).unwrap(), b"V4AUDIO");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = json_body(&requests[0]);
    assert_eq!(
        body["inputs"],
        serde_json::json!([{ "text": "Main line", "voice_id": "v_test" }])
    );
    assert_eq!(body["model_id"], "eleven_v4");
    assert_close(&body["settings"]["stability"], 0.4);
    assert_close(&body["settings"]["similarity"], 0.8);
    assert_eq!(body["settings"].as_object().unwrap().len(), 2);
    assert!(body.get("voice_settings").is_none());
    assert_eq!(body["previous_text"], "Previous line");
    assert_eq!(body["future_text"], "Future line");
    assert!(body.get("next_text").is_none());
    assert_eq!(
        body["previous_request_ids"],
        serde_json::json!(["prev_1", "prev_2"])
    );
    assert_eq!(body["next_request_ids"], serde_json::json!(["next_1"]));
    assert_eq!(body["use_pvc_as_ivc"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn v4_tts_routes_all_stream_and_timestamp_variants_and_decodes_audio() {
    let cases = [
        (false, false, "/v1/text-to-dialogue", b"RAW".as_slice()),
        (
            true,
            false,
            "/v1/text-to-dialogue/stream",
            b"STREAM1STREAM2".as_slice(),
        ),
        (
            false,
            true,
            "/v1/text-to-dialogue/with-timestamps",
            b"STAMPED".as_slice(),
        ),
        (
            true,
            true,
            "/v1/text-to-dialogue/stream/with-timestamps",
            b"BOTH1BOTH2".as_slice(),
        ),
    ];

    for (stream, timestamps, expected_path, expected_audio) in cases {
        let server = MockServer::start().await;
        let response = match (stream, timestamps) {
            (false, false) => ResponseTemplate::new(200).set_body_bytes(expected_audio.to_vec()),
            (false, true) => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "audio_base64": b64(expected_audio),
                "alignment": { "characters": [] }
            })),
            (true, false) => ResponseTemplate::new(200).set_body_string(format!(
                "{{\"audio_base64\":\"{}\"}}\n{{\"audio_base64\":\"{}\"}}\n",
                b64(b"STREAM1"),
                b64(b"STREAM2")
            )),
            (true, true) => ResponseTemplate::new(200).set_body_string(format!(
                "{{\"audio_base64\":\"{}\",\"alignment\":{{\"characters\":[]}}}}\n\
                 {{\"audio_base64\":\"{}\",\"alignment\":{{\"characters\":[]}}}}\n",
                b64(b"BOTH1"),
                b64(b"BOTH2")
            )),
        };
        Mock::given(method("POST"))
            .and(path(expected_path))
            .respond_with(response)
            .mount(&server)
            .await;
        let (_config_dir, config) = temp_config(None);
        let output_dir = tempfile::tempdir().unwrap();
        let output_path = output_dir.path().join("audio.mp3");
        let timestamps_path = output_dir.path().join("timings.jsonl");
        let mut args = vec![
            "tts".to_string(),
            "Variant".to_string(),
            "--model".to_string(),
            "eleven_v4".to_string(),
            "--voice-id".to_string(),
            "v_test".to_string(),
            "-o".to_string(),
            output_path.to_string_lossy().into_owned(),
        ];
        if stream {
            args.push("--stream".to_string());
        }
        if timestamps {
            args.push("--with-timestamps".to_string());
            args.push("--save-timestamps".to_string());
            args.push(timestamps_path.to_string_lossy().into_owned());
        }

        let output = configured_bin(&config, &server)
            .args(&args)
            .output()
            .unwrap();
        let envelope = assert_success(&output);
        assert_eq!(std::fs::read(&output_path).unwrap(), expected_audio);
        assert_eq!(envelope["data"]["voice_id"], "v_test");
        assert_eq!(envelope["data"]["model_id"], "eleven_v4");
        assert_eq!(
            envelope["data"]["bytes_written"],
            expected_audio.len() as u64
        );

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body = json_body(&requests[0]);
        assert_eq!(body["model_id"], "eleven_v4");
        assert_eq!(
            body["inputs"],
            serde_json::json!([{ "text": "Variant", "voice_id": "v_test" }])
        );
        assert!(body.get("settings").is_none());
        assert!(body.get("voice_settings").is_none());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_tts_default_remains_multilingual_v2_on_the_text_to_speech_route() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-speech/v_test"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"LEGACY".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config(None);
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("legacy.mp3");

    let output = configured_bin(&config, &server)
        .args([
            "tts",
            "Legacy",
            "--voice-id",
            "v_test",
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let envelope = assert_success(&output);
    assert_eq!(envelope["data"]["model_id"], "eleven_multilingual_v2");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = json_body(&requests[0]);
    assert_eq!(body["model_id"], "eleven_multilingual_v2");
    assert!(body.get("voice_settings").is_some());
    assert!(body.get("inputs").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn v4_dialogue_uses_similarity_and_continuity_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"DIALOGUE".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config(None);
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("dialogue.mp3");

    let output = configured_bin(&config, &server)
        .args([
            "dialogue",
            "Alice:v_alice:Hello",
            "--model",
            "eleven_v4",
            "--similarity",
            "0.8",
            "--previous-text",
            "Earlier",
            "--next-text",
            "Later",
            "--previous-request-id",
            "prev_1",
            "--next-request-id",
            "next_1",
            "--use-pvc-as-ivc",
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = json_body(&requests[0]);
    assert_close(&body["settings"]["similarity"], 0.8);
    assert!(body["settings"].get("similarity_boost").is_none());
    assert_eq!(body["previous_text"], "Earlier");
    assert_eq!(body["future_text"], "Later");
    assert_eq!(body["previous_request_ids"], serde_json::json!(["prev_1"]));
    assert_eq!(body["next_request_ids"], serde_json::json!(["next_1"]));
    assert_eq!(body["use_pvc_as_ivc"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn v4_tts_rejects_unsupported_flags_before_http() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config(None);
    let unsupported = [
        vec!["--style", "0.2"],
        vec!["--speed", "1.1"],
        vec!["--speaker-boost", "true"],
        vec!["--optimize-streaming-latency", "1"],
        vec!["--apply-language-text-normalization"],
    ];

    for flags in unsupported {
        let mut args = vec![
            "tts",
            "Hello",
            "--model",
            "eleven_v4",
            "--voice-id",
            "v_test",
        ];
        args.extend(flags);
        let output = configured_bin(&config, &server)
            .args(args)
            .output()
            .unwrap();
        assert_invalid_input(&output);
    }

    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn eleven_v4_turbo_is_rejected_with_an_eleven_v4_recovery_command() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config(None);
    let cases = [
        vec![
            "tts",
            "Hello",
            "--model",
            "eleven_v4_turbo",
            "--voice-id",
            "v_test",
        ],
        vec![
            "dialogue",
            "Alice:v_alice:Hello",
            "--model",
            "eleven_v4_turbo",
        ],
    ];

    for args in cases {
        let output = configured_bin(&config, &server)
            .args(args)
            .output()
            .unwrap();
        let error = assert_invalid_input(&output);
        assert!(
            error["error"]["suggestion"]
                .as_str()
                .unwrap_or_default()
                .contains("eleven_v4")
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_rejects_unsupported_voice_settings_for_every_model() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config(None);
    let unsupported = [
        vec!["--style", "0.2"],
        vec!["--speaker-boost", "true"],
        vec!["--optimize-streaming-latency", "1"],
    ];

    for model in ["eleven_v3", "eleven_v4"] {
        for flags in &unsupported {
            let mut args = vec!["dialogue", "Alice:v_alice:Hello", "--model", model];
            args.extend(flags.iter().copied());
            let output = configured_bin(&config, &server)
                .args(args)
                .output()
                .unwrap();
            assert_invalid_input(&output);
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn default_dialogue_model_remains_eleven_v3() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"V3".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config(None);
    let output_dir = tempfile::tempdir().unwrap();
    let output_path = output_dir.path().join("dialogue.mp3");

    let output = configured_bin(&config, &server)
        .args([
            "dialogue",
            "Alice:v_alice:Hello",
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let envelope = assert_success(&output);
    assert_eq!(envelope["data"]["model_id"], "eleven_v3");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(json_body(&requests[0])["model_id"], "eleven_v3");
}

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_and_v4_tts_enforce_context_text_limits_before_http() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config(None);
    let too_long = "x".repeat(101);

    for command in ["tts", "dialogue"] {
        for flag in ["--previous-text", "--next-text"] {
            let args = if command == "tts" {
                vec![
                    "tts".to_string(),
                    "Hello".to_string(),
                    "--model".to_string(),
                    "eleven_v4".to_string(),
                    "--voice-id".to_string(),
                    "v_test".to_string(),
                    flag.to_string(),
                    too_long.clone(),
                ]
            } else {
                vec![
                    "dialogue".to_string(),
                    "Alice:v_alice:Hello".to_string(),
                    "--model".to_string(),
                    "eleven_v4".to_string(),
                    flag.to_string(),
                    too_long.clone(),
                ]
            };
            let output = configured_bin(&config, &server)
                .args(&args)
                .output()
                .unwrap();
            assert_invalid_input(&output);
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn dialogue_and_v4_tts_limit_previous_and_next_request_ids_to_three() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config(None);

    for command in ["tts", "dialogue"] {
        for flag in ["--previous-request-id", "--next-request-id"] {
            let mut args = if command == "tts" {
                vec![
                    "tts",
                    "Hello",
                    "--model",
                    "eleven_v4",
                    "--voice-id",
                    "v_test",
                ]
            } else {
                vec!["dialogue", "Alice:v_alice:Hello", "--model", "eleven_v4"]
            };
            for id in ["id_1", "id_2", "id_3", "id_4"] {
                args.push(flag);
                args.push(id);
            }
            let output = configured_bin(&config, &server)
                .args(args)
                .output()
                .unwrap();
            assert_invalid_input(&output);
        }
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn saved_v4_model_routes_to_dialogue_and_explicit_v3_uses_legacy_tts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-dialogue"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"V4".to_vec()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-speech/v_test"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"V3".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config(Some("eleven_v4"));
    let output_dir = tempfile::tempdir().unwrap();
    let v4_path = output_dir.path().join("saved-v4.mp3");
    let v3_path = output_dir.path().join("explicit-v3.mp3");

    let saved = configured_bin(&config, &server)
        .args(["tts", "Saved", "-o", v4_path.to_str().unwrap()])
        .output()
        .unwrap();
    let saved_envelope = assert_success(&saved);
    assert_eq!(saved_envelope["data"]["model_id"], "eleven_v4");

    let explicit = configured_bin(&config, &server)
        .args([
            "tts",
            "Override",
            "--model",
            "eleven_v3",
            "-o",
            v3_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let explicit_envelope = assert_success(&explicit);
    assert_eq!(explicit_envelope["data"]["model_id"], "eleven_v3");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url.path(), "/v1/text-to-dialogue");
    assert_eq!(requests[1].url.path(), "/v1/text-to-speech/v_test");
    assert_eq!(json_body(&requests[0])["model_id"], "eleven_v4");
    assert_eq!(json_body(&requests[1])["model_id"], "eleven_v3");
}

#[tokio::test(flavor = "multi_thread")]
async fn agents_create_and_update_accept_both_v4_conversational_models() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/convai/agents/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "agent_id": "agent_created"
        })))
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path("/v1/convai/agents/agent_test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "agent_id": "agent_test"
        })))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config(None);
    let patch_dir = tempfile::tempdir().unwrap();

    for model in ["eleven_v4", "eleven_v4_turbo"] {
        let create = configured_bin(&config, &server)
            .args([
                "agents",
                "create",
                "V4 agent",
                "--system-prompt",
                "Be concise.",
                "--model-id",
                model,
            ])
            .output()
            .unwrap();
        assert_success(&create);

        let patch_path = patch_dir.path().join(format!("{model}.json"));
        std::fs::write(
            &patch_path,
            serde_json::json!({
                "conversation_config": { "tts": { "model_id": model } }
            })
            .to_string(),
        )
        .unwrap();
        let update = configured_bin(&config, &server)
            .args([
                "agents",
                "update",
                "agent_test",
                "--patch",
                patch_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert_success(&update);
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 4);
    for (index, model) in ["eleven_v4", "eleven_v4_turbo"].iter().enumerate() {
        let create_body = json_body(&requests[index * 2]);
        assert_eq!(
            create_body["conversation_config"]["tts"]["model_id"],
            *model
        );
        let update_body = json_body(&requests[index * 2 + 1]);
        assert_eq!(
            update_body["conversation_config"]["tts"]["model_id"],
            *model
        );
    }
}

#[test]
fn agent_info_model_enum_matches_the_conversational_schema_allowlist() {
    let output = bin().arg("agent-info").output().unwrap();
    assert!(
        output.status.success(),
        "agent-info failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let info: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../docs/reference/openapi.elevenlabs.json")).unwrap();
    let mut actual: Vec<_> = info["known_values"]["agent_tts_model_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let mut expected: Vec<_> = schema["components"]["schemas"]["TTSConversationalModel"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
