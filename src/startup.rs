use axum::{
    Router,
    routing::{IntoMakeService, get, post},
};

use tokio::net::TcpListener;

use crate::routes::{health_check, subscribe};

pub async fn run(
    listener: TcpListener,
) -> Result<axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe));

    Ok(axum::serve(listener, app.into_make_service()))
}
