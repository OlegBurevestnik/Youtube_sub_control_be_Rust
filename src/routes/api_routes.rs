use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::AppState;

#[derive(Deserialize, Debug)]
pub struct SubscriptionsQuery {
    query: Option<String>,
    page: Option<usize>,   // номер страницы на фронте (1..)
    limit: Option<usize>,  // сколько на страницу (по ТЗ: 25)
}

const YT_PAGE_SIZE: usize = 50; // YouTube возвращает максимум 50 на страницу

pub async fn get_subscriptions(
    State(state): State<AppState>,
    Query(params): Query<SubscriptionsQuery>,
    jar: CookieJar,
) -> Json<Value> {
    // 1) Достаём токен
    let Some(token) = jar.get("access_token").map(|c| c.value().to_string()) else {
        return Json(json!({ "error": "Missing token" }));
    };

    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(25).max(1).min(50);
    let query_lc = params.query.clone().unwrap_or_default().to_lowercase();

    // 2) user_key для кэша pageToken'ов (MVP: по access_token)
    let user_key = token.clone();

    // 3) Без фильтра — быстрый путь: 1 запрос к нужной странице YouTube
    if query_lc.is_empty() {
        match get_unfiltered_page_fast(&state, &user_key, &token, page, limit).await {
            Ok(resp) => return Json(resp),
            Err(e) => {
                eprintln!("[get_unfiltered_page_fast] {e:?}");
                return Json(json!({ "error": "Failed to load subscriptions" }));
            }
        }
    }

    // 4) С фильтром — корректный путь (MVP): тянем страницы, фильтруем и пагинируем
    match get_filtered_page_slow_mvp(&state, &user_key, &token, &query_lc, page, limit).await {
        Ok(resp) => Json(resp),
        Err(e) => {
            eprintln!("[get_filtered_page_slow_mvp] {e:?}");
            Json(json!({ "error": "Failed to load filtered subscriptions" }))
        }
    }
}

/// Быстрый путь без фильтра: вычисляем, какую YT-страницу нужно запросить,
/// делаем ОДИН вызов к YouTube, а потом режем на 25.
async fn get_unfiltered_page_fast(
    state: &AppState,
    user_key: &str,
    access_token: &str,
    page: usize,
    limit: usize,
) -> anyhow::Result<Value> {
    // На фронте limit=25; у YouTube — 50. Две клиентские страницы = одна YouTube-страница.
    let start_index = (page - 1) * limit;               // индекс в общем списке (0..)
    let yt_page_idx = (start_index / YT_PAGE_SIZE) + 1; // какая YT-страница нужна (1..)
    let offset_in_yt = start_index % YT_PAGE_SIZE;      // с какого индекса внутри YT‑страницы начать

    // Убедимся, что у нас есть pageToken для нужной YT‑страницы
    ensure_tokens_until(state, user_key, access_token, yt_page_idx).await?;

    // pageToken для этой страницы: для первой — None; для 2+ — токен предыдущей
    let page_token_opt = state.get_token_for_page(user_key, yt_page_idx).flatten();

    let client = reqwest::Client::new();
    let mut req = client
        .get("https://www.googleapis.com/youtube/v3/subscriptions")
        .bearer_auth(access_token)
        .query(&[
            ("part", "snippet"),
            ("mine", "true"),
            ("maxResults", "50"),
        ]);

    if let Some(ref pt) = page_token_opt {
        req = req.query(&[("pageToken", pt)]);
    }

    let res = req.send().await?;
    let json: Value = res.json().await?;

    // totalResults общий по аккаунту — берём из pageInfo
    let total_results = json["pageInfo"]["totalResults"].as_u64().unwrap_or(0) as usize;

    // Забираем items и режем по нужному offset/limit
    let mut items = json["items"].as_array().cloned().unwrap_or_default();

    if offset_in_yt >= items.len() {
        items.clear();
    } else {
        items = items.into_iter().skip(offset_in_yt).take(limit).collect();
    }

    Ok(json!({
        "items": items,
        "totalResults": total_results,
        "page": page,
        "limit": limit
    }))
}

