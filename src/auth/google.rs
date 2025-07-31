use oauth2::{
    basic::BasicClient,
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::env;
use axum::{
    extract::{Query, State},
    http::{StatusCode},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use crate::state::AppState;
use cookie::{Cookie, CookieBuilder};

pub fn oauth_client() -> BasicClient {
    let client_id = ClientId::new(env::var("GOOGLE_CLIENT_ID").expect("Missing GOOGLE_CLIENT_ID"));
    let client_secret = ClientSecret::new(env::var("GOOGLE_CLIENT_SECRET").expect("Missing GOOGLE_CLIENT_SECRET"));
    let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap();
    let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap();
    let redirect_url = RedirectUrl::new(env::var("GOOGLE_REDIRECT_URI").expect("Missing GOOGLE_REDIRECT_URI")).unwrap();

    BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        .set_redirect_uri(redirect_url)
}

pub async fn get_authorization_url() -> String {
    let client = oauth_client();
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/youtube".to_string()))
        .url();

    auth_url.to_string()
}

pub async fn exchange_code(code: &str) -> Result<String, String> {
    let client = oauth_client();
    match client
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .request_async(async_http_client)
        .await
    {
        Ok(token) => Ok(token.access_token().secret().to_string()),
        Err(err) => {
            eprintln!("❌ Error exchanging code: {:?}", err);
            Err("Failed to exchange code".to_string())
        }
    }
}

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