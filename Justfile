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

# run a cargo command in a musl environment
musl +ARGS:
    docker run \
        -v $PWD/.musl-cargo-cache:/root/.cargo/registry \
        -v "$PWD:/volume" \
        --rm \
        -it \
        clux/muslrust:nightly \
        cargo {{ARGS}}
