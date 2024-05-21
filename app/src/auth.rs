use leptos::{server, use_context, ServerFnError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct User {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    UnAuthorized,
    Authorized(User),
}

impl AuthType {
    pub fn as_authorized(&self) -> Option<&User> {
        if let Self::Authorized(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[server(GetUser)]
pub async fn get_auth() -> Result<AuthType, ServerFnError> {
    let user =
        use_context::<AuthType>().ok_or(ServerFnError::new(anyhow::anyhow!("AuthType Failed")))?;

    Ok(user)
}
