use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum_messages::Messages;
use secrecy::{ExposeSecret, SecretString};

use crate::{
    AppState, AuthError, Credentials, TypedSession, routes::get_username, utils::e500,
    validate_credentials,
};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

pub async fn change_password(
    State(state): State<AppState>,
    flash: Messages,
    session: TypedSession,
    Form(form): Form<FormData>,
) -> Result<Response, Response> {
    // 验证新密码与确认密码是否相同
    if form.new_password.expose_secret() != form.new_password_check.expose_secret() {
        flash.error("You entered two different new passwords - the field values must match.");
        return Ok(Redirect::to("/admin/password").into_response());
    }

    // 新密码长度必须大于或等于8位
    if form.new_password.expose_secret().len() < 8 {
        flash.error("You password is too short!");
        return Ok(Redirect::to("/admin/password").into_response());
    }

    // 如果用户未登录,重定向到登录
    let user_id = session.get_user_id().await.map_err(e500)?;
    if user_id.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }
    let user_id = user_id.unwrap();
    let username = get_username(user_id, &state.db_pool).await.map_err(e500)?;

    // 验证用户名和密码是否正确
    let credentials = Credentials {
        username,
        password: form.current_password,
    };

    if let Err(e) = validate_credentials(credentials, &state.db_pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                flash.error("The current password is incorrect.");
                Ok(Redirect::to("/admin/password").into_response())
            }
            AuthError::UnexpectedError(_) => Err(e500(e).into()),
        };
    }

    crate::authentication::change_password(user_id, form.new_password, &state.db_pool)
        .await
        .map_err(e500)?;
    flash.error("Your password has been changed.");

    Ok(Redirect::to("/admin/password").into_response())
}
