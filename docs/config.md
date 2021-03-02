Configuring Waterwheel
======================

Waterwheel takes all configuration via environment variables.

It is recommended that you create a `.env` file containing the variables 
when running locally, and use an `EnviromnentFile` when running in systemd.

# External Services

### WATERWHEEL_DB_URL
The address of the postgres database. *(Mandatory)*

    WATERWHEEL_DB_URL=postgres://<user>:<password>@<host>/

No default is provided as a Postgres password is required.

### WATERWHEEL_AMQP_ADDR
The address of the RabbitMQ broker.

    WATERWHEEL_AMQP_ADDR=amqp://<user>:<password>@<host>/<virtual host>

Default: `amqp://127.0.0.1:5672/%2f`

# Network settings

### WATERWHEEL_SERVER_ADDR
URL that tasks can contact the server on.  *(Mandatory)*

> TODO: there may be a way to autodetect this, and allow it to be optional

    WATERWHEEL_SERVER_ADDR=<server's public URL>

### WATERWHEEL_SERVER_BIND, WATERWHEEL_WORKER_BIND
Address for the server and worker to listen on

To listen on all interfaces use "0.0.0.0". To choose any available port use "0"

    WATERWHEEL_SERVER_BIND=<address>:<port>
    WATERWHEEL_WORKER_BIND=<address>:<port>

Defaults:

    WATERWHEEL_SERVER_BIND=127.0.0.1:8080
    WATERWHEEL_WORKER_BIND=127.0.0.1:0


# Task settings

### WATERWHEEL_MAX_TASKS
The number of tasks for each worker to run in parallel
    
    WATERWHEEL_MAX_TASKS=<number>

Default is `8`

### WATERWHEEL_TASK_ENGINE
The task engine to use

    WATERWHEEL_TASK_ENGINE=<docker|kubernetes>

Default is `docker`

# Security Settings

### WATERWHEEL_PUBLIC_KEY, WATERWHEEL_PRIVATE_KEY

Paths containing an RSA key pair used for signing the Stash requests sent 
from workers to the server. *(Mandatory)*

> TODO: These should not be mandatory for local testing, and only required in 
> a real deployment.

    WATERWHEEL_PUBLIC_KEY=public.pem
    WATERWHEEL_PRIVATE_KEY=private.pem

# Logging and debugging

### WATERWHEEL_JSON_LOG
Enable json logging output instead of the default formatted logs.

    WATERWHEEL_JSON_LOG=true

Default is `false`

### RUST_LOG, RUST_BACKTRACE

Control log output and capturing backtraces. You shouldn't need to change 
these except when debugging Waterwheel. Defaults as shown below.

    # Rust logging settings
    RUST_LOG="waterwheel=info,sqlx=warn,highnoon=warn"
    # Enable backtraces on unhandled errors
    RUST_BACKTRACE=0


# Example Configurations

Minimal:

    WATERWHEEL_DB_URL=postgres://postgres:${POSTGRES_PASSWORD}@localhost/
    
    WATERWHEEL_SERVER_ADDR=http://localhost:8080/
    
    WATERWHEEL_PUBLIC_KEY=public.pem
    WATERWHEEL_PRIVATE_KEY=private.pem

