use std::collections::HashMap;
use std::sync::Arc;

use axum::{extract::Query, http::StatusCode, Json};
use serde::Deserialize;
use shibui_vocab_data::lookup_queue::{LookupError, WordLookup};
use shibui_vocab_data::scraper::WordDefinition;

use crate::AppState;

pub const DEFAULT_SOURCE: &str = "cambridge";

#[derive(Debug, Deserialize)]
pub struct WordQuery {
    word: String,
    source: Option<String>,
}

fn status_of(error: &LookupError) -> StatusCode {
    match error {
        LookupError::Busy | LookupError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        LookupError::TimedOut => StatusCode::GATEWAY_TIMEOUT,
        LookupError::Failed(_) => StatusCode::BAD_GATEWAY,
    }
}

pub fn lookup_for<'a>(
    lookups: &'a HashMap<String, Box<dyn WordLookup>>,
    source: Option<&str>,
) -> Result<&'a dyn WordLookup, (StatusCode, String)> {
    let name = source.unwrap_or(DEFAULT_SOURCE).to_lowercase();
    lookups
        .get(&name)
        .map(|lookup| lookup.as_ref())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Unknown source '{name}'")))
}

pub async fn get_word(
    query: Query<WordQuery>,
    state: Arc<AppState>,
) -> Result<Json<WordDefinition>, (StatusCode, String)> {
    println!("process request for word {} from {:?}", query.0.word, query.0.source);
    let lookup = lookup_for(&state.lookups, query.0.source.as_deref())?;
    let result = lookup
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

#[cfg(test)]
mod tests {
    use super::*;
    use shibui_vocab_data::lookup_queue::LookupFuture;

    struct Stub;

    impl WordLookup for Stub {
        fn lookup<'a>(&'a self, _word: &'a str) -> LookupFuture<'a> {
            Box::pin(async { Ok(None) })
        }
    }

    fn lookups() -> HashMap<String, Box<dyn WordLookup>> {
        HashMap::from([
            ("cambridge".to_owned(), Box::new(Stub) as Box<dyn WordLookup>),
            ("wiktionary".to_owned(), Box::new(Stub) as Box<dyn WordLookup>),
        ])
    }

    #[test]
    fn no_source_means_cambridge() {
        assert!(lookup_for(&lookups(), None).is_ok());
    }

    #[test]
    fn a_known_source_is_found_whatever_its_case() {
        assert!(lookup_for(&lookups(), Some("Wiktionary")).is_ok());
    }

    #[test]
    fn an_unknown_source_is_a_bad_request() {
        let error = lookup_for(&lookups(), Some("oxford")).err().unwrap();

        assert_eq!(error.0, StatusCode::BAD_REQUEST);
    }
}
