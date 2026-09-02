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
//! Each mutation takes source text and hands back source text, reaching for
//! whichever tree serves it. The reorders and the comment insertions run on
//! the `libcst` concrete tree, where a statement's leading comment lines
//! belong to the statement and travel with it. The rename and the redundant
//! parentheses run on ruff's token stream and argument ranges, splicing the
//! text those name, since `libcst` carries no walk that reaches every node.
//! Both routes leave every byte no mutation names exactly as it was.
//!
//! A variant lands only where it parses, which holds a mutation that breaks
//! the grammar out of the corpus. The check reads ruff's parser rather than
//! CPython's own compile step, and the two part company on source CPython
//! rejects semantically while ruff parses, a walrus inside an annotation
//! among them, so a file already carrying one keeps its variants here.

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use bumpalo::Bump;
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
    Expr,
    token::TokenKind,
    visitor::source_order::{SourceOrderVisitor, walk_body, walk_expr},
};
use ruff_python_parser::parse_module as parse_ruff;
use ruff_python_stdlib::keyword::is_keyword;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

/// Every mutation this generator writes, each named by the subdirectory it
/// lands in.
const MUTATIONS: [(&str, Mutation); 7] = [
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

/// Returns `module` with a comment line leading a sample of its statements,
/// each at that statement's own indent.
fn commented(text: &str, rng: &mut StdRng) -> Option<String> {
    let mut module = parse_module(text, None).ok()?;
    let picks = sample(module.body.len(), rng);
    if picks.is_empty() {
        return None;
    }
    for slot in picks {
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

/// True where `statement` is a lone string expression, the shape a docstring
/// takes in the first seat of a body.
fn is_docstring(statement: &Statement) -> bool {
    matches!(statement, Statement::Simple(line) if line.body.len() == 1
        && matches!(&line.body[0], SmallStatement::Expr(expr)
            if matches!(expr.value, Expression::SimpleString(_)
                | Expression::ConcatenatedString(_))))
}

/// True where `statement` is a `from __future__ import ...`, which the
/// grammar admits only ahead of every other statement.
fn is_future(statement: &Statement) -> bool {
    matches!(statement, Statement::Simple(line) if line.body.iter().any(|small|
        matches!(small, SmallStatement::ImportFrom(ImportFrom { module: Some(name), .. })
            if matches!(name, NameOrAttribute::N(n) if n.value == "__future__"))))
}

/// A comment line carrying `text`.
fn led(text: &'static str) -> EmptyLine<'static> {
    EmptyLine {
        comment: Some(Comment(text)),
        ..Default::default()
    }
}

fn main() {
    let args = Args::parse();
    let started = Instant::now();
    let budget = Duration::from_secs_f64(args.budget);
    let files: Vec<PathBuf> = walk(&args.corpus);
    let (reached, written) = files
        .par_iter()
        .map(|path| {
            if started.elapsed() > budget {
                return (0, 0);
            }
            (
                1,
                mutated(path, &args.corpus, &args.destination, &args.seed),
            )
        })
        .reduce(|| (0, 0), |held, next| (held.0 + next.0, held.1 + next.1));
    if reached < files.len() {
        println!(
            "the {}s budget ran out after {reached} of {} files",
            args.budget,
            files.len()
        );
    }
    let _ = fs_err::create_dir_all(&args.destination);
    let _ = fs_err::File::create(args.destination.join(".generated"));
    println!("{written} variants written");
}

/// Returns `module` with each class body's members reordered behind its
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

/// Writes every variant of the file at `path` under `destination`, and
/// returns how many landed.
fn mutated(path: &Path, corpus: &Path, destination: &Path, seed: &str) -> usize {
    let Ok(text) = fs_err::read_to_string(path) else {
        return 0;
    };
    let Ok(relative) = path.strip_prefix(corpus) else {
        return 0;
    };
    let mut written = 0;
    for (name, mutate) in MUTATIONS {
        let mut rng = seeded(seed, relative);
        let Some(code) = mutate(&text, &mut rng) else {
            continue;
        };
        if parse_ruff(&code).is_err() {
            continue;
        }
        let target = destination.join(name).join(relative);
        if target
            .parent()
            .is_some_and(|dir| fs_err::create_dir_all(dir).is_err())
        {
            continue;
        }
        if fs_err::write(&target, &code).is_ok() {
            written += 1;
        }
    }
    written
}

/// Returns `text` with every argument of a sample of its calls wrapped in
/// redundant parentheses.
fn parenthesized(text: &str, rng: &mut StdRng) -> Option<String> {
    let parsed = parse_ruff(text).ok()?;
    let mut found = Arguments { ranges: Vec::new() };
    walk_body(&mut found, &parsed.syntax().body);
    let picks = sample(found.ranges.len(), rng);
    if picks.is_empty() {
        return None;
    }
    let mut wrap: Vec<TextRange> = picks.into_iter().map(|slot| found.ranges[slot]).collect();
    wrap.sort_by_key(Ranged::start);
    let mut out = String::with_capacity(text.len() + wrap.len() * 2);
    let mut cursor = 0;
    for range in wrap {
        let (start, end) = (range.start().to_usize(), range.end().to_usize());
        if start < cursor {
            continue;
        }
        out.push_str(&text[cursor..start]);
        out.push('(');
        out.push_str(&text[start..end]);
        out.push(')');
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    Some(out)
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

/// Reorders `body`, holding every statement `pin` names ahead of the rest
/// in the order they were written, and reports whether anything moved.
fn reordered(
    body: &mut Vec<Statement>,
    rng: &mut StdRng,
    pin: impl Fn(usize, &Statement) -> bool,
) -> bool {
    let (mut pinned, mut movable): (Vec<_>, Vec<_>) = body
        .drain(..)
        .enumerate()
        .partition(|(slot, statement)| pin(*slot, statement));
    let moved = movable.len() >= 2;
    if moved {
        movable.shuffle(rng);
    }
    pinned.append(&mut movable);
    *body = pinned.into_iter().map(|(_, statement)| statement).collect();
    moved
}

/// A wider or narrower spelling of `name`, or `None` where the result is not
/// an identifier the grammar reads as one.
fn respelled<'b>(arena: &'b Bump, name: &str, rng: &mut StdRng) -> Option<&'b str> {
    let candidate = if rng.random::<f64>() < 0.5 {
        format!("{name}{}", "_w".repeat(rng.random_range(1..=3)))
    } else {
        let cut = name
            .char_indices()
            .nth(name.chars().count().div_ceil(2))
            .map_or(name.len(), |(at, _)| at);
        name[..cut].to_owned()
    };
    let usable = candidate != name
        && !candidate.is_empty()
        && !candidate.starts_with(|c: char| c.is_ascii_digit())
        && candidate.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !is_keyword(&candidate);
    usable.then(|| &*arena.alloc_str(&candidate))
}

/// The slots a sampling mutation picks out of `count` candidates.
fn sample(count: usize, rng: &mut StdRng) -> Vec<usize> {
    let mut picked = index::sample(rng, count, SAMPLE.min(count)).into_vec();
    picked.sort_unstable();
    picked
}

/// A stream seeded from the run's seed and the file's own path, so a variant
/// is the same whatever order the walk reaches the files in.
fn seeded(seed: &str, relative: &Path) -> StdRng {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    relative.hash(&mut hasher);
    StdRng::seed_from_u64(hasher.finish())
}

/// Returns `module` with its top-level statements reordered, each keeping the
/// lines it owns. A `__future__` import and a leading docstring hold their
/// seats ahead of the shuffle.
fn shuffled(text: &str, rng: &mut StdRng) -> Option<String> {
    let mut module = parse_module(text, None).ok()?;
    reordered(&mut module.body, rng, |slot, held| {
        is_future(held) || (slot == 0 && is_docstring(held))
    })
    .then(|| render(&module))
}

/// Returns `module` with a `# prose: off` region wrapped around one top-level
/// statement and a `# prose: skip` closing one simple line.
fn suppressed(text: &str, rng: &mut StdRng) -> Option<String> {
    let mut module = parse_module(text, None).ok()?;
    if module.body.is_empty() {
        return None;
    }
    let simple: Vec<usize> = module
        .body
        .iter()
        .positions(|statement| matches!(statement, Statement::Simple(_)))
        .collect();
    let skipped = *simple.choose(rng)?;
    if let Statement::Simple(SimpleStatementLine {
        trailing_whitespace,
        ..
    }) = &mut module.body[skipped]
    {
        *trailing_whitespace = TrailingWhitespace {
            comment: Some(Comment("# prose: skip")),
            whitespace: SimpleWhitespace("  "),
            ..Default::default()
        };
    }
    let region = rng.random_range(0..module.body.len());
    module.body[region]
        .leading_lines()
        .push(led("# prose: off"));
    if region + 1 < module.body.len() {
        module.body[region + 1]
            .leading_lines()
            .insert(0, led("# prose: on"));
    } else {
        module.footer.insert(0, led("# prose: on"));
    }
    Some(render(&module))
}

/// Every `.py` file under `root`, in a stable order. The walk carries no
/// standard filter, so a hidden directory and an ignored one both enter
/// the corpus.
fn walk(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .standard_filters(false)
        .build()
        .flatten()
        .map(ignore::DirEntry::into_path)
        .filter(|path| path.extension().is_some_and(|ext| ext == "py"))
        .sorted()
        .collect()
}

/// Returns `text` with a sample of its identifiers lengthened or shortened,
/// shifting every column their width feeds.
fn widened(text: &str, rng: &mut StdRng) -> Option<String> {
    let parsed = parse_ruff(text).ok()?;
    let arena = Bump::new();
    let names: Vec<(TextRange, &str)> = parsed
        .tokens()
        .iter()
        .filter(|token| token.kind() == TokenKind::Name)
        .map(|token| (token.range(), &text[token.range()]))
        .collect();
    let distinct: Vec<&str> = names.iter().map(|(_, name)| *name).unique().collect();
    if distinct.is_empty() {
        return None;
    }
    let picks = sample(distinct.len(), rng);
    let mut renames: FxHashMap<&str, &str> = FxHashMap::default();
    for slot in picks {
        let name = distinct[slot];
        if let Some(candidate) = respelled(&arena, name, rng) {
            renames.insert(name, candidate);
        }
    }
    if renames.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (range, name) in names {
        let Some(candidate) = renames.get(name) else {
            continue;
        };
        out.push_str(&text[cursor..range.start().to_usize()]);
        out.push_str(candidate);
        cursor = range.end().to_usize();
    }
    out.push_str(&text[cursor..]);
    Some(out)
}
