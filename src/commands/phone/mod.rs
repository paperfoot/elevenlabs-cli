//! Phone: phone numbers, outbound calls, batch calling, WhatsApp channel.
//!
//! Endpoints (grounded against the elevenlabs-python raw_client files and
//! the public API docs at https://elevenlabs.io/docs/api-reference):
//!   - GET    /v1/convai/phone-numbers                              — list phone numbers
//!   - POST   /v1/convai/twilio/outbound-call                       — Twilio outbound
//!   - POST   /v1/convai/sip-trunk/outbound-call                    — SIP outbound
//!   - POST   /v1/convai/batch-calling/submit                       — submit batch
//!   - GET    /v1/convai/batch-calling/workspace                    — list batches
//!   - GET    /v1/convai/batch-calling/{id}                         — show batch
//!   - POST   /v1/convai/batch-calling/{id}/cancel                  — cancel batch
//!   - POST   /v1/convai/batch-calling/{id}/retry                   — retry batch
//!   - DELETE /v1/convai/batch-calling/{id}                         — delete batch
//!   - POST   /v1/convai/whatsapp/outbound-call                     — WA voice call
//!   - POST   /v1/convai/whatsapp/outbound-message                  — WA text/template
//!   - GET    /v1/convai/whatsapp-accounts                          — list WA accounts
//!   - GET    /v1/convai/whatsapp-accounts/{id}                     — show WA account
//!   - PATCH  /v1/convai/whatsapp-accounts/{id}                     — update WA account
//!   - DELETE /v1/convai/whatsapp-accounts/{id}                     — delete WA account

pub mod batch;
pub mod call;
pub mod list;
pub mod whatsapp;

use crate::cli::{PhoneAction, PhoneBatchAction, PhoneWhatsappAccountsAction, PhoneWhatsappAction};
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::Ctx;

pub async fn dispatch(ctx: Ctx, action: PhoneAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        PhoneAction::List => list::run(ctx, &client).await,
        PhoneAction::Call {
            agent_id,
            from_id,
            to,
            dynamic_variables,
            client_data,
            record,
            ringing_timeout_secs,
        } => {
            call::run(
                ctx,
                &client,
                agent_id,
                from_id,
                to,
                dynamic_variables,
                client_data,
                record,
                ringing_timeout_secs,
            )
            .await
        }
        PhoneAction::Batch { action } => dispatch_batch(ctx, &client, action).await,
        PhoneAction::Whatsapp { action } => dispatch_whatsapp(ctx, &client, action).await,
    }
}

async fn dispatch_batch(
    ctx: Ctx,
    client: &ElevenLabsClient,
    action: PhoneBatchAction,
) -> Result<(), AppError> {
    match action {
        PhoneBatchAction::Submit {
            agent_id,
            phone_number_id,
            recipients,
            name,
            scheduled_time_unix,
        } => {
            batch::submit::run(
                ctx,
                client,
                agent_id,
                phone_number_id,
                recipients,
                name,
                scheduled_time_unix,
            )
            .await
        }
        PhoneBatchAction::List {
            page_size,
            cursor,
            status,
            agent_id,
        } => batch::list::run(ctx, client, page_size, cursor, status, agent_id).await,
        PhoneBatchAction::Show { batch_id } => batch::show::run(ctx, client, &batch_id).await,
        PhoneBatchAction::Cancel { batch_id } => batch::cancel::run(ctx, client, &batch_id).await,
        PhoneBatchAction::Retry { batch_id } => batch::retry::run(ctx, client, &batch_id).await,
        PhoneBatchAction::Delete { batch_id, yes } => {
            batch::delete::run(ctx, client, &batch_id, yes).await
        }
    }
}

async fn dispatch_whatsapp(
    ctx: Ctx,
    client: &ElevenLabsClient,
    action: PhoneWhatsappAction,
) -> Result<(), AppError> {
    match action {
        PhoneWhatsappAction::Call {
            agent_id,
            whatsapp_phone_number_id,
            whatsapp_user_id,
            permission_template_name,
            permission_template_language_code,
        } => {
            whatsapp::call::run(
                ctx,
                client,
                agent_id,
                whatsapp_phone_number_id,
                whatsapp_user_id,
                permission_template_name,
                permission_template_language_code,
            )
            .await
        }
        PhoneWhatsappAction::Message {
            agent_id,
            whatsapp_phone_number_id,
            whatsapp_user_id,
            template_name,
            template_language_code,
            template_params,
            client_data,
        } => {
            whatsapp::message::run(
                ctx,
                client,
                agent_id,
                whatsapp_phone_number_id,
                whatsapp_user_id,
                template_name,
                template_language_code,
                template_params,
                client_data,
            )
            .await
        }
        PhoneWhatsappAction::Accounts { action } => {
            dispatch_whatsapp_accounts(ctx, client, action).await
        }
    }
}

async fn dispatch_whatsapp_accounts(
    ctx: Ctx,
    client: &ElevenLabsClient,
    action: PhoneWhatsappAccountsAction,
) -> Result<(), AppError> {
    match action {
        PhoneWhatsappAccountsAction::List => whatsapp::accounts::list::run(ctx, client).await,
        PhoneWhatsappAccountsAction::Show { account_id } => {
            whatsapp::accounts::show::run(ctx, client, &account_id).await
        }
        PhoneWhatsappAccountsAction::Update { account_id, patch } => {
            whatsapp::accounts::update::run(ctx, client, &account_id, patch).await
        }
        PhoneWhatsappAccountsAction::Delete { account_id, yes } => {
            whatsapp::accounts::delete::run(ctx, client, &account_id, yes).await
        }
    }
}
