#!/bin/bash

# Call the given command with an extra default `--config-dir` parameter value.
#
# This is useful in the context of the docker images where the config directory
# is embeded at a fixed path.

CMD=$1
shift
ARG=$@

# By default, a pipeline's exit code is the exit code of the last command. This
# will make this script exit with code 1 if $CMD fails, even if bunyan exits
# with code 0.
set -o pipefail

echo "$CMD --run-mode docker --config-dir /etc/mimirsbrunn $ARG | bunyan"
$CMD --config-dir=/etc/mimirsbrunn --run-mode=docker $ARG | bunyan
