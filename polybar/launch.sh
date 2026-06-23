#!/bin/sh
set -eu
CONFIG_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
polybar -q alpenglowed -c "$CONFIG_DIR/config.ini" &
