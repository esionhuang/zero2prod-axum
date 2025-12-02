use axum::response::IntoResponse;

pub fn e500<T>(e: T) -> axum::response::Response
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

pub fn e400<T>(e: T) -> axum::response::Response
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response()
}
