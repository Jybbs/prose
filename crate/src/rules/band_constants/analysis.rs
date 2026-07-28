//! The banding analysis. Ranks each module-scope statement into an
//! import, leading-constant, definition, or trailing-constant band and
//! tiers the constant bands through the shared `primitives::tiering`
//! graph, declining when a band's reference graph carries a cycle.

use std::collections::{HashMap, HashSet};

use ruff_python_ast::{Expr, PythonVersion, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::TextRange;

use super::{
    BandConstants,
    plan::{BandPlan, BandRank, Subcategory},
};
use crate::{
    primitives::{
        alias::{AliasContext, value_is_alias},
        binding::{
            bare_import_bound_name, from_import_bound_name, is_explicit_type_alias,
            is_screaming_case, single_name_assignment,
        },
        comments::{anchors_in_place, has_keep_marker, leading_comment_block},
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

/// Builds the module-scope hoist plan, ranking each statement. Returns
/// `None` when a constant band's reference graph carries a cycle.
pub(super) fn module_band_plan<'src>(
    source: &'src Source,
    body: &'src [Stmt],
    blocks: &[TextRange],
    defer_annotations: bool,
    group_constants: bool,
    target_version: Option<PythonVersion>,
) -> Option<BandPlan<'src>> {
    let analysis = source.binding_analysis();
    let aliases = group_constants.then(|| AliasContext::new(body, analysis));
    let builtins_minor = target_version.unwrap_or_default().minor;
    let suppression = source.suppression_map();
    let mut def_at: HashMap<&'src str, usize> = HashMap::new();
    let mut dup_defs: HashSet<&'src str> = HashSet::new();
    let mut imports: HashSet<&'src str> = HashSet::new();
    let mut ranks: HashMap<usize, BandRank> = HashMap::new();
    let mut sites: Vec<ConstSite<'src>> = Vec::new();
    for (idx, stmt) in body.iter().enumerate() {
        // A `# prose: off` span or a skip directive pins its statement, so
        // a reorder never moves a member the pipeline would then drop the
        // whole group for.
        if suppression.suppresses(stmt, BandConstants::SLUG) {
            continue;
        }
        // The own-line comment in the gap above the statement, if any.
        // `block_range` folds a statement's trailing and attached comments
        // into its own block, so a comment surviving in the gap is a
        // free-floating own-line comment a blank line separates from below.
        let gap_comment = idx.checked_sub(1).and_then(|prev| {
            leading_comment_block(source, blocks[prev].end(), blocks[idx].start())
        });
        let const_target = const_binding(stmt);
        // A definition, class, import, or any non-constant pins beneath an
        // own-line comment, bounding the bands to its side. A constant
        // instead forward-attaches a prose comment the way `blank-lines`
        // settles it, while a banner section divider or a suppression
        // directive pins the constant too, since neither may relocate.
        if gap_comment
            .is_some_and(|block| const_target.is_none() || anchors_in_place(source, block))
        {
            continue;
        }
        match stmt {
            Stmt::ClassDef(StmtClassDef { name, .. })
            | Stmt::FunctionDef(StmtFunctionDef { name, .. }) => {
                if def_at.insert(name.as_str(), idx).is_some() {
                    dup_defs.insert(name.as_str());
                }
                ranks.insert(idx, BandRank::Definition);
            }
            Stmt::Import(node) => {
                imports.extend(node.names.iter().map(bare_import_bound_name));
                ranks.insert(idx, BandRank::Import);
            }
            Stmt::ImportFrom(node) => {
                imports.extend(node.names.iter().map(from_import_bound_name));
                ranks.insert(idx, BandRank::Import);
            }
            _ => {
                if let Some((name, value)) = const_target {
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
    let n = sites.len();
    let mut anchored = vec![false; n];
    let mut reaches_def = vec![false; n];
    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (s, site) in sites.iter().enumerate() {
        if site.effectful || analysis.module_reassigned(site.name) {
            anchored[s] = true;
            continue;
        }
        // A value reference to an unresolved name pins the constant unless
        // the name is an import or a builtin, both clean terminals, whereas
        // an annotation reference only ever constrains order, so `x: int = 1`
        // rides the leading band.
        for (name, anchor_unresolved) in site.foreign_refs() {
            if dup_defs.contains(name) {
                anchored[s] = true;
            } else if def_at.contains_key(name) {
                reaches_def[s] = true;
            } else if let Some(&dep) = site_at.get(name) {
                deps[s].push(dep);
            } else if anchor_unresolved
                && !imports.contains(name)
                && !is_python_builtin(name, builtins_minor, false)
            {
                anchored[s] = true;
            }
        }
    }
    propagate(&mut anchored, &deps);
    let mut trailing: Vec<bool> = (0..n).map(|s| reaches_def[s] && !anchored[s]).collect();
    propagate(&mut trailing, &deps);
    let mut keys: HashMap<usize, (usize, Subcategory, &'src str)> = HashMap::new();
    for (band, rank) in [(false, BandRank::Leading), (true, BandRank::Trailing)] {
        let members: Vec<usize> = (0..n)
            .filter(|&s| !anchored[s] && trailing[s] == band)
            .collect();
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
        for (s, tier) in members.iter().copied().zip(tier_levels(&dep_sets)?) {
            keys.insert(sites[s].idx, (tier, sites[s].subcategory, sites[s].name));
            ranks.insert(sites[s].idx, rank);
        }
    }
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let push_site_edge = |edges: &mut Vec<(usize, usize)>, from: usize, name: &str| {
        if let Some(&dep) = site_at.get(name).filter(|&&dep| !anchored[dep]) {
            edges.push((from, sites[dep].idx));
        }
    };
    for (s, site) in sites.iter().enumerate() {
        if anchored[s] {
            continue;
        }
        for (name, _) in site.foreign_refs() {
            if let Some(&def) = def_at.get(name) {
                edges.push((site.idx, def));
            } else {
                push_site_edge(&mut edges, site.idx, name);
            }
        }
    }
    for (idx, stmt) in body.iter().enumerate() {
        if ranks.get(&idx) == Some(&BandRank::Definition) {
            for name in eval_time_refs(stmt, defer_annotations) {
                push_site_edge(&mut edges, idx, name);
            }
        }
    }
    Some(BandPlan { edges, keys, ranks })
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
    use crate::primitives::orderer::member_blocks;
    use crate::testing::parse;

    /// `src` parsed as the sole statement below a module-level definition.
    fn below_a_definition(src: &str) -> Source {
        parse(&format!("def build():\n    return 1\n\n\n{src}\n"))
    }

    fn plan_of(source: &Source) -> Option<BandPlan<'_>> {
        let body = &source.ast().body;
        let blocks = member_blocks(source, body, source.module_range());
        module_band_plan(source, body, &blocks, false, true, None)
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
            "dict is a builtin name, so TABLE rides the leading band"
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
            "{src} only builds a result, so it rides the leading band"
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
    #[case("def f():\n    pass\n\n# note\nX = 1\n")]
    #[case("def f():\n    pass\n\n# note\n\nX = 1\n")]
    fn module_band_plan_bands_a_constant_under_a_prose_comment(#[case] src: &str) {
        let source = parse(src);
        let plan = plan_of(&source).expect("acyclic module plans");
        assert_eq!(
            plan.ranks[&1],
            BandRank::Leading,
            "the comment binds to X either side of the blank, so X leads"
        );
    }

    #[rstest]
    #[case("X = 1\n\n# note\ndef f():\n    pass\n")]
    #[case("X = 1\n\n# note\n\ndef f():\n    pass\n")]
    fn module_band_plan_ranks_a_definition_under_a_prose_comment(#[case] src: &str) {
        let source = parse(src);
        let plan = plan_of(&source).expect("acyclic module plans");
        assert_eq!(
            plan.ranks[&1],
            BandRank::Definition,
            "a prose comment binds to f rather than pinning it, whatever the blank run",
        );
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
    }

    #[test]
    fn module_band_plan_pins_a_constant_below_a_banner() {
        let source = parse("def f():\n    pass\n\n# =====\n\nX = 1\n");
        let plan = plan_of(&source).expect("acyclic module plans");
        assert!(
            !plan.ranks.contains_key(&1),
            "a banner divides sections, so X pins below it"
        );
    }

    #[test]
    fn module_band_plan_pins_a_constant_below_a_directive() {
        let source = parse("def f():\n    pass\n\n# fmt: on\n\nX = 1\n");
        let plan = plan_of(&source).expect("acyclic module plans");
        assert!(
            !plan.ranks.contains_key(&1),
            "a format directive drives its own line, so X pins below it"
        );
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
