//! The source constructors only the native build reads, meaning the
//! notebook reader the CLI opens an `.ipynb` through and the parse a probe
//! rebuild clones per subset.

use ruff_notebook::{CellOffsets, Notebook};
use ruff_python_ast::{ModModule, PySourceType};
use ruff_python_parser::{ParseError, ParseOptions, Parsed};

use super::{BUFFER_NAME, Source, parse_typed_module};

impl Source {
    /// Builds the concatenated source of a parsed notebook, attaching its
    /// cell boundaries and each code cell's notebook position. The caller
    /// keeps `notebook` to re-emit the document after formatting.
    pub(crate) fn from_notebook(
        notebook: &Notebook,
        name: impl Into<Box<str>>,
    ) -> Result<Self, ParseError> {
        Self::build(
            notebook.source_code().to_owned(),
            name,
            PySourceType::Ipynb,
            notebook.cell_offsets().clone(),
        )
        .map(|source| source.numbered(notebook))
    }

    /// Builds a plain module source around an already-parsed tree, the
    /// probe-side counterpart to [`parsed_module`](Self::parsed_module).
    pub(crate) fn from_parsed_module(text: String, parsed: Parsed<ModModule>) -> Self {
        Self::from_parsed(
            text,
            BUFFER_NAME,
            PySourceType::default(),
            CellOffsets::default(),
            parsed,
        )
    }

    /// Parses `text` as a plain module, returning the tree a probe
    /// rebuild clones per subset.
    pub(crate) fn parsed_module(text: &str) -> Result<Parsed<ModModule>, ParseError> {
        parse_typed_module(text, ParseOptions::from(PySourceType::default()))
    }
}
