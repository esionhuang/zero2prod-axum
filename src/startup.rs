use axum::{
    Router,
    routing::{IntoMakeService, get, post},
};

use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use uuid::Uuid;

use crate::{
    email_client::EmailClient,
    routes::{health_check, subscribe},
};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: EmailClient,
}

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>, std::io::Error> {
    let app_state = AppState {
        email_client,
        db_pool,
    };

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(app_state.clone())
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                let request_id = Uuid::new_v4();
                info_span!(
                    "request",
                    request_id = %request_id,
                    method = %request.method(),
                    uri = %request.uri()
                )
            }),
        );
    // .layer(axum_messages::MessagesManagerLayer)
    // .with_state(db_pool);

    Ok(axum::serve(listener, app.into_make_service()))
}
