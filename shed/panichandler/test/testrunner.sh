#!/bin/bash
# Copyright (c) Facebook, Inc. and its affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

cmd="$1"
exit="$2"
want="$3"
notwant="$4"

out=$("$cmd" 2>&1)
status=$?

if ! (echo "$out" | grep -q "$want"); then
  echo "Expected output \"$want\" not found in output" >&2
  exit 1
fi

if [ "$notwant" ] && (echo "$out" | grep -q "$notwant"); then
  echo "Unwanted output \"$notwant\" found in output" >&2
  exit 1
fi

if [ "$status" != "$exit" ]; then
  echo "Bad exit status $status, wanted $exit" >&2
  exit 1
fi

exit 0
