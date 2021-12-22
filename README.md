# wmt

wmt is a tool for checking if a cargo crate passes Adam
Johnson's [The Well-Maintained Test](https://adamj.eu/tech/2021/11/04/the-well-maintained-test/).

# What it does

For a given cargo crate (or cargo manifest), `wmt` checks if the crate (or dependencies in the manifest) passes all 12
tests (or a specific one).

# Available Commands

USAGE:

``wmt [OPTIONS] <SUBCOMMAND>``

OPTIONS:

```
-h, --help       Print help information
-j, --json       Output the result in JSON
-v, --verbose    Use verbose output
-V, --version    Print version information
```

SUBCOMMANDS:

```
check       Run a check for a dependency or a list of dependencies
help        Print this message or the help of the given subcommand(s)
question    Commands related to questions. Shows the available question or a specific one
```
