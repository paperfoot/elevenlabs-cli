//! voices: list / show / search / library / clone / design / save-preview /
//! delete / add-shared / similar / edit.
//!
//! Each subcommand is its own submodule. Shared name→id resolver lives in
//! `resolve.rs` and is re-exported for other command trees (tts, audio) that
//! currently duplicate the same client-side match strategy.

use crate::cli::VoicesAction;
use crate::client::ElevenLabsClient;
use crate::config;
use crate::error::AppError;
use crate::output::Ctx;

pub mod add_shared;
pub mod clone;
pub mod delete;
pub mod design;
pub mod edit;
pub mod library;
pub mod list;
pub mod resolve;
pub mod save_preview;
pub mod search;
pub mod show;
pub mod similar;

// Other command trees (tts, audio) can migrate to this resolver instead of
// duplicating the exact>prefix>substring logic. See NOTES.md for the lead
// follow-up.
#[allow(unused_imports)]
pub use resolve::resolve_voice_id_by_name;

pub async fn dispatch(ctx: Ctx, action: VoicesAction) -> Result<(), AppError> {
    let cfg = config::load()?;
    let client = ElevenLabsClient::from_config(&cfg)?;

    match action {
        VoicesAction::List {
            search,
            sort,
            direction,
            limit,
            next_page_token,
            voice_type,
            category,
            fine_tuning_state,
            collection_id,
            include_total_count,
            voice_id,
        } => {
            list::run(
                ctx,
                &client,
                list::ListArgs {
                    search,
                    sort,
                    direction,
                    limit,
                    next_page_token,
                    voice_type,
                    category,
                    fine_tuning_state,
                    collection_id,
                    include_total_count,
                    voice_ids: voice_id,
                },
            )
            .await
        }
        VoicesAction::Show { voice_id } => show::run(ctx, &client, &voice_id).await,
        VoicesAction::Search { query } => search::run(ctx, &client, query).await,
        VoicesAction::Library {
            search,
            page,
            page_size,
            category,
            gender,
            age,
            accent,
            language,
            locale,
            use_case,
            featured,
            min_notice_days,
            include_custom_rates,
            include_live_moderated,
            reader_app_enabled,
            owner_id,
            sort,
        } => {
            library::run(
                ctx,
                &client,
                library::LibraryArgs {
                    search,
                    page,
                    page_size,
                    category,
                    gender,
                    age,
                    accent,
                    language,
                    locale,
                    use_case,
                    featured,
                    min_notice_days,
                    include_custom_rates,
                    include_live_moderated,
                    reader_app_enabled,
                    owner_id,
                    sort,
                },
            )
            .await
        }
        VoicesAction::Clone {
            name,
            files,
            description,
        } => clone::run(ctx, &client, name, files, description).await,
        VoicesAction::Design {
            description,
            text,
            output_dir,
            model,
            loudness,
            seed,
            guidance_scale,
            enhance,
            stream_previews,
            quality,
        } => {
            design::run(
                ctx,
                &client,
                design::DesignArgs {
                    description,
                    text,
                    output_dir,
                    model,
                    loudness,
                    seed,
                    guidance_scale,
                    enhance,
                    stream_previews,
                    quality,
                },
            )
            .await
        }
        VoicesAction::SavePreview {
            generated_voice_id,
            name,
            description,
        } => save_preview::run(ctx, &client, generated_voice_id, name, description).await,
        VoicesAction::Delete { voice_id, yes } => delete::run(ctx, &client, &voice_id, yes).await,
        VoicesAction::AddShared {
            public_user_id,
            voice_id,
            name,
            bookmarked,
        } => add_shared::run(ctx, &client, &public_user_id, &voice_id, name, bookmarked).await,
        VoicesAction::Similar {
            audio_file,
            similarity_threshold,
            top_k,
        } => {
            similar::run(
                ctx,
                &client,
                similar::SimilarArgs {
                    audio_file,
                    similarity_threshold,
                    top_k,
                },
            )
            .await
        }
        VoicesAction::Edit {
            voice_id,
            name,
            description,
            labels,
            add_sample,
            remove_sample,
            remove_background_noise,
        } => {
            edit::run(
                ctx,
                &client,
                edit::EditArgs {
                    voice_id,
                    name,
                    description,
                    labels,
                    add_sample,
                    remove_sample,
                    remove_background_noise,
                },
            )
            .await
        }
    }
}
