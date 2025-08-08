# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

# pyre-strict


from buck2.tests.e2e_util.api.buck import Buck
from buck2.tests.e2e_util.buck_workspace import buck_test


@buck_test(inplace=True)
async def test_cancel_test_if_re_queue_longer_than_threshold(buck: Buck) -> None:
    args = [
        "-c",
        "build.remote_execution_cancel_on_estimated_queue_time_exceeds_s=10",
        "--no-remote-cache",
        "--remote-only",
    ]
    result = await buck.test(
        *args,
        "fbcode//buck2/tests/targets/rules/sh_test:test_remote_explicit_stays_in_queue",
        env={"BUCK2_TEST_RE_QUEUE_ESTIMATE_S": "100"},
    )
    assert (
        "Omitted: buck2/tests/targets/rules/sh_test:test_remote_explicit_stays_in_queue - main"
        in result.stderr
    )
    assert (
        "The test execution stayed in RE queue for more than threshold time."
        in result.stderr
    )
