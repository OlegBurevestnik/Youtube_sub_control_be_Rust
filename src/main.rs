use axum::{Router, routing::get, serve};
use tokio::net::TcpListener;
use std::net::SocketAddr;

mod handlers;
mod routes;
mod models;

use routes::api_routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app: Router = api_routes();

    // 🎯 Собираем TcpListener вместо SocketAddr
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    println!("🚀 Server running at http://{}", addr);

    // ✅ Передаём listener и app
    serve(listener, app).await?;
    Ok(())
}
