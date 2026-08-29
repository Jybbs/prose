//! The banding analysis. Ranks each module-scope statement into an
//! import, leading-constant, definition, or trailing-constant band and
//! tiers the constant bands through the shared `primitives::tiering`
//! graph, declining when a band's reference graph carries a cycle. Each
//! own-line comment binds to the member above or below it that it
//! documents.

use std::collections::{HashMap, HashSet};

use ruff_python_ast::{Expr, PythonVersion, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::{Ranged, TextRange};
use unicode_width::UnicodeWidthStr;

use super::{
    BandConstants,
    plan::{BandPlan, BandRank, Carry, Subcategory},
};
use crate::{
    primitives::{
        alias::{AliasContext, value_is_alias},
        binding::{
            bare_import_bound_name, from_import_bound_name, is_explicit_type_alias,
            is_screaming_case, single_name_assignment,
        },
        comments::{TRAILING_GAP, anchors_in_place, has_keep_marker, leading_comment_block},
        effect::value_is_effectful,
        tiering::{eval_refs, eval_time_refs, tier_levels},
    },
    source::Source,
};

/// A module-scope single-name assignment considered for hoisting,
/// carrying its body index, target name, subcategory, the load-context
/// names in its value and its non-deferred annotation, and whether the
/// value runs code at binding. Value references pin the constant when
/// unresolved, whereas annotation references only constrain band order.
struct ConstSite<'src> {
    annot_refs: Vec<&'src str>,
    effectful: bool,
    idx: usize,
    name: &'src str,
    subcategory: Subcategory,
    value_refs: Vec<&'src str>,
}

impl<'src> ConstSite<'src> {
    /// The load-context names in the site's value and annotation, a
    /// value reference paired with `true` and an annotation reference
    /// with `false`, skipping the site's own name.
    fn foreign_refs(&self) -> impl Iterator<Item = (&'src str, bool)> {
        let name = self.name;
        self.value_refs
            .iter()
            .map(|&r| (r, true))
            .chain(self.annot_refs.iter().map(|&r| (r, false)))
            .filter(move |&(r, _)| r != name)
    }
}

