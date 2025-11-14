use axum::{
    Form, Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{IntoMakeService, get, post},
};

use tokio::net::TcpListener;

#[derive(serde::Deserialize)]
struct FormData {
    name: String,
    email: String,
}

async fn greet(Path(name): Path<String>) -> impl IntoResponse {
    format!("Hello {}!", name)
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

async fn subscribe(Form(form): Form<FormData>) -> impl IntoResponse {
    StatusCode::OK
}

pub async fn run(
    listener: TcpListener,
) -> Result<axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>, std::io::Error> {
    // let listener = TcpListener::bind("0.0.0.0:13000").await?;

    let app = Router::new()
        .route("/{name}", get(greet))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe));

    Ok(axum::serve(listener, app.into_make_service()))
}
