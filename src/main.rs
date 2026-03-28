use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new().route(
        "/",
        get(|| async { "rustdesk-server-admin — dashboard stub (OK)\n" }),
    );

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3030".into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("bind {addr}: {e}"));

    eprintln!("listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}
