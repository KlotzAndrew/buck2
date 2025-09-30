/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use allocative::Allocative;
use buck2_data::ActionErrorLocation;
use buck2_data::ActionErrorLocations;
use buck2_data::ActionSubError;
use buck2_data::CommandExecution;
use console::strip_ansi_codes;
use derive_more::Display;
use display_container::fmt_container;
use gazebo::prelude::SliceClonedExt;
use starlark::environment::GlobalsBuilder;
use starlark::environment::Methods;
use starlark::environment::MethodsBuilder;
use starlark::environment::MethodsStatic;
use starlark::starlark_module;
use starlark::typing::Ty;
use starlark::values::AllocValue;
use starlark::values::Heap;
use starlark::values::NoSerialize;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Value;
use starlark::values::ValueError;
use starlark::values::ValueOfUnchecked;
use starlark::values::dict::DictType;
use starlark::values::list::UnpackList;
use starlark::values::list_or_tuple::UnpackListOrTuple;
use starlark::values::none::NoneOr;
use starlark::values::starlark_value;
use starlark::values::starlark_value_as_type::StarlarkValueAsType;

use crate::interpreter::rule_defs::artifact::starlark_artifact::StarlarkArtifact;
use crate::interpreter::rule_defs::artifact::starlark_artifact_value::StarlarkArtifactValue;
use crate::starlark::values::ValueLike;

pub(crate) type ActionSubErrorResult<'a> = UnpackList<&'a StarlarkActionSubError<'a>>;

#[derive(Debug, buck2_error::Error)]
#[buck2(tag = Tier0)]
pub(crate) enum ActionErrorHandlerError {
    #[error("Error handler failed. Expected return type `{0}`, got value with type `{1}`")]
    TypeError(Ty, String),
}

#[derive(ProvidesStaticType, Trace, Allocative, Debug, Display, NoSerialize)]
#[display(
     "ActionErrorCtx(stderr: {}, stdout: {})",
     self.stderr,
     self.stdout
 )]
pub struct StarlarkActionErrorContext<'v> {
    stderr: String,
    stdout: String,
    output_artifacts: ValueOfUnchecked<'v, DictType<StarlarkArtifact, StarlarkArtifactValue>>,
}

impl<'v> StarlarkActionErrorContext<'v> {
    pub(crate) fn new_from_command_execution(
        command: Option<&CommandExecution>,
        output_artifacts: ValueOfUnchecked<'v, DictType<StarlarkArtifact, StarlarkArtifactValue>>,
    ) -> Self {
        let stderr = command.map_or(String::default(), |c| {
            c.details.as_ref().map_or(String::default(), |c| {
                strip_ansi_codes(&c.stderr).to_string()
            })
        });
        let stdout = command.map_or(String::default(), |c| {
            c.details.as_ref().map_or(String::default(), |c| {
                strip_ansi_codes(&c.stdout).to_string()
            })
        });

        StarlarkActionErrorContext {
            stderr,
            stdout,
            output_artifacts,
        }
    }
}

impl<'v> AllocValue<'v> for StarlarkActionErrorContext<'v> {
    fn alloc_value(self, heap: &'v Heap) -> Value<'v> {
        heap.alloc_complex_no_freeze(self)
    }
}

#[starlark_value(type = "ActionErrorCtx", StarlarkTypeRepr, UnpackValue)]
impl<'v> StarlarkValue<'v> for StarlarkActionErrorContext<'v> {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(action_error_context_methods)
    }
}

