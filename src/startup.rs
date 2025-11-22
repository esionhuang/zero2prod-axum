use axum::{
    Router,
    routing::{IntoMakeService, get, post},
};

use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info_span;
use uuid::Uuid;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{confirm, health_check, subscribe},
};

#[derive(Clone)]
pub struct ApplicationBaseUrl(pub String);

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: EmailClient,
    pub base_url: ApplicationBaseUrl,
}

type Server = axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let sender_email = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = configuration.email_client.timeout();
        let email_client = EmailClient::new(
            configuration.email_client.base_url,
            sender_email,
            configuration.email_client.authorization_token,
            timeout,
        );

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address).await?;
        let port = listener.local_addr().unwrap().port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
        )
        .await?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>, std::io::Error> {
    let base_url = ApplicationBaseUrl(base_url);
    let app_state = AppState {
        email_client,
        db_pool,
        base_url,
    };

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .with_state(app_state.clone())
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                let request_id = Uuid::new_v4();
                info_span!(
                    "request",
                    request_id = %request_id,
                    method = %request.method(),
                    uri = %request.uri()
                )
            }),
        );
    // .layer(axum_messages::MessagesManagerLayer)
    // .with_state(db_pool);

    Ok(axum::serve(listener, app.into_make_service()))
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}
