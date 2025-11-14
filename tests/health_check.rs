use tokio::net::TcpListener;
use zero2prod_axum::run;

async fn spawn_app() -> String {
    let listener = TcpListener::bind("0.0.0.0:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let serve = run(listener).await.expect("Failed to bind address");
    let _ = tokio::spawn(serve.into_future());

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_work() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
