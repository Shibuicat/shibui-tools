use axum::{routing::get, Router};
mod routes;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(routes::get::get_word));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
