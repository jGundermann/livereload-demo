use std::net::SocketAddr;
use axum::{Router, routing::get, extract::State, response::IntoResponse};
use minijinja::context;
use state::AppState;

mod state;

#[tokio::main]
async fn main() {
    let state = state::AppState::new();
    let app = Router::new().route("/", get(home_handler)).with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn home_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.render("index.jinja", context!())
}