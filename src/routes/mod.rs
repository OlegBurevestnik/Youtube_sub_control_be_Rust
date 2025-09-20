pub mod auth_routes;
pub mod api_routes;

use crate::{
    routes::{
        auth_routes::{auth_callback_handler, auth_start_handler},
        api_routes::get_subscriptions,
        api_routes::unsubscribe_handler,
    },
    state::AppState,
};
use axum::{routing::get, Router};
use axum::routing::post;
use crate::routes::api_routes::refresh_subs_cache_handler;

pub fn api_routes(app_state: AppState) -> Router {
    Router::new()
        .route("/api/auth/start", get(auth_start_handler))
        .route("/api/auth/callback", get(auth_callback_handler))
        .route("/api/subscriptions", get(get_subscriptions))
        .route("/api/unsubscribe", post(unsubscribe_handler))
        .route("/api/subscriptions/refresh", post(refresh_subs_cache_handler)) // ← новый
        .with_state(app_state)
}
