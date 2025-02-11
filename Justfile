# show this help
help:
    just --list

# start all the dependencies locally
up:
    docker-compose up -d

# stop dependencies - destroy all local state too
down:
    docker-compose rm --stop -v --force

# do a full release build (builds the UI too, to bypass this just call `cargo build` directly)
build: 
    bazel build -c opt //:waterwheel

# build Waterwheel into a docker image for local use
package:
    bazel run //:waterwheel_load

# connect to the local database interactively
psql:
    psql ${WATERWHEEL_DB_URL}

