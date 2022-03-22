use crate::server::api::State;
use highnoon::headers::ContentType;
use serde::de::DeserializeOwned;

const YAML_MIMES: &[&str] = &["application/x-yaml"];
const SUPPORTED_MIMES: &[&str] = &["application/json"];

pub fn content_type_is_yaml(content_type: &ContentType) -> bool {
    YAML_MIMES
        .iter()
        .any(|mime| *content_type == ContentType::from(mime.parse::<mime::Mime>().unwrap()))
}

pub async fn read_from_body<T: DeserializeOwned>(
    req: &mut highnoon::Request<State>,
) -> highnoon::Result<T> {
    let content_type = req
        .header::<ContentType>()
        .unwrap_or_else(ContentType::json);
    let reader = req.reader().await?;

    if content_type == ContentType::json() {
        return serde_json::from_reader(reader).map_err(|err| {
            highnoon::Error::bad_request(format!("error parsing request body as json: {}", err))
        });
    } else if content_type_is_yaml(&content_type) {
        return serde_yaml::from_reader(reader).map_err(|err| {
            highnoon::Error::bad_request(format!("error parsing request body as yaml: {}", err))
        });
    }

    return Err(highnoon::Error::http((
        highnoon::StatusCode::UNSUPPORTED_MEDIA_TYPE,
        format!(
            "Unsupported media type.\n\
            Content-Type must be one of:\n\
            {}\n{}",
            SUPPORTED_MIMES.join("\n"),
            YAML_MIMES.join("\n"),
        ),
    )));
}
