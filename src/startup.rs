use axum::{
    Router,
    routing::{IntoMakeService, get, post},
};

use axum_messages::MessagesManagerLayer;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::Duration;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_redis_store::{
    RedisStore,
    fred::prelude::{ClientLike, Config, Pool},
};
use tracing::info_span;
use uuid::Uuid;

use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    reject_anonymous_user,
    routes::{
        admin::password::{change_password, change_password_form},
        admin_dashboard, confirm, health_check,
        home::{
            self,
            login::{login, login_form},
        },
        log_out,
        newsletter::{publish_newsletter, publish_newsletter_form},
        subscribe,
    },
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

#[derive(Clone)]
pub struct HmacSecret(pub SecretString);

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
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
            configuration.application.redis_uri,
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
    redis_uri: SecretString,
) -> Result<axum::serve::Serve<TcpListener, IntoMakeService<Router>, Router>, anyhow::Error> {
    let base_url = ApplicationBaseUrl(base_url);
    let app_state = AppState {
        email_client,
        db_pool,
        base_url,
    };

    let redis_pool = Pool::new(
        Config::from_url(redis_uri.expose_secret())?,
        None,
        None,
        None,
        6,
    )
    .unwrap();

    let _redis_conn = redis_pool.connect();
    redis_pool.wait_for_connect().await?;
    let redis_store = RedisStore::new(redis_pool);
    let session_layer = SessionManagerLayer::new(redis_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));

    let app = Router::new()
        .route("/", get(home::home))
        .route("/login", get(login_form))
        .route("/login", post(login))
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .nest(
            "/admin",
            Router::new()
                .route("/dashboard", get(admin_dashboard))
                .route("/password", get(change_password_form))
                .route("/password", post(change_password))
                .route("/logout", post(log_out))
                .route("/newsletters", post(publish_newsletter))
                .route("/newsletters", get(publish_newsletter_form))
                .layer(axum::middleware::from_fn(reject_anonymous_user)),
        )
        .with_state(app_state.clone())
        .layer(MessagesManagerLayer)
        .layer(session_layer)
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

    let server = axum::serve(listener, app.into_make_service());
    Ok(server)
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(configuration.with_db())
}
