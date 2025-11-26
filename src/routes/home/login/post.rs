use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use hmac::{Hmac, Mac};
use reqwest::header::LOCATION;
use secrecy::{ExposeSecret, SecretString};

use crate::{AppState, AuthError, Credentials, routes::error_chain_fmt, validate_credentials};

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: SecretString,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        let query_string = format!("{}", urlencoding::Encoded::new(self.to_string()));

        let secret: &[u8] = &Vec::new();
        let hmac_tag = {
            let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret).unwrap();
            mac.update(query_string.as_bytes());
            mac.finalize().into_bytes()
        };

        Response::builder()
            .status(axum::http::StatusCode::SEE_OTHER)
            .header(
                LOCATION,
                format!("/login?{}&tag={:x}", query_string, hmac_tag),
            )
            .body(axum::body::Body::empty())
            .unwrap()
    }
}

#[tracing::instrument(
    name = "", 
    skip(state, form),
    fields(username = tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn login(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };

    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    match validate_credentials(credentials, &state.db_pool).await {
        Ok(user_id) => {
            tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
            Ok((axum::http::StatusCode::SEE_OTHER, Redirect::to("/")).into_response())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(e.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(e.into()),
            };

            let query_string = format!("error={}", urlencoding::Encoded::new(e.to_string()));
            let hmac_tag = {
                let mut mac =
                    Hmac::<sha2::Sha256>::new_from_slice(state.secret.0.expose_secret().as_bytes())
                        .unwrap();
                mac.update(query_string.as_bytes());
                mac.finalize().into_bytes()
            };

            Err(
                Redirect::to(&format!("/login?{}&tag={:x}", &query_string, hmac_tag))
                    .into_response(),
            )
        }
    }
}
