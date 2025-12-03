use anyhow::Context;

use axum::{
    Extension, Form,
    extract::State,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    AppState, UserId,
    idempotency::{IdempotencyKey, NextAction, save_response, try_processing},
    routes::error_chain_fmt,
    utils::{e400, e500},
};

#[derive(Debug, serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for PublishError {
    fn into_response(self) -> axum::response::Response {
        match self {
            PublishError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            PublishError::AuthError(_) => {
                let mut response = StatusCode::UNAUTHORIZED.into_response();
                let healder_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

                response
                    .headers_mut()
                    .insert("WWW-Authenticate", healder_value);

                response
            }
        }
    }
}

fn success_message(flash: Messages) {
    flash.info(
        "The newsletter issue has been accepted - \
    emails will go out shortly.",
    );
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip_all,
    fields(user_id=%&*user_id)
)]
pub async fn publish_newsletter(
    State(state): State<AppState>,
    flash: Messages,
    Extension(user_id): Extension<UserId>,
    Form(form): Form<FormData>,
) -> anyhow::Result<Response, Response> {
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form;

    let idempotency_key: IdempotencyKey = idempotency_key.try_into().map_err(e400)?;

    let mut transaction = match try_processing(&state.db_pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(saved_response) => {
            success_message(flash.clone());
            return Ok(saved_response);
        }
    };

    let issue_id = insert_newsletter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("Failed to store newsletter issue details")
        .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    let response = Redirect::to("/admin/newsletters").into_response();
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;

    success_message(flash.clone());

    Ok(response)
}

/// 新增新闻发布记录
#[tracing::instrument(name = "Add new newsletter issue", skip_all)]
async fn insert_newsletter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
    INSERT INTO newsletter_issues(
        newsletter_issue_id, 
        title, 
        text_content, 
        html_content, 
        published_at
    )
    VALUES ($1, $2, $3, $4, NOW())"#,
        newsletter_issue_id,
        title,
        text_content,
        html_content
    );

    transaction.execute(query).await?;

    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newslettter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
    INSERT INTO issue_delivery_queue(
    newsletter_issue_id,
    subscriber_email
    )
    SELECT $1, email
    FROM subscriptions
    WHERE status = 'confirmed'"#,
        newslettter_issue_id
    );
    transaction.execute(query).await?;
    Ok(())
}
