Getting Started
===============

## Install Waterwheel

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
WATERWHEEL_HMAC_SECRET=shared secret
```

Now you can start the Waterwheel server. This process schedules jobs, provides
the REST API for creating jobs and serves a web interface.

```shell
./target/release/waterwheel server
```

Check the web interface at [localhost:8080].

