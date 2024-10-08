use std::sync::Arc;

use axum::{extract::Query, Json};
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
) -> Result<Json<WordDefinition>, String> {
    println!("process request for word {}", query.0.word);
    let result = state
        .fetcher
        .fetch(query.0.word)
        .await
        .map_err(|err| err.to_string());

    if let Ok(word) = result {
        if word.is_some() {
            let word = word.unwrap();
            println!("returned {:?}", &word);
            return Ok(Json(word));
        }
        return Err("Word doesn't exist".to_string());
    }

    Err(result.err().unwrap())
}
