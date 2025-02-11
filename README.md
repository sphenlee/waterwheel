Waterwheel
==========

Waterwheel is a job scheduler similar to Airflow but with a very different design.

For more information see [here](./docs/intro.md).

To get started with a basic deployment see [here](./docs/getting-started.md).

Developing Locally
--------------------

Waterwheel is built using Rust.
Command automation uses [Just](https://github.com/casey/just#packages).
Build system is [Bazel](https://bazel.build/)

Create a `.env` file with the mandatory config settings:

```
# docker compose will use this when creating the database:
POSTGRES_PASSWORD=password

WATERWHEEL_DB_URL=postgres://postgres:${POSTGRES_PASSWORD}@localhost/
WATERWHEEL_SERVER_ADDR=http://localhost:8080/
WATERWHEEL_HMAC_SECRET=shared secret
WATERWHEEL_NO_AUTHZ=true
```

Now launch the services (Postgres and RabbitMQ):

```
just up
```

The web interface is built with bazel

```
bazel build //ui
```

In two separate terminals build and launch Waterwheel server and worker:

```
# run the server
cargo run server

# in a separate terminal
cargo run worker
```

Building a Release binary
--------------------------

Build a release binary using bazel:

```
just build
```

Build a docker image for local use:

```
just package
```

> Other commands are available in the Justfile. Run `just help` for a list.