/// Builds the module-scope hoist plan, ranking each statement and
/// pairing each banded member with the comment it carries onto another
/// member's line. Returns `None` when a constant band's reference graph
/// carries a cycle.
pub(super) fn module_band_plan<'src>(
    source: &'src Source,
    body: &'src [Stmt],
    blocks: &[TextRange],
    code_width: usize,
    defer_annotations: bool,
    group_subcategories: bool,
    target_version: Option<PythonVersion>,
) -> Option<BandPlan<'src>> {
    let analysis = source.binding_analysis();
    let aliases = group_subcategories.then(|| AliasContext::new(body, analysis));
    let builtins_minor = target_version.unwrap_or_default().minor;
    let notebook = source.is_notebook();
    let suppression = source.suppression_map();
    let mut def_at: HashMap<&'src str, usize> = HashMap::new();
    let mut dup_defs: HashSet<&'src str> = HashSet::new();
    let mut imports: HashSet<&'src str> = HashSet::new();
    let mut ranks: HashMap<usize, BandRank> = HashMap::new();
    let mut attached: HashMap<usize, TextRange> = HashMap::new();
    let mut carries: Vec<Carry> = Vec::new();
    let mut sites: Vec<ConstSite<'src>> = Vec::new();
    for (idx, stmt) in body.iter().enumerate() {
        // A `# prose: off` span or a skip directive pins its statement, as
        // does a row a `\` join continues, whose relocation would take the
        // break the join rests on. So does an own-line comment run left
        // standing between two blocks,
        // one `member_block` declined to bind because it anchors in place,
        // opens at another indent, or sits behind a notebook cell wall. A
        // pinned member holds its slot, bounding the bands to its side so
        // no reorder drops the run out of the gap holding it, while its
        // name still binds below, so a reference to a pinned definition or
        // import reads as resolved. A cell wall already holds a run clear
        // of the cell-local reorder below it, leaving a constant behind one
        // free to band.
        let gap_comment = idx.checked_sub(1).and_then(|prev| {
            leading_comment_block(source, blocks[prev].end(), blocks[idx].start())
        });
        let const_target = const_binding(stmt);
        let pinned = suppression.suppresses(stmt, BandConstants::SLUG)
            || source.continues_a_logical_line(stmt.start())
            || gap_comment.is_some_and(|block| {
                const_target.is_none()
                    || anchors_in_place(source, block)
                    || source.same_cell(block.start(), stmt.start())
            });
        // The run this member's block folds in ahead of its code. One
        // sitting directly below the previous member and a blank line
        // off this one documents that member and carries backward onto
        // it, while every other run heads this member and relocates
        // only when a sort reseats it.
        if !pinned
            && let Some(block) = leading_comment_block(source, blocks[idx].start(), stmt.start())
        {
            match backward_carry(source, body, blocks, idx, block, code_width) {
                Some(carry) => carries.push(carry),
                None => {
                    attached.insert(idx, block);
                }
            }
        }
        match stmt {
            Stmt::ClassDef(StmtClassDef { name, .. })
            | Stmt::FunctionDef(StmtFunctionDef { name, .. }) => {
                if def_at.insert(name.as_str(), idx).is_some() {
                    dup_defs.insert(name.as_str());
                }
                if !pinned {
                    ranks.insert(idx, BandRank::Definition);
                }
            }
            Stmt::Import(node) => {
                imports.extend(node.names.iter().map(bare_import_bound_name));
                if !pinned {
                    ranks.insert(idx, BandRank::Import);
                }
            }
            Stmt::ImportFrom(node) => {
                imports.extend(node.names.iter().map(from_import_bound_name));
                if !pinned {
                    ranks.insert(idx, BandRank::Import);
                }
            }
            _ => {
                if !pinned && let Some((name, value)) = const_target {
                    // A `# prose: keep` dict pins its statement, so the
                    // marker freezes module position as well as entry order.
                    if let Some(Expr::Dict(dict)) = value
                        && has_keep_marker(source, dict)
                    {
                        continue;
                    }
                    sites.push(ConstSite {
                        annot_refs: stmt
                            .as_ann_assign_stmt()
                            .filter(|_| !defer_annotations)
                            .map_or_else(Vec::new, |ann| eval_refs(&ann.annotation)),
                        effectful: value.is_some_and(value_is_effectful),
                        idx,
                        name,
                        subcategory: aliases
                            .as_ref()
                            .map_or_else(Subcategory::default, |aliases| {
                                subcategory_of(stmt, name, value, aliases)
                            }),
                        value_refs: value.map_or_else(Vec::new, eval_refs),
                    });
                }
            }
        }
    }
    let site_at: HashMap<&'src str, usize> =
        sites.iter().enumerate().map(|(s, c)| (c.name, s)).collect();
    let mut eager_reader_at: HashMap<&'src str, usize> = HashMap::new();
    for (idx, stmt) in body.iter().enumerate() {
        if matches!(stmt, Stmt::ClassDef(_) | Stmt::FunctionDef(_)) {
            for name in eval_time_refs(stmt, defer_annotations) {
                eager_reader_at.entry(name).or_insert(idx);
            }
        }
    }
    let n = sites.len();
    let mut anchored = vec![false; n];
    let mut reaches_def = vec![false; n];
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (s, site) in sites.iter().enumerate() {
        if site.effectful || analysis.module_reassigned(site.name) {
            anchored[s] = true;
            continue;
        }
        // A definition above the site reads the site's name at
        // evaluation time, resolving it against the builtin, so seating
        // the site above that definition rebinds what it read.
        if is_python_builtin(site.name, builtins_minor, notebook)
            && eager_reader_at
                .get(site.name)
                .is_some_and(|&reader| reader < site.idx)
        {
            anchored[s] = true;
            continue;
        }
        // A value reference to an unresolved name pins the constant unless
        // the name is an import or a builtin, both clean terminals, whereas
        // an annotation reference only ever constrains order, so `x: int = 1`
        // sits in the leading band.
        for (name, anchor_unresolved) in site.foreign_refs() {
            if dup_defs.contains(name) {
                anchored[s] = true;
            } else if let Some(&def) = def_at.get(name) {
                // A definition below the site rebinds a name the site
                // already resolves against a builtin or an earlier
                // module-scope write, so the site pins rather than
                // reaching the trailing band. A write inside a branch
                // counts as that earlier binding.
                let rebinds_below = def > site.idx
                    && (is_python_builtin(name, builtins_minor, notebook)
                        || analysis.is_bound_before(name, body[site.idx].start()));
                if rebinds_below {
                    anchored[s] = true;
                } else {
                    reaches_def[s] = true;
                }
            } else if let Some(&dep) = site_at.get(name) {
                deps[s].push(dep);
            } else if anchor_unresolved
                && !imports.contains(name)
                && !is_python_builtin(name, builtins_minor, notebook)
            {
                anchored[s] = true;
            }
        }
    }
    propagate(&mut anchored, &deps);
    let mut trailing: Vec<bool> = (0..n).map(|s| reaches_def[s] && !anchored[s]).collect();
    propagate(&mut trailing, &deps);
    let (trailing_members, leading_members): (Vec<usize>, Vec<usize>) =
        (0..n).filter(|&s| !anchored[s]).partition(|&s| trailing[s]);
    let mut keys: HashMap<usize, (usize, Subcategory, &'src str)> = HashMap::new();
    for (rank, members) in [
        (BandRank::Leading, leading_members),
        (BandRank::Trailing, trailing_members),
    ] {
        let local: HashMap<usize, usize> =
            members.iter().enumerate().map(|(at, &s)| (s, at)).collect();
        let dep_sets: Vec<HashSet<usize>> = members
            .iter()
            .map(|&s| {
                deps[s]
                    .iter()
                    .filter_map(|dep| local.get(dep).copied())
                    .collect()
            })
            .collect();
        for (s, tier) in members.into_iter().zip(tier_levels(&dep_sets)?) {
            keys.insert(sites[s].idx, (tier, sites[s].subcategory, sites[s].name));
            ranks.insert(sites[s].idx, rank);
        }
    }
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let site_edge = |from: usize, name: &str| {
        site_at
            .get(name)
            .filter(|&&dep| !anchored[dep])
            .map(|&dep| (from, sites[dep].idx))
    };
    for (s, site) in sites.iter().enumerate() {
        if anchored[s] {
            continue;
        }
        for (name, _) in site.foreign_refs() {
            if let Some(&def) = def_at.get(name) {
                edges.push((site.idx, def));
            } else {
                edges.extend(site_edge(site.idx, name));
            }
        }
    }
    for (idx, stmt) in body.iter().enumerate() {
        if ranks.get(&idx) == Some(&BandRank::Definition) {
            for name in eval_time_refs(stmt, defer_annotations) {
                edges.extend(site_edge(idx, name));
            }
        }
    }
    // A bound comment only travels when its member bands, leaving an
    // anchored member's comment where the source put it. A carry onto an
    // anchored member reverts to heading the member whose block folds it
    // in, so the run travels as that member's own heading rather than
    // holding a shape the reassembled text reads back as a carry.
    carries.retain(|carry| {
        let banded = ranks.contains_key(&carry.carrier);
        if !banded {
            attached.insert(carry.absorbs, carry.comment);
        }
        banded
    });
    attached.retain(|idx, _| ranks.contains_key(idx));
    Some(BandPlan {
        attached,
        carries,
        edges,
        keys,
        ranks,
    })
}

