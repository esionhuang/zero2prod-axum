use axum::{
    extract::{Query, State},
    http::status::StatusCode,
    response::IntoResponse,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppState;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, state))]
pub async fn confirm(
    Query(parameters): Query<Parameters>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let id =
        match get_subscriber_id_from_token(&state.db_pool, &parameters.subscription_token).await {
            Ok(id) => id,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        };

    match id {
        Some(subscriber_id) => {
            if confirm_subscriber(&state.db_pool, subscriber_id)
                .await
                .is_err()
            {
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
            StatusCode::OK
        }
        None => StatusCode::UNAUTHORIZED,
    }
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(subscription_token, pool))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Make subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(pool)
    .await?;

    Ok(())
}
