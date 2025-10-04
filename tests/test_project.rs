use std::time::Duration;

use highnoon::StatusCode;
use lapin::{
    options::{BasicGetOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use tokio::time::timeout;
use waterwheel::server::{Server, api::make_app};

mod common;

#[tokio::main]
#[test]
pub async fn test_project() -> highnoon::Result<()> {
    common::with_external_services(|config| async {
        let server = Server::new(config.clone()).await?;

        let tc = make_app(config).await?.test();

        // (setup a queue to receive the config updates - these are a fanout broadcast so
        // no queue is subscribed by default)
        let amqp_chan = server.amqp_conn.create_channel().await?;
        let queue = amqp_chan
            .queue_declare(
                "",
                QueueDeclareOptions {
                    auto_delete: true,
                    exclusive: true,
                    ..QueueDeclareOptions::default()
                },
                FieldTable::default(),
            )
            .await?;
        amqp_chan
            .queue_bind(
                queue.name().as_str(),
                "waterwheel.config",
                "",
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        // CREATE A PROJECT
        let project = json!({
              "uuid": "00000000-0000-0000-0000-000000000000",
              "name": "integration_tests",
              "description": "Project used for integration tests"
        });

        let resp = tc.post("/api/projects").json(project)?.send().await?;

        assert_eq!(resp.status(), StatusCode::CREATED);

        // CHECK FOR CONFIG UPDATE MESSAGE
        let get = amqp_chan
            .basic_get(queue.name().as_str(), BasicGetOptions::default());

        let msg = timeout(Duration::from_secs(30), get)
            .await??
            .expect("no message on the config update queue");
        let data = String::from_utf8(msg.delivery.data)?;
        assert_eq!(
            data,
            r#"{"Project":"00000000-0000-0000-0000-000000000000"}"#
        );

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
        let mut resp = tc
            .get("/api/projects?name=integration_tests")
            .send()
            .await?;
        let proj_list: Value = resp.body_json().await?;
        let expected_project = json!({
              "id": "00000000-0000-0000-0000-000000000000",
              "name": "integration_tests",
              "description": "Project used for integration tests"
        });
        assert_eq!(proj_list, expected_project);

        let resp = tc.get("/api/projects?name=no_such_name").send().await?;
        assert_eq!(resp.status(), highnoon::StatusCode::NOT_FOUND);

        Ok(())
    })
    .await
}
