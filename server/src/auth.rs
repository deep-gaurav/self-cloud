use std::collections::HashMap;

use anyhow::anyhow;
use app::auth::{User, UserWithPass};
use tower_cookies::Cookie;

pub async fn get_authorized_users() -> anyhow::Result<HashMap<String, UserWithPass>> {
    let users = tokio::fs::read("users.json").await?;
    let users = serde_json::from_slice::<HashMap<String, UserWithPass>>(&users)?;
    Ok(users)
}
