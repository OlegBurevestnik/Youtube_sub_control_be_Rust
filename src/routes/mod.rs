use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers::{get_hello, create_user};

pub fn api_routes() -> Router {
    Router::new()
        .route("/", get(get_hello))
        .route("/users", post(create_user))
}
