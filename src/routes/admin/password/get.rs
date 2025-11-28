use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_messages::Messages;
use handlebars::Handlebars;
use std::fmt::Write;

use crate::{TypedSession, utils::e500};

pub async fn change_password_form(
    flash: Messages,
    session: TypedSession,
) -> Result<Response, Response> {
    if session.get_user_id().await.map_err(e500)?.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }

    let mut error_html = String::new();
    for m in flash.into_iter() {
        writeln!(error_html, "<p><i>{}</i></p>", m.message).map_err(e500)?;
    }

    let html = Handlebars::new()
        .render_template(
            include_str!("get.html"),
            &serde_json::json!({"error_html":error_html}),
        )
        .map_err(e500);

    Ok(Html::from(html).into_response())
}
