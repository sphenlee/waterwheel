Waterwheel Roadmap
==================

High level overview of things to be done.
Not necessarily in order, or committed.

Features
--------

* [ ] show task logs in the web UI
    * need to decide if waterwheel is going to be opinionated about the logging server (currently Vector, but should be
      compatible with FluentD too)
    * [ ] display logs after job completion
    * [ ] tail logs of a running job
* [x] overview interface to show recent job runs (like the box view of Airflow but less ugly)
    * plenty more improvements to be made here
* [ ] APIs for activating tasks based on query criteria (eg. past/future)
* [x] keep history of jobs runs, record task attempts
    * need to consider the data model to get this right    
* [ ] built in task retries - you can do this currently with cyclic graphs ;)
* [ ] job concurrency - limit backfills from flooding the queue
* [x] task value stash
    * to replace Airflow's xcom, variables and connections
    * most likely needs to be an HTTP API exposed to each container
* [ ] task routing - send tasks to specific workers to support workers running on "privileged" hardware
    * maybe just based on projects, or maybe fully custom (with separate ACLs to control it)
* [ ] ACLs
    * [ ] Web UI logins and edit/view permissions
    * [ ] API permissions - CRUD operations
    * considering using an ACL language like Rego or Casbin

Operational
-----------

* [ ] emit metrics to statsd
* [ ] better control over server and worker logs (send them to fluentd/Vector too?)
* [ ] High Availability
    * [x] separate the server from the web interface
        * update messages are sent from the api to the scheduler over AMQP
    * [ ] server is stateless, but has in-memory caches, verify and test
    * [ ] HA mode for the server
        * initially active/passive mode, active/active in future?
        
Bug Fixes / Paper Cuts
----------------------

* [ ] task backfills with cross-job dependencies
    * need to check the cross-job tasks for status and possibly trigger tasks