/// Methods available on `ActionErrorCtx` to help categorize the action failure. These
/// categorizations should be finer grain, and most likely language specific.
#[starlark_module]
fn action_error_context_methods(builder: &mut MethodsBuilder) {
    /// Retrieve the stderr of the failed action.
    /// Can use string/regex matching to identify the error in order to categorize it.
    #[starlark(attribute)]
    fn stderr<'v>(this: &'v StarlarkActionErrorContext) -> starlark::Result<&'v str> {
        Ok(&this.stderr)
    }

    /// Retrieve the stdout of the failed action.
    /// Can use string/regex matching to identify the patterns in order to categorize it.
    #[starlark(attribute)]
    fn stdout<'v>(this: &'v StarlarkActionErrorContext) -> starlark::Result<&'v str> {
        Ok(&this.stdout)
    }

    /// Allows the output artifacts to be retrieve if [`outputs_for_error_handler`](https://buck2.build/docs/api/build/AnalysisActions/#analysisactionsrun)
    /// is set and the output artifact exists. This is useful for languages with structured error output, making the error retrieval process simpler.
    ///
    /// This is also the recommended way to retrieve file path and line number, as reliably extracting that information
    /// from stdout/stderr can be challenging
    #[starlark(attribute)]
    fn output_artifacts<'v>(
        this: &'v StarlarkActionErrorContext<'v>,
    ) -> starlark::Result<ValueOfUnchecked<'v, DictType<StarlarkArtifact, StarlarkArtifactValue>>>
    {
        Ok(this.output_artifacts)
    }

    /// Create a new error location, specifying a file path and an optional line number.
    ///
    /// The file path should be either a project-relative path, or an absolute path.
    fn new_error_location<'v>(
        #[starlark(this)] _this: &'v StarlarkActionErrorContext,
        #[starlark(require = named)] file: String,
        #[starlark(require = named, default = NoneOr::None)] line: NoneOr<u64>,
    ) -> starlark::Result<StarlarkActionErrorLocation> {
        // @TODO(wendyy) - actually enforce/validate the path types.
        Ok(StarlarkActionErrorLocation {
            file,
            line: line.into_option(),
        })
    }

    /// Create a new sub error, specifying an error category name, optional message, and
    /// an optional list of error locations.
    ///
    /// The category should be finer grain error categorizations provided by the rule authors,
    /// and tend to be language specific. These should not be any kind of shared concepts
    /// among all errors for all languages/rules. For example, timeouts and infra errors
    /// should not go here - buck2 tries to categorize these types of errors automatically.
    /// An example of a finer grain error category may be the error code for rustc outputs.
    ///
    /// 'category': Required, useful for providing a more granular error category for action errors.
    /// 'message': Optional, provide users with additional context about the error to help with debugging/understandability/resolution, etc.
    /// 'locations': Optional, file path and line number of the error location, useful for external integration to highlight where the error is.
    ///
    /// The message will be emitted to the build report, and to the stderr in the error diagnostics
    /// section.
    fn new_sub_error<'v>(
        #[starlark(this)] _this: &'v StarlarkActionErrorContext,
        #[starlark(require = named)] category: String,
        #[starlark(require = named, default = NoneOr::None)] message: NoneOr<String>,
        #[starlark(require = named, default = NoneOr::None)] locations: NoneOr<
            UnpackListOrTuple<&'v StarlarkActionErrorLocation>,
        >,
    ) -> starlark::Result<StarlarkActionSubError<'v>> {
        Ok(StarlarkActionSubError {
            category,
            message: message.into_option(),
            locations: locations.into_option(),
        })
    }

    /// Parse error text using vim errorformat patterns to create structured error information.
    /// This method leverages vim's proven errorformat system to extract file paths, line numbers,
    /// and error messages from compiler/tool output, automatically creating ActionSubError objects.
    ///
    /// For errorformat pattern syntax, see: https://neovim.io/doc/user/quickfix.html#errorformat
    ///
    /// Multiple patterns can be provided and will be tried in order until one matches.
    /// This is useful for tools that may output errors in different formats.
    ///
    /// Args:
    /// - `category`: Base category name for the generated sub-errors (e.g., "rust", "gcc")
    /// - `error`: The error text to parse (typically stderr or stdout from the failed action)
    /// - `errorformats`: List of vim errorformat pattern strings to try matching against
    ///
    /// Returns a list of ActionSubError objects with structured error information including
    /// file locations when successfully parsed from the error text.
    fn parse_with_errorformat<'v>(
        #[starlark(this)] _this: &'v StarlarkActionErrorContext,
        #[starlark(require = named)] category: String,
        #[starlark(require = named)] error: String,
        #[starlark(require = named)] errorformats: UnpackListOrTuple<String>,
        heap: &'v Heap,
    ) -> starlark::Result<Vec<StarlarkActionSubError<'v>>> {
        let error_lines = buck2_errorformat::split_lines(&error);
        let error_entries = buck2_errorformat::parse_error_format(errorformats.items, error_lines)
            .map_err(buck2_error::Error::from)?;
        let res = error_entries
            .into_iter()
            .map(|e| StarlarkActionSubError::from_errorformat_entry(e, category.clone(), heap))
            .collect();
        Ok(res)
    }
}

