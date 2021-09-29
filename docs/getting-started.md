Getting Started
===============

This document will describe how to get a local version of Waterwheel running 
as a demo.

## Prerequisites

 * [Cargo toolchain](https://rustup.rs/)
 * [Just command runner](https://github.com/casey/just#packages)
 * [Docker](https://docs.docker.com/get-docker/)
 * Curl

## Setup Waterwheel

There are no builds available for Waterwheel yet, so you will have to build it
locally. Run `just build`.

Waterwheel depends on a PostgreSQL database and a RabbitMQ message broker.
For a production deployment you would configure these externally, but if you 
just want to try out Waterwheel you can use the `docker-compose.yml` 
file to run them locally. Run `just up` to launch these.

Create a `.env` file with the mandatory config settings:

```shell
# docker compose will use this when creating the database:
POSTGRES_PASSWORD=password

WATERWHEEL_DB_URL=postgres://postgres:${POSTGRES_PASSWORD}@localhost/
WATERWHEEL_SERVER_ADDR=http://localhost:8080/
WATERWHEEL_HMAC_SECRET=shared secret # set this to something random
WATERWHEEL_NO_AUTHZ=true # disable authz for the demo
```

Now you can start the Waterwheel server. This process schedules jobs, provides
the REST API for creating jobs and serves a web interface.

```shell
./target/release/waterwheel server
```

Check the web interface at [localhost:8080](http://localhost:8080). There 
won't be much to see because there are no jobs created.

## Create Jobs

First create a project:

```shell
curl -XPOST http://localhost:8080/api/projects -d@./sample/project.json
```

Now create a job:

```shell
curl -XPOST http://localhost:8080/api/jobs -d@./sample/jobs/simple.json
```

Back in the web interface you should see both of these have been created. No 
work has been done yet because there are no workers.

## Start working

Launch a worker to begin processing tasks. In a new terminal:

```shell
./target/release/waterwheel worker
```
