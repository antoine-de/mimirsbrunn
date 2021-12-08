#!/bin/bash

# Call the given command with extra defaults `--run-mode` `--config-dir`
# parameter values.
#
# You can still override those default parameters if you need with through
# variables $RUN_MODE and $CONFIG_DIR.
#
# This is useful in the context of the docker images where the config directory
# is embeded at a fixed path.

CMD=$1
shift
ARG=$@

BUNYAN=${BUNYAN:-"1"}
RUN_MODE=${RUN_MODE:-"docker"}
CONFIG_DIR=${CONFIG_DIR:-"/etc/mimirsbrunn"}

# By default, a pipeline's exit code is the exit code of the last command. This
# will make this script exit with code 1 if $CMD fails, even if bunyan exits
# with code 0.
set -o pipefail

if [ $BUNYAN ] ; then
    $CMD --config-dir $CONFIG_DIR --run-mode $RUN_MODE $ARG | bunyan
else
    $CMD --config-dir $CONFIG_DIR --run-mode $RUN_MODE $ARG
fi
