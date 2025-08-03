use axum::{
    http::{StatusCode},
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use crate::state::AppState;
use cookie::{CookieBuilder};

pub async fn auth_start() -> impl IntoResponse {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap();
    let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI").unwrap();
    let scopes = "https://www.googleapis.com/auth/youtube.readonly https://www.googleapis.com/auth/youtube.force-ssl";

    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?response_type=code&client_id={}&redirect_uri={}&scope={}&access_type=offline&prompt=consent",
        client_id,
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(scopes)
    );

    Redirect::temporary(&auth_url)
}

// После возврата от Google
#[derive(Debug, Deserialize)]
pub struct AuthCallback {
    code: String,
}

pub async fn auth_callback(
    params: AuthCallback,
    state: AppState,
    jar: CookieJar,
) -> impl IntoResponse {
    let client_id = std::env::var("GOOGLE_CLIENT_ID").unwrap();
    let client_secret = std::env::var("GOOGLE_CLIENT_SECRET").unwrap();
    let redirect_uri = std::env::var("GOOGLE_REDIRECT_URI").unwrap();

    let client = reqwest::Client::new();

    let params = vec![
        ("code", params.code.clone()),
        ("client_id", client_id.clone()),
        ("client_secret", client_secret.clone()),
        ("redirect_uri", redirect_uri.clone()),
        ("grant_type", "authorization_code".to_string()),
    ];

    let res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await
        .unwrap();

    if !res.status().is_success() {
        return (StatusCode::UNAUTHORIZED, "OAuth failed").into_response();
    }

    let json: serde_json::Value = res.json().await.unwrap();
    let token = json["access_token"].as_str().unwrap_or("").to_string();

    // Сохраняем токен в cookie (httpOnly, secure желательно в проде)
    let cookie = CookieBuilder::new("access_token", token)
        .http_only(true)
        .path("/")
        .finish();

    // Можно редиректнуть обратно на фронтенд
    let jar = jar.add(cookie);
    (jar, Redirect::temporary("http://localhost:5173")).into_response()
}