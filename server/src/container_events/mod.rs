use app::auth::{server::get_user_from_cookie, AuthType, User};
use http::StatusCode;
use tower_cookies::Cookies;

pub mod attach;
pub mod logs;
pub mod stats;

pub fn get_auth(cookies: Cookies) -> AuthType {
    let auth = if let Some(cookie) = cookies.get("sessionId") {
        if let Ok(user) = get_user_from_cookie(cookie) {
            AuthType::Authorized(user)
        } else {
            AuthType::UnAuthorized
        }
    } else {
        AuthType::UnAuthorized
    };
    auth
}

pub fn ensure_authorized_user(cookies: Cookies) -> Result<User, (StatusCode, String)> {
    let auth = get_auth(cookies);
    match auth {
        AuthType::UnAuthorized => Err((StatusCode::UNAUTHORIZED, "UnAuthorized".to_string())),
        AuthType::Authorized(user) => Ok(user),
    }
}
