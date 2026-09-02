//! Renders a run's unsettled rewrites as bug notices.
//!
//! Files reproducing under one rule subset fold into a single block, so
//! a tree-wide run reports each distinct defect once. A block covering
//! one file carries the diff between the two passes beneath it, and a
//! block covering several names the count and points its invocation at
//! the first of them.

use std::{collections::BTreeMap, io::Write};

use itertools::Itertools;
use ruff_source_file::SourceFile;

use super::{
    FileOutcome, STDIN_NAME,
    diff::{Heading, write_diff},
};
use crate::{
    cli::output::{Presentation, Summary, pluralize, report},
    unstable::{UnstableRewrite, blame, report_url},
};

const FILING: &str = "Filing is one click, the form already carrying the version, the reproducing slugs, and the resolved configuration, with anything too long for a link left to paste:";

const WRAP: usize = 88;

/// One block per reproducing subset, each naming the defect as Prose's
/// own, the invocation that replays it, and the pre-filled report URL.
pub(super) fn render_reports<E: Write>(
    stderr: &mut E,
    present: &Presentation,
    outcomes: &[FileOutcome],
) {
    for files in grouped(outcomes).into_values() {
        let (file, rewrite) = files[0];
        let name = file.name();
        let subject = match files.len() {
            1 => name.to_owned(),
            several => pluralize(several, "file"),
        };
        let _ = report(stderr, present, &Summary::UnstableRewrite { subject });
        let _ = writeln!(stderr);
        paragraph(
            stderr,
            &reproduce(files.len()),
            &rewrite.invocation((name != STDIN_NAME).then_some(name)),
        );
        paragraph(stderr, FILING, &report_url(rewrite, file.source_text()));
        if files.len() == 1 {
            let _ = write_diff(
                stderr,
                name,
                &rewrite.first,
                &rewrite.second,
                Heading::Passes,
            );
            let _ = writeln!(stderr);
        }
    }
}

/// Each group's files sort by name, so the block a tree-wide run heads
/// with names the same file whichever directory the walk reached first.
fn grouped(outcomes: &[FileOutcome]) -> BTreeMap<String, Vec<(&SourceFile, &UnstableRewrite)>> {
    outcomes
        .iter()
        .filter_map(FileOutcome::unstable)
        .sorted_by_key(|(file, _)| file.name())
        .into_group_map_by(|(_, rewrite)| rewrite.slugs())
        .into_iter()
        .collect()
}

/// A paragraph wrapped to `WRAP`, then its own indented line.
fn paragraph<E: Write>(stderr: &mut E, prose: &str, indented: &str) {
    let _ = writeln!(stderr, "{}\n", textwrap::fill(prose, WRAP));
    let _ = writeln!(stderr, "    {indented}\n");
}

fn reproduce(files: usize) -> String {
    format!(
        "{}, so reproduce it now and confirm it gone after an upgrade with:",
        blame(files),
    )
}

#[cfg(test)]
mod tests {
    use ruff_source_file::SourceFileBuilder;

    use super::*;
    use crate::cache::Rewrite;

    const FIRST: &str = "yy = 1\n";

    fn rendered(present: &Presentation, outcomes: &[FileOutcome]) -> String {
        crate::testing::rendered(|buf| render_reports(buf, present, outcomes))
    }

    fn unsettled(name: &str, slug: &'static str) -> FileOutcome {
        FileOutcome::Done {
            cached: false,
            diagnostics: Vec::new(),
            file: SourceFileBuilder::new(name, FIRST).finish(),
            notebook_index: None,
            rewrite: Rewrite::text(FIRST.to_owned()),
            unstable: Some(Box::new(UnstableRewrite::sample(slug))),
        }
    }

    #[test]
    fn render_reports_folds_a_shared_subset_onto_the_first_file() {
        let outcomes = [
            unsettled("src/b.py", "align-equals"),
            unsettled("src/a.py", "align-equals"),
        ];

        let out = rendered(&Presentation::windowed(), &outcomes);

        assert!(out.contains("prose rewrote 2 files"), "{out}");
        assert!(out.contains("rather than in the files"), "{out}");
        assert!(
            out.contains("prose format --select align-equals src/a.py"),
            "{out}",
        );
        assert!(!out.contains("@@"), "a folded block carries no diff: {out}");
    }

    #[test]
    fn render_reports_holds_the_notice_under_quiet() {
        let out = rendered(
            &Presentation::quieted(),
            &[unsettled("src/a.py", "align-equals")],
        );

        assert!(out.contains("prose rewrote src/a.py"), "{out}");
        assert!(out.contains("issues/new"), "{out}");
        assert!(!out.contains('🐞'), "quiet kept the anchor: {out}");
        assert!(!out.contains('\u{1b}'), "quiet kept the paint: {out}");
    }

    #[test]
    fn render_reports_names_the_stdin_positional_in_its_invocation() {
        let out = rendered(
            &Presentation::quieted(),
            &[unsettled(STDIN_NAME, "align-equals")],
        );

        assert!(out.contains("prose rewrote <stdin>"), "{out}");
        assert!(
            out.contains("prose format --select align-equals -"),
            "{out}"
        );
    }

    #[test]
    fn render_reports_opens_a_block_per_distinct_subset() {
        let outcomes = [
            unsettled("src/a.py", "align-equals"),
            unsettled("src/b.py", "align-colons"),
        ];

        let out = rendered(&Presentation::windowed(), &outcomes);

        assert_eq!(out.matches("prose rewrote").count(), 2, "{out}");
    }

    #[test]
    fn render_reports_sets_the_invocation_and_the_form_apart() {
        let out = rendered(
            &Presentation::quieted(),
            &[unsettled("src/a.py", "align-equals")],
        );
        let indented: Vec<&str> = out
            .lines()
            .filter_map(|line| line.strip_prefix("    "))
            .collect();

        assert_eq!(indented.len(), 2, "{out}");
        assert_eq!(indented[0], "prose format --select align-equals src/a.py");
        assert!(indented[1].contains("issues/new"), "{out}");
    }

    #[test]
    fn render_reports_wraps_each_paragraph_inside_the_column() {
        let out = rendered(
            &Presentation::windowed(),
            &[unsettled("src/a.py", "align-equals")],
        );

        let widest = out
            .lines()
            .filter(|line| !line.starts_with("    "))
            .map(str::len)
            .max()
            .unwrap_or_default();
        assert!(widest <= WRAP, "{out}");
    }

    #[test]
    fn render_reports_writes_nothing_for_a_settled_run() {
        assert!(rendered(&Presentation::windowed(), &[]).is_empty());
    }

    #[test]
    fn render_reports_writes_the_second_pass_diff_for_one_file() {
        let out = rendered(
            &Presentation::windowed(),
            &[unsettled("src/a.py", "align-equals")],
        );

        assert!(out.contains("--- src/a.py (first pass)"), "{out}");
        assert!(out.contains("+++ src/a.py (second pass)"), "{out}");
        assert!(out.contains("-yy = 1"), "{out}");
        assert!(out.contains("+yyy = 1"), "{out}");
    }
}
