use std::collections::HashMap;

use leptos::{server, use_context, ServerFnError};
use serde::{Deserialize, Serialize};
use tracing::info;

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

    /// Returns `true` if the auth type is [`UnAuthorized`].
    ///
    /// [`UnAuthorized`]: AuthType::UnAuthorized
    #[must_use]
    pub fn is_un_authorized(&self) -> bool {
        matches!(self, Self::UnAuthorized)
    }

    /// Returns `true` if the auth type is [`Authorized`].
    ///
    /// [`Authorized`]: AuthType::Authorized
    #[must_use]
    pub fn is_authorized(&self) -> bool {
        matches!(self, Self::Authorized(..))
    }
}

#[cfg(feature = "ssr")]
pub type AuthorizedUsers = HashMap<String, UserWithPass>;

#[cfg(feature = "ssr")]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct UserWithPass {
    pub user: User,
    pub pass: String,
}

#[server(GetUser)]
pub async fn get_auth() -> Result<AuthType, ServerFnError> {
    let user =
        use_context::<AuthType>().ok_or(ServerFnError::new(anyhow::anyhow!("AuthType Failed")))?;

    Ok(user)
}

#[server(Login)]
pub async fn login(email: String, password: String) -> Result<(), ServerFnError> {
    use self::server::get_encrypted_user_cookie;

    let users = use_context::<AuthorizedUsers>().ok_or(ServerFnError::new(anyhow::anyhow!(
        "authorized user not found"
    )))?;
    use tower_cookies::Cookies;

    let cookies =
        use_context::<Cookies>().ok_or(ServerFnError::new(anyhow::anyhow!("no cookies")))?;

    let user = users.get(&email);
    if let Some(user) = user {
        if user.pass == password {
            let cookie = get_encrypted_user_cookie(&user.user).unwrap();
            cookies.add(cookie);
            info!("Login successful");
            leptos_axum::redirect("/dashboard");
            Ok(())
        } else {
            info!("User password mismatch {}!={}", user.pass, password);
            Err(ServerFnError::new("UnAuthorized"))
        }
    } else {
        info!("User not found with email {email}");
        Err(ServerFnError::new("UnAuthorized"))
    }
}

#[cfg(feature = "ssr")]
pub mod server {

    use super::User;
    use anyhow::anyhow;
    use tower_cookies::Cookie;

    pub const AUTH_KEY: [u8; 32] = *b"AED4841B431AA729E2FEC22AA7653E1D";

    pub fn get_encrypted_user_cookie(user: &User) -> anyhow::Result<Cookie<'static>> {
        use aes_gcm_siv::aead::Aead;
        use aes_gcm_siv::AeadCore;
        use aes_gcm_siv::{Aes256GcmSiv, KeyInit};

        use rand::rngs::OsRng;

        let cipher =
            Aes256GcmSiv::new_from_slice(&AUTH_KEY).map_err(|_e| anyhow!("Invalid key"))?;
        let nonce = Aes256GcmSiv::generate_nonce(&mut OsRng);

        let encoded_user = bincode::serialize(&user)?;
        let mut ciphertext = cipher
            .encrypt(&nonce, encoded_user.as_ref())
            .map_err(|_e| anyhow!("Cant encrypt"))?;
        ciphertext.extend(nonce);

        use base64::{engine::general_purpose::URL_SAFE, Engine as _};

        let encoded_value = URL_SAFE.encode(&ciphertext);
        let mut cookie = Cookie::new("sessionId", encoded_value);
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_path("/");
        Ok(cookie)
    }

    pub fn get_user_from_cookie(cookie: Cookie) -> anyhow::Result<User> {
        if let Some(expires) = cookie.expires_datetime() {
            let now = time::OffsetDateTime::now_utc();
            if expires > now {
                return Err(anyhow!("Session Expired"));
            }
        }

        use aes_gcm_siv::aead::Aead;
        use aes_gcm_siv::Nonce;
        use aes_gcm_siv::{Aes256GcmSiv, KeyInit};
        use base64::{engine::general_purpose::URL_SAFE, Engine as _};

        let encoded_value = URL_SAFE.decode(cookie.value())?;
        // let value = ;

        let cipher =
            Aes256GcmSiv::new_from_slice(&AUTH_KEY).map_err(|_e| anyhow!("Invalid key"))?;
        let nonce_size = Nonce::default().len();
        let len = encoded_value.len();
        let nonce = Nonce::from_slice(&encoded_value[len - nonce_size..]);

        let ciphertext = cipher
            .decrypt(nonce, &encoded_value[..len - nonce_size])
            .map_err(|_e| anyhow!("Invalid decrypt"))?;

        let decoded_user = bincode::deserialize::<User>(&ciphertext)?;

        Ok(decoded_user)
    }
}
