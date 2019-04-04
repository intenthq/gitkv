# Gitkv [![Build Status](https://travis-ci.org/intenthq/gitkv.svg?branch=master)](https://travis-ci.org/intenthq/gitkv)

Gitkv is a server for using git as a key value store for text files

## Installation

### Binary

Releases of Gitkv are available as pre-compiled static binaries on the corresponding GitHub release. Simply download the appropriate build for your machine and make sure it's in your PATH (or use it directly).

### Docker

Gitkv is also distributed as a docker image that can be pulled from [Docker Hub](https://hub.docker.com/r/intenthq/gitkv)

## Usage

```
gitkv is a server for using git as a key value store for text files

USAGE:
    gitkv [OPTIONS]

FLAGS:
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -h, --host <HOST>    host to listen to [default: localhost]
    -p, --port <PORT>    port to listen to [default: 7791]
```

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
