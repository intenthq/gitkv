# git-puller

Clones the given list of git repositories into the specified docker volume.

## Usage

```
docker run
  -v <VOLUME_NAME>:/git-repos \
  -e GIT_REPOS=<GIT_REPOS> \
  -e PRIVATE_KEY_PATH=<PRIVATE_KEY_PATH> \
  -e PRIVATE_KEY_PASS=<PRIVATE_KEY_PASS> \
  [-e GIT_CLONE_BARE=yes] \
  intenthq/git-puller
```

Note that by default the image will expect the volume to be mounted in the `/git-repos` directory inside the container and will throw an error if this directory doesn't exist.

## Arguments

The image supports the following environment variables:

- `VOLUME_NAME`: The name of the docker volume in which you want the git repositories to be cloned to.
- `GIT_REPOS`: A comma separated list of git repositories to clone.
- `PRIVATE_KEY_PATH`: The path to the private key.
- `PRIVATE_KEY_PASS`: The passphrase for the private key.
- `GIT_CLONE_BARE`: Optional. If defined, does a bare git clone.

## Examples

In its simplest form it can be run with:

```sh
docker run \
  -v gitkv-volume:/git-repos \
  -v ~/.ssh/id_rsa:/id_rsa \
  -e PRIVATE_KEY_PATH=/id_rsa \
  -e PRIVATE_KEY_PASS=“supersecretpass” \
  -e GIT_REPOS=git@github.com:intenthq/gitkv.git,git@github.com:intenthq/anon.git
  intenthq/git-puller
```

If the env var `GIT_CLONE_BARE` is defined, a bare clone is done instead of a regular clone.
