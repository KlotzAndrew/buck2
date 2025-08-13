# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is dual-licensed under either the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree or the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree. You may select, at your option, one of the
# above-listed licenses.

import argparse
import os


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--out",
        type=str,
        required=True,
    )

    args = parser.parse_args()

    if os.path.exists(args.out):
        with open(args.out, "r") as file:
            content = f"{file.read()} bar"
    else:
        content = "foo"

    with open(args.out, "w") as f:
        f.write(f"{content}")


if __name__ == "__main__":
    main()