#[derive(
    ProvidesStaticType,
    Trace,
    Allocative,
    Debug,
    Display,
    NoSerialize,
    Clone,
    Default,
    Ord,
    PartialOrd,
    Eq,
    PartialEq
)]
#[display(
     "ActionErrorLocation(file={}, line={})",
     self.file,
     self.line.map_or("None".to_owned(), |l| l.to_string())
 )]
pub struct StarlarkActionErrorLocation {
    file: String,
    line: Option<u64>,
}

#[starlark_value(type = "ActionErrorLocation", StarlarkTypeRepr, UnpackValue)]
impl<'v> StarlarkValue<'v> for StarlarkActionErrorLocation {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(action_error_location_methods)
    }

    fn equals(&self, other: Value<'v>) -> starlark::Result<bool> {
        if let Some(other) = other.downcast_ref::<Self>() {
            Ok(self.eq(other))
        } else {
            Ok(false)
        }
    }

    fn compare(&self, other: Value<'v>) -> starlark::Result<std::cmp::Ordering> {
        if let Some(other) = other.downcast_ref::<Self>() {
            Ok(self.cmp(other))
        } else {
            ValueError::unsupported_with(self, "compare", other)
        }
    }
}

impl<'v> AllocValue<'v> for StarlarkActionErrorLocation {
    fn alloc_value(self, heap: &'v Heap) -> Value<'v> {
        heap.alloc_complex_no_freeze(self)
    }
}

/// Methods available on `StarlarkActionErrorLocation` to help with testing the error
/// handler implementation
#[starlark_module]
fn action_error_location_methods(builder: &mut MethodsBuilder) {
    /// Useful for external integration to highlight which file the error resides in.
    /// Currently only used for action error handler unit testing.
    #[starlark(attribute)]
    fn file<'v>(this: &'v StarlarkActionErrorLocation) -> starlark::Result<&'v str> {
        Ok(&this.file)
    }

    /// Useful for external integration to highlight which line the error resides in.
    /// Currently only used for action error handler unit testing.
    #[starlark(attribute)]
    fn line<'v>(this: &'v StarlarkActionErrorLocation) -> starlark::Result<NoneOr<u64>> {
        Ok(NoneOr::from_option(this.line))
    }
}

#[derive(
    ProvidesStaticType,
    Trace,
    Allocative,
    Debug,
    NoSerialize,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq
)]
pub(crate) struct StarlarkActionSubError<'v> {
    category: String,
    message: Option<String>,
    #[allocative(skip)]
    #[trace(unsafe_ignore)]
    locations: Option<UnpackListOrTuple<&'v StarlarkActionErrorLocation>>,
}

impl<'v> Display for StarlarkActionSubError<'v> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = format!(
            "ActionSubError(category={}, message={}, locations=[",
            self.category,
            self.message.clone().unwrap_or_default()
        );
        fmt_container(
            f,
            &prefix,
            "])",
            self.locations
                .as_ref()
                .map_or(Vec::new(), |l| l.items.iter().collect()),
        )
    }
}

impl<'v> StarlarkActionSubError<'v> {
    pub(crate) fn from_errorformat_entry(
        entry: buck2_errorformat::Entry,
        category: String,
        heap: &'v Heap,
    ) -> Self {
        let location = entry
            .filename
            .map(|f| StarlarkActionErrorLocation {
                file: f,
                line: entry.lnum.map(|l| l as u64),
            })
            .map(|l| heap.alloc_typed(l));

        StarlarkActionSubError {
            category,
            message: entry.message,
            locations: location.map(|l| UnpackListOrTuple {
                items: vec![l.as_ref()],
            }),
        }
    }
}

