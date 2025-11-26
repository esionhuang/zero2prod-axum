use axum::{
    extract::{Query, State},
    response::Html,
};
use handlebars::Handlebars;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;

use crate::{AppState, HmacSecret};

#[derive(serde::Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));

        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;

        Ok(self.error)
    }
}

#[tracing::instrument(name = "Get login html", skip(state, query))]
pub async fn login_form(
    State(state): State<AppState>,
    query: Result<Query<QueryParams>, axum::extract::rejection::QueryRejection>,
) -> (axum::http::StatusCode, Html<String>) {
    // query 由上一次登录失败后的重定向时(/login post)提供,首次登录时该值为 None
    let error_html = match query {
        Err(_) => "".into(),
        // 如果提取到 query 参数,则验证该参数(防止XSS攻击)
        Ok(Query(params)) => match params.verify(&state.secret) {
            Ok(error) => format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error)),
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = ?e,
                    "Failed to veriry query parameters using the HMAC tag"
                );
                "".into()
            }
        },
    };

    let html_template = include_str!("login.html");
    let reg = Handlebars::new();
    let login_form = reg
        .render_template(html_template, &serde_json::json!({"error_html":error_html}))
        .expect("Failed to render login form.");

    (axum::http::StatusCode::OK, Html::from(login_form))
}
