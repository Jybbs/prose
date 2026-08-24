//! Lays out an import block one module per line with its members
//! gathered behind it. `split-multi-module` breaks a comma-joined
//! `import a, b` into one statement per module, `merge-members` gathers
//! every `from <module> import …` line of one import run onto one
//! statement carrying each member once, and a roster overrunning
//! `Config::import_line_length` splits into repeated-prefix lines
//! greedily packed to that budget, each row seated at the column
//! `align-imports` settles it to once the rules between the two have
//! laid the block out. A multi-line import stays untouched, and a lone
//! name whose own line overflows keeps it rather than splitting further.

use std::{
    borrow::Cow,
    cell::OnceCell,
    collections::{HashMap, HashSet},
    ops::Range,
};

use itertools::Itertools;
use ruff_diagnostics::Edit;
use ruff_python_ast::{
    Alias, Stmt, StmtImport, StmtImportFrom, helpers::format_import_from, token::TokenKind,
};
use ruff_python_trivia::indentation_at_offset;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use unicode_width::UnicodeWidthStr;

use crate::{
    config::Config,
    primitives::{
        aligner,
        comments::TRAILING_GAP,
        edit::{apply_inline_edits, narrowed_replacement, whole_line_deletion},
        imports::{
            IMPORT_KEYWORD_WIDTH, ModuleKey, fold_landing, import_blank_lines, import_group,
            import_sort_key, is_import, module_key, stands_alone,
        },
        layout::pack,
        orderer::{any_sibling_shares_line, member_blocks},
        scope::{scoped_body, sub_bodies},
        sections::Sections,
        slots::slot_runs,
    },
    rule::{Rule, RuleId},
    rules::{
        align_imports::{AlignImports, qualify_from},
        band_constants::{BandConstants, Bands, Carry},
    },
    source::Source,
};

/// What joins two members sharing one line, written between them and
/// counted against the budget each line packs to.
const MEMBER_SEPARATOR: &str = ", ";

pub(crate) struct ReflowImports {
    align_settings: Option<aligner::Settings>,
    bands: Option<BandConstants>,
    divides: bool,
    first_party: Vec<String>,
    group_imports: bool,
    import_line_length: usize,
    merge_members: bool,
    sorts: bool,
    split_multi_module: bool,
}

impl ReflowImports {
    pub(crate) const MESSAGE: &'static str = "lay out an import block one module per line";

    pub(crate) fn from_config(config: &Config) -> Self {
        let align = &config.rules.align_imports;
        let rules = &config.rules.reflow_imports;
        Self {
            // Forecast the aligned column only when `align-imports`
            // runs, under the settings that rule resolves within, so
            // the column the forecast names is one the capped run seats.
            align_settings: align.enabled.then(|| config.import_align_settings()),
            bands: band_forecast(config),
            divides: config.group_imports_enabled() && config.rules.space_statements.enabled,
            first_party: config.first_party(),
            group_imports: config.group_imports_enabled(),
            import_line_length: config.import_width(),
            merge_members: rules.merge_members,
            sorts: config.alphabetize_siblings_enabled(),
            split_multi_module: rules.split_multi_module,
        }
    }
}

impl Rule for ReflowImports {
    fn apply(&self, source: &Source) -> Vec<Vec<Edit>> {
        let mut layout = Layout {
            groups: Vec::new(),
            newline: source.newline_str(),
            packings: HashMap::new(),
            rule: self,
            source,
        };
        layout.layout_scope(&source.ast().body, source.module_range(), true);
        layout.groups
    }

    fn id(&self) -> RuleId {
        Self::SLUG
    }
}

/// The rows a from-import packs into, each the members it carries
/// beside the gap it holds ahead of `import`.
type Packing = Vec<(Range<usize>, usize)>;

