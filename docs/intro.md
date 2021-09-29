# Introduction

## Basics

Waterwheel is a job scheduler. It manages a set of projects containing jobs.

Each job is a directed graph of triggers and tasks. The triggers generate 
"tokens" based on the schedule specified. The tokens then flow through the 
graph into tasks.

Tokens remember the scheduled time that triggered them. When a task receives 
a threshold number of tokens for the same time it fires and begins execution.

Task execution uses Docker images on either a Docker server or a Kubernetes 
cluster.

Task success and failure will create more tokens which flow into the graph 
to downstream tasks.

This concept is based on a Petri Net.

## Projects

A project is a container for jobs. It is the unit of access control in 
Waterwheel. Therefore, if a user has access to a project, they get access to 
all jobs in the project. It is suggested to create projects for each team.

Projects may store secrets such as database credentials or access tokens 
that are used by jobs into the "stash". A job may only read the stash from the 
project that contains it.

(There is also a global stash readable by all jobs, and a job stash 
explained in the next section).

## Jobs

A job is the unit for creating and updating. A whole job is created or 
updated as a unit. You cannot create or edit individual triggers and tasks.

Jobs are represented by a JSON document (but can also be written in YAML as 
the conversion is trivial).

A job may store values into a stash, but unlike global and project stash 
this stash is local to each execution time. This allows a job to store a 
value that is used by downstream tasks executing for the same scheduled time.

For details about writing jobs see [Jobs](./jobs.md).

