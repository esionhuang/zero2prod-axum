use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use zero2prod_axum::{
    Application,
    configuration::{DatabaseSettings, get_configuration},
    get_connection_pool,
    temeletry::{get_subscriber, init_subscriber},
};

// 使用 once_cell 的懒加载定义遥测实现,确保在多个测试并发执行时只初始化一次遥测实现
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let configuration = {
        let mut config = get_configuration().expect("Failed to read configuration.");
        config.database.database_name = format!("test_{}", Uuid::new_v4());
        config.application.port = 0;
        // config.email_client.base_url = email_server.uri();
        config
    };

    configure_database(&configuration.database).await;

    let application = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let address = format!("http://127.0.0.1:{}", application.port());
    let _ = tokio::spawn(application.run_until_stopped());

    TestApp {
        address,
        db_pool: get_connection_pool(&configuration.database),
    }
}

/// 配置数据库
///
/// # params
///
/// config <DatabaseSettings> - 数据库设置实例引用
///
async fn configure_database(config: &DatabaseSettings) -> sqlx::Pool<sqlx::Postgres> {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");

    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
