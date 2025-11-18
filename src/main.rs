use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;
use zero2prod_axum::{
    configuration::get_configuration,
    run,
    temeletry::{get_subscriber, init_subscriber},
};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber = get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let db_pool = PgPool::connect(&configuration.database.connection_string().expose_secret())
        .await
        .expect("Failed to connection postgres.");
    let address = format!("127.0.0.1:{}", configuration.application_port);
    println!("Listening on: http://{}", address.clone());
    let listener = TcpListener::bind(address).await?;

    run(listener, db_pool).await?.await?;
    Ok(())
}
