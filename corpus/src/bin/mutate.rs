//! Writes parseable mutations of a corpus, one subdirectory per mutation.
//!
//!     commented      A comment line above a sample of statements.
//!     crlf           Every line ending rewritten to CRLF.
//!     members        Each class body's members reordered behind its docstring.
//!     parenthesized  Every argument of a sample of calls wrapped in parentheses.
//!     shuffled       Top-level statements reordered, each keeping its lines.
//!     suppressed     A `# prose: off` region and a logical-line `# prose: skip`.
//!     widened        Identifiers lengthened or shortened.
//!
//! Each mutation takes source text and returns source text, running on
//! whichever tree models it. The reorders and the comment insertions run on
//! the `libcst` concrete tree, where a statement's leading comment lines
//! belong to the statement and travel with it. The rename and the redundant
//! parentheses run on ruff's token stream and argument ranges, splicing the
//! text those name, since `libcst` carries no walk that reaches every node.
//! Both routes leave every byte no mutation names exactly as it was.
//!
//! A variant is written only where it parses, which keeps a mutation that
//! breaks the grammar out of the corpus. The check is ruff's parser, which
//! accepts some source CPython rejects semantically, a walrus inside an
//! annotation among them. A file already carrying one keeps its variants.

use std::{
    error::Error,
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher},
    io,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use clap::Parser;
use ignore::WalkBuilder;
use itertools::Itertools;
use libcst_native::{
    ClassDef, Codegen, CodegenState, Comment, CompoundStatement, EmptyLine, Expression, ImportFrom,
    Module, NameOrAttribute, SimpleStatementLine, SimpleWhitespace, SmallStatement, Statement,
    Suite, TrailingWhitespace, WithLeadingLines, parse_module,
};
use rand::{
    RngExt, SeedableRng,
    rngs::StdRng,
    seq::{IndexedRandom, SliceRandom, index},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use ruff_python_ast::{
    Expr, PySourceType,
    token::TokenKind,
    visitor::source_order::{SourceOrderVisitor, walk_body, walk_expr},
};
use ruff_python_parser::parse_module as parse_ruff;
use ruff_python_stdlib::identifiers::is_identifier;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

/// Every mutation this generator writes, each named by the subdirectory it
/// lands in.
const MUTATIONS: &[(&str, Mutation)] = &[
    ("commented", commented),
    ("crlf", crlf),
    ("members", members),
    ("parenthesized", parenthesized),
    ("shuffled", shuffled),
    ("suppressed", suppressed),
    ("widened", widened),
];

/// How many nodes a sampling mutation touches.
const SAMPLE: usize = 8;

/// Writes parseable mutations of a corpus, one subdirectory per mutation.
#[derive(Parser)]
struct Args {
    /// The corpus to read.
    corpus: PathBuf,
    /// Where the mutation subdirectories land.
    destination: PathBuf,
    /// How long the walk may run before it stops.
    #[arg(default_value_t = 60.0)]
    budget: f64,
    /// The seed each file's sampling derives from.
    #[arg(default_value = "0")]
    seed: String,
}

/// Collects every call argument's range, which the redundant-parenthesis
/// mutation wraps.
#[derive(Default)]
struct Arguments {
    ranges: Vec<TextRange>,
}

impl<'a> SourceOrderVisitor<'a> for Arguments {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Call(call) = expr {
            self.ranges
                .extend(call.arguments.args.iter().map(Ranged::range));
        }
        walk_expr(self, expr);
    }
}

/// One mutation's rewrite of a module's text, `None` where it does not apply.
type Mutation = fn(&str, &mut StdRng) -> Option<String>;

/// Returns `text` with a comment line leading a sample of its statements,
/// each at that statement's own indent.
fn commented(text: &str, rng: &mut StdRng) -> Option<String> {
    let mut module = parse_module(text, None).ok()?;
    for slot in sample(module.body.len(), rng)? {
        module.body[slot].leading_lines().push(led("# probe"));
    }
    Some(render(&module))
}

/// Returns `text` with every line ending rewritten to CRLF.
fn crlf(text: &str, _rng: &mut StdRng) -> Option<String> {
    if !text.contains('\n') {
        return None;
    }
    let mut module = parse_module(text, None).ok()?;
    module.default_newline = "\r\n";
    Some(render(&module))
}

/// True where `statement` is a lone string expression, the form a docstring
/// takes as the first statement of a body.
fn is_docstring(statement: &Statement) -> bool {
    matches!(statement, Statement::Simple(line)
        if matches!(line.body.as_slice(), [SmallStatement::Expr(expr)]
            if matches!(expr.value, Expression::SimpleString(_)
                | Expression::ConcatenatedString(_))))
}

/// True where `statement` is a `from __future__ import ...`, which the
/// grammar admits only ahead of every other statement.
fn is_future(statement: &Statement) -> bool {
    matches!(statement, Statement::Simple(line) if line.body.iter().any(|small|
        matches!(small, SmallStatement::ImportFrom(ImportFrom {
            module: Some(NameOrAttribute::N(n)), ..
        }) if n.value == "__future__")))
}

