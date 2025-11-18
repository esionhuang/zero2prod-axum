use axum::{http::StatusCode, response::IntoResponse};

#[tracing::instrument(name = "Health check")]
pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
