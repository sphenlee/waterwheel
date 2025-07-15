use highnoon::StatusCode;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use waterwheel::server::api::make_app;

mod common;

#[tokio::main]
#[test]

pub async fn test_project_jobs() -> highnoon::Result<()> {
    common::with_external_services(|config| async {
        let tc = make_app(config).await?.test();

        let project_uuid = "00000000-0000-0000-0000-000000000000";
        let project_name = "integration_tests";

        // CREATE A PROJECT
        let resp = tc
            .post("/api/projects")
            .json(json!({
              "uuid": project_uuid,
              "name": project_name,
              "description": "Project used for integration tests"
            }))?
            .send()
            .await?;

        assert_eq!(resp.status(), StatusCode::CREATED);

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

        let resp = tc.post("/api/jobs").json(job1)?.send().await?;
        assert_eq!(resp.status(), StatusCode::CREATED);

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

        let resp = tc.post("/api/jobs").json(job2)?.send().await?;
        assert_eq!(resp.status(), StatusCode::CREATED);

        // LIST JOBS
        let mut resp = tc
            .get(format!("/api/projects/{}/jobs", project_uuid))
            .send()
            .await?;
        let job_list: Value = resp.body_json().await?;
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
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(job_list, expected_list);

        // GET A JOB BY NAME
        let mut resp = tc
            .get(format!(
                "/api/projects/{}/jobs?name={}",
                project_uuid, job1_name
            ))
            .send()
            .await?;
        let job_list: Value = resp.body_json().await?;
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
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(job_list, expected_list);

        // GET A NON-EXISTENT JOB BY NAME
        let mut resp = tc
            .get(format!(
                "/api/projects/{}/jobs?name={}",
                project_uuid, "idontexist"
            ))
            .send()
            .await?;
        let job_list: Value = resp.body_json().await?;
        let expected_list = json!([]);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(job_list, expected_list);

        Ok(())
    })
    .await
}
