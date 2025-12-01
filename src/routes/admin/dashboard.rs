use anyhow::Context;
use axum::{
    Extension,
    extract::State,
    response::{Html, IntoResponse},
};
use handlebars::Handlebars;
use reqwest::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AppState, UserId, utils::e500};

pub async fn admin_dashboard(
    State(state): State<AppState>,
    Extension(user_id): Extension<UserId>,
) -> Result<axum::response::Response, axum::response::Response> {
    let username = get_username(*user_id, &state.db_pool).await.map_err(e500)?;

    let html = Handlebars::new()
        .render_template(
            include_str!("dashboard.html"),
            &serde_json::json!({"username":username}),
        )
        .map_err(e500)?;

    Ok((StatusCode::OK, Html::from(html)).into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;

    Ok(row.username)
}
