//! Diagnostic model and output emitters.

use std::io::{self, Write};

use crate::source::Source;

pub(crate) mod github;
pub(crate) mod json;
pub(crate) mod model;
pub(crate) mod sarif;
pub(crate) mod text;

pub use model::{Diagnostic, Severity};

pub(crate) use github::Github;
pub(crate) use json::Json;
pub(crate) use sarif::Sarif;
pub(crate) use text::Text;

/// One pipeline run paired with the diagnostics it produced.
pub type Run<'a> = (&'a Source, &'a [Diagnostic]);

pub trait Emitter {
    fn emit(&self, writer: &mut dyn Write, runs: &[Run<'_>]) -> io::Result<()>;
}
