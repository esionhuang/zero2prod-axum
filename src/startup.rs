use axum::{
    Router,
    routing::{IntoMakeService, get, post},
};

use sqlx::PgPool;
use tokio::net::TcpListener;

use crate::routes::{health_check, subscribe};

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
) -> Result<axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(db_pool);

    Ok(axum::serve(listener, app.into_make_service()))
}
