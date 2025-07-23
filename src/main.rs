use anyhow::Result;
use axum::serve;
use axum::Router;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod auth;
mod routes;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let app_state = state::AppState::new();
    let app: Router = routes::api_routes(app_state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    let listener = TcpListener::bind(addr).await?;
    println!("🚀 Server running at http://{}", addr);

    serve(listener, app).await?;
    Ok(())
}
