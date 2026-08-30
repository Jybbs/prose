//! The `align-imports` forecast `reflow-imports` packs against: the
//! block's runs as the later rules seat them, each roster expanded at
//! the column its run settles, and every row's gap read off the
//! aligner's own column math.

use std::ops::Range;

use ruff_python_ast::Stmt;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Layout, MEMBER_SEPARATOR, Packing, own_line_indent, runs::MergeRuns};
use crate::{
    primitives::{
        aligner,
        comments::TRAILING_GAP,
        imports::{
            IMPORT_KEYWORD_WIDTH, import_blank_lines, import_group, import_sort_key, is_import,
        },
        inline::display_width,
        layout::pack,
        orderer::any_sibling_shares_line,
        sections::Sections,
    },
    rules::{
        align_imports::{AlignImports, qualify_from},
        band_constants::Carry,
    },
    source::Source,
};

/// The rows a banding moves comments between, read by slot: the rows
/// whose heading leaves them, the rows an own-line comment lands
/// above, and the rows a comment trails, each with the columns it
/// adds.
struct Carried {
    headed: FxHashSet<usize>,
    trailed: FxHashMap<usize, usize>,
    unheaded: FxHashSet<usize>,
}

impl Carried {
    /// Reads `carries` with each carrier resolved through `seat_of`, the
    /// row a comment lands on where the carrier folds into a merge.
    fn of(source: &Source, carries: &[Carry], seat_of: impl Fn(usize) -> usize) -> Self {
        let mut carried = Self {
            headed: FxHashSet::default(),
            trailed: FxHashMap::default(),
            unheaded: FxHashSet::default(),
        };
        for carry in carries {
            carried.unheaded.insert(carry.absorbs);
            let carrier = seat_of(carry.carrier);
            if carry.trails {
                let width = display_width(TRAILING_GAP)
                    + display_width(source.slice(carry.comment).trim_start());
                *carried.trailed.entry(carrier).or_default() += width;
            } else {
                carried.headed.insert(carrier);
            }
        }
        carried
    }
}

/// What a seat packs onto its rows.
#[derive(Clone)]
enum Packs<'a> {
    /// The roster the rule packs to the seat's column.
    Roster(Vec<&'a str>),
    /// The last row of a packed roster, holding these of its members,
    /// spilled beneath a carried comment at the width `Seat::width`
    /// names.
    Spilled(Range<usize>),
    /// The row as the source wrote it.
    Written,
}

/// One row of an `align-imports` run as the later rules seat it. `tail`
/// is the columns the seat's last row keeps past its members for a
/// trailing comment, the settled one it holds and any a banding carries
/// onto it, `width` the width its line reads at where a rule writes it
/// and `None` where it reads as the source wrote it, and `splits` marks
/// a roster whose last row a carried own-line comment lands above.
#[derive(Clone)]
struct Seat<'a> {
    member: aligner::Member,
    packs: Packs<'a>,
    splits: bool,
    stmt: &'a Stmt,
    tail: usize,
    width: Option<usize>,
}

impl Seat<'_> {
    /// The columns the members of `range` take on one row of this
    /// seat, `widths` measuring the roster in step, with the seat's
    /// tail on the roster's last row.
    fn content(&self, widths: &[usize], range: &Range<usize>) -> usize {
        widths[range.clone()].iter().sum::<usize>()
            + MEMBER_SEPARATOR.len() * (range.len() - 1)
            + if range.end == widths.len() {
                self.tail
            } else {
                0
            }
    }

    /// The width a row of this seat reads at carrying `content` past
    /// `import`, the gap ahead of the keyword read as the source wrote
    /// it.
    fn row_width(&self, source: &Source, content: usize) -> usize {
        self.member.baseline
            + self.member.width
            + display_width(source.slice(self.member.gap))
            + IMPORT_KEYWORD_WIDTH
            + content
    }

    /// The seat of this roster's last row alone, holding the members of
    /// `range` at the width they packed to, opening the run a carried
    /// comment seats beneath the rows above it.
    fn spill(&self, source: &Source, range: Range<usize>) -> Self {
        let Packs::Roster(names) = &self.packs else {
            unreachable!("invariant: a seat spills a row of its roster");
        };
        let widths: Vec<usize> = names.iter().map(|name| display_width(name)).collect();
        let content = self.content(&widths, &range);
        Self {
            member: self.member,
            packs: Packs::Spilled(range),
            splits: false,
            stmt: self.stmt,
            tail: 0,
            width: Some(self.row_width(source, content)),
        }
    }
}

