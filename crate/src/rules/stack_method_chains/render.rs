//! The parenthesized text a broken chain is replaced by, the head on
//! the opening row and every later link on a row of its own.

use super::spine::Chain;
use crate::{
    primitives::{
        inline::end_column,
        layout::{Separator, explode_parens, item_indent},
    },
    source::Source,
};

/// `chain` broken across lines inside a parenthesis pair, its head one
/// indent step past `indent` and its closing `)` back at `indent`.
/// `hang` is the columns each later link's dot sits past the head's
/// indent, `None` standing the receiver alone and running every link
/// flush beneath it. `segment` writes the receiver at index zero and
/// each link at its index past that, given the column and the row
/// indent the segment lands at.
pub(super) fn broken(
    source: &Source,
    chain: &Chain,
    indent: usize,
    hang: Option<usize>,
    mut segment: impl FnMut(usize, usize, usize) -> String,
) -> String {
    let (head, tail) = chain.links.split_at(usize::from(hang.is_some()));
    let pad = hang.unwrap_or(0);
    let item = item_indent(indent);
    explode_parens(
        source.newline_str(),
        indent,
        1 + tail.len(),
        |out, row| match row.checked_sub(1) {
            None => {
                let mut column = item;
                for index in 0..=head.len() {
                    let text = segment(index, column, item);
                    column = end_column(&text, column);
                    out.push_str(&text);
                }
            }
            Some(link) => {
                out.extend(std::iter::repeat_n(' ', pad));
                out.push_str(&segment(1 + head.len() + link, item + pad, item + pad));
            }
        },
        Separator::None,
    )
}
