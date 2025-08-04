use axum::{
    response::IntoResponse,
    http::StatusCode,
};

use axum::{extract::State, Json};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use crate::state::AppState;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

pub async fn get_subscriptions(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Json<Value> {
    let Some(token) = jar.get("access_token").map(|c| c.value().to_string()) else {
        return Json(serde_json::json!({ "error": "No token" }));
    };

    let res = Client::new()
        .get("https://www.googleapis.com/youtube/v3/subscriptions")
        .bearer_auth(token)
        .query(&[
            ("part", "snippet"),
            ("mine", "true"),
            ("maxResults", "50"),
        ])
        .send()
        .await
        .unwrap();

    let data = res.json::<Value>().await.unwrap();
    Json(data)
}

#[derive(Deserialize)]
pub struct UnsubscribeRequest {
    ids: Vec<String>,
}

pub async fn unsubscribe_handler(
    State(_state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<UnsubscribeRequest>,
) -> impl IntoResponse {
    let Some(token) = jar.get("access_token").map(|c| c.value().to_string()) else {
        return (StatusCode::UNAUTHORIZED, "Missing token").into_response();
    };

    let client = reqwest::Client::new();
    let mut success_ids = Vec::new();

    for id in payload.ids {
        let res = client
            .delete("https://www.googleapis.com/youtube/v3/subscriptions")
            .bearer_auth(&token)
            .query(&[("id", &id)])
            .send()
            .await;

        if let Ok(response) = res {
            if response.status().is_success() {
                success_ids.push(id);
            }
        }
    }

    Json(success_ids).into_response()
}