impl<'a> Layout<'a> {
    /// The `align-imports` runs of `body` as the later rules seat them,
    /// the merges in `groups` folded into their leads, each section's
    /// run sorted where `alphabetize-siblings` or a band sorts it, each
    /// comment the banding carries read on the row it lands on, and a
    /// run closing where two rows land apart or change canonical
    /// group. A row held for `align-imports` bridges its run without
    /// seating.
    fn align_runs(
        &self,
        settings: aligner::Settings,
        body: &'a [Stmt],
        outer: TextRange,
        runs: &MergeRuns,
        groups: &[Vec<usize>],
    ) -> Vec<Vec<Seat<'a>>> {
        let source = self.source;
        let rule = self.rule;
        let blocks = runs.blocks(source, body, outer);
        let sections = Sections::of(source, blocks);
        let mut folded = vec![false; body.len()];
        let mut lead_of: FxHashMap<usize, usize> = FxHashMap::default();
        let mut rosters: FxHashMap<usize, Vec<&'a str>> = FxHashMap::default();
        for group in groups {
            for &slot in &group[1..] {
                folded[slot] = true;
                lead_of.insert(slot, group[0]);
            }
            rosters.insert(group[0], self.roster(super::group_aliases(body, group)));
        }
        let carried = Carried::of(source, &runs.carries, |slot| {
            lead_of.get(&slot).copied().unwrap_or(slot)
        });
        let sorts = (rule.sorts || runs.banded) && !any_sibling_shares_line(source, body);
        let mut seated: Vec<Vec<Seat<'a>>> = Vec::new();
        for run in &runs.runs {
            for section in run.chunk_by(|_, &next| !sections.is_boundary(next)) {
                let survivors: Vec<usize> = section
                    .iter()
                    .copied()
                    .filter(|&slot| !folded[slot])
                    .collect();
                let mut order = survivors.clone();
                if sorts {
                    order.sort_by_key(|&slot| {
                        import_sort_key(&body[slot], &rule.first_party, rule.group_imports)
                    });
                }
                let mut key = None;
                for (position, &slot) in order.iter().enumerate() {
                    let stmt = &body[slot];
                    let Some(member) = qualify_from(source, stmt) else {
                        key = None;
                        continue;
                    };
                    if aligner::is_held(source, AlignImports::SLUG, stmt.start()) {
                        continue;
                    }
                    let group = rule
                        .divides
                        .then(|| import_group(stmt, &rule.first_party))
                        .flatten();
                    let roster = rosters.remove(&slot).or_else(|| {
                        stmt.as_import_from_stmt()
                            .filter(|node| {
                                node.names.len() > 1 && own_line_indent(source, node).is_some()
                            })
                            .map(|node| self.roster(node.names.iter()))
                    });
                    let headed = carried.headed.contains(&slot);
                    let led = (headed && roster.is_none())
                        || (!carried.unheaded.contains(&slot)
                            && blocks[slot].start() != source.text().line_start(stmt.start()));
                    let adjacent = position > 0
                        && !led
                        && self.lands_under(
                            body,
                            blocks,
                            [survivors[position - 1], survivors[position]],
                            [order[position - 1], slot],
                            sorts,
                        );
                    if !adjacent || key != Some(group) {
                        seated.push(Vec::new());
                        key = Some(group);
                    }
                    let trailed = carried.trailed.get(&slot).copied().unwrap_or(0);
                    seated
                        .last_mut()
                        .expect("a run opens before its row seats")
                        .push(self.seat(settings, member, stmt, roster, headed, trailed));
                }
            }
        }
        seated
    }

    /// True when the row of `moved[1]`, opening on its own statement,
    /// lands directly under the row of `moved[0]` once the later rules
    /// lay the block out, the two seated at the run positions `held`
    /// hold as written. Same-group rows seat tight where `sorts` holds
    /// because a rule collapses their gap, or where `band-constants`
    /// writes it across a hoisted constant, unless a comment outside the
    /// hoisted blocks sits between them, whereas any other pair keeps
    /// the gap the source wrote, less the lines of the folded rows.
    fn lands_under(
        &self,
        body: &[Stmt],
        blocks: &[TextRange],
        held: [usize; 2],
        moved: [usize; 2],
        sorts: bool,
    ) -> bool {
        let source = self.source;
        let rule = self.rule;
        let [above, below] = held;
        let [prev, this] = moved;
        let between = above + 1..below;
        let hoisted = between.clone().any(|slot| !is_import(&body[slot]));
        let tight = import_blank_lines(
            &body[prev],
            &body[this],
            &rule.first_party,
            rule.group_imports,
        ) == Some(0);
        if tight && (sorts || hoisted) {
            let gap = TextRange::new(blocks[above].end(), blocks[below].start());
            return source
                .comment_ranges()
                .comments_in_range(gap)
                .iter()
                .all(|comment| {
                    between.clone().any(|slot| {
                        !is_import(&body[slot]) && blocks[slot].contains_range(*comment)
                    })
                });
        }
        !hoisted
            && (above + 1..=below)
                .all(|slot| source.consecutive_lines(blocks[slot - 1].end(), blocks[slot].start()))
    }

    /// One row of an `align-imports` run for `stmt`, packing `roster`
    /// where the rule could split it, `headed` where a banding carries
    /// an own-line comment onto it and `trailed` wide where it carries a
    /// trailing one. A roster alone reads its tail, the comment its last
    /// row keeps, so a written row measures none.
    fn seat(
        &self,
        settings: aligner::Settings,
        member: aligner::Member,
        stmt: &'a Stmt,
        roster: Option<Vec<&'a str>>,
        headed: bool,
        trailed: usize,
    ) -> Seat<'a> {
        let source = self.source;
        let tail = roster.as_ref().map_or(0, |_| {
            aligner::settled_tail(source, member, settings, stmt.end()) + trailed
        });
        Seat {
            member,
            splits: headed && roster.is_some(),
            packs: roster.map_or(Packs::Written, Packs::Roster),
            stmt,
            tail,
            width: (trailed > 0).then(|| {
                display_width(source.slice(source.text().line_range(member.line_start))) + trailed
            }),
        }
    }

    /// The packing of every roster in `seats`, one `align-imports` run
    /// in reading order, each row seated at the column the aligner
    /// settles it to. Each roster first reads at its widest row to
    /// resolve the run's column, and the rows it packs into then seat
    /// one by one, a row fitting a wider column above joining it while
    /// the rows beneath keep their own.
    fn seat_run(&self, settings: aligner::Settings, seats: &[Seat<'a>]) -> Vec<(usize, Packing)> {
        let source = self.source;
        let budget = self.rule.import_line_length;
        let widenings = aligner::Widenings::default();
        let members: Vec<aligner::Member> = seats.iter().map(|seat| seat.member).collect();
        let elastic: Vec<Option<usize>> = seats
            .iter()
            .map(|seat| match &seat.packs {
                Packs::Roster(names) => {
                    let widest = names
                        .iter()
                        .map(|name| display_width(name))
                        .max()
                        .unwrap_or(0);
                    let last = names.last().map_or(0, |name| display_width(name)) + seat.tail;
                    Some(seat.row_width(source, widest.max(last)))
                }
                Packs::Spilled(_) | Packs::Written => seat.width,
            })
            .collect();
        let columns = aligner::operator_columns(source, &members, settings, &widenings, &elastic);
        let mut rows = Vec::new();
        let mut joined = Vec::new();
        let mut packed = Vec::new();
        for (index, ((seat, column), width)) in seats.iter().zip(columns).zip(elastic).enumerate() {
            let first = rows.len();
            match &seat.packs {
                Packs::Roster(names) => {
                    let widths: Vec<usize> = names.iter().map(|name| display_width(name)).collect();
                    let ranges = pack(
                        &widths,
                        column + IMPORT_KEYWORD_WIDTH,
                        MEMBER_SEPARATOR.len(),
                        budget,
                    );
                    for range in &ranges {
                        rows.push(seat.member);
                        joined.push(Some(seat.row_width(source, seat.content(&widths, range))));
                    }
                    packed.push((index, ranges, first..rows.len()));
                }
                Packs::Spilled(range) => {
                    rows.push(seat.member);
                    joined.push(width);
                    packed.push((index, vec![range.clone()], first..rows.len()));
                }
                Packs::Written => {
                    rows.push(seat.member);
                    joined.push(width);
                }
            }
        }
        let columns = aligner::forecast_columns(source, &rows, settings, &widenings, &joined);
        packed
            .into_iter()
            .map(|(index, ranges, at)| {
                let member = seats[index].member;
                let gaps = columns[at]
                    .iter()
                    .map(|column| column - member.baseline - member.settled_width);
                (index, ranges.into_iter().zip(gaps).collect())
            })
            .collect()
    }

    /// The packing of every from-import `reflow-imports` could split in
    /// `body`, each row seated at the column `align-imports` settles it
    /// to, keyed by statement start. A roster a carried comment splits
    /// packs with the rows above it and spills its last row, fixed at
    /// the width it packed to, into the run the comment opens beneath,
    /// or spills whole where it packs into one row.
    pub(super) fn forecast(
        &self,
        settings: aligner::Settings,
        body: &'a [Stmt],
        outer: TextRange,
        runs: &MergeRuns,
        groups: &[Vec<usize>],
    ) -> FxHashMap<TextSize, Packing> {
        let mut packings: FxHashMap<TextSize, Packing> = FxHashMap::default();
        let mut record = |seats: &[Seat<'a>], packed: Vec<(usize, Packing)>| {
            for (index, packing) in packed {
                packings
                    .entry(seats[index].stmt.start())
                    .or_default()
                    .extend(packing);
            }
        };
        for run in self.align_runs(settings, body, outer, runs, groups) {
            let mut at = 0;
            let mut spilled: Option<Seat<'a>> = None;
            while at < run.len() {
                let split = run[at..]
                    .iter()
                    .position(|seat| seat.splits)
                    .map(|found| at + found);
                let end = split.map_or(run.len(), |split| split + 1);
                let mut seats: Vec<Seat<'a>> = spilled
                    .take()
                    .into_iter()
                    .chain(run[at..end].iter().cloned())
                    .collect();
                let mut packed = self.seat_run(settings, &seats);
                if split.is_some() {
                    let (_, mut packing) = packed.pop().expect("a splitting seat packs its roster");
                    let seat = seats.pop().expect("a splitting seat closes its run");
                    if packing.len() == 1 {
                        // The comment lands above the roster's only row,
                        // so the roster opens the run beneath instead.
                        packed = self.seat_run(settings, &seats);
                        spilled = Some(Seat {
                            splits: false,
                            ..seat
                        });
                    } else {
                        let (range, _) = packing.pop().expect("a packed roster holds a row");
                        spilled = Some(seat.spill(self.source, range));
                        seats.push(seat);
                        packed.push((seats.len() - 1, packing));
                    }
                }
                record(&seats, packed);
                at = end;
            }
            if let Some(seat) = spilled {
                let seats = [seat];
                record(&seats, self.seat_run(settings, &seats));
            }
        }
        packings
    }
}

#[cfg(test)]
mod tests {
    use ruff_text_size::TextSize;

    use super::*;
    use crate::testing::parse;

    #[test]
    fn carried_reads_each_comment_on_the_row_it_lands_on() {
        let source = parse("# documents os\nimport os\n");
        let comment = TextRange::new(TextSize::new(0), TextSize::new(14));
        let carries = [
            Carry {
                absorbs: 3,
                carrier: 2,
                comment,
                trails: false,
            },
            Carry {
                absorbs: 4,
                carrier: 1,
                comment,
                trails: true,
            },
        ];
        let carried = Carried::of(&source, &carries, |slot| if slot == 2 { 0 } else { slot });

        // The own-line carry heads its carrier through the seat map, the
        // trailing carry charges its carrier the gap and comment width,
        // and both absorbing rows read as unheaded.
        assert!(carried.headed.contains(&0));
        assert!(carried.unheaded.contains(&3) && carried.unheaded.contains(&4));
        assert_eq!(carried.trailed.get(&1), Some(&16));
    }
}
