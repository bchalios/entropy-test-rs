#!/bin/bash

set -euo pipefail

usage() {
    echo "usage: $0 rng-type request size filename"
    echo "support RNG types: os-rng, thread-rng"
}

if [[ $# -ne 3 ]]
then
    usage
    exit 0
fi

rng=$1
size=$2
filename=$3

RUST_LOG=info ./entropy-test -b ${size} -m 10 -i 1000 -t 1000 --rng-type $rng -s ${filename}
if [[ $? -ne 0 ]] ; then exit 1; fi
