//! `phone batch submit` — POST /v1/convai/batch-calling/submit
//!
//! Recipients are supplied via `--recipients <csv_or_json>`. The value is
//! treated as a path. The file extension (.csv vs .json) picks the parser,
//! and `-` reads from stdin (parsed by content heuristics — if it starts
//! with `[` it's JSON, otherwise CSV).
//!
//! CSV format (one recipient per row, header optional):
//!   phone_number,conversation_initiation_client_data
//!   +14155550001,{"dynamic_variables":{"name":"Alice"}}
//!   +14155550002,
//!
//! JSON format (array of objects):
//!   [
//!     {"phone_number":"+14155550001","conversation_initiation_client_data":{...}},
//!     {"phone_number":"+14155550002"}
//!   ]

use std::path::Path;

use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::{self, Ctx};

pub async fn run(
    ctx: Ctx,
    client: &ElevenLabsClient,
    agent_id: String,
    phone_number_id: String,
    recipients: String,
    name: Option<String>,
    scheduled_time_unix: Option<i64>,
) -> Result<(), AppError> {
    let recipients_json = load_recipients(&recipients).await?;
    if recipients_json.is_empty() {
        return Err(AppError::InvalidInput {
            msg: "recipients list is empty — supply at least one phone number".into(),
            suggestion: None,
        });
    }

    let mut body = serde_json::Map::new();
    body.insert("agent_id".into(), serde_json::Value::String(agent_id));
    body.insert(
        "agent_phone_number_id".into(),
        serde_json::Value::String(phone_number_id),
    );
    body.insert(
        "recipients".into(),
        serde_json::Value::Array(recipients_json.clone()),
    );
    // The SDK field is `call_name` (elevenlabs-python raw_client:
    // batch_calls.create). The CLI flag stays `--name` for ergonomic
    // consistency with every other `list`/`submit` surface; we map
    // internally to the SDK-correct field name.
    if let Some(n) = name {
        body.insert("call_name".into(), serde_json::Value::String(n));
    }
    if let Some(ts) = scheduled_time_unix {
        body.insert(
            "scheduled_time_unix".into(),
            serde_json::Value::Number(serde_json::Number::from(ts)),
        );
    }

    let resp: serde_json::Value = client
        .post_json(
            "/v1/convai/batch-calling/submit",
            &serde_json::Value::Object(body),
        )
        .await?;

    let recipient_count = recipients_json.len();
    output::print_success_or(ctx, &resp, |v| {
        use owo_colors::OwoColorize;
        let batch_id = v
            .get("id")
            .or_else(|| v.get("batch_id"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        println!(
            "{} submitted batch {} ({} recipient{})",
            "+".green(),
            batch_id.bold(),
            recipient_count,
            if recipient_count == 1 { "" } else { "s" }
        );
        if !batch_id.is_empty() {
            println!(
                "  {} {}",
                "poll:".dimmed(),
                format!("elevenlabs phone batch show {batch_id}").dimmed()
            );
        }
    })?;
    Ok(())
}

async fn load_recipients(source: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let text = if source == "-" {
        let mut buf = String::new();
        use tokio::io::AsyncReadExt;
        tokio::io::stdin()
            .read_to_string(&mut buf)
            .await
            .map_err(AppError::Io)?;
        buf
    } else {
        let path = Path::new(source);
        if !path.exists() {
            return Err(AppError::bad_input_with(
                format!("recipients file does not exist: {}", path.display()),
                "pass an existing CSV or JSON list, e.g. --recipients /path/to/recipients.csv  \
                 (or `--recipients -` to read JSON/CSV from stdin)",
            ));
        }
        tokio::fs::read_to_string(path)
            .await
            .map_err(AppError::Io)?
    };

    let trimmed = text.trim_start();
    if trimmed.starts_with('[') {
        parse_recipients_json(&text)
    } else {
        parse_recipients_csv(&text)
    }
}

fn parse_recipients_json(text: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let parsed: serde_json::Value =
        serde_json::from_str(text).map_err(|e| AppError::InvalidInput {
            msg: format!("recipients JSON is invalid: {e}"),
            suggestion: None,
        })?;
    let arr = parsed.as_array().ok_or_else(|| AppError::InvalidInput {
        msg: "recipients JSON must be an array of objects".into(),
        suggestion: None,
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for (idx, item) in arr.iter().enumerate() {
        let obj = item.as_object().ok_or_else(|| AppError::InvalidInput {
            msg: format!("recipients JSON entry #{} must be an object", idx + 1),
            suggestion: None,
        })?;
        if !obj.contains_key("phone_number") {
            return Err(AppError::InvalidInput {
                msg: format!("recipients JSON entry #{} is missing phone_number", idx + 1),
                suggestion: None,
            });
        }
        out.push(serde_json::Value::Object(obj.clone()));
    }
    Ok(out)
}

/// Minimal RFC-4180-style CSV parser for the recipients file. Supports two
/// columns: `phone_number` (required) and `conversation_initiation_client_data`
/// (optional JSON). The header row is optional — if the first row's first
/// cell starts with `phone_number` we skip it.
fn parse_recipients_csv(text: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = parse_csv_rows(text)?;
    let mut iter = rows.into_iter().peekable();

    // Optional header.
    if let Some(first) = iter.peek() {
        if let Some(cell) = first.first() {
            if cell.eq_ignore_ascii_case("phone_number") {
                iter.next();
            }
        }
    }

    let mut out = Vec::new();
    for (idx, row) in iter.enumerate() {
        // Skip only truly-blank rows (trailing newline, blank separator line).
        // A row like `,` has two empty cells and should still be validated.
        if row.is_empty() || (row.len() == 1 && row[0].trim().is_empty()) {
            continue;
        }
        let phone = row
            .first()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if phone.is_empty() {
            return Err(AppError::InvalidInput {
                msg: format!("recipients CSV row #{} has empty phone_number", idx + 1),
                suggestion: None,
            });
        }
        let mut entry = serde_json::Map::new();
        entry.insert("phone_number".into(), serde_json::Value::String(phone));

        if let Some(data_cell) = row.get(1) {
            let trimmed = data_cell.trim();
            if !trimmed.is_empty() {
                let parsed: serde_json::Value = serde_json::from_str(trimmed).map_err(|e| {
                    AppError::InvalidInput { msg: format!(
                        "recipients CSV row #{} has invalid JSON in conversation_initiation_client_data: {e}",
                        idx + 1
                    ), suggestion: None }
                })?;
                entry.insert("conversation_initiation_client_data".into(), parsed);
            }
        }
        out.push(serde_json::Value::Object(entry));
    }

    Ok(out)
}

/// Parse a CSV blob into rows of cells. Supports quoted cells with embedded
/// commas and `""` escaping per RFC-4180. Line endings may be `\n` or `\r\n`.
fn parse_csv_rows(text: &str) -> Result<Vec<Vec<String>>, AppError> {
    let mut rows = Vec::new();
    let mut row = Vec::<String>::new();
    let mut cell = String::new();
    let mut in_quotes = false;

    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    cell.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                cell.push(c);
            }
        } else {
            match c {
                '"' => {
                    if cell.is_empty() {
                        in_quotes = true;
                    } else {
                        // Inline quote in unquoted cell — treat literally.
                        cell.push('"');
                    }
                }
                ',' => {
                    row.push(std::mem::take(&mut cell));
                }
                '\r' => {
                    // Consume optional \n and end the row.
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                '\n' => {
                    row.push(std::mem::take(&mut cell));
                    rows.push(std::mem::take(&mut row));
                }
                _ => cell.push(c),
            }
        }
    }

    if in_quotes {
        return Err(AppError::InvalidInput {
            msg: "recipients CSV has an unterminated quoted cell".into(),
            suggestion: None,
        });
    }

    // Flush the trailing cell/row if no final newline.
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_basic_no_header() {
        let text = "+14155550001,\n+14155550002,\n";
        let out = parse_recipients_csv(text).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["phone_number"], "+14155550001");
        assert_eq!(out[1]["phone_number"], "+14155550002");
        assert!(out[0].get("conversation_initiation_client_data").is_none());
    }

    #[test]
    fn csv_with_header_and_json_col() {
        let text = "phone_number,conversation_initiation_client_data\n\
                    +14155550001,\"{\"\"dynamic_variables\"\":{\"\"name\"\":\"\"Alice\"\"}}\"\n\
                    +14155550002,\n";
        let out = parse_recipients_csv(text).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["phone_number"], "+14155550001");
        assert_eq!(
            out[0]["conversation_initiation_client_data"]["dynamic_variables"]["name"],
            "Alice"
        );
        assert!(out[1].get("conversation_initiation_client_data").is_none());
    }

    #[test]
    fn csv_empty_phone_rejected() {
        let text = ",\n";
        let err = parse_recipients_csv(text).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[test]
    fn csv_bad_json_in_data_column_rejected() {
        let text = "+14155550001,{not json}\n";
        let err = parse_recipients_csv(text).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[test]
    fn json_array_ok() {
        let text = r#"[{"phone_number":"+14155550001"},{"phone_number":"+14155550002","conversation_initiation_client_data":{"k":"v"}}]"#;
        let out = parse_recipients_json(text).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[1]["conversation_initiation_client_data"]["k"], "v");
    }

    #[test]
    fn json_missing_phone_number_rejected() {
        let text = r#"[{"conversation_initiation_client_data":{}}]"#;
        let err = parse_recipients_json(text).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[test]
    fn json_non_array_rejected() {
        let text = r#"{"phone_number":"+1"}"#;
        let err = parse_recipients_json(text).unwrap_err();
        assert!(matches!(err, AppError::InvalidInput { .. }));
    }

    #[test]
    fn csv_crlf_and_trailing_newline() {
        let text = "+14155550001,\r\n+14155550002,\r\n";
        let out = parse_recipients_csv(text).unwrap();
        assert_eq!(out.len(), 2);
    }
}
