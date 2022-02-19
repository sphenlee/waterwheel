use crate::config;
use anyhow::{anyhow, Result};
use highnoon::{Error, Request, State, StatusCode};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use std::fs;
use std::time::{Duration, SystemTime};
use tracing::debug;
use crate::config::Config;

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

static ALGORITHM: OnceCell<Algorithm> = OnceCell::new();
static DECODING_KEY: OnceCell<DecodingKey> = OnceCell::new();
static ENCODING_KEY: OnceCell<EncodingKey> = OnceCell::new();

/// Loads the encryption/decryption keys used to verify access to the stash
/// Prefers an RSA key pair if one is provided, otherwise will use an HMAC shared secret
/// (which is easier to generate and share for local development)
pub fn load_keys(config: &Config) -> Result<()> {
    match config.public_key.as_deref() {
        Some(pub_key_file) => {
            let priv_key_file = config
                .private_key
                .as_deref()
                .ok_or_else(|| anyhow!("RSA private key not set
                    (either both public and private keys must be set, or the HMAC secret must be set)"))?;

            load_rsa_keys(pub_key_file, priv_key_file)
        }
        None => {
            let secret = config.hmac_secret.as_deref().ok_or_else(|| {
                anyhow!(
                    "HMAC secret set
                (either both public and private keys must be set, or the HMAC secret must be set)"
                )
            })?;
            load_hmac_secret(secret)
        }
    }
}

fn load_rsa_keys(pub_key_file: &str, priv_key_file: &str) -> Result<()> {
    debug!("using RSA for stash keys");

    let pub_key = fs::read(pub_key_file)?;
    DECODING_KEY
        .set(DecodingKey::from_rsa_pem(&pub_key)?.into_static())
        .expect("public key already set??");

    let priv_key = fs::read(priv_key_file)?;
    ENCODING_KEY
        .set(EncodingKey::from_rsa_pem(&priv_key)?)
        .expect("private key already set??");

    ALGORITHM
        .set(Algorithm::RS256)
        .expect("algorithm already set??");

    Ok(())
}

#[allow(clippy::unnecessary_wraps)] // for consistency with `load_rsa_keys`
fn load_hmac_secret(secret: &str) -> Result<()> {
    debug!("using HMAC for stash keys");

    DECODING_KEY
        .set(DecodingKey::from_secret(secret.as_bytes()).into_static())
        .expect("secret already set??");

    ENCODING_KEY
        .set(EncodingKey::from_secret(secret.as_bytes()))
        .expect("secret already set??");

    ALGORITHM
        .set(Algorithm::HS256)
        .expect("algorithm already set??");

    Ok(())
}

pub fn generate_stash_jwt(task_id: &str) -> Result<String> {
    generate_jwt(STASH_AUDIENCE.to_owned(), task_id.to_owned())
}

pub fn generate_config_jwt(id: Uuid) -> Result<String> {
    // note ID is either a task_id or a project_id depending on what config we need
    generate_jwt(CONFIG_AUDIENCE.to_owned(), id.to_string())
}

pub fn generate_jwt(aud: String, sub: String) -> Result<String> {
    let header = Header::new(*ALGORITHM.get().unwrap());

    let claims = Claims {
        iss: WATERWHEEL_ISSUER.to_owned(),
        sub,
        aud,
        exp: (SystemTime::now() + Duration::from_secs(5 * 60))
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs(),
    };

    let token = jsonwebtoken::encode(&header, &claims, ENCODING_KEY.get().unwrap())?;
    Ok(token)
}

pub fn validate_stash_jwt(jwt: &str) -> Result<String> {
    validate_jwt(jwt, STASH_AUDIENCE)
}

pub fn validate_config_jwt<S: State>(req: &Request<S>, id: Uuid) -> highnoon::Result<String> {
    use highnoon::headers::{authorization::Bearer, Authorization};

    let bearer = req
        .header::<Authorization<Bearer>>()
        .ok_or_else(|| Error::http(StatusCode::FORBIDDEN))?;

    let sub = validate_jwt(bearer.0.token(), CONFIG_AUDIENCE)?;
    if sub != id.to_string() {
        Err(Error::http(StatusCode::FORBIDDEN))
    } else {
        Ok(sub)
    }
}

fn validate_jwt(jwt: &str, aud: &str) -> Result<String> {
    let mut validation = Validation::new(*ALGORITHM.get().unwrap());
    validation.set_audience(&[aud]);
    validation.iss = Some(WATERWHEEL_ISSUER.to_owned());

    let token: TokenData<Claims> =
        jsonwebtoken::decode(jwt, DECODING_KEY.get().unwrap(), &validation)?;

    Ok(token.claims.sub)
}
