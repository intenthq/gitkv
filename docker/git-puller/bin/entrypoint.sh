#!/bin/sh

set -e

if [ -z $GIT_REPOS ] || [ -z $PRIVATE_KEY_PATH ] || [ -z $PRIVATE_KEY_PASS ]; then
  echo "Missing required parameters."
  echo "You need to define GIT_REPOS, PRIVATE_KEY_PATH and PRIVATE_KEY_PATH env variables."
  echo ""
  cat /README.md
  exit 1
fi

if [ ! -d "${GIT_REPOS_PATH}" ]; then
  echo "Directory ${GIT_REPOS_PATH} does not exist, make sure you mount the volume into that directory."
  echo ""
  cat /README.md
  exit 1
fi

echo "Adds some common hosts keys to the known hosts file"
for host in "github.com gitlab.com bitbucket.org"; do
  ssh-keyscan -H $host > /etc/ssh/ssh_known_hosts
done

# Tells git to use the provided private key
export GIT_SSH_COMMAND="ssh -i ${PRIVATE_KEY_PATH}"

# Provides private key passphrase instead of prompting the user
export SSH_ASKPASS=/ssh-askpass.sh

# Needs setting DISPLAY so that the script specified by SSH_ASKPASS is run
export DISPLAY=:0

cd ${GIT_REPOS_PATH}

BARE_ARG=${GIT_CLONE_BARE:+"--bare"}
IFS=,; for repo in $GIT_REPOS; do
  git clone ${BARE_ARG} ${repo}
done
