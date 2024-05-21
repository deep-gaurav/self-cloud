use anyhow::anyhow;
use app::auth::User;
use tower_cookies::Cookie;

const AUTH_KEY: [u8; 32] = *b"AED4841B431AA729E2FEC22AA7653E1D";

pub fn get_encrypted_user_cookie(user: &User) -> anyhow::Result<Cookie> {
    use aes_gcm_siv::aead::Aead;
    use aes_gcm_siv::AeadCore;
    use aes_gcm_siv::{Aes256GcmSiv, KeyInit};

    use rand::rngs::OsRng;

    let cipher = Aes256GcmSiv::new_from_slice(&AUTH_KEY).map_err(|_e| anyhow!("Invalid key"))?;
    let nonce = Aes256GcmSiv::generate_nonce(&mut OsRng);

    let encoded_user = bincode::serialize(user)?;
    let mut ciphertext = cipher
        .encrypt(&nonce, encoded_user.as_ref())
        .map_err(|_e| anyhow!("Cant encrypt"))?;
    ciphertext.extend(nonce);

    use base64::{engine::general_purpose::URL_SAFE, Engine as _};

    let encoded_value = URL_SAFE.encode(&ciphertext);
    let mut cookie = Cookie::new("sessionId", encoded_value);
    cookie.set_http_only(true);
    cookie.set_secure(true);
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

    let cipher = Aes256GcmSiv::new_from_slice(&AUTH_KEY).map_err(|e| anyhow!("Invalid key"))?;
    let nonce = Nonce::from_slice(&encoded_value[..=96]);

    let ciphertext = cipher
        .decrypt(nonce, &encoded_value[97..])
        .map_err(|e| anyhow!("Invalid decrypt"))?;

    let decoded_user = bincode::deserialize::<User>(&ciphertext)?;

    Ok(decoded_user)
}
