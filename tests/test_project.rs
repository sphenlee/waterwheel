use pretty_assertions::assert_eq;
use highnoon::StatusCode;
use serde_json::{json, Value};
use testcontainers::Docker;
use waterwheel::server::api::make_app;
use waterwheel::server::Server;
use waterwheel::config;

mod common;

#[tokio::main]
#[test]
pub async fn test_project() -> highnoon::Result<()> {
    common::with_external_services(|| async {
        let config = config::load()?;
        let server = Server::new(config).await?;

        let tc = make_app(&server).await?.test();

        // CREATE A PROJECT
        let project = json!({
              "uuid": "00000000-0000-0000-0000-000000000000",
              "name": "integration_tests",
              "description": "Project used for integration tests"
        });

        let resp = tc.post("/api/projects")
            .json(project)?
            .send()
            .await?;

        assert_eq!(resp.status(), StatusCode::CREATED);

        // LIST ALL PROJECTS
        let mut resp = tc.get("/api/projects").send().await?;
        let proj_list: Value = resp.body_json().await?;
        let expected_list = json!([
            {
                "id": "00000000-0000-0000-0000-000000000000", // TODO - consistency, why id here, uuid above?
                "name": "integration_tests",
                "description": "Project used for integration tests"
            }
        ]);
        assert_eq!(proj_list, expected_list);

        // GET PROJECT BY NAME
        let mut resp = tc.get("/api/projects?name=integration_tests").send().await?;
        let proj_list: Value = resp.body_json().await?;
        let expected_project = json!({
              "id": "00000000-0000-0000-0000-000000000000",
              "name": "integration_tests",
              "description": "Project used for integration tests"
        });
        assert_eq!(proj_list, expected_project);

        let mut resp = tc.get("/api/projects?name=no_such_name").send().await?;
        assert_eq!(resp.status(), highnoon::StatusCode::NOT_FOUND);

        Ok(())
    }).await
}