//! What one run of a module left behind, as `probe.py` records it, meaning
//! the tagged rows the probe writes, the names a comparison drops, and what
//! a run amounted to.

use std::{collections::BTreeMap, path::Path};

/// The names the formatter reorders by design, whose value a comparison
/// therefore leaves out.
const REORDERED: &[&str] = &["__all__", "__slots__"];

/// The names every module binds through the loader rather than through its
/// own code, which a comparison leaves out.
const UNBOUND: &[&str] = &[
    "__builtins__",
    "__cached__",
    "__doc__",
    "__file__",
    "__loader__",
    "__path__",
    "__spec__",
];

/// What a run of a module amounted to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Kind {
    /// The run bound a namespace.
    Ok,
    /// The run raised.
    Raised,
    /// The run outran its deadline.
    Timeout,
    /// The run left no record to read.
    #[default]
    Unmeasured,
}

impl Kind {
    /// The kind a probe's `kind` row names.
    fn of(named: &str) -> Self {
        match named {
            "ok" => Self::Ok,
            "raised" => Self::Raised,
            _ => Self::Unmeasured,
        }
    }
}

/// What one run of a module left behind, as the probe records it.
#[derive(Clone, Default)]
pub(crate) struct Outcome {
    /// The plain constants the run bound, each spelt.
    pub(crate) constants: BTreeMap<String, String>,
    /// The predicate of a sentence naming the module.
    pub(crate) error: String,
    /// The file and row of every frame a raise passed through.
    pub(crate) frames: Vec<(String, usize)>,
    /// What the run amounted to.
    pub(crate) kind: Kind,
    /// Every module the run took from a tree, relative to it, sorted.
    pub(crate) loaded: Vec<String>,
    /// The name a raised run could not find.
    pub(crate) name: Option<String>,
    /// The names an `ok` run bound, sorted.
    pub(crate) names: Vec<String>,
}

impl Outcome {
    /// An outcome of `kind` carrying `error` and nothing else.
    pub(crate) fn of(kind: Kind, error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            kind,
            ..Self::default()
        }
    }

    /// Reads back the record the probe wrote, which is one tagged row per
    /// line separated by `RS`, each field separated by `NUL`.
    ///
    /// The probe reports the namespace as it stands and every module path as
    /// the interpreter saw it, so the loader's own names come out here, the
    /// names sort here, and a loaded path is made relative to whichever of
    /// `trees` carries it.
    pub(crate) fn parse(record: &str, trees: &[&Path]) -> Self {
        let mut read = Self::default();
        for row in record.split('\u{1e}') {
            let mut fields = row.split('\0');
            match (fields.next(), fields.next(), fields.next()) {
                (Some("bound"), Some(name), _) if !UNBOUND.contains(&name) => {
                    read.names.push(name.to_owned());
                }
                (Some("const"), Some(name), Some(spelt))
                    if !UNBOUND.contains(&name) && !REORDERED.contains(&name) =>
                {
                    read.constants.insert(name.to_owned(), spelt.to_owned());
                }
                (Some("frame"), Some(row), Some(file)) => {
                    if let Ok(row) = row.parse() {
                        read.frames.push((file.to_owned(), row));
                    }
                }
                (Some("kind"), Some(kind), _) => read.kind = Kind::of(kind),
                (Some("loaded"), Some(path), _) => {
                    if let Some(relative) = relative_to(path, trees) {
                        read.loaded.push(relative);
                    }
                }
                (Some("missing"), Some(name), _) => read.name = Some(name.to_owned()),
                (Some("raise"), Some(raised), Some(message)) => {
                    read.error = format!("raises {raised}: {message}");
                }
                _ => {}
            }
        }
        read.loaded.sort_unstable();
        read.loaded.dedup();
        read.names.sort();
        read
    }
}

/// A path named relative to whichever of `trees` carries it, `None` for one
/// under none of them.
pub(crate) fn relative_to(path: &str, trees: &[&Path]) -> Option<String> {
    trees.iter().find_map(|tree| {
        Path::new(path)
            .strip_prefix(tree)
            .ok()
            .map(|relative| relative.to_string_lossy().into_owned())
    })
}
