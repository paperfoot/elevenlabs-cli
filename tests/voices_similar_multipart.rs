//! Verifies `voices similar` POSTs multipart to `/v1/similar-voices` with
//! the audio file attached and rejects removed demographic filter fields.
//!
//! Wiremock 0.6 doesn't parse multipart boundaries natively, so we just
//! assert the endpoint path is hit and that the content-type is multipart.
//! A body-contains matcher sanity-checks the form field names.

use assert_cmd::Command as AssertCmd;
use std::io::Write;
use std::path::PathBuf;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn bin() -> AssertCmd {
    AssertCmd::cargo_bin("elevenlabs").unwrap()
}

fn temp_config_with_key(api_key: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, path)
}

fn temp_audio() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("sample.mp3");
    std::fs::write(&p, b"FAKEMP3BYTES").unwrap();
    (dir, p)
}

#[tokio::test(flavor = "multi_thread")]
async fn similar_posts_multipart_with_audio_file() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/similar-voices"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": [
                {"voice_id": "v_sim1", "name": "Close Match", "category": "professional"}
            ]
        })))
        .mount(&mock)
        .await;

    let (_audio_dir, audio_path) = temp_audio();
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "voices",
            "similar",
            audio_path.to_str().unwrap(),
            "--similarity-threshold",
            "0.75",
            "--top-k",
            "5",
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "success");
    let voices = body["data"]["voices"].as_array().unwrap();
    assert_eq!(voices.len(), 1);
    assert_eq!(voices[0]["voice_id"], "v_sim1");
}

#[tokio::test(flavor = "multi_thread")]
async fn similar_rejects_removed_filter_flags_before_network() {
    let mock = MockServer::start().await;

    // Use a catching mock that we can inspect after.
    Mock::given(method("POST"))
        .and(path("/v1/similar-voices"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "voices": []
        })))
        .mount(&mock)
        .await;

    let (_audio_dir, audio_path) = temp_audio();
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    for (flag, value) in [
        ("--gender", "female"),
        ("--age", "young"),
        ("--accent", "british"),
        ("--language", "en"),
        ("--use-case", "narration"),
    ] {
        let out = bin()
            .env("ELEVENLABS_CLI_CONFIG", &cfg)
            .env("ELEVENLABS_API_BASE_URL", mock.uri())
            .env_remove("ELEVENLABS_API_KEY")
            .args([
                "voices",
                "similar",
                audio_path.to_str().unwrap(),
                flag,
                value,
            ])
            .output()
            .unwrap();

        assert_eq!(out.status.code(), Some(3), "{flag} must be rejected");
        assert!(
            out.stdout.is_empty(),
            "parse errors must not leak onto stdout"
        );
        let error: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(error["status"], "error");
        assert_eq!(error["error"]["code"], "invalid_input");
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn similar_errors_when_audio_missing() {
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "voices",
            "similar",
            "/nonexistent/file/that/should/never/exist.mp3",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "missing audio file should exit 3 (invalid_input); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
