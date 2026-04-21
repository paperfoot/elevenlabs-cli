//! Conversational AI agents — list, show, create, update, duplicate, delete,
//! knowledge base attachment, and tool management.

pub mod agent_config;
pub mod create;
pub mod delete;
pub mod duplicate;
pub mod knowledge;
pub mod list;
pub mod llms;
pub mod show;
pub mod signed_url;
pub mod tools;
pub mod update;

use crate::cli::{AgentsAction, AgentsKnowledgeAction};
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::Ctx;

pub async fn dispatch(ctx: Ctx, action: AgentsAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        AgentsAction::List => list::run(ctx, &client).await,
        AgentsAction::Show { agent_id } => show::run(ctx, &client, &agent_id).await,
        AgentsAction::Llms => llms::run(ctx, &client).await,
        AgentsAction::SignedUrl { agent_id } => signed_url::run(ctx, &client, &agent_id).await,
        AgentsAction::Create {
            name,
            system_prompt,
            first_message,
            voice_id,
            language,
            llm,
            temperature,
            model_id,
            expressive_mode,
            max_duration_seconds,
            voicemail_detection,
            voicemail_message,
        } => {
            create::run(
                ctx,
                &cfg,
                &client,
                name,
                system_prompt,
                first_message,
                voice_id,
                language,
                llm,
                temperature,
                model_id,
                expressive_mode,
                max_duration_seconds,
                voicemail_detection,
                voicemail_message,
            )
            .await
        }
        AgentsAction::Update { agent_id, patch } => {
            update::run(ctx, &client, agent_id, patch).await
        }
        AgentsAction::Duplicate { agent_id, name } => {
            duplicate::run(ctx, &client, agent_id, name).await
        }
        AgentsAction::Delete { agent_id, yes } => delete::run(ctx, &client, &agent_id, yes).await,
        AgentsAction::AddKnowledge {
            agent_id,
            name,
            url,
            file,
            text,
        } => knowledge::add(ctx, &client, agent_id, name, url, file, text).await,
        AgentsAction::Tools { action } => tools::dispatch(ctx, &client, action).await,
        AgentsAction::Knowledge { action } => dispatch_knowledge(ctx, &client, action).await,
    }
}

async fn dispatch_knowledge(
    ctx: Ctx,
    client: &ElevenLabsClient,
    action: AgentsKnowledgeAction,
) -> Result<(), AppError> {
    match action {
        AgentsKnowledgeAction::List {
            search,
            page_size,
            cursor,
        } => knowledge::list(ctx, client, search, page_size, cursor).await,
        AgentsKnowledgeAction::Search {
            query,
            document_id,
            limit,
        } => knowledge::search(ctx, client, query, document_id, limit).await,
        AgentsKnowledgeAction::Refresh { document_id } => {
            knowledge::refresh(ctx, client, &document_id).await
        }
    }
}
