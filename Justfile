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
ui-watch: ui-install
    cd ui && npm run watch

# build the UI for a release
ui-build: ui-install
    cd ui && npm run build

# do a full release build (builds the UI too, to bypass this just call `cargo build` directly)
build: ui-build
    cargo build --release

# build Waterwheel into a docker image for local use
package:
    DOCKER_BUILDKIT=1 docker build . -t waterwheel:local

# connect to the database interactively
psql:
    psql ${WATERWHEEL_DB_URL}

