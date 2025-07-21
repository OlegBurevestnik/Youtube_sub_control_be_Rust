use oauth2::{
    basic::BasicClient,
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::env;

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
