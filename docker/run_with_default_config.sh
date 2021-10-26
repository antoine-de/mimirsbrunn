#!/bin/bash

# Call the given command with an extra default `--config-dir` parameter value.
#
# This is useful in the context of the docker images where the config directory
# is embeded at a fixed path.

CMD=$1
shift
ARG=$@

$CMD --config-dir /etc/mimirsbrunn --run-mode docker $@
