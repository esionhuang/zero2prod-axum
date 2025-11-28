use axum::extract::FromRequestParts;
use tower_sessions::Session;
use uuid::Uuid;

type SessionError = tower_sessions::session::Error;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    /// 插入 user id session
    pub async fn insert_user_id(&self, user_id: Uuid) -> Result<(), SessionError> {
        self.0.insert(Self::USER_ID_KEY, user_id).await
    }

    /// 获取 user id session
    pub async fn get_user_id(&self) -> Result<Option<Uuid>, SessionError> {
        self.0.get::<Uuid>(Self::USER_ID_KEY).await
    }

    /// 循环id
    pub async fn cycle_id(&self) -> Result<(), SessionError> {
        self.0.cycle_id().await
    }
}

impl<S> FromRequestParts<S> for TypedSession
where
    S: Send + Sync,
{
    type Rejection = (axum::http::StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state).await?;
        Ok(Self(session))
    }
}
