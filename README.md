# Rustic - Easy to setup CI-server

`rustic` is an easy to setup CI-server that focuses on simplicity and flexibility.

Projects are defined in a config-file along with the projects build-steps. 
It is up to the user to setup build-steps suitable for the threat-level of built
repositories.

A single user building some smaller hobby projects may be fine with just running
the build-steps as a regular user on the host-system.

A group of developers working on a shared project might want to isolate the commands
in the build-step, through a `chroot`, `docker`-container or other technology.

It is up to the user of `rustic` to decide.

# Config file

`rustic`s entire configuration resides in `rustic-conf.toml`.