struct Layout<'a> {
    groups: Vec<Vec<Edit>>,
    newline: &'static str,
    /// The forecast packing of every from-import the body under layout
    /// could split, keyed by statement start.
    packings: HashMap<TextSize, Packing>,
    rule: &'a ReflowImports,
    source: &'a Source,
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
        let mut lead_of: HashMap<usize, usize> = HashMap::new();
        let mut rosters: HashMap<usize, Vec<&'a str>> = HashMap::new();
        for group in groups {
            for &slot in &group[1..] {
                folded[slot] = true;
                lead_of.insert(slot, group[0]);
            }
            rosters.insert(group[0], self.roster(group_aliases(body, group)));
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

    /// The packing of every from-import `reflow-imports` could split in
    /// `body`, each row seated at the column `align-imports` settles it
    /// to, keyed by statement start. A roster a carried comment splits
    /// packs with the rows above it and spills its last row, fixed at
    /// the width it packed to, into the run the comment opens beneath,
    /// or spills whole where it packs into one row.
    fn forecast(
        &self,
        settings: aligner::Settings,
        body: &'a [Stmt],
        outer: TextRange,
        runs: &MergeRuns,
        groups: &[Vec<usize>],
    ) -> HashMap<TextSize, Packing> {
        let mut packings: HashMap<TextSize, Packing> = HashMap::new();
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

    /// Lays out `body` and then every body beneath it, a class or
    /// function suite leaving module scope so no band forecast reaches
    /// the imports inside it.
    fn layout_scope(&mut self, body: &'a [Stmt], outer: TextRange, module_scope: bool) {
        self.process_body(body, outer, module_scope);
        for stmt in body {
            let nested = module_scope && scoped_body(stmt).is_none();
            for (sub, sub_outer) in sub_bodies(stmt) {
                self.layout_scope(sub, sub_outer, nested);
            }
        }
    }

    /// Folds every member of `group` into its first statement, laying
    /// the gathered roster out under the shared head and clearing each
    /// folded member's line. A group whose members already read that way
    /// emits nothing.
    fn merge_group(&mut self, body: &'a [Stmt], group: &[usize]) {
        let [lead, .., last] = group else {
            unreachable!("invariant: a merge group holds two or more members");
        };
        let node = body[*lead]
            .as_import_from_stmt()
            .expect("a merge group holds `from`-imports alone");
        let names = self.roster(group_aliases(body, group));
        let mut edits: Vec<Edit> = self
            .rows(node, &names)
            .and_then(|rows| self.packed_edit(node, &names, &rows))
            .into_iter()
            .collect();
        edits.extend(
            group[1..]
                .iter()
                .map(|&slot| whole_line_deletion(self.source, body[slot].range())),
        );
        let span = self
            .source
            .full_lines_within_cell(TextRange::new(body[*lead].start(), body[*last].end()));
        if apply_inline_edits(self.source, span, &edits) != self.source.slice(span) {
            self.groups.push(edits);
        }
    }

    /// Emits the packed rewrite of `node` when its roster overruns the
    /// row it opens.
    fn pack_lone(&mut self, node: &'a StmtImportFrom) {
        let [_, _, ..] = node.names.as_slice() else {
            return;
        };
        let names = self.roster(node.names.iter());
        let Some(rows) = self.rows(node, &names).filter(|rows| rows.len() > 1) else {
            return;
        };
        self.groups
            .extend(self.packed_edit(node, &names, &rows).map(|edit| vec![edit]));
    }

    /// The edit rewriting `node` to carry `names` on `rows`, the head
    /// repeated on each row ahead of its gap and `import`. `None` when
    /// the statement does not open its own line or already reads that
    /// way.
    fn packed_edit(
        &self,
        node: &StmtImportFrom,
        names: &[&str],
        rows: &[(Range<usize>, Cow<'a, str>)],
    ) -> Option<Edit> {
        let indent = own_line_indent(self.source, node)?;
        let head = import_head(node);
        let joiner = format!("{}{indent}", self.newline);
        let rewrite = rows
            .iter()
            .map(|(range, gap)| {
                format!(
                    "{head}{gap}import {}",
                    names[range.clone()].join(MEMBER_SEPARATOR)
                )
            })
            .join(&joiner);
        narrowed_replacement(self.source, node.range(), rewrite)
    }

    /// Folds each repeated module in `body` into one statement and
    /// splits every comma-joined bare import, one fix group apiece. At
    /// module scope a repeated module gathers across the constants
    /// `band-constants` hoists from between its statements.
    fn process_body(&mut self, body: &'a [Stmt], outer: TextRange, module_scope: bool) {
        let rule = self.rule;
        let source = self.source;
        let runs = MergeRuns::of(
            rule.bands.as_ref().filter(|_| module_scope),
            source,
            body,
            outer,
            |runs| {
                rule.align_settings.is_some()
                    && (runs.len() > 1 || !rule.sorts || comments_beside(source, body, outer, runs))
            },
        );
        if runs.runs.is_empty() {
            return;
        }
        let groups = if rule.merge_members {
            module_groups(self.source, body, outer, &runs)
        } else {
            Vec::new()
        };
        self.packings = rule.align_settings.map_or_else(HashMap::new, |settings| {
            self.forecast(settings, body, outer, &runs, &groups)
        });
        let gathered: Vec<usize> = groups.iter().flatten().copied().collect();
        for group in &groups {
            self.merge_group(body, group);
        }
        for (slot, stmt) in body.iter().enumerate() {
            match stmt {
                Stmt::Import(bare) if rule.split_multi_module => self.split_bare_import(bare),
                Stmt::ImportFrom(lone) if !gathered.contains(&slot) => self.pack_lone(lone),
                _ => {}
            }
        }
    }

    /// The de-duplicated source text of `aliases`, the member roster one
    /// module's rows share, ordered as `alphabetize-siblings` would leave
    /// it unless that rule is off.
    fn roster(&self, aliases: impl Iterator<Item = &'a Alias>) -> Vec<&'a str> {
        let mut names: Vec<&str> = aliases
            .map(|alias| self.source.slice(alias.range()))
            .unique()
            .collect();
        if self.rule.sorts {
            names.sort_unstable();
        }
        names
    }

    /// The rows `node` packs `names` into and the gap each row holds
    /// ahead of `import`, the forecast packing where `align-imports`
    /// runs and otherwise the roster packed from the keyword's own
    /// column with the gap the source wrote repeated on every row.
    /// `None` where the keyword opens a line of its own.
    fn rows(
        &self,
        node: &StmtImportFrom,
        names: &[&str],
    ) -> Option<Vec<(Range<usize>, Cow<'a, str>)>> {
        if let Some(packing) = self.packings.get(&node.start()) {
            return Some(
                packing
                    .iter()
                    .map(|(range, gap)| (range.clone(), Cow::Owned(" ".repeat(*gap))))
                    .collect(),
            );
        }
        let gap = import_keyword_gap(self.source, node)?;
        let widths: Vec<usize> = names.iter().map(|name| name.width()).collect();
        let prefix = self.source.line_indent_width(node.start())
            + import_head(node).width()
            + gap.width()
            + IMPORT_KEYWORD_WIDTH;
        Some(
            pack(
                &widths,
                prefix,
                MEMBER_SEPARATOR.len(),
                self.rule.import_line_length,
            )
            .into_iter()
            .map(|range| (range, Cow::Borrowed(gap)))
            .collect(),
        )
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
                source
                    .slice(source.text().line_range(member.line_start))
                    .width()
                    + trailed
            }),
        }
    }

    /// The packing of every roster in `seats`, one `align-imports` run
    /// in reading order, each row seated at the column the aligner
    /// settles it to. The rosters first read at their widest row, the
    /// widest member or the last member with the trailing comment it
    /// keeps, packing to whatever column the run settles on, and the
    /// rows each roster then packs into seat one by one under the run
    /// the aligner reads them in, so a row fitting a wider column above
    /// joins it while the rows beneath keep their own.
    fn seat_run(&self, settings: aligner::Settings, seats: &[Seat<'a>]) -> Vec<(usize, Packing)> {
        let source = self.source;
        let budget = self.rule.import_line_length;
        let widenings = aligner::Widenings::default();
        let members: Vec<aligner::Member> = seats.iter().map(|seat| seat.member).collect();
        let elastic: Vec<Option<usize>> = seats
            .iter()
            .map(|seat| match &seat.packs {
                Packs::Roster(names) => {
                    let widest = names.iter().map(|name| name.width()).max().unwrap_or(0);
                    let last = names.last().map_or(0, |name| name.width()) + seat.tail;
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
                    let widths: Vec<usize> = names.iter().map(|name| name.width()).collect();
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
        let columns = aligner::operator_columns(source, &rows, settings, &widenings, &joined);
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

    /// Emits the one-statement-per-module rewrite of a comma-joined
    /// bare import.
    fn split_bare_import(&mut self, node: &StmtImport) {
        let [_, _, ..] = node.names.as_slice() else {
            return;
        };
        let Some(indent) = own_line_indent(self.source, node) else {
            return;
        };
        let joiner = format!("{}{indent}", self.newline);
        let rewrite = node
            .names
            .iter()
            .map(|alias| format!("import {}", self.source.slice(alias.range())))
            .join(&joiner);
        self.groups.extend(
            narrowed_replacement(self.source, node.range(), rewrite).map(|edit| vec![edit]),
        );
    }
}

/// The rows a banding moves comments between, read by slot: the rows
/// whose heading leaves them, the rows an own-line comment lands
/// above, and the rows a comment trails, each with the columns it
/// adds.
struct Carried {
    headed: HashSet<usize>,
    trailed: HashMap<usize, usize>,
    unheaded: HashSet<usize>,
}

impl Carried {
    /// Reads `carries` with each carrier resolved through `seat_of`, the
    /// row a comment lands on where the carrier folds into a merge.
    fn of(source: &Source, carries: &[Carry], seat_of: impl Fn(usize) -> usize) -> Self {
        let mut carried = Self {
            headed: HashSet::new(),
            trailed: HashMap::new(),
            unheaded: HashSet::new(),
        };
        for carry in carries {
            carried.unheaded.insert(carry.absorbs);
            let carrier = seat_of(carry.carrier);
            if carry.trails {
                let width = TRAILING_GAP.width() + source.slice(carry.comment).trim_start().width();
                *carried.trailed.entry(carrier).or_default() += width;
            } else {
                carried.headed.insert(carrier);
            }
        }
        carried
    }
}

/// The runs a body's merges gather within, beside the member blocks a
/// comment between two members is read against, built on first use
/// through [`Self::blocks`]. `banded` marks runs that are the bands
/// `band-constants` sorts,
/// `carries` then holding every comment the banding moves between
/// members.
struct MergeRuns {
    banded: bool,
    blocks: OnceCell<Vec<TextRange>>,
    carries: Vec<Carry>,
    runs: Vec<Vec<usize>>,
    /// The slot each band's sort seats first, in step with `runs` and
    /// empty where the runs are the imports as written.
    sorted_heads: Vec<usize>,
}

impl MergeRuns {
    /// The member blocks of `body` through the statement after its
    /// last import, every slot the runs and the gaps between them
    /// read, built on first use.
    fn blocks(&self, source: &Source, body: &[Stmt], outer: TextRange) -> &[TextRange] {
        self.blocks.get_or_init(|| {
            let reach = self
                .runs
                .iter()
                .flatten()
                .max()
                .map_or(0, |&last| (last + 2).min(body.len()));
            member_blocks(source, &body[..reach], outer)
        })
    }

    /// The import runs of `body` as written, or the bands `bands`
    /// forecasts once it hoists the constants between two runs and
    /// sorts each, sought where a module repeats across the runs as
    /// written or where `seek` reads them as needed over the runs as
    /// written.
    fn of(
        bands: Option<&BandConstants>,
        source: &Source,
        body: &[Stmt],
        outer: TextRange,
        seek: impl FnOnce(&[Vec<usize>]) -> bool,
    ) -> Self {
        let runs = import_runs(body);
        let joined = bands
            .filter(|_| seek(&runs) || repeats_across(source, body, &runs))
            .and_then(|rule| rule.import_bands(source, body, outer));
        match joined {
            Some(Bands {
                blocks,
                carries,
                imports,
            }) => {
                let (runs, sorted_heads) = imports
                    .into_iter()
                    .map(|band| (band.slots, band.sorted_head))
                    .unzip();
                Self {
                    banded: true,
                    blocks: OnceCell::from(blocks),
                    carries,
                    runs,
                    sorted_heads,
                }
            }
            None => Self {
                banded: false,
                blocks: OnceCell::new(),
                carries: Vec::new(),
                runs,
                sorted_heads: Vec::new(),
            },
        }
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
            + source.slice(self.member.gap).width()
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
        let widths: Vec<usize> = names.iter().map(|name| name.width()).collect();
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

/// The same-module merges `reflow-imports` makes and the import bands
/// `band-constants` heads, forecast by a rule seated ahead of both
/// whose drop of a comment-led statement then lands on the import the
/// comment reads over.
pub(crate) struct Folds {
    bands: Option<BandConstants>,
    merges: bool,
}

impl Folds {
    pub(crate) fn from_config(config: &Config) -> Self {
        let rules = &config.rules.reflow_imports;
        Self {
            bands: band_forecast(config),
            merges: rules.enabled && rules.merge_members,
        }
    }

    /// The module-body slot whose import the drop of `slot` lands on,
    /// per [`fold_landing`], `None` when no later rule carries the
    /// comment onto a sibling that `survives` the drops.
    pub(crate) fn landing(
        &self,
        source: &Source,
        slot: usize,
        survives: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let body = &source.ast().body;
        let runs = MergeRuns::of(
            self.bands.as_ref(),
            source,
            body,
            source.module_range(),
            |_| true,
        );
        fold_landing(
            source,
            body,
            &runs.runs,
            &runs.sorted_heads,
            self.merges,
            slot,
            survives,
        )
    }
}

/// `band-constants` as configured, `None` when the rule is off.
fn band_forecast(config: &Config) -> Option<BandConstants> {
    config
        .rules
        .band_constants
        .enabled
        .then(|| BandConstants::from_config(config))
}

/// True when a comment sits within or beside one of `runs`, from the
/// end of the statement before the run to the start of the one after
/// it, the reach a banding carries a comment across.
fn comments_beside(source: &Source, body: &[Stmt], outer: TextRange, runs: &[Vec<usize>]) -> bool {
    runs.iter().any(|run| {
        let (first, last) = (run[0], run[run.len() - 1]);
        let lower = first
            .checked_sub(1)
            .map_or(outer.start(), |prev| body[prev].end());
        let upper = body.get(last + 1).map_or(outer.end(), Ranged::start);
        !source
            .comment_ranges()
            .comments_in_range(TextRange::new(lower, upper))
            .is_empty()
    })
}

/// True when the members at `slots` fold into one statement without
/// disturbing their surroundings, being two or more statements of one
/// notebook cell whose span from first to last carries no comment
/// outside the block of a statement between them, which stays in place
/// while the gather clears the member lines.
fn gathers_cleanly(
    source: &Source,
    body: &[Stmt],
    outer: TextRange,
    slots: &[usize],
    runs: &MergeRuns,
) -> bool {
    let [first, .., last] = slots else {
        return false;
    };
    let span =
        source.full_lines_within_cell(TextRange::new(body[*first].start(), body[*last].end()));
    if !source.same_cell(span.start(), span.end()) {
        return false;
    }
    let comments = source.comment_ranges().comments_in_range(span);
    if comments.is_empty() {
        return true;
    }
    let blocks = runs.blocks(source, body, outer);
    comments.iter().all(|comment| {
        (*first + 1..*last)
            .filter(|slot| !slots.contains(slot))
            .any(|slot| blocks[slot].contains_range(*comment))
    })
}

/// Every alias the `from`-imports at `slots` of `body` carry, in slot
/// order.
fn group_aliases<'a>(body: &'a [Stmt], slots: &[usize]) -> impl Iterator<Item = &'a Alias> {
    slots
        .iter()
        .filter_map(|&slot| body[slot].as_import_from_stmt())
        .flat_map(|node| &node.names)
}

/// The `from <dots><module>` head each row of `node` repeats, with the
/// relative-import leading dots folded into it.
fn import_head(node: &StmtImportFrom) -> String {
    format!(
        "from {}",
        format_import_from(node.level, node.module.as_deref())
    )
}

/// The whitespace between `node`'s module and its `import` keyword, the
/// column `align-imports` pads the keyword to. `None` when the keyword
/// opens a line of its own.
fn import_keyword_gap<'src>(source: &'src Source, node: &StmtImportFrom) -> Option<&'src str> {
    let anchored = aligner::line_anchored_member_at_kind(
        source,
        node.start(),
        node.range(),
        TokenKind::Import,
    )?;
    Some(source.slice(anchored.gap))
}

/// The runs of adjacent import statements in `body`, a lone import a
/// run of its own.
fn import_runs(body: &[Stmt]) -> Vec<Vec<usize>> {
    slot_runs(body, |a, b| is_import(a) && is_import(b))
        .filter(|run| is_import(&body[run.start]))
        .map(Iterator::collect)
        .collect()
}

/// True when `node` can join a merged roster, being a single-line
/// `from`-import holding its line alone, so the fold clears no code
/// sharing it, and binding no star member, since `*` admits no sibling
/// on its statement.
fn mergeable(source: &Source, node: &StmtImportFrom) -> bool {
    own_line_indent(source, node).is_some()
        && stands_alone(source, node.range())
        && !node.names.iter().any(|alias| alias.name.as_str() == "*")
}

/// Slot groups of two or more mergeable `from`-imports sharing one
/// module within one of `runs`, each group spanning one notebook cell
/// and gathering cleanly per [`gathers_cleanly`].
fn module_groups(
    source: &Source,
    body: &[Stmt],
    outer: TextRange,
    runs: &MergeRuns,
) -> Vec<Vec<usize>> {
    let mut groups = Vec::new();
    for run in &runs.runs {
        let mut by_module: Vec<(ModuleKey, Vec<usize>)> = Vec::new();
        for &slot in run {
            let Some(node) = body[slot]
                .as_import_from_stmt()
                .filter(|n| mergeable(source, n))
            else {
                continue;
            };
            let key = module_key(node);
            match by_module.iter_mut().find(|(seen, _)| *seen == key) {
                Some((_, slots)) => slots.push(slot),
                None => by_module.push((key, vec![slot])),
            }
        }
        groups.extend(
            by_module
                .into_iter()
                .map(|(_, slots)| slots)
                .filter(|slots| gathers_cleanly(source, body, outer, slots, runs)),
        );
    }
    groups
}

/// True when a mergeable `from`-import's module recurs in a second run
/// of `runs`, the one shape a hoist between the runs would gather.
fn repeats_across(source: &Source, body: &[Stmt], runs: &[Vec<usize>]) -> bool {
    let mut seen: HashMap<ModuleKey, usize> = HashMap::new();
    runs.iter().enumerate().any(|(index, run)| {
        run.iter()
            .filter_map(|&slot| body[slot].as_import_from_stmt())
            .filter(|node| mergeable(source, node))
            .any(|node| *seen.entry(module_key(node)).or_insert(index) != index)
    })
}

/// The leading-whitespace prefix of `node`'s line when `node` is a
/// single-line statement beginning that line, or `None` when it spans
/// a line break or other code precedes it (a `;`-joined statement).
fn own_line_indent<'src>(source: &'src Source, node: &impl Ranged) -> Option<&'src str> {
    if source.contains_line_break(node.range()) {
        return None;
    }
    indentation_at_offset(node.start(), source.text())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::parse;

    #[rstest]
    #[case("from a.b.c import x\n", "from a.b.c")]
    #[case("from . import x\n", "from .")]
    #[case("from .sub import x\n", "from .sub")]
    #[case("from ..pkg import x\n", "from ..pkg")]
    #[case("from typing     import x\n", "from typing")]
    fn import_head_folds_relative_dots_into_the_repeated_head(
        #[case] src: &str,
        #[case] expected: &str,
    ) {
        let source = parse(src);
        let node = source.ast().body[0]
            .as_import_from_stmt()
            .expect("first statement is a from-import");
        assert_eq!(import_head(node), expected);
    }

    #[rstest]
    #[case("from pkg import x\n", Some(" "))]
    #[case("from pkg     import x\n", Some("     "))]
    #[case("from pkg\timport x\n", Some("\t"))]
    #[case("from . import x\n", Some(" "))]
    #[case("from pkg import (\n    x,\n)\n", Some(" "))]
    #[case("from pkg \\\n    import x\n", None)]
    fn import_keyword_gap_reads_the_spaces_before_the_keyword(
        #[case] src: &str,
        #[case] expected: Option<&str>,
    ) {
        let source = parse(src);
        let node = source.ast().body[0]
            .as_import_from_stmt()
            .expect("first statement is a from-import");

        assert_eq!(import_keyword_gap(&source, node), expected);
    }

    /// The rule with every facet on and a ten-column import budget,
    /// forecasting no aligned column.
    fn tight_rule() -> ReflowImports {
        ReflowImports {
            align_settings: None,
            bands: None,
            divides: false,
            first_party: Vec::new(),
            group_imports: true,
            import_line_length: 10,
            merge_members: true,
            sorts: true,
            split_multi_module: true,
        }
    }

    #[test]
    fn a_merge_leaves_a_statement_sharing_its_line_with_code() {
        let source = parse(
            "from pkg import alpha
from pkg import beta; x = 1
",
        );
        assert!(tight_rule().apply(&source).is_empty());
    }

    #[test]
    fn multi_line_import_is_left_untouched() {
        let source = parse("from pkg import (\n    alpha,\n    beta,\n    gamma,\n)\n");
        assert!(tight_rule().apply(&source).is_empty());
    }

    #[test]
    fn semicolon_joined_bare_import_is_left_untouched() {
        let source = parse("x = 1; import os, sys\n");
        assert!(tight_rule().apply(&source).is_empty());
    }

    #[test]
    fn semicolon_joined_import_is_left_untouched() {
        let source = parse("x = 1; from pkg import alpha, beta, gamma\n");
        assert!(tight_rule().apply(&source).is_empty());
    }
}
