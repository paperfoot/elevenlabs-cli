//! Compatibility regressions for the September 2026 STT and Music API refresh.
//!
//! Every HTTP assertion targets a local wiremock server. Invalid-input cases
//! also assert that validation completes before any request is sent.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::{Path, PathBuf};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const TEST_KEY: &str = "sk_test_keyyyyyyyyy";

fn bin() -> AssertCmd {
    AssertCmd::cargo_bin("elevenlabs").unwrap()
}

fn temp_config_with_key() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut file = std::fs::File::create(&path).unwrap();
    writeln!(file, "api_key = \"{TEST_KEY}\"").unwrap();
    (dir, path)
}

fn configured_bin(config: &Path, server: &MockServer) -> AssertCmd {
    let mut command = bin();
    command
        .env("ELEVENLABS_CLI_CONFIG", config)
        .env("ELEVENLABS_API_BASE_URL", server.uri())
        .env_remove("ELEVENLABS_API_KEY");
    command
}

fn assert_success(output: &std::process::Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["version"], "1");
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
    assert!(output.stdout.is_empty(), "errors must not leak onto stdout");
    let envelope: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(envelope["version"], "1");
    assert_eq!(envelope["status"], "error");
    assert_eq!(envelope["error"]["code"], "invalid_input");
    envelope
}

fn json_body(request: &Request) -> serde_json::Value {
    serde_json::from_slice(&request.body).expect("request body must be JSON")
}

fn multipart_field(request: &Request, field: &str) -> Option<String> {
    let body = String::from_utf8_lossy(&request.body);
    let marker = format!("name=\"{field}\"");
    let field_start = body.find(&marker)?;
    let content_start = body[field_start..].find("\r\n\r\n")? + field_start + 4;
    let content_end = body[content_start..].find("\r\n--")? + content_start;
    Some(body[content_start..content_end].to_string())
}

fn multipart_mixed(boundary: &str) -> Vec<u8> {
    format!(
        "--{boundary}\r\nContent-Type: application/json\r\n\r\n{{\"bpm\":120}}\r\n\
         --{boundary}\r\nContent-Type: audio/mpeg\r\n\r\nFAKEAUDIO\r\n\
         --{boundary}--\r\n"
    )
    .into_bytes()
}

