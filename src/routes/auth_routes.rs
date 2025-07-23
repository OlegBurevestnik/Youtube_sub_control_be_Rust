use axum::{extract::{Query, State}, http::StatusCode, response::IntoResponse};
use std::collections::HashMap;
use crate::{auth::google, state::AppState};

pub async fn auth_url_handler() -> impl IntoResponse {
    google::get_authorization_url().await
}

pub async fn auth_callback_handler(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if let Some(code) = params.get("code") {
        match google::exchange_code(code).await {
            Ok(token) => {
                state.set_token(token.clone());
                format!("✅ Access Token: {}", token).into_response()
            }
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to exchange code").into_response(),
        }
    } else {
        (StatusCode::BAD_REQUEST, "Missing code parameter").into_response()
    }
}