/// The carry binding `block` back onto the member above `body[idx]`,
/// which `block` sits on the line directly below while a blank line
/// holds it off `body[idx]`. `None` for every other run, leaving it
/// bound to `body[idx]`, a run touching both members reading as the
/// description of the one beneath it. The comment trails the member's
/// code when both hold to one source line and the joined line fits
/// inside `code_width`, and climbs onto the line above it otherwise.
fn backward_carry(
    source: &Source,
    body: &[Stmt],
    blocks: &[TextRange],
    idx: usize,
    block: TextRange,
    code_width: usize,
) -> Option<Carry> {
    let prev = idx.checked_sub(1)?;
    if !source.consecutive_lines(blocks[prev].end(), block.start())
        || source.consecutive_lines(block.end(), body[idx].start())
    {
        return None;
    }
    Some(Carry {
        absorbs: idx,
        carrier: prev,
        comment: block,
        trails: !source.contains_line_break(block)
            && !source.contains_line_break(&body[prev])
            && !source.column_overflows(
                blocks[prev].end(),
                TRAILING_GAP.width() + source.slice(block).trim_start().width(),
                code_width,
            ),
    })
}

/// The target name and value of a module constant candidate: an `Assign`
/// or initialized `AnnAssign` through `single_name_assignment`, or a
/// PEP 695 `type X` alias statement, whose value is always inert. `None`
/// for any other shape.
fn const_binding(stmt: &Stmt) -> Option<(&str, Option<&Expr>)> {
    match stmt {
        Stmt::TypeAlias(alias) => Some((
            alias.name.as_name_expr()?.id.as_str(),
            Some(alias.value.as_ref()),
        )),
        _ => single_name_assignment(stmt).map(|(target, value)| (target.id.as_str(), value)),
    }
}

