use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use secrecy::SecretString;

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

#[tracing::instrument(
    name = "",
    skip(state, form),
    fields(username = tracing::field::Empty,user_id=tracing::field::Empty)
)]
pub async fn login(
    State(state): State<AppState>,
    flash: Messages,
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
        Err(err) => {
            let error = match err {
                AuthError::InvalidCredentials(_) => LoginError::AuthError(err.into()),
                AuthError::UnexpectedError(_) => LoginError::UnexpectedError(err.into()),
            };

            let _flash = flash.error(error.to_string());

            Err(Redirect::to("/login").into_response())
        }
    }
}
