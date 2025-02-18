# taskmaster
Job control daemon inspired by [supervisord](https://supervisord.org/index.html).

## Goal
The idea is to build a daemon, configurable to manage background jobs reliably and with customizable options. 

Its key components are:
- `taskmaster`, daemon managing the jobs.
- `taskshell`, shell communicating commands to the daemon via UNIX sockets.

## Challenges
### State Management
The first challenge I faced was managing the state of the processes efficiently. I broke down the possible states of any process to the following:
```rust
pub enum ProcessState {
    Idle,
    HealthCheck,
    Running,
    Failed,
    WaitingForRetry(time::Instant),
    Completed,
}
```
---
The states and their transition triggers can be represented as follows:

![alt text](assets/image.png)
---
This lays out a rough process for decision making during daemon execution. We can easily define those states and their transitioning rules in code. 