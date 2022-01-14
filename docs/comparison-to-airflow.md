Comparison to Airflow
=====================

Waterwheel fulfils a similar role to Apache Airflow but has many different 
design decisions based on my experience of running Airflow.

(This is not meant as a criticism of Airflow; many of their decisions made 
sense at the time, and many still do - Waterwheel just makes different choices).

| Airflow                                 | Waterwheel                                  |
|-----------------------------------------|---------------------------------------------|
| **Unit of work is a DAG**. DAGs         | **Unit of work is a Job**. Jobs are         |
| have an owner but there is no           | grouped into Projects which are used for    |
| container for DAGs.                     | access controls.                            |
| ------------------------------------    | ------------------------------------------- |
| **Secrets stored globally**. No way     | **Secrets can be global or project          |
| to scope a secret to a DAG.             | scoped**. Teams can store credentials       |
| without grating access to all users.    |                                             |
| ------------------------------------    | ------------------------------------------- |
| **DAGs are built from Python code**.    | **Jobs are a JSON document**. No user code  |
| Tasks are also Python code so it        | executes in the scheduler.                  |
| can be hard to see what code will       |                                             |
| execute in the scheduer and what        | Job documents can be stored or generated    |
| is in the executor.                     | using code at deployment time.              |
| To get the definition of a DAG you      |                                             |
| have to execute the Python code         |                                             |
| ------------------------------------    | ------------------------------------------- |
| **Tasks run on worker node** which      | **Tasks run in Docker or Kubernetes**       |
| means there is no isolation between     | and are isolated by default.                |
| different tasks.                        |                                             |
| `KubernetesOperator` is recommended     |                                             |
| to solve this.                          |                                             |
| ------------------------------------    | ------------------------------------------- |
| **DAGS are independent**. This means    | **Jobs may depend on tasks in other         |
| either:                                 | jobs**. Complex workflows can be divided    |
| into logical groups and different teams |                                             |
| 1. create a few large DAGs which is     | can work independently and declare          |
| hard to develop and maintain            | dependencies between their jobs.            |
| 2. create many small DAGs and use       |                                             |
| schedule times to coordinate.           | Reruns in a job automatically trigger       |
| This is error-prone and delays          | reruns on downstream tasks in any           |
| can break this.                         | other job.                                  |
| 3. Use the `ExternalTaskSensor`         |                                             |
| which feels like a hack                 |                                             |
| ------------------------------------    | ------------------------------------------- |
| **Basic API.** DAGs are created by      | **API first**. Jobs are only created via    |
| uploading code to the server.           | the API and all state and admin tasks       |
| have APIs.                              |                                             |
| APIs for inspecting state and           |                                             |
| performing admin tasks are              |                                             |
| incomplete                              |                                             |
| ------------------------------------    | ------------------------------------------- |
| Performance suffers as the number       | Designed to scale to 1000s of jobs with     |
| of tasks and DAGs increases.            | 100s of tasks each.                         |
| ------------------------------------    | ------------------------------------------- |
