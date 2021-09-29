# Jobs

A job is the basic unit that can be created in Waterwheel.
A job is made up of identification, metadata, triggers and tasks.

An example job: [demo.yml](../sample/jobs/demo.yml)

## Identification

A job has a UUID and a name. The UUID uniquely identifies the job, but the name
is used when jobs refer to each other. The UUID must be unique within the entire
Waterwheel deployment; the name must be unique within a project.

## Metadata

Jobs have a description field which is freeform text. It's likely more 
metadata fields will be added over time.

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

## Tasks

Tasks represent work to be executed. A task is executed when it 