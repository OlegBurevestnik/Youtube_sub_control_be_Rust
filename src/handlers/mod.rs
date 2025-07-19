use axum::Json;

use crate::models::{NewUser, CreatedUser};

pub async fn get_hello() -> &'static str {
    "Hello, Axum!!!"
}

pub async fn create_user(Json(payload): Json<NewUser>) -> Json<CreatedUser> {
    println!("create_user");

    let user = CreatedUser {
        id: 1,
        name: payload.name,
    };

    Json(user)
}
