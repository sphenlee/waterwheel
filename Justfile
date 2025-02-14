# show this help
help:
    just --list

# start all the dependencies locally
up:
    docker-compose up -d

# stop dependencies - destroy all local state too
down:
    docker-compose rm --stop -v --force

# connect to the local database interactively
psql:
    psql ${WATERWHEEL_DB_URL}

