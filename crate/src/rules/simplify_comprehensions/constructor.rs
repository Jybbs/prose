//! The rule's constructor taxonomy, read by both the classifier and
//! the renderer.

/// The builtins whose calls the rule collapses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Constructor {
    Dict,
    List,
    Set,
    Tuple,
}

impl Constructor {
    pub(super) const ALL: [Self; 4] = [Self::Dict, Self::List, Self::Set, Self::Tuple];

    /// The constructor `name` spells, or `None` for any other name.
    pub(super) fn from_name(name: &str) -> Option<Self> {
        match name {
            "dict" => Some(Self::Dict),
            "list" => Some(Self::List),
            "set" => Some(Self::Set),
            "tuple" => Some(Self::Tuple),
            _ => None,
        }
    }

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Dict => "dict",
            Self::List => "list",
            Self::Set => "set",
            Self::Tuple => "tuple",
        }
    }

    /// The delimiter pair this constructor's literal form carries.
    pub(super) const fn brackets(self) -> (&'static str, &'static str) {
        match self {
            Self::Dict | Self::Set => ("{", "}"),
            Self::List => ("[", "]"),
            Self::Tuple => ("(", ")"),
        }
    }
}