/// Closes `state` over `deps` to a fixed point, flipping a slot true
/// once any slot it depends on is true, so an initially-seeded flag
/// reaches every slot transitively downstream of a seed.
fn propagate(state: &mut [bool], deps: &[Vec<usize>]) {
    let mut changed = true;
    while changed {
        changed = false;
        for slot in 0..state.len() {
            if !state[slot] && deps[slot].iter().any(|&dep| state[dep]) {
                state[slot] = true;
                changed = true;
            }
        }
    }
}

/// The subcategory a banded constant sorts into. A PEP 695 `type X`
/// statement or a `TypeAlias`-annotated assignment reads as an alias, a
/// `SCREAMING_CASE` name as a constant, a remaining value that names an
/// existing object as an alias, and everything else as module state.
fn subcategory_of(
    stmt: &Stmt,
    name: &str,
    value: Option<&Expr>,
    aliases: &AliasContext<'_>,
) -> Subcategory {
    if is_explicit_type_alias(stmt) {
        Subcategory::Alias
    } else if is_screaming_case(name) {
        Subcategory::Constant
    } else if value.is_some_and(|value| value_is_alias(value, aliases)) {
        Subcategory::Alias
    } else {
        Subcategory::State
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{
        primitives::orderer::member_blocks,
        testing::{notebook, parse},
    };

    /// A constant naming `get_ipython`, a builtin only inside a notebook
    /// cell, at body slot 1 below a definition.
    const IPYTHON_REF: &str = "def helper():\n    return 1\nSHELL = get_ipython\n";

    /// `src` parsed as the sole statement below a module-level definition.
    fn below_a_definition(src: &str) -> Source {
        parse(&format!("def build():\n    return 1\n\n\n{src}\n"))
    }

    fn plan_of(source: &Source) -> Option<BandPlan<'_>> {
        let body = &source.ast().body;
        let blocks = member_blocks(source, body, source.module_range());
        module_band_plan(source, body, &blocks, 88, false, true, None)
    }

    #[test]
    fn const_binding_accepts_a_type_alias_and_rejects_a_non_binding() {
        let source = parse("type Seconds = float\nx, y = 1, 2\n");
        let body = &source.ast().body;
        let (name, _) = const_binding(&body[0]).expect("a type alias binds");
        assert_eq!(name, "Seconds");
        assert!(const_binding(&body[1]).is_none());
    }

    #[test]
    fn module_band_plan_anchors_a_constant_naming_a_duplicated_definition() {
        let source = parse("def f():\n    pass\n\n\ndef f():\n    pass\n\n\nALIAS = f\n");
        let plan = plan_of(&source).expect("acyclic module plans");
        assert!(
            !plan.ranks.contains_key(&2),
            "ALIAS names an ambiguous f, so it pins in place"
        );
    }

    #[rstest]
    #[case("TABLE = dict(timeout=30)")]
    #[case("SIZES = [sum([1, 2]), 3]")]
    fn module_band_plan_anchors_an_effectful_constant(#[case] src: &str) {
        let source = below_a_definition(src);
        let plan = plan_of(&source).expect("acyclic module plans");
        assert!(
            !plan.ranks.contains_key(&1),
            "{src} runs code as it binds, so it pins in place"
        );
    }

    #[test]
    fn module_band_plan_bands_a_builtin_named_constant_as_leading() {
        let source = below_a_definition("TABLE = dict");
        let plan = plan_of(&source).expect("acyclic module plans");
        assert_eq!(
            plan.ranks[&1],
            BandRank::Leading,
            "dict is a builtin name, so TABLE sits in the leading band"
        );
    }

    #[rstest]
    #[case("def f():\n    pass\n\n# note\nX = 1\n", BandRank::Leading)]
    #[case("def f():\n    pass\n\n# note\n\nX = 1\n", BandRank::Leading)]
    #[case("X = 1\n\n# note\ndef f():\n    pass\n", BandRank::Definition)]
    #[case("X = 1\n\n# note\n\ndef f():\n    pass\n", BandRank::Definition)]
    #[case("X = 1\n\n# note\nimport os\n", BandRank::Import)]
    #[case("X = 1\n\n# note\n\nimport os\n", BandRank::Import)]
    fn module_band_plan_bands_a_member_under_a_prose_comment(
        #[case] src: &str,
        #[case] expected: BandRank,
    ) {
        let source = parse(src);
        let plan = plan_of(&source).expect("acyclic module plans");
        assert_eq!(
            plan.ranks[&1], expected,
            "a prose comment binds to the member below it, whatever the blank run"
        );
    }

    #[rstest]
    #[case("SCALE = 2 * 3")]
    #[case("LIMITS = [1, 2]")]
    #[case("LOOKUP = {\"a\": 1}")]
    #[case("KEY = lambda row: row.score")]
    fn module_band_plan_bands_an_inert_constant_as_leading(#[case] src: &str) {
        let source = below_a_definition(src);
        let plan = plan_of(&source).expect("acyclic module plans");
        assert_eq!(
            plan.ranks[&1],
            BandRank::Leading,
            "{src} only builds a result, so it sits in the leading band"
        );
    }

    #[test]
    fn module_band_plan_bands_leading_and_trailing_constants() {
        let source = parse("LEAD = 1\n\n\ndef make():\n    return 1\n\n\nTRAIL = make\n");
        let plan = plan_of(&source).expect("acyclic module plans");
        assert_eq!(
            plan.ranks[&0],
            BandRank::Leading,
            "LEAD touches only a literal"
        );
        assert_eq!(plan.ranks[&1], BandRank::Definition, "make is a definition");
        assert_eq!(plan.ranks[&2], BandRank::Trailing, "TRAIL names make");
    }

    #[rstest]
    #[case::single_line_note("ZETA = 1\n# documents ZETA\n\nALPHA = 2\n", true)]
    #[case::multi_line_note("ZETA = 1\n# documents ZETA\n# at length\n\nALPHA = 2\n", false)]
    #[case::multi_line_member("def f():\n    pass\n# documents f\n\nALPHA = 2\n", false)]
    #[case::note_past_the_budget(
        "ZETA = 1\n# a note long enough that joining it onto the constant line would outrun the code budget\n\nALPHA = 2\n",
        false
    )]
    fn module_band_plan_binds_a_comment_below_a_member_backward(
        #[case] src: &str,
        #[case] trails: bool,
    ) {
        let source = parse(src);
        let plan = plan_of(&source).expect("acyclic module plans");
        let carry = plan
            .carries
            .first()
            .expect("the member carries its comment");
        assert_eq!(carry.carrier, 0);
        assert_eq!(carry.trails, trails);
    }

    #[test]
    fn module_band_plan_declines_a_constant_cycle() {
        let source = parse("A = B\nB = A\n");
        assert!(plan_of(&source).is_none());
    }

    #[test]
    fn module_band_plan_ignores_a_constant_self_reference() {
        let source = parse("X = X\n");
        let plan = plan_of(&source).expect("self-reference does not cycle");
        assert_eq!(
            plan.ranks[&0],
            BandRank::Leading,
            "a self-reference constrains nothing, so X leads"
        );
        assert!(
            plan.edges.is_empty(),
            "a self-reference emits no edge, leaving the assembled order sound"
        );
    }

    #[rstest]
    #[case::banner_above_a_constant("def f():\n    pass\n\n# =====\n\nX = 1\n")]
    #[case::banner_above_a_definition("X = 1\n\n# =====\n\ndef f():\n    pass\n")]
    #[case::banner_below_a_constant("ZETA = 1\n# =========\n\nALPHA = 2\n")]
    #[case::directive_above_a_constant("def f():\n    pass\n\n# fmt: on\n\nX = 1\n")]
    #[case::directive_above_a_definition("X = 1\n\n# fmt: on\n\ndef f():\n    pass\n")]
    #[case::pragma_below_a_constant("ZETA = 1\n# type: ignore\n\nALPHA = 2\n")]
    fn module_band_plan_pins_a_member_below_an_anchor(#[case] src: &str) {
        let source = parse(src);
        let plan = plan_of(&source).expect("acyclic module plans");
        assert!(
            !plan.ranks.contains_key(&1),
            "a banner, a directive, and a pragma all anchor in place, so the member below pins"
        );
        assert!(plan.carries.is_empty(), "the comment binds to neither side");
    }

    #[test]
    fn module_band_plan_pins_an_inert_constant_referencing_an_effectful_one() {
        let source = parse("def helper():\n    return 1\n\n\nRAW = compute()\nSCALED = RAW\n");
        let plan = plan_of(&source).expect("acyclic module plans");
        assert!(
            !plan.ranks.contains_key(&2),
            "SCALED references effectful RAW, so anchoring propagates and it pins"
        );
    }

    #[rstest]
    #[case(notebook(&[IPYTHON_REF]), Some(BandRank::Leading))]
    #[case(parse(IPYTHON_REF), None)]
    fn module_band_plan_reads_an_ipython_builtin_only_inside_a_cell(
        #[case] source: Source,
        #[case] expected: Option<BandRank>,
    ) {
        let plan = plan_of(&source).expect("acyclic plans");
        assert_eq!(
            plan.ranks.get(&1).copied(),
            expected,
            "get_ipython is a builtin in a cell and unresolved in a module"
        );
    }

    #[test]
    fn module_band_plan_records_the_comment_attached_above_a_member() {
        let source = parse("# the tunable knobs\nZETA = 1\nALPHA = 2\n");
        let plan = plan_of(&source).expect("acyclic module plans");
        let attached = plan.attached.get(&0).expect("ZETA folds in its comment");
        assert_eq!(source.slice(*attached), "# the tunable knobs");
        assert!(!plan.attached.contains_key(&1));
    }

    #[test]
    fn propagate_flips_slots_reachable_from_a_seed() {
        let deps = vec![vec![], vec![0], vec![1]];
        let mut state = vec![true, false, false];
        propagate(&mut state, &deps);
        assert_eq!(state, vec![true, true, true]);
    }

    #[test]
    fn propagate_leaves_unreached_slots_untouched() {
        let deps = vec![vec![], vec![], vec![]];
        let mut state = vec![false, true, false];
        propagate(&mut state, &deps);
        assert_eq!(state, vec![false, true, false]);
    }

    #[rstest]
    #[case("Handler = make", Subcategory::Alias)]
    #[case("Payload: TypeAlias = dict", Subcategory::Alias)]
    #[case("type Seconds = float", Subcategory::Alias)]
    #[case("Interval = int | float", Subcategory::Alias)]
    #[case("opener = TarFile.open", Subcategory::Alias)]
    #[case("MAX_RETRIES = 5", Subcategory::Constant)]
    #[case("Config = {\"debug\": True}", Subcategory::State)]
    #[case("threshold = 5", Subcategory::State)]
    #[case("_cache = {}", Subcategory::State)]
    #[case(
        "SETTINGS = {\"db\": \"pg\"}\ndatabase = SETTINGS[\"db\"]",
        Subcategory::State
    )]
    fn subcategory_of_classifies_by_value_shape_and_structure(
        #[case] src: &str,
        #[case] expected: Subcategory,
    ) {
        let source = parse(&format!("{src}\n"));
        let stmt = source.ast().body.last().expect("a statement");
        let aliases = AliasContext::new(&source.ast().body, source.binding_analysis());
        let (name, value) = const_binding(stmt).expect("a constant binding");
        assert_eq!(subcategory_of(stmt, name, value, &aliases), expected);
    }
}
