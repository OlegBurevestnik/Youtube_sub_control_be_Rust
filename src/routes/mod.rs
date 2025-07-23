pub mod auth_routes;
pub mod api_routes;

use crate::{
    routes::{
        auth_routes::{auth_callback_handler, auth_url_handler},
        api_routes::get_subscriptions,
    },
    state::AppState,
};
use axum::{routing::get, Router};

pub fn api_routes(app_state: AppState) -> Router {
    Router::new()
        .route("/auth-url", get(auth_url_handler))
        .route("/auth/callback", get(auth_callback_handler))
        .route("/api/subscriptions", get(get_subscriptions))
        .with_state(app_state)
}
