use std::sync::Arc;

use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;
use shibui_vocab_data::lookup_queue::LookupError;
use shibui_vocab_data::scraper::WordDefinition;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct WordQuery {
    word: String,
}

fn status_of(error: &LookupError) -> StatusCode {
    match error {
        LookupError::Busy | LookupError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        LookupError::TimedOut => StatusCode::GATEWAY_TIMEOUT,
        LookupError::Failed(_) => StatusCode::BAD_GATEWAY,
    }
}

pub async fn get_word(
    query: Query<WordQuery>,
    state: Arc<AppState>,
) -> Result<Json<WordDefinition>, (StatusCode, String)> {
    println!("process request for word {}", query.0.word);
    let result = state
        .lookup
        .lookup(&query.0.word)
        .await
        .map_err(|err| (status_of(&err), err.to_string()))?;

    match result {
        Some(word) => {
            println!("returned {:?}", &word);
            Ok(Json(word))
        }
        None => Err((StatusCode::NOT_FOUND, "Word doesn't exist".to_string())),
    }
}
