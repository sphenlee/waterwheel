# show this help
help:
    just --list

# start Postgres and RabbitMQ locally
up:
    docker-compose up -d

# stop Postgres and RabbitMQ - destroy all local state too
down:
    docker-compose rm --stop -v --force

# Docker compose logs
logs:
    docker-compose logs

# connect to the database interactively
psql:
    psql postgres://postgres:${POSTGRES_PASSWORD}@localhost/
