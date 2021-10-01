Authentication and Authorization
================================

Waterwheel offloads Authn and Authz to external processes/sidecars.

## Authentication

Waterwheel expects to be deployed behind an authentication proxy sidecar 
such as [Ory Oathkeeper](https://www.ory.sh/oathkeeper/) or
[envoy](https://www.envoyproxy.io/).

URLs in Waterwheel are grouped according to what authentication should be 
applied.

 * `/api/*` are used by end users (either the web interface or directly) and 
   should accept some kind of token for scripting use, and a session 
   cookie for web interface use
 * `/int-api/*` are internal APIs used for communication between workers and 
   the server. These do not need Authentication as they will have a JWT token 
   which is validated by Waterwheel
 * `/*` all other paths are used for serving the web interface and should 
   use a session cookie and redirect to the login system if not provided

## Authorization

Waterwheel asks the Open Policy Agent to make authorization decisions. The 
decision endpoint is `/v1/data/waterwheel/authorize` and thus corresponds to 
a rule `authorize` in the `package waterwheel` rego file.

The `input` will be set to the context of the current request:

```json
{
  "object": {
    "project_id": "<project uuid>",
    "job_id": "<job uuid>",
    "kind": "project|job|stash|workers|status"
  },
  "principal": {
    "bearer": "<bearer token if present>"
  },
  "action": "Get|List|Update|Delete",
  "http": {
    "method": "<http method (uppercase)>",
    "headers": {
      "<header name>": "<header value>",
      ...
    }
  }
}
```

`project_id` and `job_id` will be unset in contexts where they aren't available.
For example neither will be set when listing projects. Only `project_id` 
will be set when listing jobs in a project, and both are set when listing 
tasks in a job.

`http.headers` are provided to allow any custom headers to be used for 
determining the principal of the request. `principal.bearer` is a 
convenience for using bearer tokens.
