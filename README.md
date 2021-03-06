# Gitkv [![Build Status](https://travis-ci.org/intenthq/gitkv.svg?branch=master)](https://travis-ci.org/intenthq/gitkv)[![codecov](https://codecov.io/gh/intenthq/gitkv/branch/master/graph/badge.svg)](https://codecov.io/gh/intenthq/gitkv)

Gitkv is a server for using git as a key value store for text files

## Installation

### Binary

Releases of Gitkv are available as pre-compiled static binaries on the corresponding GitHub release. Simply download the appropriate build for your machine and make sure it's in your PATH (or use it directly).

### Docker

Gitkv is also distributed as a Docker image that can be pulled from [Docker Hub](https://hub.docker.com/r/intenthq/gitkv):

```sh
docker run intenthq/gitkv
```

### Source

To run Gitkv from source first [install Rust](https://www.rust-lang.org/tools/install).

This is a standard Cargo project — [here is a link to the Rust documentation on how to use Cargo](https://doc.rust-lang.org/cargo/), but some common tasks you may wish to use are as follows:

* `cargo build` — build the Gitkv binary, but in debug mode (unoptimised).
* `cargo build --release` — same as the above, but in release mode (optimised).
* `cargo run` — build and run the binary in one step.
* `cargo test` — run the tests.

## Usage

```
gitkv is a server for using git as a key value store for text files

USAGE:
    gitkv [OPTIONS]

FLAGS:
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -h, --host <HOST>         host to listen to [default: localhost]
    -p, --port <PORT>         port to listen to [default: 7791]
    -r, --repo-root <PATH>    path where the different repositories are located [default: ./]
```

You can modify the amount of logging with the `RUST_LOG` parameter:

For basic application info (default): `RUST_LOG=gitkv=info ./gitkv`  
Including incoming HTTP requests: `RUST_LOG=info ./gitkv`  
For more information check [env_logger](https://docs.rs/env_logger/*/env_logger/index.html)'s documentation.

## Security

Note that git stores all the content plain so that it's not a good place to store secrets and sensitive information.

There are solutions that offer [encrypted git](https://keybase.io/blog/encrypted-git-for-everyone), but we do recommend to store the secrets using a different solution like [Vault](https://www.vaultproject.io/).

## When is it useful?

This server can be used when you need a data store that can easily support:
- Small to medium text based data
- Versioning of this data
- Data follows some kind of hierarchy
- Access using HTTP + Ability to use the configuration without a central server (just the git repo itself)
- And you can't use GitHub/GitLab api directly

### Why git?

[Git](https://git-scm.com/) is an excellent version control system with lots of tooling around to compare files, have approval mechanisms for the data you store (i.e. pull requests) or to have different flows for editing your files.

Although it's not designed with performance in mind, for some use cases like pulling configuration files with a specific version.

Some people has previously mentioned the idea to use [git as a database](https://www.kenneth-truyers.net/2016/10/13/git-nosql-database/) with some pretty interesting thoughts.

### Alternatives/Similar projects

- GitHub/GitLab/Bitbucket APIs
- https://github.com/gitpython-developers/gitdb
- https://github.com/attic-labs/noms
- https://github.com/mirage/irmin
- http://orpheus-db.github.io/
- https://www.klonio.com/

## How to contribute

Any contribution will be welcome, please refer to our [contributing guidelines](CONTRIBUTING.md) for more information.

# License

This project is [licensed under the MIT license](LICENSE).
