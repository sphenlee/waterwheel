Waterwheel Internals
====================

Waterwheel is composed of two separate processes - the server and worker.
These communicate via the message queue (RabbitMQ) only. The server process 
accesses the database (PostgreSQL) to store state, but the worker has no 
access to this.

## Server

The Server itself is composed of several interacting tasks. These 
communicate via channels, and do not share any memory structures.
The tasks use `async` and are executed by the `tokio` multi-threaded runtime.

### Trigger Processor

This is the most complex of all the tasks, and is the most important part of 
Waterwheel.  When this task starts it first loads all the triggers from 
the database, and does a catchup for any that would have fired while it 
wasn't running. It holds the triggers in a priority queue, sorted by the 
next trigger time.  It then enters the scheduler loop:

1. First check if any *Trigger Updates* messages are pending.
   If there is an update it removes that trigger from the queue, reloads 
   the trigger's configuration from the database and then puts it back into the 
   queue. There may be multiple updates pending, and they are all processed 
   before moving on.
   
1. If the queue is now empty (because there were no triggers, or the trigger 
   updates removed the last one) then the **Trigger Processor** waits on the 
   *Trigger Update* channel. When an update happens it is handled and then 
   control returns to the start of the scheduler loop.
   
1. Now the queue is not empty, so the next trigger is popped from the queue. 
   The **Trigger Processor** checks if the target time has already passed:
   
    1. If the time has already passed then the trigger is activated, and its 
       next trigger time is re-queued. Control returns to the top of the 
       scheduler loop.
   
    1. If the trigger time has not passed, the **Trigger Processor** will either
       sleep until the target time, or until a *Trigger Update* is received.
        
        1. If the sleep time is reached then the trigger is activated, and its 
           next trigger time is re-queued
          
        1. If a *Trigger Update* is received then the trigger that was being 
           waited for is re-queued, then the updated trigger is handled
           
       In both cases control then returns to the top of the scheduler loop.
    

To activate a trigger the **Trigger Processor** finds each task that depends on 
the trigger and increments its token in the database. Then the trigger's last
trigger time is updated. Finally, it sends an *Increment Token* message to 
the Token Processor for each token.


### Token Processor

The **Token Processor** holds a map of Tokens to 