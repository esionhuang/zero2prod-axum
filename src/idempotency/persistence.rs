use std::usize;

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue},
    response::Response,
};
use reqwest::StatusCode;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::idempotency::IdempotencyKey;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(Response),
}

// impl PgHasArrayType for HeaderPairRecord {
//     fn array_type_info() -> sqlx::postgres::PgTypeInfo {
//         sqlx::postgres::PgTypeInfo::with_name("_header_pair")
//     }
// }

/// 获取已保存的幂等性响应数据
pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<Response>, anyhow::Error> {
    let saved_response = sqlx::query!(
        r#"
        SELECT
        response_status_code as "response_status_code!",
        response_headers as "response_headers!: Vec<HeaderPairRecord>",
        response_body as "response_body!"
        FROM idempotency
        WHERE user_id = $1
        AND idempotency_key = $2"#,
        user_id,
        idempotency_key.as_ref()
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = saved_response {
        let status_code = StatusCode::from_u16(r.response_status_code.try_into()?)?;

        let mut response = Response::builder()
            .status(status_code)
            .body(axum::body::Body::from(r.response_body))
            .map_err(|e| anyhow::anyhow!(e))?;

        for HeaderPairRecord { name, value } in r.response_headers {
            let hader_name =
                HeaderName::from_bytes(name.as_bytes()).map_err(|e| anyhow::anyhow!(e))?;
            let header_value = HeaderValue::from_bytes(&value).map_err(|e| anyhow::anyhow!(e))?;
            response.headers_mut().append(hader_name, header_value);
        }

        Ok(Some(response))
    } else {
        Ok(None)
    }
}

/// 保存幂等性响应数据
pub async fn save_response(
    mut transaction: Transaction<'static, Postgres>,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    http_response: axum::response::Response,
) -> Result<Response, anyhow::Error> {
    let (parts, body) = http_response.into_parts();
    let status_code = parts.status.as_u16() as i16;
    let headers = {
        let mut h = Vec::with_capacity(parts.headers.len());
        for (name, value) in parts.headers.iter() {
            let name = name.as_str().to_owned();
            let value = value.as_bytes().to_owned();
            h.push(HeaderPairRecord { name, value });
        }
        h
    };

    let body_bytes = axum::body::to_bytes(body, usize::MAX).await?.to_vec();

    // Todo 保存 response 到数据库
    transaction
        .execute(sqlx::query_unchecked!(
            r#"
        UPDATE idempotency
        SET 
            response_status_code = $3,
            response_headers     = $4,
            response_body        = $5
        WHERE user_id = $1
        AND idempotency_key = $2"#,
            user_id,
            idempotency_key.as_ref(),
            status_code,
            headers,
            body_bytes
        ))
        .await?;
    transaction.commit().await?;

    let new_body = Body::from(body_bytes);

    Ok(Response::from_parts(parts, new_body))
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    let mut transaction = pool.begin().await?;

    let query = sqlx::query!(
        r#"
    INSERT INTO idempotency ( 
        user_id, 
        idempotency_key, 
        create_at
    )
    VALUES ($1, $2, now())
    ON CONFLICT DO NOTHING
    "#,
        user_id,
        idempotency_key.as_ref()
    );

    let n_inserted_rows = transaction.execute(query).await?.rows_affected();

    if n_inserted_rows > 0 {
        Ok(NextAction::StartProcessing(transaction))
    } else {
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("We expected a saved response, we didn't find it"))?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
