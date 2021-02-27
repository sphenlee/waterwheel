use anyhow::Result;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{Duration, SystemTime};

const WATERWHEEL_ISSUER: &str = "waterwheel";
const STASH_AUDIENCE: &str = "stash";

static RSA_PUBLIC_KEY: OnceCell<DecodingKey> = OnceCell::new();
static RSA_PRIVATE_KEY: OnceCell<EncodingKey> = OnceCell::new();

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
}

pub fn load_rsa_keys() -> Result<()> {
    let pub_key_file = std::env::var("WATERWHEEL_PUBLIC_KEY")?;
    let pub_key = fs::read(pub_key_file)?;
    RSA_PUBLIC_KEY
        .set(DecodingKey::from_rsa_pem(&pub_key)?.into_static())
        .expect("public key already set??");

    let priv_key_file = std::env::var("WATERWHEEL_PRIVATE_KEY")?;
    let priv_key = fs::read(priv_key_file)?;
    RSA_PRIVATE_KEY
        .set(EncodingKey::from_rsa_pem(&priv_key)?)
        .expect("private key already set??");

    Ok(())
}

pub fn generate_jwt(task_id: &str) -> Result<String> {
    let header = Header::new(Algorithm::RS256);

    let claims = Claims {
        iss: WATERWHEEL_ISSUER.to_owned(),
        sub: task_id.to_owned(),
        aud: STASH_AUDIENCE.to_owned(),
        exp: (SystemTime::now() + Duration::from_secs(5 * 60))
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs(),
    };

    let token = jsonwebtoken::encode(&header, &claims, RSA_PRIVATE_KEY.get().unwrap())?;
    Ok(token)
}

pub fn validate_jtw(jwt: &str) -> Result<String> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[STASH_AUDIENCE]);
    validation.iss = Some(WATERWHEEL_ISSUER.to_owned());

    let token: TokenData<Claims> =
        jsonwebtoken::decode(jwt, RSA_PUBLIC_KEY.get().unwrap(), &validation)?;

    Ok(token.claims.sub)
}
