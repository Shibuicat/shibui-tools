use std::sync::Arc;

use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;
use shibui_vocab_data::scraper::WordDefinition;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct WordQuery {
    word: String,
}

pub async fn get_word(
    query: Query<WordQuery>,
    state: Arc<AppState>,
) -> Result<Json<WordDefinition>, (StatusCode, String)> {
    println!("process request for word {}", query.0.word);
    let result = state
        .fetcher
        .fetch(query.0.word)
        .await
        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()));

    match result {
        Ok(Some(word)) => {
            println!("returned {:?}", &word);
            Ok(Json(word))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "Word doesn't exist".to_string())),
        Err(err) => Err(err),
    }
}
