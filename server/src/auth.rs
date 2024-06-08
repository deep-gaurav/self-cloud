use std::collections::HashMap;

use app::{auth::UserWithPass, common::get_home_path};

pub async fn get_authorized_users() -> anyhow::Result<HashMap<String, UserWithPass>> {
    let users = tokio::fs::read(get_home_path().join("users.json")).await?;
    let users = serde_json::from_slice::<HashMap<String, UserWithPass>>(&users)?;
    Ok(users)
}
