#!/usr/bin/env python
# Copyright (c) Facebook, Inc. and its affiliates.
#
# This source code is licensed under both the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree and the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree.

from __future__ import absolute_import
from __future__ import division
from __future__ import print_function
from __future__ import unicode_literals

"fbcode_builder steps to build and test Facebook rust-shed"

from shell_quoting import ShellQuoted
import specs.rust_shed as rust_shed


def fbcode_builder_spec(builder):
    return {
        "depends_on": [rust_shed],
        "steps": [
            builder.step(
                "Run rust-shed tests",
                [
                    builder.run(ShellQuoted("cargo test")),
                    builder.run(ShellQuoted("cargo doc --no-deps")),
                ],
            )
        ],
    }


config = {
    "github_project": "facebookexperimental/rust-shed",
    "fbcode_builder_spec": fbcode_builder_spec,
}
