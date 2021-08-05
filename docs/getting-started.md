Getting Started
===============

## Install Waterwheel

Waterwheel depends on a PostgreSQL database and a RabbitMQ message broker.
You can configure these manually or use the `docker-compose.yml` if you want 
to try out Waterwheel locally.

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

