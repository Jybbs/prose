//! How far a run on this machine gets with a module the sweep could not
//! compare, read from the import a failed run named, the exception it
//! raised, and the module's path.

use std::fmt::{self, Display, Formatter};

use ruff_python_ast::name::QualifiedName;

use crate::{execute::module_name, outcome::Outcome};

/// The exception a module raises where the machine lacks a package it
/// imports.
const ABSENT: &str = "ModuleNotFoundError";

/// The platform tokens that appear in the path of a module written for another
/// operating system. Such a module's own error message often does not name the
/// platform, so the path is what identifies it.
const PLATFORMS: &[&str] = &["darwin", "emscripten", "macos", "win32", "windows"];

/// How far a run on this machine gets with a module the sweep could not
/// compare. It separates the modules nothing here could import and the ones
/// the harness's own loading failed from the ones that fail on their own
/// terms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Reach {
    /// The module imports a package this machine does not carry.
    Absent,
    /// An import the run made names the module itself or a package holding
    /// it, so the harness's loading failed rather than the module.
    Loader,
    /// The module runs here and fails on its own terms.
    Module,
    /// The module is written for another platform.
    Platform,
}

impl Reach {
    /// The reach of the module at `relative`, whose run left `ran`. The
    /// module a failed import read from is read first, the exception second,
    /// and the path third, since a module written for another platform often
    /// raises an error that does not name one.
    pub(crate) fn of(relative: &str, ran: &Outcome) -> Self {
        let dotted = module_name(relative);
        let own = ran.importing.as_deref().is_some_and(|importing| {
            QualifiedName::user_defined(&dotted)
                .starts_with(&QualifiedName::user_defined(importing))
        });
        if own {
            Self::Loader
        } else if ran.raised == ABSENT {
            Self::Absent
        } else if PLATFORMS.iter().any(|token| relative.contains(token)) {
            Self::Platform
        } else {
            Self::Module
        }
    }
}

impl Display for Reach {
    fn fmt(&self, form: &mut Formatter<'_>) -> fmt::Result {
        form.write_str(match self {
            Self::Absent => "absent",
            Self::Loader => "loader",
            Self::Module => "module",
            Self::Platform => "platform",
        })
    }
}
