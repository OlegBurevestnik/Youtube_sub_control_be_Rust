use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub access_token: Arc<RwLock<Option<String>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_token(&self, token: String) {
        let mut lock = self.access_token.write().unwrap();
        *lock = Some(token);
    }

    pub fn get_token(&self) -> Option<String> {
        let lock = self.access_token.read().unwrap();
        lock.clone()
    }
}
