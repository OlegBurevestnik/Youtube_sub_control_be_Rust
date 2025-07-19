use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CreatedUser {
    pub id: u32,
    pub name: String,
}
