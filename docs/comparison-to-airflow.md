Comparison to Airflow
=====================

Waterwheel fulfils a similar role to Apache Airflow but has many different 
design decisions based on my experience of running Airflow.

(This is not meant as a criticism of Airflow; many of their decisions made 
sense at the time, and many still do - Waterwheel just makes different choices).

A: **Unit of work is a DAG**. DAGs have an owner but there is no container for 
DAGs.

W: **Unit of work is a Job**. Jobs are grouped into Projects which are used 
for access controls.   


A: **Secrets stored globally**. No way to scope a secret to a DAG.

W: **Secrets can be global or project scoped**. Teams can store credentials without
grating access to all users.                                       


A: **DAGs are built from Python code**. Tasks are also Python code, so it can 
be hard to see what code will execute in the scheduler and what is in the 
executor. To get the definition of a DAG you have to execute the Python code.

W: **Jobs are a JSON document**. No user code executes in the scheduler. Job 
documents can be stored or generated using code at deployment time.


A: **Tasks run on worker node** which means there is no isolation between 
different tasks. `KubernetesOperator` is recommended to solve this.

W: **Tasks run in Docker or Kubernetes** and are isolated by default.


A: **DAGS are independent**. This means either:                                
   1. create a few large DAGs which is hard to develop and maintain
   2. create many small DAGs and use schedule times to coordinate.          
      This is error-prone and delays can break this.
   3. Use the `ExternalTaskSensor` which feels like a hack

W: **Jobs may depend on tasks in other jobs**. Complex workflows can be divided
into logical groups and different teams can work independently and declare 
dependencies between their jobs. Reruns in a job automatically trigger 
reruns on downstream tasks in any other job.                              


A: **Basic API.** DAGs are created by uploading code to the server. APIs for 
inspecting state and performing admin tasks are incomplete

W: **API first**. Jobs are only created via the API and all state and admin 
tasks have APIs.

A: Performance suffers as the number of tasks and DAGs increases.

W: Designed to scale to 1000s of jobs with 100s of tasks each.
