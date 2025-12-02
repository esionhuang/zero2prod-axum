use anyhow::Context;

use axum::{
    Extension, Form,
    extract::State,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use sqlx::PgPool;

use crate::{
    AppState, UserId,
    domain::SubscriberEmail,
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

struct ConfirmedSubscriber {
    email: SubscriberEmail,
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

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(state,  form),
    fields(username = tracing::field::Empty,user_id = tracing::field::Empty)
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

    let transaction = match try_processing(&state.db_pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(response) => {
            success_message(flash.clone());
            return Ok(response);
        }
    };

    let subscribers = get_confirmed_subscriber(&state.db_pool)
        .await
        .map_err(e500)?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => {
                state
                    .email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(e500)?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid"
                );
            }
        }
    }

    success_message(flash.clone());
    let response = Redirect::to("/admin/newsletters").into_response();
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;

    Ok(response)
}

/// 获取所有确认的订阅者
#[tracing::instrument(name = "Get confirmed subscriptions", skip(pool))]
async fn get_confirmed_subscriber(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email FROM subscriptions WHERE status = 'confirmed'
        "#
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}

fn success_message(flash: Messages) {
    flash.info("The newsletter issue has been published!");
}
