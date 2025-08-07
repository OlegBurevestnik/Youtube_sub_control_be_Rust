use axum::{
    response::IntoResponse,
    http::StatusCode,
};

use axum::{extract::{Query, State}, Json};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use crate::state::AppState;
use reqwest::Client;
use serde_json::Value;
use serde_json::json;

#[derive(Deserialize)]
pub struct SubscriptionsQuery {
    query: Option<String>,
    page: Option<usize>,  // номер страницы
    limit: Option<usize>, // сколько на страницу
}

pub async fn get_subscriptions(
    State(_state): State<AppState>,
    Query(params): Query<SubscriptionsQuery>,
    jar: CookieJar,
) -> Json<Value> {
    let Some(token) = jar.get("access_token").map(|c| c.value().to_string()) else {
        return Json(json!({ "error": "Missing token" }));
    };

    let client = reqwest::Client::new();

    let mut all_items = vec![];
    let mut page_token: Option<String> = None;

    loop {
        let mut req = client
            .get("https://www.googleapis.com/youtube/v3/subscriptions")
            .bearer_auth(&token)
            .query(&[
                ("part", "snippet"),
                ("mine", "true"),
                ("maxResults", "50"),
            ]);

        if let Some(pt) = &page_token {
            req = req.query(&[("pageToken", pt)]);
        }

        let res = req.send().await.unwrap();
        let json: Value = res.json().await.unwrap();

        if let Some(items) = json.get("items").and_then(|v| v.as_array()) {
            all_items.extend(items.clone());
        }

        page_token = json
            .get("nextPageToken")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if page_token.is_none() {
            break;
        }

    }

    // 🔍 Фильтрация по query
    let query_lc = params.query.unwrap_or_default().to_lowercase();
    let filtered_items: Vec<_> = all_items
        .into_iter()
        .filter(|item| {
            let title = item["snippet"]["title"].as_str().unwrap_or("").to_lowercase();
            let channel = item["snippet"]["channelTitle"].as_str().unwrap_or("").to_lowercase();
            title.contains(&query_lc) || channel.contains(&query_lc)
        })
        .collect();

    let total_results = filtered_items.len();

    Json(json!({
        "items": filtered_items,
        "totalResults": total_results
    }));

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(25);
    let start = (page - 1) * limit;
    let end = start + limit;

    let paginated_items = filtered_items
        .into_iter()
        .skip(start)
        .take(limit)
        .collect::<Vec<_>>();

    let total = paginated_items.len(); // или total_filtered_items

    Json(json!({
        "items": paginated_items,
        "totalResults": total_results,
        "page": page,
        "limit": limit
    }))
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
