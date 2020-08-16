# show this help
help:
    just --list

up:
    docker-compose up -d

down:
    docker-compose rm --stop -v --force

logs:
    docker-compose logs

psql:
    psql postgres://postgres:${POSTGRES_PASSWORD}@localhost/