/// С фильтром (MVP): идём по страницам по очереди, фильтруем в процессе,
/// собираем достаточно для заданной страницы и считаем totalFiltered.
/// Кэшируем pageToken'ы по пути.
async fn get_filtered_page_slow_mvp(
    state: &AppState,
    user_key: &str,
    access_token: &str,
    query_lc: &str,
    page: usize,
    limit: usize,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();

    let mut filtered: Vec<Value> = Vec::new();
    let target_end = page * limit;

    // Итерируем по YT‑страницам, пока не соберём хотя бы target_end отфильтрованных
    let mut yt_idx: usize = 1;
    loop {
        // гарантируем наличие pageToken’ов до этой страницы
        ensure_tokens_until(state, user_key, access_token, yt_idx).await?;

        // pageToken для этой YT‑страницы
        let page_token_opt = state.get_token_for_page(user_key, yt_idx).flatten();

        let mut req = client
            .get("https://www.googleapis.com/youtube/v3/subscriptions")
            .bearer_auth(access_token)
            .query(&[
                ("part", "snippet"),
                ("mine", "true"),
                ("maxResults", "50"),
            ]);

        if let Some(ref pt) = page_token_opt {
            req = req.query(&[("pageToken", pt)]);
        }

        let res = req.send().await?;
        let json: Value = res.json().await?;
        let items = json["items"].as_array().cloned().unwrap_or_default();

        // фильтруем текущую страницу
        for it in items {
            let title = it["snippet"]["title"].as_str().unwrap_or("").to_lowercase();
            let ch    = it["snippet"]["channelTitle"].as_str().unwrap_or("").to_lowercase();
            if title.contains(query_lc) || ch.contains(query_lc) {
                filtered.push(it);
            }
        }

        // Сохраняем nextPageToken (если ещё не сохранён) — для следующей страницы
        let next = json["nextPageToken"].as_str().map(|s| s.to_string());
        // entry.len() == yt_idx ? можно пушнуть next
        if state.get_token_for_page(user_key, yt_idx + 1).is_none() {
            state.push_next_token(user_key, next.clone());
        }

        // Если собрали достаточно элементов для этой страницы — можно прервать раньше
        let no_more_pages = next.is_none();
        if filtered.len() >= target_end || no_more_pages {
            let total_filtered = if no_more_pages {
                filtered.len()
            } else {
                // Можно дочитать до конца, если нужен точный total; для MVP — достаточно текущего
                filtered.len()
            };

            let start = (page - 1) * limit;
            let page_items = if start >= filtered.len() {
                vec![]
            } else {
                filtered.into_iter().skip(start).take(limit).collect()
            };

            return Ok(json!({
                "items": page_items,
                "totalResults": total_filtered,
                "page": page,
                "limit": limit
            }));
        }

        // Иначе двигаемся к следующей YT‑странице
        yt_idx += 1;
    }
}

/// Гарантируем, что у нас в кэше есть цепочка pageToken’ов как минимум до `target_yt_page`.
/// Схема хранения:
///   tokens[user_key][0] = None (для стр.1)
///   tokens[user_key][1] = Some(pageToken для стр.2)
///   tokens[user_key][2] = Some(pageToken для стр.3)
async fn ensure_tokens_until(
    state: &AppState,
    user_key: &str,
    access_token: &str,
    target_yt_page: usize,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    loop {
        // текущая длина цепочки токенов и последний известный токен
        let (known_len, last_token_opt) = state.tokens_len_and_last(user_key);

        // инициализация: стр.1 = None
        if known_len == 0 {
            state.ensure_user_tokens_init(user_key);
            continue; // пересчитать known_len на следующей итерации
        }

        if known_len >= target_yt_page {
            // уже достаточно
            return Ok(());
        }

        // Узнаём nextPageToken для следующей страницы.
        // Нам нужна "следующая" после известной: запрос с pageToken = токен предыдущей страницы.
        let mut req = client
            .get("https://www.googleapis.com/youtube/v3/subscriptions")
            .bearer_auth(access_token)
            .query(&[
                ("part", "snippet"),
                ("mine", "true"),
                ("maxResults", "50"),
            ]);

        if let Some(ref pt) = last_token_opt {
            req = req.query(&[("pageToken", pt)]);
        }

        let res = client.execute(req.build()?).await?;
        if !res.status().is_success() {
            anyhow::bail!("YouTube returned non-success: {}", res.status());
        }
        let json: Value = res.json().await?;
        let next = json.get("nextPageToken").and_then(|v| v.as_str()).map(|s| s.to_string());

        // добавляем токен для следующей страницы
        state.push_next_token(user_key, next.clone());

        // Если next == None — дальше страниц нет
        if next.is_none() {
            return Ok(());
        }
        // иначе цикл продолжит до достижения target_yt_page
    }
}

// ---------------- Unsubscribe (без изменений) ----------------

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
