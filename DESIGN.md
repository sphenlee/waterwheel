Waterwheel
==========

(Very rough thoughts)

Data Model
----------

* Projects
    * Contain jobs
    * Access control unit

* Jobs
    * Contain nodes
	* Unit of "update" in the API - submit jobs as a whole
	* Need a YAML/TOML/some other format representation

* Nodes
	* Can be tasks, or triggers, (decisions or sinks?)
	* Trigger nodes create tokens on a schedule
	* Tokens flow between nodes like a petri net
	* Task nodes execute once they get a token from each incoming edge, and on success they generate a token for each outgoing edge
	* Task failure generates tokens for each outgoing failure edge
	* Node triggers when its threshold is reached
		* Default threshold is number of incoming success edges, or 1 if there are none
		* Failure edges don't count for thresholds - so any number of failure edges
		  will give a threshold of 1
		* Threshold can be changed to create interesting workflows
	* It's an error for a task to have zero incoming edges - the task can never run
