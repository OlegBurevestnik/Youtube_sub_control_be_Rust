use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::{json, Value};
use crate::state::AppState;
use std::time::Duration;

#[derive(Deserialize, Debug)]
pub struct SubscriptionsQuery {
    query: Option<String>,
    page: Option<usize>,   // 1..N
    limit: Option<usize>,  // по умолчанию 25
    sort: Option<String>,  // "asc" | "desc"
}

const CACHE_TTL: Duration = Duration::from_secs(10 * 60); // 10 минут
const YT_PAGE_SIZE: usize = 50;

pub async fn get_subscriptions(
    State(state): State<AppState>,
    Query(params): Query<SubscriptionsQuery>,
    jar: CookieJar,
) -> impl IntoResponse {
    // 1) Достаём токен из cookie (MVP)
    let Some(token) = jar.get("access_token").map(|c| c.value().to_string()) else {
        return (StatusCode::UNAUTHORIZED, "Missing token").into_response();
    };

    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(25).clamp(1, 100);
    let query_lc = params.query.as_deref().unwrap_or("").trim().to_lowercase();

    // нормализуем порядок сортировки
    let sort_order = match params.sort.as_deref() {
        Some("asc") => "asc",
        _ => "desc", // по умолчанию
    };

    // 🔑 Ключ кэша: пользователь + query + sort
    let cache_key = format!("subs::user={}:q={}:sort={}", token, query_lc, sort_order);

    // 2) Пытаемся достать из кэша уже отсортированный и (если задан) отфильтрованный список
    if let Some(cached_items) = state.subs_cache_get(&cache_key, CACHE_TTL) {
        return paginate_and_json_with_meta(cached_items, page, limit, sort_order);
    }

    // 3) Если кэша нет — тянем ВСЕ страницы из YouTube
    let client = reqwest::Client::new();
    let mut all_items: Vec<Value> = Vec::new();
    let mut page_token: Option<String> = None;

    loop {
        let mut req = client
            .get("https://www.googleapis.com/youtube/v3/subscriptions")
            .bearer_auth(&token)
            .query(&[
                ("part", "snippet,contentDetails"),
                ("mine", "true"),
                ("maxResults", "50"),
            ]);

        if let Some(ref tok) = page_token {
            req = req.query(&[("pageToken", tok)]);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(err) => {
                eprintln!("YouTube API request error: {:?}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Request failed").into_response();
            }
        };

        if !resp.status().is_success() {
            let status = StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            return (status, "YouTube API error").into_response();
        }

        let json: Value = match resp.json().await {
            Ok(v) => v,
            Err(err) => {
                eprintln!("Parse error: {:?}", err);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Invalid JSON").into_response();
            }
        };

        if let Some(items) = json["items"].as_array() {
            all_items.extend(items.clone());
        }

        page_token = json["nextPageToken"].as_str().map(|s| s.to_string());
        if page_token.is_none() {
            break;
        }
    }

    // 4) Глобальная сортировка по contentDetails.totalItemCount
    all_items.sort_by(|a, b| {
        let av = a["contentDetails"]["totalItemCount"].as_i64().unwrap_or(0);
        let bv = b["contentDetails"]["totalItemCount"].as_i64().unwrap_or(0);
        match sort_order {
            "asc" => av.cmp(&bv),
            _ => bv.cmp(&av), // desc
        }
    });

    // 5) Фильтрация (если задан query) по title / channelTitle
    let filtered: Vec<Value> = if query_lc.is_empty() {
        all_items
    } else {
        all_items
            .into_iter()
            .filter(|it| {
                let t = it["snippet"]["title"].as_str().unwrap_or("").to_lowercase();
                let ct = it["snippet"]["channelTitle"].as_str().unwrap_or("").to_lowercase();
                t.contains(&query_lc) || ct.contains(&query_lc)
            })
            .collect()
    };

    // 6) Кладём результат в кэш (полный массив после сортировки/фильтра)
    state.subs_cache_put(cache_key, filtered.clone());

    // 7) Возвращаем нужную страницу
    paginate_and_json_with_meta(filtered, page, limit, sort_order)
}

/// Режем массив на страницу и возвращаем JSON-ответ + текущее направление сортировки.
fn paginate_and_json_with_meta(items: Vec<Value>, page: usize, limit: usize, sort: &str) -> Response {
    let total_results = items.len();
    let total_pages = ((total_results as f64) / (limit as f64)).ceil() as usize;

    let start = (page - 1) * limit;
    let end = start.saturating_add(limit).min(total_results);

    let page_items = if start < total_results {
        items[start..end].to_vec()
    } else {
        Vec::new()
    };

    Json(json!({
        "totalResults": total_results,
        "totalPages": total_pages,
        "page": page,
        "limit": limit,
        "sort": sort, // 👈 возвращаем текущее направление сортировки
        "items": page_items,
    }))
        .into_response()
}


// ---------------- Unsubscribe (как было) ----------------

#[derive(Deserialize)]
pub struct UnsubscribeRequest {
    ids: Vec<String>,
}

pub async fn unsubscribe_handler(
    State(state): State<AppState>,          // ⬅️ нужно получить state
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

    // ⬇️ ВАЖНО: после любых успешных удалений — сбрасываем кэш и page_tokens пользователя
    if !success_ids.is_empty() {
        state.subs_cache_invalidate_user(&token);
        state.page_tokens_clear_user(&token);
    }

    Json(json!({
        "deleted": success_ids
    }))
        .into_response()
}

pub async fn refresh_subs_cache_handler(
    State(state): State<AppState>,
    jar: CookieJar,
) -> impl IntoResponse {
    let Some(token) = jar.get("access_token").map(|c| c.value().to_string()) else {
        return (StatusCode::UNAUTHORIZED, "Missing token").into_response();
    };

    // Сбрасываем кэш и pageTokens
    state.subs_cache_invalidate_user(&token);
    state.page_tokens_clear_user(&token);

    StatusCode::NO_CONTENT.into_response()
}