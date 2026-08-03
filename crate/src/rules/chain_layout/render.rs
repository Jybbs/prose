//! The parenthesized text a broken chain is replaced by, the head on
//! the opening row and every later link on a row of its own.

use super::spine::Chain;
use crate::{
    primitives::layout::{Separator, explode_parens},
    source::Source,
};

/// `chain` broken across lines inside a parenthesis pair, its head one
/// indent step past `indent` and its closing `)` back at `indent`.
/// `hang` is the columns each later link's dot sits past the head's
/// indent, `None` standing the receiver alone and running every link
/// flush beneath it.
pub(super) fn broken(source: &Source, chain: &Chain, indent: usize, hang: Option<usize>) -> String {
    let (head, tail) = chain.links.split_at(usize::from(hang.is_some()));
    let pad = " ".repeat(hang.unwrap_or(0));
    explode_parens(
        source.newline_str(),
        indent,
        1 + tail.len(),
        |out, row| match row.checked_sub(1) {
            None => {
                out.push_str(source.slice(chain.receiver_range));
                for &link in head {
                    out.push_str(source.slice(link));
                }
            }
            Some(link) => {
                out.push_str(&pad);
                out.push_str(source.slice(tail[link]));
            }
        },
        Separator::None,
    )
}
