#!/usr/bin/env bash
basedir=$(dirname "$0")

case "$(uname -s)" in
  *CYGWIN*) basedir=$(cygpath -w "$basedir");;
  *MSYS*) basedir=$(cygpath -w "$basedir");;
esac

exec node "$basedir/{bin_path}" "$@"

ret=$?
exit $ret
