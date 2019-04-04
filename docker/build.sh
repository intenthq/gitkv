#!/usr/bin/env bash

set -e

if [[ $# -ne 1 ]]; then
  echo "Expected 1 parameter, but got $#."

  cat <<USAGE_DOC
Usage: build.sh <VERSION>

Builds the gitkv binary for the given version and tags the container as 'gitkv:<VERSION>'.

Parameters:
    VERSION: The version to build, in the format 'vX.Y.Z' — note the 'v' prefix is needed.
USAGE_DOC
  exit 1
fi

VERSION=$1

echo "Building gitkv:$VERSION"

docker build -t gitkv:$VERSION --build-arg version=$VERSION .
