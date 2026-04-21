//! `dubbing get-transcript --format <srt|webvtt|json>` must route to the
//! `/v1/dubbing/{id}/transcripts/{lang}/format/{format_type}` endpoint
//! (OpenAPI operationId `get_dubbing_transcripts`, plural `/transcripts/`)
//! with the requested format in the path.

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
    let p = dir.path().join("config.toml");
    let mut f = std::fs::File::create(&p).unwrap();
    writeln!(f, "api_key = \"{api_key}\"").unwrap();
    (dir, p)
}

async fn run_format(fmt: &str, expected_path: &str, body: &'static [u8]) {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(expected_path))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
        .mount(&mock)
        .await;

    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let tmp_out = tempfile::tempdir().unwrap();
    let out_path = tmp_out.path().join(format!("t.{fmt}"));
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", mock.uri())
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dubbing",
            "get-transcript",
            "dub_abc",
            "es",
            "--format",
            fmt,
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "format={fmt} expected success; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let saved = std::fs::read(&out_path).unwrap();
    assert_eq!(saved, body, "format={fmt} saved bytes must match response");
}

#[tokio::test(flavor = "multi_thread")]
async fn transcript_srt_route() {
    run_format(
        "srt",
        "/v1/dubbing/dub_abc/transcripts/es/format/srt",
        b"1\n00:00:00,000 --> 00:00:01,000\nhola\n",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn transcript_webvtt_route() {
    run_format(
        "webvtt",
        "/v1/dubbing/dub_abc/transcripts/es/format/webvtt",
        b"WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nhola\n",
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn transcript_json_route() {
    run_format(
        "json",
        "/v1/dubbing/dub_abc/transcripts/es/format/json",
        br#"{"segments":[{"text":"hola","start":0.0,"end":1.0}]}"#,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn transcript_rejects_unknown_format() {
    // clap's value_parser catches this before the handler runs.
    let (_dir, cfg) = temp_config_with_key("sk_test_keyyyyyyyyy");
    let out = bin()
        .env("ELEVENLABS_CLI_CONFIG", &cfg)
        .env("ELEVENLABS_API_BASE_URL", "http://127.0.0.1:1")
        .env_remove("ELEVENLABS_API_KEY")
        .args([
            "dubbing",
            "get-transcript",
            "dub_abc",
            "es",
            "--format",
            "docx",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(3),
        "expected exit 3 for unknown --format; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
