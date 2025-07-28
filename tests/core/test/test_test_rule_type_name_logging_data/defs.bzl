# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

def _one(ctx):
    return [
        DefaultInfo(default_output = ctx.actions.write("out", "one")),
        ExternalRunnerTestInfo(
            command = ["fbpython", "-c", "import sys; sys.exit(0)"],
            type = "custom",
        ),
    ]

one = rule(
    impl = _one,
    attrs = {
        "deps": attrs.list(attrs.dep(), default = []),
    },
)

def _two(ctx):
    return [
        DefaultInfo(default_output = ctx.actions.write("out", "two")),
        ExternalRunnerTestInfo(
            command = ["fbpython", "-c", "import sys; sys.exit(0)"],
            type = "custom",
        ),
    ]

two = rule(
    impl = _two,
    attrs = {},
)
