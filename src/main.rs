use tokio::net::TcpListener;
use zero2prod_axum::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("0.0.0.0:13000").await?;

    run(listener).await?.await?;

    Ok(())
}
