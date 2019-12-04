#!/usr/bin/env python
# Copyright (c) Facebook, Inc. and its affiliates.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

from __future__ import absolute_import
from __future__ import division
from __future__ import print_function
from __future__ import unicode_literals
'fbcode_builder steps to build and test Facebook rust-shed'

from shell_quoting import ShellQuoted
import specs.rust_shed as rust_shed


def fbcode_builder_spec(builder):
    return {
        'depends_on': [rust_shed],
        "steps": [
            builder.step(
                "Run rust-shed tests",
                [
                    builder.run(ShellQuoted("cargo test")),
                    builder.run(ShellQuoted("cargo doc --no-deps")),
                ],
            ),
        ],
    }


config = {
    'github_project': 'facebookexperimental/rust-shed',
    'fbcode_builder_spec': fbcode_builder_spec,
}
