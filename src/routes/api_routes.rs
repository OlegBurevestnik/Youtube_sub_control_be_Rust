use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use reqwest::Client;
use crate::state::AppState;

pub async fn get_subscriptions(State(state): State<AppState>) -> impl IntoResponse {
    let Some(token) = state.get_token() else {
        return (StatusCode::UNAUTHORIZED, "Missing token").into_response();
    };

    let url = "https://www.googleapis.com/youtube/v3/subscriptions";
    let client = Client::new();

    let res = client
        .get(url)
        .bearer_auth(token)
        .query(&[
            ("part", "snippet"),
            ("mine", "true"),
            ("maxResults", "50"),
        ])
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                let json = response
                    .json::<serde_json::Value>()
                    .await
                    .unwrap_or_else(|_| serde_json::json!({ "error": "Invalid JSON" }));
                Json(json).into_response()
            } else {
                // ❗ Преобразуем reqwest::StatusCode → axum::http::StatusCode
                let status = StatusCode::from_u16(response.status().as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                (status, "YouTube API error").into_response()
            }
        }
        Err(err) => {
            eprintln!("Request error: {:?}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Request failed").into_response()
        }
    }
}