/// Builds a comment line carrying `text`.
fn led(text: &'static str) -> EmptyLine<'static> {
    EmptyLine {
        comment: Some(Comment(text)),
        ..Default::default()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let started = Instant::now();
    let budget = Duration::from_secs_f64(args.budget);
    let files = walk(&args.corpus);
    let (reached, unread, written) = files
        .par_iter()
        .map(|path| -> io::Result<(usize, usize, usize)> {
            if started.elapsed() > budget {
                return Ok((0, 0, 0));
            }
            let written = mutated(path, &args.corpus, &args.destination, &args.seed)?;
            Ok(written.map_or((1, 1, 0), |count| (1, 0, count)))
        })
        .try_reduce(
            || (0, 0, 0),
            |held, next| Ok((held.0 + next.0, held.1 + next.1, held.2 + next.2)),
        )?;
    if unread > 0 {
        eprintln!(
            "the generator could not read {unread} of the {} files under the corpus root",
            files.len()
        );
    }
    if reached < files.len() {
        println!(
            "the {}s budget ran out after {reached} of {} files",
            args.budget,
            files.len()
        );
    }
    fs_err::create_dir_all(&args.destination)?;
    fs_err::File::create(args.destination.join(".generated"))?;
    println!("{written} variants written");
    Ok(())
}

/// Returns `text` with each class body's members reordered behind its
/// docstring.
fn members(text: &str, rng: &mut StdRng) -> Option<String> {
    let mut module = parse_module(text, None).ok()?;
    let mut moved = false;
    for statement in &mut module.body {
        let Statement::Compound(CompoundStatement::ClassDef(ClassDef {
            body: Suite::IndentedBlock(block),
            ..
        })) = statement
        else {
            continue;
        };
        moved |= reordered(&mut block.body, rng, |slot, held| {
            slot == 0 && is_docstring(held)
        });
    }
    moved.then(|| render(&module))
}

/// Writes every variant of the file at `path` under `destination` and returns
/// how many landed, or `None` where the file cannot be read. A variant it
/// cannot write fails the whole call.
fn mutated(
    path: &Path,
    corpus: &Path,
    destination: &Path,
    seed: &str,
) -> io::Result<Option<usize>> {
    let Ok(text) = fs_err::read_to_string(path) else {
        return Ok(None);
    };
    let relative = path
        .strip_prefix(corpus)
        .expect("invariant: the walk yields paths under the corpus");
    let mut written = 0;
    for &(name, mutate) in MUTATIONS {
        let mut rng = seeded(seed, relative);
        let Some(code) = mutate(&text, &mut rng).filter(|code| parse_ruff(code).is_ok()) else {
            continue;
        };
        let target = destination.join(name).join(relative);
        let dir = target
            .parent()
            .expect("invariant: a target sits inside its mutation directory");
        fs_err::create_dir_all(dir)?;
        fs_err::write(&target, &code)?;
        written += 1;
    }
    Ok(Some(written))
}

/// Returns `text` with every argument of a sample of its calls wrapped in
/// redundant parentheses.
fn parenthesized(text: &str, rng: &mut StdRng) -> Option<String> {
    let parsed = parse_ruff(text).ok()?;
    let mut found = Arguments::default();
    walk_body(&mut found, &parsed.syntax().body);
    let wrap = sample(found.ranges.len(), rng)?
        .into_iter()
        .map(|slot| found.ranges[slot])
        .sorted_by_key(Ranged::start)
        .map(|range| (range, format!("({})", &text[range])));
    Some(spliced(text, wrap))
}

/// Renders `module` back to source.
fn render(module: &Module) -> String {
    let mut state = CodegenState {
        default_newline: module.default_newline,
        default_indent: module.default_indent,
        ..Default::default()
    };
    module.codegen(&mut state);
    state.to_string()
}

/// Reorders `body`, keeping every statement `pin` names ahead of the rest
/// in the order they were written. Returns whether the shuffle had two or
/// more statements to move.
fn reordered(
    body: &mut Vec<Statement>,
    rng: &mut StdRng,
    pin: impl Fn(usize, &Statement) -> bool,
) -> bool {
    let (pinned, mut movable): (Vec<_>, Vec<_>) = body
        .drain(..)
        .enumerate()
        .partition(|(slot, statement)| pin(*slot, statement));
    let moved = movable.len() >= 2;
    movable.shuffle(rng);
    body.extend(
        pinned
            .into_iter()
            .chain(movable)
            .map(|(_, statement)| statement),
    );
    moved
}

/// Returns a wider or narrower spelling of `name`, or `None` where the result
/// is not an identifier the grammar reads as one.
fn respelled(name: &str, rng: &mut StdRng) -> Option<String> {
    let candidate = if rng.random::<f64>() < 0.5 {
        format!("{name}{}", "_w".repeat(rng.random_range(1..=3)))
    } else {
        let half = name.chars().count().div_ceil(2);
        name.chars().take(half).collect()
    };
    (candidate != name && is_identifier(&candidate)).then_some(candidate)
}

/// Returns the slots a sampling mutation picks out of `count` candidates, or
/// `None` where there are none.
fn sample(count: usize, rng: &mut StdRng) -> Option<Vec<usize>> {
    (count > 0).then(|| {
        index::sample(rng, count, SAMPLE.min(count))
            .into_iter()
            .sorted_unstable()
            .collect()
    })
}

/// Seeds a stream from the run's seed and the file's own path, so a variant is
/// the same whatever order the walk reaches the files in.
fn seeded(seed: &str, relative: &Path) -> StdRng {
    StdRng::seed_from_u64(BuildHasherDefault::<DefaultHasher>::default().hash_one((seed, relative)))
}

/// Returns `text` with its top-level statements reordered, each keeping the
/// lines it owns. A `__future__` import and a leading docstring keep their
/// places ahead of the shuffle.
fn shuffled(text: &str, rng: &mut StdRng) -> Option<String> {
    let mut module = parse_module(text, None).ok()?;
    reordered(&mut module.body, rng, |slot, held| {
        is_future(held) || (slot == 0 && is_docstring(held))
    })
    .then(|| render(&module))
}

/// Returns `text` with each range in `replacements` replaced by the string
/// paired with it, skipping any range that starts inside the range before it.
fn spliced(
    text: &str,
    replacements: impl IntoIterator<Item = (TextRange, impl AsRef<str>)>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (range, replacement) in replacements {
        let (start, end) = (range.start().to_usize(), range.end().to_usize());
        if start < cursor {
            continue;
        }
        out.push_str(&text[cursor..start]);
        out.push_str(replacement.as_ref());
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

/// Returns `text` with a `# prose: off` region wrapped around one top-level
/// statement and a `# prose: skip` closing one simple line.
fn suppressed(text: &str, rng: &mut StdRng) -> Option<String> {
    let mut module = parse_module(text, None).ok()?;
    let simple = module
        .body
        .iter()
        .positions(|statement| matches!(statement, Statement::Simple(_)))
        .collect_vec();
    let skipped = *simple.choose(rng)?;
    let Statement::Simple(SimpleStatementLine {
        trailing_whitespace,
        ..
    }) = &mut module.body[skipped]
    else {
        unreachable!("invariant: `skipped` indexes a simple statement");
    };
    *trailing_whitespace = TrailingWhitespace {
        comment: Some(Comment("# prose: skip")),
        whitespace: SimpleWhitespace("  "),
        ..Default::default()
    };
    let region = rng.random_range(0..module.body.len());
    module.body[region]
        .leading_lines()
        .push(led("# prose: off"));
    let lines = match module.body.get_mut(region + 1) {
        Some(next) => next.leading_lines(),
        None => &mut module.footer,
    };
    lines.insert(0, led("# prose: on"));
    Some(render(&module))
}

/// Returns every Python source under `root` in a stable order, covering the
/// `.py`, `.pyw`, and `.pyi` files the walker formats. The walk carries no
/// standard filter, so a hidden directory and an ignored one both enter the
/// corpus.
fn walk(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(false)
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| {
            PySourceType::try_from_path(path).is_some_and(PySourceType::is_py_file_or_stub)
        })
        .sorted()
        .collect()
}

/// Returns `text` with a sample of its identifiers lengthened or shortened,
/// shifting every column their width feeds.
fn widened(text: &str, rng: &mut StdRng) -> Option<String> {
    let parsed = parse_ruff(text).ok()?;
    let names: Vec<_> = parsed
        .tokens()
        .iter()
        .filter(|token| token.kind() == TokenKind::Name)
        .map(|token| (token.range(), &text[token.range()]))
        .collect();
    let distinct: Vec<_> = names.iter().map(|(_, name)| *name).unique().collect();
    let renames: FxHashMap<_, _> = sample(distinct.len(), rng)?
        .into_iter()
        .filter_map(|slot| {
            let name = distinct[slot];
            respelled(name, rng).map(|candidate| (name, candidate))
        })
        .collect();
    let replacements = names
        .into_iter()
        .filter_map(|(range, name)| renames.get(name).map(|candidate| (range, candidate)));
    (!renames.is_empty()).then(|| spliced(text, replacements))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spliced_replaces_each_range() {
        let swaps = [
            (TextRange::new(6.into(), 7.into()), "(x)"),
            (TextRange::new(9.into(), 10.into()), "(y)"),
        ];
        assert_eq!(spliced("a = f(x, y)", swaps), "a = f((x), (y))");
    }

    #[test]
    fn spliced_skips_a_range_starting_inside_the_one_before() {
        let swaps = [
            (TextRange::new(2.into(), 6.into()), "(g(x))"),
            (TextRange::new(4.into(), 5.into()), "(x)"),
        ];
        assert_eq!(spliced("f(g(x))", swaps), "f((g(x)))");
    }
}
