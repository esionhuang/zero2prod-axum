use axum::response::{Html, IntoResponse, Response};
use axum_messages::Messages;
use handlebars::Handlebars;
use std::fmt::Write;
use uuid::Uuid;

use crate::utils::e500;

#[tracing::instrument(name = "Get newsletter form", skip(flash))]
pub async fn publish_newsletter_form(flash: Messages) -> Result<Response, Response> {
    let idempotency_key = Uuid::new_v4();
    let mut msg_html = String::new();

    for m in flash.into_iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", m.message).map_err(e500)?;
    }

    let html = Handlebars::new()
        .render_template(
            include_str!("get.html"),
            &serde_json::json!({
                "messages":msg_html,
                "idempotency_key":idempotency_key
            }),
        )
        .map_err(e500)?;

    Ok(Html::from(html).into_response())
}
