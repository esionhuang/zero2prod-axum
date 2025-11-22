#![allow(dead_code, unused_variables)]
use axum::{Form, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use rand::{Rng, distr::Alphanumeric, rng};
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{NewSubscriber, SubscriberEmail, SubscriberName},
    email_client::EmailClient,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Add a new subscriber",
    skip(state, form),
    fields(
        subscriber_name = %form.name,
        subscriber_email = %form.email
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    let new_subscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let mut transcation = match state.db_pool.begin().await {
        Ok(transcation) => transcation,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let subscriber_id = match insert_subscriber(&mut transcation, &new_subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let subscription_token = generate_subscription_token();

    if store_token(&mut transcation, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if transcation.commit().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    if send_confirmation_email(
        &state.email_client,
        new_subscriber,
        &state.base_url.0,
        &subscription_token,
    )
    .await
    .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

/// 向数据库写入订阅者数据,并返回新订阅者的 id
#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(transcation, new_subscriber)
)]
pub async fn insert_subscriber(
    transcation: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    let query = sqlx::query!(
        r#"
        INSERT INTO subscriptions (id, email, name, subscribed_at, status)
        VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    );

    transcation.execute(query).await?;

    Ok(subscriber_id)
}

/// 发送订阅邮件
/// # 参数
/// email_client -
#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />\
     Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );

    email_client
        .send_email(new_subscriber.email, "Welecome!", &html_body, &plain_body)
        .await
}

/// 生成一个随机的25位长度的区分大小写的订阅令牌
fn generate_subscription_token() -> String {
    let mut rng = rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

/// 存储订阅停息到数据库
#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transcation)
)]
async fn store_token(
    transcation: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );

    transcation.execute(query).await?;
    Ok(())
}
