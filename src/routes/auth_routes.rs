use crate::auth::google::{self, AuthCallback};
use crate::state::AppState;
use axum::{extract::{Query, State}, response::IntoResponse};
use axum_extra::extract::cookie::CookieJar;

pub async fn auth_start_handler() -> impl IntoResponse {
    google::auth_start().await
}

pub async fn auth_callback_handler(
    Query(params): Query<AuthCallback>,
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    google::auth_callback(params, state, jar).await
}
