# show this help
help:
    just --list

# start all the dependencies locally
up:
    docker-compose up -d

# stop dependencies - destroy all local state too
down:
    docker-compose rm --stop -v --force

# connect to the database interactively
psql:
    psql postgres://postgres:${POSTGRES_PASSWORD}@localhost/

# cross compile to musl
compile-musl +ARGS:
    docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder:nightly-2021-02-13 {{ARGS}}
