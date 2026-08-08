//! The parenthesized text a broken chain is replaced by, the head on
//! the opening row and every later link on a row of its own.

use super::spine::Chain;
use crate::{
    primitives::{
        fracture,
        layout::{Separator, explode_parens},
    },
    source::Source,
};

/// `chain` broken across lines inside a parenthesis pair, its head one
/// indent step past `indent` and its closing `)` back at `indent`.
/// `hang` is the columns each later link's dot sits past the head's
/// indent, `None` standing the receiver alone and running every link
/// flush beneath it. Each segment renders at the width `joins` settles
/// it to, so a row carries no break the measure did not count.
pub(super) fn broken(
    source: &Source,
    chain: &Chain,
    indent: usize,
    hang: Option<usize>,
    joins: &fracture::Joins,
) -> String {
    let (head, tail) = chain.links.split_at(usize::from(hang.is_some()));
    let pad = " ".repeat(hang.unwrap_or(0));
    explode_parens(
        source.newline_str(),
        indent,
        1 + tail.len(),
        |out, row| match row.checked_sub(1) {
            None => {
                out.push_str(&joins.settled(source, chain.receiver_range));
                for &link in head {
                    out.push_str(&joins.settled(source, link));
                }
            }
            Some(link) => {
                out.push_str(&pad);
                out.push_str(&joins.settled(source, tail[link]));
            }
        },
        Separator::None,
    )
}
