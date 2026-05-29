//! The body scope a statement sits in: module, class, or function.

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum BodyScope {
    Class,
    Function,
    Module,
}
