//! JSON envelope + human output detection. Stdout is always parseable:
//! JSON envelope when piped or `--json`, coloured output on a TTY.

use serde::Serialize;
use std::io::{IsTerminal, Write};

use crate::error::AppError;

#[derive(Clone, Copy)]
pub enum Format {
    Json,
    Human,
}

impl Format {
    pub fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() {
            Format::Json
        } else {
            Format::Human
        }
    }
}

/// Output context passed to commands. Bundles format + quiet flag.
#[derive(Clone, Copy)]
pub struct Ctx {
    pub format: Format,
    pub quiet: bool,
}

impl Ctx {
    pub fn new(json_flag: bool, quiet: bool) -> Self {
        Self {
            format: Format::detect(json_flag),
            quiet,
        }
    }

    #[allow(dead_code)]
    pub fn is_json(&self) -> bool {
        matches!(self.format, Format::Json)
    }
}

/// Serialize before writing, so serialization errors never produce success output.
/// Machine output is compact; consumers can use `jq` when indentation is useful.
fn write_json<T: Serialize>(writer: &mut impl Write, value: &T) -> Result<(), AppError> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|_| AppError::Output("failed to serialize command output".into()))?;
    bytes.push(b'\n');
    writer
        .write_all(&bytes)
        .map_err(|error| AppError::Output(error.to_string()))?;
    Ok(())
}

pub fn print_json<T: Serialize>(value: &T) -> Result<(), AppError> {
    write_json(&mut std::io::stdout().lock(), value)
}

#[derive(Serialize)]
struct SuccessEnvelope<'a, T> {
    version: &'static str,
    status: &'static str,
    data: &'a T,
}

/// Print success envelope (JSON) or call the human closure.
/// Quiet suppresses human output. JSON is never suppressed.
pub fn print_success_or<T: Serialize, F: FnOnce(&T)>(
    ctx: Ctx,
    data: &T,
    human: F,
) -> Result<(), AppError> {
    match ctx.format {
        Format::Json => print_json(&SuccessEnvelope {
            version: "1",
            status: "success",
            data,
        })?,
        Format::Human if !ctx.quiet => human(data),
        Format::Human => {}
    }
    Ok(())
}

pub fn print_error(format: Format, err: &AppError) {
    let envelope = serde_json::json!({
        "version": "1",
        "status": "error",
        "error": {
            "code": err.error_code(),
            "message": err.to_string(),
            "suggestion": err.suggestion(),
        },
    });
    match format {
        Format::Json => {
            // Error reporting must not panic if the consumer also closed stderr.
            let _ = write_json(&mut std::io::stderr().lock(), &envelope);
        }
        Format::Human => {
            use owo_colors::OwoColorize;
            let _ = writeln!(
                std::io::stderr().lock(),
                "{} {}\n  {}",
                "error:".red().bold(),
                err,
                err.suggestion().dimmed()
            );
        }
    }
}

pub fn print_help_json(err: clap::Error) -> Result<(), AppError> {
    let envelope = serde_json::json!({
        "version": "1",
        "status": "success",
        "data": { "usage": err.to_string().trim_end() },
    });
    print_json(&envelope)
}

pub fn print_clap_error(format: Format, err: &clap::Error) {
    match format {
        Format::Json => {
            let envelope = serde_json::json!({
                "version": "1",
                "status": "error",
                "error": {
                    "code": "invalid_input",
                    "message": err.to_string(),
                    "suggestion": "Check arguments with: elevenlabs --help",
                },
            });
            let _ = write_json(&mut std::io::stderr().lock(), &envelope);
        }
        Format::Human => {
            let _ = write!(std::io::stderr().lock(), "{err}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FailsSerialization;

    impl Serialize for FailsSerialization {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("test failure"))
        }
    }

    #[test]
    fn serialization_failure_writes_nothing_and_returns_runtime_error() {
        let mut output = Vec::new();
        let envelope = SuccessEnvelope {
            version: "1",
            status: "success",
            data: &FailsSerialization,
        };
        let err = write_json(&mut output, &envelope).unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert!(output.is_empty());
    }

    #[test]
    fn write_failure_returns_runtime_error() {
        let mut insufficient = [0_u8; 1];
        let err = write_json(
            &mut insufficient.as_mut_slice(),
            &serde_json::json!({"ok": true}),
        )
        .unwrap_err();
        assert_eq!(err.exit_code(), 1);
        assert_eq!(err.error_code(), "output_error");
    }
}
