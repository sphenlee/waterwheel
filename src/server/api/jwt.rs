use crate::{config::Config, server::api::State};
use anyhow::{Result, format_err};
use highnoon::{Error, Request, StatusCode};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::{
    fs,
    time::{Duration, SystemTime},
};
use tracing::{debug, trace};

const WATERWHEEL_ISSUER: &str = "waterwheel";
const STASH_AUDIENCE: &str = "waterwheel.stash";
const CONFIG_AUDIENCE: &str = "waterwheel.config";

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
}

pub struct JwtKeys {
    algorithm: Algorithm,
    decoding: DecodingKey,
    encoding: EncodingKey,
}

/// Loads the encryption/decryption keys used to verify access to the stash
///
/// Prefers an RSA key pair if one is provided, otherwise will use an HMAC shared secret
/// (which is easier to generate and share for local development)
pub fn load_keys(config: &Config) -> Result<JwtKeys> {
    match config.public_key.as_deref() {
        Some(pub_key_file) => {
            let priv_key_file = config.private_key.as_deref().ok_or_else(|| {
                format_err!(
                    "RSA private key not set (either both public and private keys must be set, \
                    or the HMAC secret must be set)"
                )
            })?;

            load_rsa_keys(pub_key_file, priv_key_file)
        }
        None => {
            let secret = config.hmac_secret.as_deref().ok_or_else(|| {
                format_err!(
                    "HMAC secret set (either both public and private keys must be set, \
                    or the HMAC secret must be set)"
                )
            })?;
            load_hmac_secret(secret)
        }
    }
}

fn load_rsa_keys(pub_key_file: &str, priv_key_file: &str) -> Result<JwtKeys> {
    debug!("using RSA for stash keys");

    let pub_key = fs::read(pub_key_file)?;
    let priv_key = fs::read(priv_key_file)?;

    Ok(JwtKeys {
        algorithm: Algorithm::RS256,
        decoding: DecodingKey::from_rsa_pem(&pub_key)?,
        encoding: EncodingKey::from_rsa_pem(&priv_key)?,
    })
}

#[allow(clippy::unnecessary_wraps)] // for consistency with `load_rsa_keys`
fn load_hmac_secret(secret: &str) -> Result<JwtKeys> {
    debug!("using HMAC for stash keys");

    Ok(JwtKeys {
        algorithm: Algorithm::HS256,
        decoding: DecodingKey::from_secret(secret.as_bytes()),
        encoding: EncodingKey::from_secret(secret.as_bytes()),
    })
}

pub fn generate_stash_jwt(keys: &JwtKeys, task_id: &str) -> Result<String> {
    generate_jwt(keys, STASH_AUDIENCE.to_owned(), task_id.to_owned())
}

pub fn generate_config_jwt(keys: &JwtKeys, id: Uuid) -> Result<String> {
    // note ID is either a task_id or a project_id depending on what config we need
    generate_jwt(keys, CONFIG_AUDIENCE.to_owned(), id.to_string())
}

pub fn generate_jwt(keys: &JwtKeys, aud: String, sub: String) -> Result<String> {
    trace!("generating jwt for aud={} sub={}", aud, sub);
    let header = Header::new(keys.algorithm);

    let claims = Claims {
        iss: WATERWHEEL_ISSUER.to_owned(),
        sub,
        aud,
        exp: (SystemTime::now() + Duration::from_secs(5 * 60))
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs(),
    };

    let token = jsonwebtoken::encode(&header, &claims, &keys.encoding)?;
    Ok(token)
}

pub fn validate_stash_jwt(keys: &JwtKeys, jwt: &str) -> Result<String> {
    validate_jwt(keys, jwt, STASH_AUDIENCE)
}

pub fn validate_config_jwt(req: &Request<State>, id: Uuid) -> highnoon::Result<String> {
    use highnoon::headers::{Authorization, authorization::Bearer};

    let bearer = req
        .header::<Authorization<Bearer>>()
        .ok_or_else(|| Error::http(StatusCode::FORBIDDEN))?;

    let keys = &req.state().jwt_keys;

    let sub = validate_jwt(keys, bearer.0.token(), CONFIG_AUDIENCE)?;
    if sub != id.to_string() {
        Err(Error::http(StatusCode::FORBIDDEN))
    } else {
        Ok(sub)
    }
}

fn validate_jwt(keys: &JwtKeys, jwt: &str, aud: &str) -> Result<String> {
    let mut validation = Validation::new(keys.algorithm);
    validation.set_audience(&[aud]);
    validation.set_issuer(&[WATERWHEEL_ISSUER]);

    let token: TokenData<Claims> = jsonwebtoken::decode(jwt, &keys.decoding, &validation)?;

    Ok(token.claims.sub)
}
