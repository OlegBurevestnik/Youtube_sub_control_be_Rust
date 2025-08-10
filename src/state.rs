use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    // Твой текущий токен (можешь оставить, даже если используешь cookie)
    pub access_token: Arc<RwLock<Option<String>>>,
    // Кэш YouTube pageToken’ов по пользователю (MVP: ключом будет access_token)
    // tokens[user][0] = None (стр.1), tokens[user][1] = Some(token для стр.2), ...
    pub page_tokens: Arc<RwLock<HashMap<String, Vec<Option<String>>>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            access_token: Arc::new(RwLock::new(None)),
            page_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ---- Твой код работы с access_token (оставляем как есть) ----
    pub fn set_token(&self, token: String) {
        let mut lock = self.access_token.write().unwrap();
        *lock = Some(token);
    }

    pub fn get_token(&self) -> Option<String> {
        let lock = self.access_token.read().unwrap();
        lock.clone()
    }

    // ---- Удобные хелперы для page_tokens ----

    /// Убедиться, что для user_key инициализирована цепочка токенов (стр.1 = None).
    pub fn ensure_user_tokens_init(&self, user_key: &str) {
        let mut map = self.page_tokens.write().unwrap();
        map.entry(user_key.to_string()).or_insert_with(|| vec![None]); // страница 1
    }

    /// Прочитать текущую длину цепочки и последний токен.
    pub fn tokens_len_and_last(&self, user_key: &str) -> (usize, Option<String>) {
        let map = self.page_tokens.read().unwrap();
        match map.get(user_key) {
            Some(vec) => (vec.len(), vec.last().cloned().flatten()),
            None => (0, None),
        }
    }

    /// Получить pageToken для YT-страницы (1..): None для 1-й, Some(..) для 2+,
    /// если уже известен.
    pub fn get_token_for_page(&self, user_key: &str, yt_page: usize) -> Option<Option<String>> {
        let map = self.page_tokens.read().unwrap();
        map.get(user_key).and_then(|v| v.get(yt_page - 1)).cloned()
    }

    /// Добавить nextPageToken в цепочку (т.е. сделать известной следующую страницу).
    pub fn push_next_token(&self, user_key: &str, next: Option<String>) {
        let mut map = self.page_tokens.write().unwrap();
        let entry = map.entry(user_key.to_string()).or_insert_with(|| vec![None]);
        entry.push(next);
    }
}
