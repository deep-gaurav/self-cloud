use app::auth::{server::get_user_from_cookie, AuthType, User};
use axum_extra::extract::cookie::CookieJar;
use http::StatusCode;

pub mod attach;
pub mod logs;
pub mod stats;

pub fn get_auth(jar: CookieJar) -> AuthType {
    let auth = if let Some(cookie) = jar.get("sessionId") {
        if let Ok(user) = get_user_from_cookie(cookie.clone()) {
            AuthType::Authorized(user)
        } else {
            AuthType::UnAuthorized
        }
    } else {
        AuthType::UnAuthorized
    };
    auth
}

pub fn ensure_authorized_user(jar: CookieJar) -> Result<User, (StatusCode, String)> {
    let auth = get_auth(jar);
    match auth {
        AuthType::UnAuthorized => Err((StatusCode::UNAUTHORIZED, "UnAuthorized".to_string())),
        AuthType::Authorized(user) => Ok(user),
    }
}
