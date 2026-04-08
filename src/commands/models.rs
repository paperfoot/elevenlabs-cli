//! models list

use serde::{Deserialize, Serialize};

use crate::cli::ModelsAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Model {
    model_id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    can_do_text_to_speech: Option<bool>,
    #[serde(default)]
    can_use_speaker_boost: Option<bool>,
    #[serde(default)]
    languages: Vec<Language>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Language {
    language_id: String,
    name: String,
}

pub async fn dispatch(ctx: Ctx, action: ModelsAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;
    match action {
        ModelsAction::List => list(ctx, &client).await,
    }
}

async fn list(ctx: Ctx, client: &ElevenLabsClient) -> Result<(), AppError> {
    let models: Vec<Model> = client.get_json("/v1/models").await?;
    output::print_success_or(ctx, &models, |list| {
        use owo_colors::OwoColorize;
        let mut t = comfy_table::Table::new();
        t.set_header(vec!["Model ID", "Name", "Languages", "TTS"]);
        for m in list {
            t.add_row(vec![
                m.model_id.dimmed().to_string(),
                m.name.bold().to_string(),
                m.languages.len().to_string(),
                if m.can_do_text_to_speech.unwrap_or(false) {
                    "Yes".green().to_string()
                } else {
                    "No".dimmed().to_string()
                },
            ]);
        }
        println!("{t}");
    });
    Ok(())
}
