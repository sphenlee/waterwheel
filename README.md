Waterwheel
==========

Waterwheel is a job scheduler similar to Airflow but with a very different design.

Developing Locally
--------------------

Waterwheel is built with Rust and Cargo.

Create a `.env` file with the mandatory config settings:

```
# docker compose will use this when creating the database:
POSTGRES_PASSWORD=password

WATERWHEEL_DB_URL=postgres://postgres:${POSTGRES_PASSWORD}@localhost/
WATERWHEEL_SERVER_ADDR=http://localhost:8080/
WATERWHEEL_HMAC_SECRET=shared secret
```

Now launch the services (Postgres and RabbitMQ):

```
docker-compose up -d
```

The web interface is built with npm and webpack (and nvm for installing node).
This will run webpack in watch mode, so it recompiles when you edit any files.

```
cd ui
nvm use
npm install
npm run watch
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

The release binary embeds the UI, so you need to build it first:

```
cd ui
nvm use
npm run build

cd ..
cargo build --release

stat ./target/release/waterwheel
```

> Many of these commands are in a Justfile, see https://github.com/casey/just#packages
