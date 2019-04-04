#!/usr/bin/env bash

set -e

# Make all paths relative to the dir of this script.
cd "$(dirname "$0")" > /dev/null

if [[ $# -ne 1 ]] && [[ $# -ne 2 ]]; then
  echo "Expected 1 or 2 parameters, but got $#."

  cat <<USAGE_DOC
Usage: build.sh <VERSION> [<TARGET>]

Builds the Gitkv docker image for the given version and tags the container as
'intenthq/gitkv:<VERSION>'.

The binary is copied from the Cargo release build for the given target.

Parameters:
    VERSION: The version to build, in the format 'vX.Y.Z' — note the 'v' prefix
             is needed.
    TARGET:  The binary target triple to build into the image,
             eg. 'x86_64-unknown-linux-musl'. If not given, will assume this
             machine's target triple.
USAGE_DOC
  exit 1
fi

VERSION=$1

if [ -z $2 ]; then
  BINARY_PATH=../target/release/gitkv
else
  TARGET=$2
  BINARY_PATH=../target/$TARGET/release/gitkv
fi

if [ ! -f $BINARY_PATH ]; then
  echo 2>&1 "Could not find binary at expected path '$BINARY_PATH'. Have you run 'cargo build --release'?"
  exit 1
fi

echo "Building 'gitkv:$VERSION' Docker container..."
cp $BINARY_PATH ./gitkv
docker build -t intenthq/gitkv:$VERSION .
