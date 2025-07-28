# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

def _cp_impl(ctx: AnalysisContext):
    out = ctx.actions.declare_output("out")
    ctx.actions.run(["fbpython", "-c", "import shutil, sys; from pathlib import Path; shutil.copyfile(Path(sys.argv[1]), Path(sys.argv[2]))", ctx.attrs.src, out.as_output()], category = "cp", local_only = True)

    return [
        DefaultInfo(default_output = out),
    ]

cp = rule(
    impl = _cp_impl,
    attrs = {"src": attrs.source()},
)
