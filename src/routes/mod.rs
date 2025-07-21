pub mod auth_routes;

use crate::routes::auth_routes::{auth_callback_handler, auth_url_handler};
use axum::{
    routing::get,
    Router,
};

pub fn api_routes() -> Router {
    Router::new()
        .route("/auth-url", get(auth_url_handler))
        .route("/auth/callback", get(auth_callback_handler))
}
