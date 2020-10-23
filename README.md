# Fast Thread-pool

This thread-pool implementation is optimized to minimize latency.
In particular, you are guaranteed never to pay the cost of thread spawn before your task is executed.
New threads are spawned only during the "useless time" of the workers (for example, after returning the result of a job).
The only possible case of latency is the lack of "available" workers.
To minimize the probability of this case, this thread-pool constantly keeps a certain number of workers available (configurable).
This implementation allows you to wait for the result of a job asynchronously,
so you can use it as a replacement for the `spawn_blocking` function of your async runtime.
