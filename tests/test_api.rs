use axum_test::TestServer;
use waterwheel::server::api::make_app;

mod common;

#[tokio::main]
#[test]
pub async fn test_healthcheck() -> anyhow::Result<()> {
    common::with_external_services(|config| async {
        let tc = TestServer::new(make_app(config).await?);

        let resp = tc.get("/healthcheck").await;
        resp.assert_status_ok()
         .assert_text("OK");

        Ok(())
    })
    .await
}
