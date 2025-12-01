use crate::{TypedSession, utils::e500};
use std::ops::Deref;
use uuid::Uuid;

// 身份认证中间件
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};

#[derive(Clone, Copy, Debug)]
pub struct UserId(Uuid);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for UserId {
    type Target = Uuid;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// 拒绝匿名用户
pub async fn reject_anonymous_user(
    session: TypedSession,
    mut req: Request,
    next: Next,
) -> Result<Response, Response> {
    match session.get_user_id().await.map_err(e500)? {
        Some(user_id) => {
            req.extensions_mut().insert(UserId(user_id));
            let response = next.run(req).await;
            Ok(response)
        }
        None => {
            let resp = Redirect::to("/login").into_response();
            // let e = anyhow::anyhow!("The user has not logged in");
            // tracing::error!(error = %e);

            Err(resp)
        }
    }
}
