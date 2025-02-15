# taskmaster
Job control daemon inspired by [supervisord](https://supervisord.org/index.html).

## Goal
The idea is to build a daemon, configurable to manage background jobs reliably and with customizable options. 

Its key components are:
- `taskmaster`, daemon managing the jobs.
- `taskshell`, shell communicating commands to the daemon via UNIX sockets.
