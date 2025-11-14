use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{IntoMakeService, get},
};

use tokio::net::TcpListener;

async fn greet(Path(name): Path<String>) -> impl IntoResponse {
    format!("Hello {}!", name)
}

async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

pub async fn run(
    listener: TcpListener,
) -> Result<axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>, std::io::Error> {
    // let listener = TcpListener::bind("0.0.0.0:13000").await?;

    let app = Router::new()
        .route("/{name}", get(greet))
        .route("/health_check", get(health_check));

    Ok(axum::serve(listener, app.into_make_service()))
}
