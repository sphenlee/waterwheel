Waterwheel
==========

Waterwheel is a job scheduler similar to Airflow but with a very different design.

Developing Locally
--------------------

Waterwheel is built with Rust and Cargo.

```
# start the services
docker-compose up -d

# run the server
cargo run server

# in a separate terminal
cargo run worker
```

The web interface is built with npm and webpack (and nvm for installing node):

```
cd ui
nvm use
npm install
npm run watch
```

This will run webpack in watch mode so it recompiles when you edit any files.

Building a Release binary
--------------------------

The release binary embeds the UI so you need to build it first:

```
cd ui
nvm use
npm run build

cd ..
cargo build --release

stat ./target/release/waterwheel
```
