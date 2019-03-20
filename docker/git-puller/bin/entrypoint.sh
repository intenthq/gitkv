#!/bin/sh

set -e

if [ -z $GIT_REPOS ] || [ -z $PRIVATE_KEY_PATH ]; then
  echo "Missing required parameters."
  echo "You need to define both the GIT_REPOS and PRIVATE_KEY_PATH env variables."
  cat /README.md
  exit 1
fi

# Adds some common hosts keys to the known hosts file
for host in "github.com gitlab.com bitbucket.org"; do
  ssh-keyscan -H $host > /etc/ssh/ssh_known_hosts
done

# Tells git to use the provided private key
export GIT_SSH_COMMAND="ssh -i ${PRIVATE_KEY_PATH}"

# Provides private key passphrase instead of prompting the user
export SSH_ASKPASS=/ssh-askpass.sh

# Needs setting DISPLAY so that the script specified by SSH_ASKPASS is run
export DISPLAY=:0

cd /git-puller

BARE_ARG=${GIT_CLONE_BARE:+"--bare"}
IFS=,; for repo in $GIT_REPOS; do
  git clone ${BARE_ARG} ${repo}
done
