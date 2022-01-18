# Jobs

A job is the basic unit that can be created in Waterwheel.
A job is made up of identification, metadata, triggers and tasks.

An example job: [simple.json](../sample/jobs/simple.json)

Jobs are stored as JSON documents, but if you're writing them by hand you may 
prefer to use a more friendly format such as YAML or JSON5. These formats 
can be easily converted into JSON by the deployment process. Alternatively you
may wish to generate the JSON using a data specific language (eg. Jsonnet) 
or a general purpose language (eg. Javascript or Python).

> Examples below are fragments of a YAML document

## Identification

A job has a UUID and a name. The UUID uniquely identifies the job, but the name
is used when jobs refer to each other. The UUID must be unique within the entire
Waterwheel deployment; the name must be unique within a project.

A job also includes the name of project it is contained within.

```yaml
uuid: '01234567-89ab-cdef-0123-456789abcd'

project: example_project
name: example_job
```

## Metadata

Jobs have a description field which is freeform text. It's likely more 
metadata fields will be added over time.

There is a paused flag to temporarily prevent all triggers from firing. When 
a job is unpaused it will catch up on any triggers that were skipped while 
being paused.

```yaml
description: An Example Job
paused: false
```

## Triggers

Triggers are what cause a job to start executing. A trigger has a start time,
and optional end time and either a period or a cron expression.

The period is a duration specified as a number and a unit. eg. `1h`, `1d`, `30m`
The trigger will fire at the start time and then again at intervals of this 
period until the end time, or forever if no end time is set.

The cron expression may be used instead to give more control over scheduling.
The trigger will fire at the start time and then again based on the cron 
expression.

Triggers also have a name which is used to refer to them in tasks.

```yaml
triggers:
  - name: daily
    start: 2022-01-01T00:00:00Z
    period: 1d

  - name: start-of-month
    start: 2022-01-01T00:00:00Z
    cron: "0 0 1 * *"
```

## Tasks

Tasks represent work to be executed. A task specifies a Docker image, 
optional arguments and environment variables.

Tasks also specify their upstream dependencies which may be triggers or 
other tasks. When the upstream trigger fires, or the upstream tasks succeeds 
it will create a token for this task. Dependencies may also include 
failure tasks which will generate a token upon failing.

Dependencies are specified by name using 2, 3 or 4 slash separated parts. 
The parts are an optional project name, optional job name, one of the 
keywords 'task' or 'trigger' and finally the task or trigger name. If 
project is omitted it defaults to the same project as the current job. If 
job is omitted it defaults to the current job.

Examples of dependency values:

```
demo_project/demo_job/task/step1
other_job/trigger/daily
task/step2
trigger/every_hour
```

The threshold number of tokens needed to activate a task may be specified. If 
not, it will be determined as either the number of upstream success dependencies
(i.e. not including the failure dependencies) or 1 if there are only failure 
dependencies.

```yaml
tasks:
  - name: step1
    image: bash:latest
    args: ["-c", "echo step1"]
    depends:
      - trigger/daily

  - name: step2
    image: bash:latest
    args: ["-c", "env"]
    depends:
      - task/step1
  
  - name: fail
    image: my-custom-fail-image:v1
    env:
      MESSAGE: "example job failed!"
      MAILTO: "admin@example.com"
    depends_failure:
      - task/step1
      - task/step2
```

The full JSONSchema for Jobs is [here](./job-schema.json).
