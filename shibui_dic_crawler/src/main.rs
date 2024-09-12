use axum::{routing::get, Router};
use shibui_vocab_data::{
    scraper::CambridgeDictionaryScraper,
    utils::{
        html_parser::cambridge_parser::CambridgeHtmlParser, http_request::DefaultHttpRequestMaker,
    },
};
mod routes;

#[tokio::main]
async fn main() {
    let cambridge =
        CambridgeDictionaryScraper::new(DefaultHttpRequestMaker::new(), CambridgeHtmlParser);
    let app = Router::new().route("/", get(routes::cambridge_dic::get_word));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
