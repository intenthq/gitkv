# git-puller

Clones the given list of git repositories into the specified docker volume.

## Usage

```
docker run
  -v <VOLUME_NAME>:/git-puller
  -e GIT_REPOS=<GIT_REPOS>
  -e PRIVATE_KEY_PATH=<PRIVATE_KEY_PATH>
  [-e PRIVATE_KEY_PASS=<PRIVATE_KEY_PASS>]
  [-e GIT_CLONE_BARE=yes]
  git-puller
```

## Parameters

- VOLUME_NAME:      The name of the docker volume in which you want the git repositories to be cloned to.
- GIT_REPOS:        A comma separated list of git repositories to clone.
- PRIVATE_KEY_PATH: The path to the private key.
- PRIVATE_KEY_PATH: Optional. The passphrase to the private key.
- GIT_CLONE_BARE:   Optional. If defined, does a bare git clone.

## Examples

In its simplest form it can be run with:

```
docker run
  -v gitkv-volume:/git-puller
  -v ~/.ssh/id_rsa:/id_rsa
  -e GIT_REPOS=git@github.com:intenthq/gitkv.git,git@github.com:intenthq/anon.git
  -e PRIVATE_KEY_PATH=/id_rsa
  git-puller
```

Providing a private key passphrase is supported by using the `PRIVATE_KEY_PASS` env variable:

```
docker run (...) -e PRIVATE_KEY_PASS=“supersecretpass” git-puller
```

If the env var `GIT_CLONE_BARE` is defined, a bare clone is done instead of a regular clone.
