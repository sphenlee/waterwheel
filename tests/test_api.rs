use highnoon::StatusCode;
use waterwheel::{
    config,
    server::{api::make_app, Server},
};

mod common;

#[tokio::main]
#[test]
pub async fn test_healthcheck() -> highnoon::Result<()> {
    common::with_external_services(|| async {
        let config = config::load()?;
        let server = Server::new(config).await?;
        let tc = make_app(server).await?.test();

        let mut resp = tc.get("/healthcheck").send().await?;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.body_string().await?, "OK");

        Ok(())
    })
    .await
}
