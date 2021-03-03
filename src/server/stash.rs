use crate::config;
use anyhow::Result;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{Duration, SystemTime};
use log::debug;

const WATERWHEEL_ISSUER: &str = "waterwheel";
const STASH_AUDIENCE: &str = "stash";

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
pub fn load_keys() -> Result<()> {
    match config::get("WATERWHEEL_PUBLIC_KEY") {
        Ok(pub_key_file) => {
            let priv_key_file = config::get("WATERWHEEL_PRIVATE_KEY")?;
            load_rsa_keys(pub_key_file, priv_key_file)
        },
        Err(_) => {
            let secret = config::get("WATERWHEEL_HMAC_SECRET")?;
            load_hmac_secret(secret)
        }
    }
}

fn load_rsa_keys(pub_key_file: String, priv_key_file: String) -> Result<()> {
    debug!("using RSA for stash keys");

    let pub_key = fs::read(pub_key_file)?;
    DECODING_KEY
        .set(DecodingKey::from_rsa_pem(&pub_key)?.into_static())
        .expect("public key already set??");

    let priv_key = fs::read(priv_key_file)?;
    ENCODING_KEY
        .set(EncodingKey::from_rsa_pem(&priv_key)?)
        .expect("private key already set??");

    ALGORITHM.set(Algorithm::RS256).expect("algorithm already set??");

    Ok(())
}

fn load_hmac_secret(secret: String) -> Result<()> {
    debug!("using HMAC for stash keys");

    DECODING_KEY
        .set(DecodingKey::from_secret(secret.as_bytes()).into_static())
        .expect("secret already set??");

    ENCODING_KEY
        .set(EncodingKey::from_secret(secret.as_bytes()))
        .expect("secret already set??");

    ALGORITHM.set(Algorithm::HS256).expect("algorithm already set??");

    Ok(())
}

pub fn generate_jwt(task_id: &str) -> Result<String> {
    let header = Header::new(*ALGORITHM.get().unwrap());

    let claims = Claims {
        iss: WATERWHEEL_ISSUER.to_owned(),
        sub: task_id.to_owned(),
        aud: STASH_AUDIENCE.to_owned(),
        exp: (SystemTime::now() + Duration::from_secs(5 * 60))
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs(),
    };

    let token = jsonwebtoken::encode(&header, &claims, ENCODING_KEY.get().unwrap())?;
    Ok(token)
}

pub fn validate_jtw(jwt: &str) -> Result<String> {
    let mut validation = Validation::new(*ALGORITHM.get().unwrap());
    validation.set_audience(&[STASH_AUDIENCE]);
    validation.iss = Some(WATERWHEEL_ISSUER.to_owned());

    let token: TokenData<Claims> =
        jsonwebtoken::decode(jwt, DECODING_KEY.get().unwrap(), &validation)?;

    Ok(token.claims.sub)
}

