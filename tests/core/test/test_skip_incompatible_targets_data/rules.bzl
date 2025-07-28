# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

def _impl(_ctx):
    return [DefaultInfo(), ExternalRunnerTestInfo(type = "custom", command = ["fbpython", "-c", "import sys; sys.exit(0)"])]

test_rule = rule(
    impl = _impl,
    attrs = {
    },
)
