use highnoon::StatusCode;
use waterwheel::server::api::make_app;

mod common;

#[tokio::main]
#[test]
pub async fn test_healthcheck() -> highnoon::Result<()> {
    common::with_external_services(|config| async {
        let tc = make_app(config).await?.test();

        let mut resp = tc.get("/healthcheck").send().await?;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.body_string().await?, "OK");

        Ok(())
    })
    .await
}
