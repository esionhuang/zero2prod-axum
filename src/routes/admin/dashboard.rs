use anyhow::Context;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
};
use handlebars::Handlebars;
use reqwest::StatusCode;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AppState, TypedSession};

pub fn e500<T>(e: T) -> axum::response::Response
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

pub async fn admin_dashboard(
    State(state): State<AppState>,
    session: TypedSession,
) -> Result<axum::response::Response, axum::response::Response> {
    let username = if let Some(user_id) = session.get_user_id().await.map_err(e500)? {
        tracing::info!("User id is:{}", user_id.clone());
        get_username(user_id, &state.db_pool).await.map_err(e500)?
    } else {
        return Ok(Redirect::to("/login").into_response());
    };

    let html = Handlebars::new()
        .render_template(
            include_str!("dashboard.html"),
            &serde_json::json!({"username":username}),
        )
        .map_err(e500)?;

    Ok((StatusCode::OK, Html::from(html)).into_response())
}

#[tracing::instrument(name = "Get username", skip(pool))]
async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(r#"SELECT username FROM users WHERE user_id = $1"#, user_id)
        .fetch_one(pool)
        .await
        .context("Failed to perform a query to retrieve a username.")?;

    Ok(row.username)
}
