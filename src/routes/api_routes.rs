use axum::{extract::State, Json};
use axum_extra::extract::cookie::CookieJar;
use crate::state::AppState;
use reqwest::Client;
use serde_json::Value;

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
