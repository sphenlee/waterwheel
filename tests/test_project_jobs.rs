use axum_test::TestServer;
use axum::http::StatusCode;
use serde_json::json;
use waterwheel::server::api::make_app;

mod common;

#[tokio::main]
#[test]

pub async fn test_project_jobs() -> anyhow::Result<()> {
    common::with_external_services(|config| async {
        let tc = TestServer::new(make_app(config).await?);

        let project_uuid = "00000000-0000-0000-0000-000000000000";
        let project_name = "integration_tests";

        // CREATE A PROJECT
        let resp = tc
            .post("/api/projects")
            .json(&json!({
              "uuid": project_uuid,
              "name": project_name,
              "description": "Project used for integration tests"
            }))
            .await;

        resp.assert_status(StatusCode::CREATED);

        // CREATE A JOB
        let job1_uuid = "00000000-0000-0000-0000-000000000001";
        let job1_name = "test_job_1";
        let job1 = json!({
            "uuid": job1_uuid,
            "name": job1_name,
            "project": project_name,
            "description": "A test job",
            "paused": false,
            "triggers": [],
            "tasks": [],
        });

        let resp = tc.post("/api/jobs").json(&job1).await;

        resp.assert_status(StatusCode::CREATED);

        // CREATE ANOTHER JOB
        let job2_uuid = "00000000-0000-0000-0000-000000000002";
        let job2_name = "test_job_2";
        let job2 = json!({
            "uuid": job2_uuid,
            "name": job2_name,
            "project": project_name,
            "description": "A test job",
            "paused": false,
            "triggers": [],
            "tasks": [],
        });

        let resp = tc.post("/api/jobs").json(&job2).await;
        
        resp.assert_status(StatusCode::CREATED);

        // LIST JOBS
        let resp = tc
            .get(&format!("/api/projects/{}/jobs", project_uuid))
            .await;

        let expected_list = json!([
            {
                "job_id": job1_uuid,
                "name": job1_name,
                "description": "A test job",
                "paused": false,
                "success": 0,
                "running": 0,
                "failure": 0,
                "waiting": 0,
                "error": 0,
            },
            {
                "job_id": job2_uuid,
                "name": job2_name,
                "description": "A test job",
                "paused": false,
                "success": 0,
                "running": 0,
                "failure": 0,
                "waiting": 0,
                "error": 0,
            },
        ]);
        resp.assert_status_ok()
            .assert_json(&expected_list);
        
        
        // GET A JOB BY NAME
        let resp = tc
            .get(&format!(
                "/api/projects/{}/jobs?name={}",
                project_uuid, job1_name
            ))
            .await;
        
        let expected_list = json!([
            {
                "job_id": job1_uuid,
                "name": job1_name,
                "description": "A test job",
                "paused": false,
                "success": 0,
                "running": 0,
                "failure": 0,
                "waiting": 0,
                "error": 0,
            },
        ]);
        resp.assert_status_ok()
            .assert_json(&expected_list);
        
        // GET A NON-EXISTENT JOB BY NAME
        let resp = tc
            .get(&format!(
                "/api/projects/{}/jobs?name={}",
                project_uuid, "idontexist"
            ))
            .await;

        let expected_list = json!([]);
        resp.assert_status_ok()
            .assert_json(&expected_list);

        Ok(())
    })
    .await
}
