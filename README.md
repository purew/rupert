# Rupert - Flexible and simple CI-server built in Rust

`rupert` is an easy to setup CI-server that focuses on simplicity and flexibility.

Projects are defined in a config-file along with the projects build-steps. 
It is up to the user to setup build-steps suitable for the threat-level of built
repositories.

A single user building some smaller hobby projects may be fine with just running
the build-steps as a regular user on the host-system.

A group of developers working on a shared project might want to isolate the commands
in the build-step, through a `chroot`, `docker`-container or similar technology.

It is up to the user of `rupert` to decide.

# Config file

`rupert`s entire configuration resides in `rupert-conf.toml`.

