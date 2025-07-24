# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

# pyre-strict


import os
import pathlib
import shutil
import tempfile
from pathlib import Path

from buck2.tests.e2e_util.api.buck import Buck
from buck2.tests.e2e_util.buck_workspace import buck_test
from buck2.tests.e2e_util.helper.utils import expect_exec_count


def setup_symlink(symlink_path: Path, src: Path) -> Path:
    # We want to check in a symlink but given Buck is running this and symlinks
    # do not exist we need to put it back and make it be an actual symlink.
    if os.path.isdir(symlink_path):
        shutil.rmtree(symlink_path)
    else:
        os.remove(symlink_path)

    os.symlink(src, symlink_path, target_is_directory=True)
    return symlink_path


@buck_test(extra_buck_config={"buck2": {"use_correct_source_symlink_reading": "true"}})
async def test_symlink_target_tracked_for_rebuild(buck: Buck) -> None:
    setup_symlink(buck.cwd / "src" / "link", pathlib.Path("../dir"))

    await buck.build("//:cp")
    await expect_exec_count(buck, 1)

    await buck.build("//:cp")
    await expect_exec_count(buck, 0)

    with open(buck.cwd / "dir/file", "w") as file:
        file.write("GOODBYE\n")

    # This isn't really behavior  we want to guarantee and we'd rather users
    # don't use symlinks, but this is very observable (and it's not worse than
    # just reading the files then pretending they are never used!)
    await buck.build("//:cp")
    await expect_exec_count(buck, 1)


@buck_test(
    setup_eden=True,
    extra_buck_config={"buck2": {"use_correct_source_symlink_reading": "true"}},
)
async def test_symlinks_redirection(buck: Buck) -> None:
    symlink_path = setup_symlink(buck.cwd / "src" / "link", pathlib.Path("../dir"))

    await buck.build("//:cp")
    await expect_exec_count(buck, 1)

    await buck.build("//:cp")
    await expect_exec_count(buck, 0)

    # We change the symlink which should invalidate all files depending on it
    os.remove(symlink_path)
    src2 = pathlib.Path("../dir2")
    os.symlink(src2, symlink_path)

    await buck.build("//:cp")
    await expect_exec_count(buck, 1)


@buck_test(
    setup_eden=True,
    extra_buck_config={"buck2": {"use_correct_source_symlink_reading": "true"}},
)
async def test_symlinks_external(buck: Buck) -> None:
    symlink_path = os.path.join(buck.cwd, "ext", "link")
    shutil.rmtree(symlink_path)
    top_level = tempfile.mkdtemp()

    os.mkdir(os.path.join(top_level, "nested1"))
    os.mkdir(os.path.join(top_level, "nested2"))
    with open(os.path.join(top_level, "nested1", "file"), "w") as f:
        f.write("HELLO")
    with open(os.path.join(top_level, "nested2", "file"), "w") as f:
        f.write("GOODBYE")

    os.symlink(os.path.join(top_level, "nested1"), symlink_path)

    await buck.build("//:ext")
    await expect_exec_count(buck, 1)

    await buck.build("//:ext")
    await expect_exec_count(buck, 0)

    os.remove(symlink_path)
    os.symlink(os.path.join(top_level, "nested2"), symlink_path)

    await buck.build("//:ext")
    await expect_exec_count(buck, 1)


@buck_test(extra_buck_config={"buck2": {"use_correct_source_symlink_reading": "true"}})
async def test_no_read_through_symlinks(buck: Buck) -> None:
    res = await buck.build_without_report(
        "//:stat_symlink",
        "--out",
        "-",
        "--remote-only",
    )
    # Just check that we don't always return `True`
    assert res.stdout.strip() == "False"

    setup_symlink(buck.cwd / "src" / "link", buck.cwd / "dir")

    res = await buck.build_without_report(
        "//:stat_symlink",
        "--out",
        "-",
        "--remote-only",
    )
    assert res.stdout.strip() == "True"

    res = await buck.build_without_report(
        "//:stat_symlink_in_dir",
        "--out",
        "-",
        "--remote-only",
    )
    assert res.stdout.strip() == "True"


@buck_test(setup_eden=True)
async def test_eden_io_read_symlink_dir_build_target(buck: Buck) -> None:
    setup_symlink(buck.cwd / "testlink", buck.cwd / "symdir" / "dir")

    await buck.build("//:symlink_dep")


@buck_test(setup_eden=True)
async def test_eden_io_read_symlink_dir_list_target(buck: Buck) -> None:
    setup_symlink(buck.cwd / "testlink", buck.cwd / "symdir")

    await buck.targets("//testlink/dir:")
