use crate::server::api::{State};
use serde::de::DeserializeOwned;

const YAML_MIMES: &[&str] = &[
    "application/x-yaml",
];
const SUPPORTED_MIMES: &[&str] = &[
    "application/json",
];


pub fn content_type_is_yaml(content_type: highnoon::headers::ContentType) -> bool {
    YAML_MIMES.iter().any(|mime| content_type == highnoon::headers::ContentType::from(
        mime.parse::<mime::Mime>().unwrap()
    ))
}

pub async fn read_from_body<T: DeserializeOwned>(req: &mut highnoon::Request<State>) -> highnoon::Result<T> {
    let content_type = req.header::<highnoon::headers::ContentType>().unwrap_or(
        highnoon::headers::ContentType::json(),
    );
    let reader = req.reader().await?;

    if content_type == highnoon::headers::ContentType::json() {
        return serde_json::from_reader(reader).map_err(|err| {
            let msg = format!("error parsing request body as json: {}", err);
            highnoon::Error::http((highnoon::StatusCode::BAD_REQUEST, msg))
        });
    } else if content_type_is_yaml(content_type) {
        return serde_yaml::from_reader(reader).map_err(|err| {
            let msg = format!("error parsing request body as yaml: {}", err);
            highnoon::Error::http((highnoon::StatusCode::BAD_REQUEST, msg))
        })
    }

    return Err(highnoon::Error::http((
        highnoon::StatusCode::UNSUPPORTED_MEDIA_TYPE,
        format!(
            "Unsupported media type must be one of:\n{}\n{}",
            SUPPORTED_MIMES.join("\n"),
            YAML_MIMES.join("\n"),
        ),
    )))
}
