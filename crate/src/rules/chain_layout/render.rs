//! The parenthesized text a broken chain is replaced by, the head on
//! the opening row and every later link on a row of its own.

use super::spine::Chain;
use crate::{primitives::INDENT_STEP, source::Source};

/// `chain` broken across lines inside a parenthesis pair, its head one
/// indent step past `indent` and its closing `)` back at `indent`.
/// `hang` is the columns each later link's dot sits past the head's
/// indent, `None` standing the receiver alone and running every link
/// flush beneath it.
pub(super) fn broken(source: &Source, chain: &Chain, indent: usize, hang: Option<usize>) -> String {
    let item_indent = indent + INDENT_STEP;
    let newline = source.newline_str();
    let prefix = " ".repeat(item_indent + hang.unwrap_or(0));
    let mut links = chain.links.iter();
    let mut out = String::from("(");
    out.push_str(newline);
    out.push_str(&prefix[..item_indent]);
    out.push_str(source.slice(chain.receiver_range));
    if hang.is_some()
        && let Some(head) = links.next()
    {
        out.push_str(source.slice(head.range));
    }
    for link in links {
        out.push_str(newline);
        out.push_str(&prefix);
        out.push_str(source.slice(link.range));
    }
    out.push_str(newline);
    out.push_str(&prefix[..indent]);
    out.push(')');
    out
}
