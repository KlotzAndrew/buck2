/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#![feature(error_generic_member_access)]

//! Implementation of several server commands.

#![feature(box_patterns)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![feature(used_with_arg)]

pub mod commands;
pub mod dot;
pub mod html;
pub(crate) mod query_output_format;

pub fn init_late_bindings() {
    commands::init_commands::init_other_server_commands();
    commands::query::printer::init_print_action_node();
}