impl<'v> AllocValue<'v> for StarlarkActionSubError<'v> {
    fn alloc_value(self, heap: &'v Heap) -> Value<'v> {
        heap.alloc_complex_no_freeze(self)
    }
}

#[starlark_value(type = "ActionSubError", StarlarkTypeRepr, UnpackValue)]
impl<'v> StarlarkValue<'v> for StarlarkActionSubError<'v> {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new();
        RES.methods(action_sub_error_methods)
    }

    fn equals(&self, other: Value<'v>) -> starlark::Result<bool> {
        if let Some(other) = other.downcast_ref::<Self>() {
            Ok(self.eq(other))
        } else {
            Ok(false)
        }
    }

    fn compare(&self, other: Value<'v>) -> starlark::Result<std::cmp::Ordering> {
        if let Some(other) = other.downcast_ref::<Self>() {
            Ok(self.cmp(other))
        } else {
            ValueError::unsupported_with(self, "compare", other)
        }
    }
}

/// Methods available on `StarlarkActionSubError` to help with testing the error
/// handler implementation
#[starlark_module]
fn action_sub_error_methods(builder: &mut MethodsBuilder) {
    /// A more granular category for the action error.
    /// Currently only used for action error handler unit testing.
    #[starlark(attribute)]
    fn category<'v>(this: &'v StarlarkActionSubError) -> starlark::Result<&'v str> {
        Ok(&this.category)
    }

    /// An optional message to be displayed with the error, used to provide additoinal context
    /// Currently only used for action error handler unit testing.
    #[starlark(attribute)]
    fn message<'v>(this: &'v StarlarkActionSubError) -> starlark::Result<NoneOr<&'v str>> {
        Ok(match &this.message {
            Some(message) => NoneOr::Other(message.as_str()),
            None => NoneOr::None,
        })
    }

    /// File/line information for the error, useful for external integration to highlight where the error resides
    /// Currently only used for action error handler unit testing.
    #[starlark(attribute)]
    fn locations<'v>(
        this: &'v StarlarkActionSubError,
    ) -> starlark::Result<NoneOr<Vec<StarlarkActionErrorLocation>>> {
        match &this.locations {
            None => Ok(NoneOr::None),
            Some(locations) => Ok(NoneOr::Other(locations.items.cloned())),
        }
    }
}

impl<'v> StarlarkActionSubError<'v> {
    pub(crate) fn to_proto(&self) -> ActionSubError {
        ActionSubError {
            category: self.category.clone(),
            message: self.message.clone(),
            locations: self
                .locations
                .clone()
                .map(|locations| ActionErrorLocations {
                    locations: locations
                        .items
                        .iter()
                        .map(|l| ActionErrorLocation {
                            file: l.file.clone(),
                            line: l.line,
                        })
                        .collect(),
                }),
        }
    }
}

#[starlark_module]
pub(crate) fn register_action_error_types(globals: &mut GlobalsBuilder) {
    const ActionSubError: StarlarkValueAsType<StarlarkActionSubError> = StarlarkValueAsType::new();
    const ActionErrorCtx: StarlarkValueAsType<StarlarkActionErrorContext> =
        StarlarkValueAsType::new();
    const ActionErrorLocation: StarlarkValueAsType<StarlarkActionErrorLocation> =
        StarlarkValueAsType::new();
}

/// Global methods for testing starlark action error handler.
#[starlark_module]
pub(crate) fn register_action_error_handler_for_testing(builder: &mut GlobalsBuilder) {
    /// Global function to create a new `ActionErrorContext` for testing a starlark action error
    /// handler via `bxl_test`.
    fn new_test_action_error_ctx<'v>(
        #[starlark(require=named, default = "")] stderr: &str,
        #[starlark(require=named, default = "")] stdout: &str,
    ) -> starlark::Result<StarlarkActionErrorContext<'v>> {
        Ok(StarlarkActionErrorContext {
            stderr: stderr.to_owned(),
            stdout: stdout.to_owned(),
            output_artifacts: ValueOfUnchecked::new(starlark::values::Value::new_none()),
        })
    }
}
