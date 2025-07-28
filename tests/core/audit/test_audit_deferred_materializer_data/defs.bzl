# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

def _simple_impl(ctx):
    out = ctx.actions.declare_output("out")

    ctx.actions.run(
        [
            "fbpython",
            "-c",
            "import sys; open(sys.argv[1], 'w')",
            out.as_output(),
        ],
        category = "write",
    )

    return [
        DefaultInfo(
            default_output = out,
        ),
    ]

simple = rule(
    attrs = {},
    impl = _simple_impl,
)
