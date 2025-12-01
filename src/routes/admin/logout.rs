use axum::response::{IntoResponse, Redirect, Response};
use axum_messages::Messages;

use crate::{TypedSession, utils::e500};

pub async fn log_out(session: TypedSession, flash: Messages) -> Result<Response, Response> {
    session.log_out().await.map_err(e500)?;
    flash.info("You have successfully logged out.");
    Ok(Redirect::to("/login").into_response())
}