async fn mount_stt(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/v1/speech-to-text"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "text": "hello",
            "words": []
        })))
        .mount(server)
        .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn stt_defaults_to_scribe_v2_and_accepts_future_model_ids() {
    let server = MockServer::start().await;
    mount_stt(&server).await;
    let (_config_dir, config) = temp_config_with_key();
    let input_dir = tempfile::tempdir().unwrap();
    let input = input_dir.path().join("sample.wav");
    std::fs::write(&input, b"FAKEWAV").unwrap();

    let default = configured_bin(&config, &server)
        .args(["stt", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_success(&default);

    let future = configured_bin(&config, &server)
        .args([
            "stt",
            input.to_str().unwrap(),
            "--model",
            "scribe_v3_future",
        ])
        .output()
        .unwrap();
    assert_success(&future);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        multipart_field(&requests[0], "model_id").as_deref(),
        Some("scribe_v2")
    );
    assert_eq!(
        multipart_field(&requests[1], "model_id").as_deref(),
        Some("scribe_v3_future")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn stt_medical_no_verbatim_sends_both_multipart_fields() {
    let server = MockServer::start().await;
    mount_stt(&server).await;
    let (_config_dir, config) = temp_config_with_key();
    let input_dir = tempfile::tempdir().unwrap();
    let input = input_dir.path().join("consultation.wav");
    std::fs::write(&input, b"FAKEWAV").unwrap();

    let output = configured_bin(&config, &server)
        .args([
            "stt",
            input.to_str().unwrap(),
            "--model",
            "scribe_v2_medical",
            "--no-verbatim",
        ])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        multipart_field(&requests[0], "model_id").as_deref(),
        Some("scribe_v2_medical")
    );
    assert_eq!(
        multipart_field(&requests[0], "no_verbatim").as_deref(),
        Some("true")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn retired_scribe_v1_is_rejected_before_network_with_recovery_command() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config_with_key();
    let input_dir = tempfile::tempdir().unwrap();
    let input = input_dir.path().join("sample.wav");
    std::fs::write(&input, b"FAKEWAV").unwrap();

    let output = configured_bin(&config, &server)
        .args(["stt", input.to_str().unwrap(), "--model", "scribe_v1"])
        .output()
        .unwrap();
    let error = assert_invalid_input(&output);
    assert!(
        error["error"]["suggestion"]
            .as_str()
            .unwrap()
            .contains("--model scribe_v2")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn music_compose_defaults_to_v2_5_and_honours_override() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEAUDIO".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();
    let output_dir = tempfile::tempdir().unwrap();
    let default_path = output_dir.path().join("default.mp3");
    let override_path = output_dir.path().join("override.mp3");

    let default = configured_bin(&config, &server)
        .args([
            "music",
            "compose",
            "ambient",
            "-o",
            default_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&default);
    let override_output = configured_bin(&config, &server)
        .args([
            "music",
            "compose",
            "ambient",
            "--model",
            "music_v2",
            "-o",
            override_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&override_output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(json_body(&requests[0])["model_id"], "music_v2_5");
    assert_eq!(json_body(&requests[1])["model_id"], "music_v2");
}

#[tokio::test(flavor = "multi_thread")]
async fn music_stream_defaults_to_v2_5_and_honours_override() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/stream"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEAUDIO".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();
    let output_dir = tempfile::tempdir().unwrap();
    let default_path = output_dir.path().join("default.mp3");
    let override_path = output_dir.path().join("override.mp3");

    let default = configured_bin(&config, &server)
        .args([
            "music",
            "stream",
            "ambient",
            "-o",
            default_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&default);
    let override_output = configured_bin(&config, &server)
        .args([
            "music",
            "stream",
            "ambient",
            "--model",
            "music_v2",
            "-o",
            override_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&override_output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(json_body(&requests[0])["model_id"], "music_v2_5");
    assert_eq!(json_body(&requests[1])["model_id"], "music_v2");
}

#[tokio::test(flavor = "multi_thread")]
async fn music_detailed_defaults_to_v2_5_and_honours_override() {
    let boundary = "api-refresh-boundary";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/detailed"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "content-type",
                    format!("multipart/mixed; boundary={boundary}").as_str(),
                )
                .set_body_bytes(multipart_mixed(boundary)),
        )
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();
    let output_dir = tempfile::tempdir().unwrap();
    let default_path = output_dir.path().join("default.mp3");
    let override_path = output_dir.path().join("override.mp3");

    let default = configured_bin(&config, &server)
        .args([
            "music",
            "detailed",
            "ambient",
            "-o",
            default_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&default);
    let override_output = configured_bin(&config, &server)
        .args([
            "music",
            "detailed",
            "ambient",
            "--model",
            "music_v2",
            "-o",
            override_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&override_output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(json_body(&requests[0])["model_id"], "music_v2_5");
    assert_eq!(json_body(&requests[1])["model_id"], "music_v2");
}

#[tokio::test(flavor = "multi_thread")]
async fn music_plan_defaults_to_v2_5_and_honours_override() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/plan"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "chunks": []
        })))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();

    let default = configured_bin(&config, &server)
        .args(["music", "plan", "ambient"])
        .output()
        .unwrap();
    assert_success(&default);
    let override_output = configured_bin(&config, &server)
        .args(["music", "plan", "ambient", "--model", "music_v1"])
        .output()
        .unwrap();
    assert_success(&override_output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(json_body(&requests[0])["model_id"], "music_v2_5");
    assert_eq!(json_body(&requests[1])["model_id"], "music_v1");
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_sections_plan_auto_selects_music_v1() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEAUDIO".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let plan_path = files.path().join("sections.json");
    let output_path = files.path().join("sections.mp3");
    let plan = serde_json::json!({
        "sections": [{"section_name": "intro", "duration_ms": 3000}]
    });
    std::fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();

    let output = configured_bin(&config, &server)
        .args([
            "music",
            "compose",
            "--composition-plan",
            plan_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = json_body(&requests[0]);
    assert_eq!(body["model_id"], "music_v1");
    assert_eq!(body["composition_plan"], plan);
}

#[tokio::test(flavor = "multi_thread")]
async fn enveloped_chunks_plan_is_unwrapped_and_auto_selects_v2_5() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"FAKEAUDIO".to_vec()))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let plan_path = files.path().join("chunks.json");
    let output_path = files.path().join("chunks.mp3");
    let chunks = serde_json::json!({
        "chunks": [{"duration_ms": 3000, "lines": ["instrumental intro"]}]
    });
    let envelope = serde_json::json!({
        "status": "success",
        "data": chunks,
        "version": "1"
    });
    std::fs::write(&plan_path, serde_json::to_vec(&envelope).unwrap()).unwrap();

    let output = configured_bin(&config, &server)
        .args([
            "music",
            "compose",
            "--composition-plan",
            plan_path.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = json_body(&requests[0]);
    assert_eq!(body["model_id"], "music_v2_5");
    assert_eq!(body["composition_plan"], chunks);
    assert!(body["composition_plan"].get("status").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn music_v1_rejects_chunk_plans_before_network() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let plan_path = files.path().join("chunks.json");
    std::fs::write(&plan_path, r#"{"chunks":[]}"#).unwrap();

    let output = configured_bin(&config, &server)
        .args([
            "music",
            "compose",
            "--composition-plan",
            plan_path.to_str().unwrap(),
            "--model",
            "music_v1",
        ])
        .output()
        .unwrap();
    let error = assert_invalid_input(&output);
    let suggestion = error["error"]["suggestion"].as_str().unwrap();
    assert!(suggestion.contains("elevenlabs music compose"));
    assert!(suggestion.contains("--model music_v2_5"));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn v2_music_models_reject_legacy_sections_before_network() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let plan_path = files.path().join("sections.json");
    std::fs::write(&plan_path, r#"{"sections":[]}"#).unwrap();

    for model in ["music_v2", "music_v2_5"] {
        let output = configured_bin(&config, &server)
            .args([
                "music",
                "compose",
                "--composition-plan",
                plan_path.to_str().unwrap(),
                "--model",
                model,
            ])
            .output()
            .unwrap();
        let error = assert_invalid_input(&output);
        let suggestion = error["error"]["suggestion"].as_str().unwrap();
        assert!(suggestion.contains("elevenlabs music compose"));
        assert!(suggestion.contains("--model music_v1"));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_composition_plan_is_rejected_before_network() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let plan_path = files.path().join("malformed.json");
    std::fs::write(&plan_path, b"{not-json").unwrap();

    let output = configured_bin(&config, &server)
        .args([
            "music",
            "compose",
            "--composition-plan",
            plan_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let error = assert_invalid_input(&output);
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not valid JSON")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn music_upload_extract_flag_sends_default_or_overridden_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "song_id": "song_123"
        })))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let input = files.path().join("track.mp3");
    std::fs::write(&input, b"FAKEAUDIO").unwrap();

    for model in [None, Some("music_v1"), Some("music_v2")] {
        let mut args = vec![
            "music",
            "upload",
            input.to_str().unwrap(),
            "--extract-composition-plan",
        ];
        if let Some(model) = model {
            args.extend(["--model", model]);
        }
        let output = configured_bin(&config, &server)
            .args(args)
            .output()
            .unwrap();
        assert_success(&output);
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        multipart_field(&requests[0], "extract_composition_plan").as_deref(),
        Some("music_v2_5")
    );
    assert_eq!(
        multipart_field(&requests[1], "extract_composition_plan").as_deref(),
        Some("music_v1")
    );
    assert_eq!(
        multipart_field(&requests[2], "extract_composition_plan").as_deref(),
        Some("music_v2")
    );
    for request in &requests {
        assert!(multipart_field(request, "model_id").is_none());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn music_upload_without_extraction_sends_no_plan_model_field() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/music/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "song_id": "song_123"
        })))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let input = files.path().join("track.mp3");
    std::fs::write(&input, b"FAKEAUDIO").unwrap();

    let output = configured_bin(&config, &server)
        .args(["music", "upload", input.to_str().unwrap()])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(multipart_field(&requests[0], "extract_composition_plan").is_none());
    assert!(multipart_field(&requests[0], "model_id").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn music_upload_model_requires_extraction_flag_before_network() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config_with_key();
    let files = tempfile::tempdir().unwrap();
    let input = files.path().join("track.mp3");
    std::fs::write(&input, b"FAKEAUDIO").unwrap();

    let output = configured_bin(&config, &server)
        .args([
            "music",
            "upload",
            input.to_str().unwrap(),
            "--model",
            "music_v1",
        ])
        .output()
        .unwrap();
    let error = assert_invalid_input(&output);
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--extract-composition-plan")
    );
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn voice_design_v3_stream_previews_sends_current_request_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/text-to-voice/design"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "previews": [
                {"generated_voice_id": "preview_a"},
                {"generated_voice_id": "preview_b"}
            ]
        })))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();

    let output = configured_bin(&config, &server)
        .args([
            "voices",
            "design",
            "warm, calm British narrator",
            "--model",
            "eleven_ttv_v3",
            "--stream-previews",
        ])
        .output()
        .unwrap();
    let envelope = assert_success(&output);
    let previews = envelope["data"]["previews"].as_array().unwrap();
    assert_eq!(previews.len(), 2);
    assert_eq!(previews[0]["generated_voice_id"], "preview_a");
    assert!(previews[0]["file"].is_null());

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = json_body(&requests[0]);
    assert_eq!(body["model_id"], "eleven_ttv_v3");
    assert_eq!(body["stream_previews"], true);
}

#[tokio::test(flavor = "multi_thread")]
async fn voices_library_sends_zero_based_default_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/shared-voices"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": []
        })))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();

    let output = configured_bin(&config, &server)
        .args(["voices", "library"])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let query: Vec<_> = requests[0].url.query_pairs().collect();
    assert!(
        query
            .iter()
            .any(|(key, value)| key == "page" && value == "0")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn voices_list_rejects_removed_show_legacy_flag_before_network() {
    let server = MockServer::start().await;
    let (_config_dir, config) = temp_config_with_key();

    let output = configured_bin(&config, &server)
        .args(["voices", "list", "--show-legacy"])
        .output()
        .unwrap();
    assert_invalid_input(&output);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_create_uses_current_asr_shape_and_retains_verified_turn_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/convai/agents/create"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "agent_id": "agent_123"
        })))
        .mount(&server)
        .await;
    let (_config_dir, config) = temp_config_with_key();

    let output = configured_bin(&config, &server)
        .args([
            "agents",
            "create",
            "Test agent",
            "--system-prompt",
            "Be helpful.",
        ])
        .output()
        .unwrap();
    assert_success(&output);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let body = json_body(&requests[0]);
    let config = &body["conversation_config"];
    assert_eq!(config["agent"]["prompt"]["llm"], "gemini-3.1-flash-lite");
    assert_eq!(config["asr"]["provider"], "scribe_realtime");
    assert!(config["asr"].get("optimize_streaming_latency").is_none());
    assert!(config["tts"].get("optimize_streaming_latency").is_none());
    assert_eq!(config["turn"]["turn_model"], "turn_v2");
}
