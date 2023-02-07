# show this help
help:
    just --list

# start all the dependencies locally
up:
    docker-compose up -d

# stop dependencies - destroy all local state too
down:
    docker-compose rm --stop -v --force

# install node dependencies for building the UI
ui-install:
    cd ui && npm install

# build the UI in watch mode (recompile on file changes)
ui-watch:
    cd ui && npm run watch

# build the UI for a release
ui-build:
    cd ui && npm run build

# do a full release build
build:
    @# (don't depend on ui-build because webpack writes the output files even
    @# if there are no changes. This causes cargo to have to rebuild the binary,
    @# even if no Rust code has changed either)
    cargo build --release

# build Waterwheel into a docker image for local use
package:
    DOCKER_BUILDKIT=1 docker build . -t waterwheel:local

# connect to the database interactively
psql:
    psql ${WATERWHEEL_DB_URL}

# run a cargo command in a musl environment
musl +ARGS:
    docker run \
        -v $PWD/.musl-cargo-cache:/root/.cargo/registry \
        -v "$PWD:/volume" \
        --rm \
        -it \
        clux/muslrust:nightly \
        cargo {{ARGS}}
