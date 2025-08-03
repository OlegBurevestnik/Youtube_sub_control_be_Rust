pub mod auth_routes;
pub mod api_routes;

use crate::{
    routes::{
        auth_routes::{auth_callback_handler, auth_start_handler},
        api_routes::get_subscriptions,
    },
    state::AppState,
};
use axum::{routing::get, Router};

pub fn api_routes(app_state: AppState) -> Router {
    Router::new()
        .route("/api/auth/start", get(auth_start_handler))
        .route("/api/auth/callback", get(auth_callback_handler))
        .route("/api/subscriptions", get(get_subscriptions))
        .with_state(app_state)
}
