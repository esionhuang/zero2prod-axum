use sqlx::PgPool;
use tokio::net::TcpListener;
use zero2prod_axum::{configuration::get_configuration, run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let configuration = get_configuration().expect("Failed to read configuration.");
    let db_pool = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connection postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).await?;

    run(listener, db_pool).await?.await?;
    Ok(())
}
