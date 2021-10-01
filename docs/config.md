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

> TODO: may need a different address for Worker -> API (for heartbeats) and 
> for Task -> API (for accessing stash variables)

    WATERWHEEL_SERVER_ADDR=<server's public URL>

### WATERWHEEL_SERVER_BIND, WATERWHEEL_WORKER_BIND
Address for the server and worker to listen on

To listen on all interfaces use "0.0.0.0". To choose any available port use "0"

    WATERWHEEL_SERVER_BIND=<address>:<port>
    WATERWHEEL_WORKER_BIND=<address>:<port>

Defaults:

    WATERWHEEL_SERVER_BIND=127.0.0.1:8080
    WATERWHEEL_WORKER_BIND=127.0.0.1:0

> Note that the default value for the server bind will not allow docker 
> containers or kubernetes pods to access the stash. You should specify the 
> specific address to bind to depending on your networking setup.

# Task settings

### WATERWHEEL_MAX_TASKS
The number of tasks for each worker to run in parallel
    
    WATERWHEEL_MAX_TASKS=<number>

Default is `8`

### WATERWHEEL_TASK_ENGINE
The task engine to use

    WATERWHEEL_TASK_ENGINE=<docker|kubernetes>

Default is `docker`

When using the `kubernetes` engine Waterwheel expects a `kubeconfig` file in 
the usual location: either the file specified by the `KUBECONFIG` 
environment variable or `$HOME/.kube/config` otherwise.

# Security Settings

### WATERWHEEL_HMAC_SECRET
The secret to be used for HMAC signing the Stash requests sent from workers 
to the server. Either this must be set, or both the public and private keys 
used for RSA signing must be set.

    WATERWHEEL_HMAC_SECRET=<shared secret value>

### WATERWHEEL_PUBLIC_KEY, WATERWHEEL_PRIVATE_KEY

Paths containing an RSA key pair used for signing the Stash requests sent 
from workers to the server. Either both public and private key must be 
provided, or an HMAC secret must be provided.

    WATERWHEEL_PUBLIC_KEY=public.pem
    WATERWHEEL_PRIVATE_KEY=private.pem

### WATERWHEEL_OPA_SIDECAR_ADDR
The address of the OPA sidecar used for authorization decisions.
For more information about configuring OPA see [Authorization](./auth.md)

    WATERWHEEL_OPA_SIDECAR_ADDR=http://localhost:8181/

Default is unset, in order to disable authorization checks you must also set
`WATERWHEEL_NO_AUTHZ=true`.

### WATERWHEEL_NO_AUTHZ

Set to `true` to disable authorization checks for API actions. This will *not* 
disable the authn/z checks between workers and the server which are used 
for controlling access to stash variables. This value must be `true` if the 
OPA sidecar address is unset, and is not recommended in production.

# Logging and debugging

### WATERWHEEL_STATSD_SERVER
Address of the statsd server to receive metrics data.

    WATERWHEEL_STATSD_SERVER=localhost:8081

### WATERWHEEL_JSON_LOG
Enable json logging output instead of the default formatted logs.

    WATERWHEEL_JSON_LOG=true

Default is `false`

### WATERWHEEL_LOG, RUST_BACKTRACE

Control log output and capturing backtraces. You shouldn't need to change 
these except when debugging Waterwheel. Defaults as shown below.

    # Rust logging settings
    WATERWHEEL_LOG="waterwheel=info,sqlx=warn,highnoon=warn"
    # Disable backtraces on unhandled errors
    RUST_BACKTRACE=0


# Example Configurations

Minimal:

    WATERWHEEL_DB_URL=postgres://postgres:${POSTGRES_PASSWORD}@localhost/
    WATERWHEEL_SERVER_ADDR=http://localhost:8080/
    WATERWHEEL_HMAC_SECRET=${SECRET_DATA}
    WATERWHEEL_NO_AUTHZ=true

Everything:

> Uses Kubernetes with an RSA keypair for Stash access

    WATERWHEEL_DB_URL=postgres://postgres:${POSTGRES_PASSWORD}@localhost/
    WATERWHEEL_SERVER_ADDR=http://localhost:8080/
    WATERWHEEL_OPA_SIDECAR_ADDR=http://localhost:8181/
    WATERWHEEL_SERVER_BIND=127.0.0.1:8080
    WATERWHEEL_WORKER_BIND=127.0.0.1:0
    WATERWHEEL_MAX_TASKS=8
    WATERWHEEL_TASK_ENGINE=kubernetes
    WATERWHEEL_KUBE_NAMESPACE=my-ns
    WATERWHEEL_PUBLIC_KEY=public.pem
    WATERWHEEL_PRIVATE_KEY=private.pem
    WATERWHEEL_JSON_LOG=true
    WATERWHEEL_LOG="waterwheel=info,sqlx=warn,highnoon=warn"
    RUST_BACKTRACE=0
