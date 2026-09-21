//! voices search — convenience wrapper around `list`.

use super::list::{self, ListArgs};
use crate::client::ElevenLabsClient;
use crate::error::AppError;
use crate::output::Ctx;

pub async fn run(ctx: Ctx, client: &ElevenLabsClient, query: String) -> Result<(), AppError> {
    list::run(
        ctx,
        client,
        ListArgs {
            search: Some(query),
            sort: "name".into(),
            direction: "asc".into(),
            limit: 50,
            next_page_token: None,
            voice_type: None,
            category: None,
            fine_tuning_state: None,
            collection_id: None,
            include_total_count: false,
            voice_ids: Vec::new(),
        },
    )
    .await
}
