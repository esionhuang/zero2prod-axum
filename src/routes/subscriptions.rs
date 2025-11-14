#![allow(dead_code, unused_variables)]
use axum::{Form, http::StatusCode, response::IntoResponse};

#[derive(serde::Deserialize)]
pub struct FormData {
    name: String,
    email: String,
}

pub async fn subscribe(Form(form): Form<FormData>) -> impl IntoResponse {
    StatusCode::OK
}
