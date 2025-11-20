use sqlx::postgres::PgPoolOptions;
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
    let connection_pool = PgPoolOptions::new().connect_lazy_with(configuration.database.with_db());

    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    println!("Listening on: http://{}", address.clone());
    let listener = TcpListener::bind(address).await?;

    run(listener, connection_pool).await?.await?;
    Ok(())
}
