//! Text emitter: rustc-style snippet rendering with carets and fix
//! suggestions.

use std::io::{self, Write};

use annotate_snippets::{AnnotationKind, Level, Patch, Renderer, Snippet};
use ruff_text_size::Ranged;

use crate::diagnostics::{Emitter, EmitterSummary, Run, diagnostics};

pub(crate) struct Text {
    renderer: Renderer,
}

impl Text {
    pub(crate) fn new() -> Self {
        Self {
            renderer: Renderer::styled(),
        }
    }
}

impl Emitter for Text {
    fn emit(
        &self,
        writer: &mut dyn Write,
        runs: &[Run<'_>],
        _summary: &EmitterSummary,
    ) -> io::Result<()> {
        for (file, diag) in diagnostics(runs) {
            let warning = Level::WARNING.primary_title(diag.message.as_str()).element(
                Snippet::source(file.source_text())
                    .line_start(1)
                    .path(file.name())
                    .annotation(
                        AnnotationKind::Primary
                            .span(diag.range.to_std_range())
                            .label(diag.rule.as_str()),
                    ),
            );
            let mut groups = vec![warning];
            if let Some(fix) = &diag.fix {
                let snippet = Snippet::source(file.source_text())
                    .line_start(1)
                    .path(file.name())
                    .patches(fix.edits().iter().map(|edit| {
                        Patch::new(
                            edit.range().to_std_range(),
                            edit.content().unwrap_or_default(),
                        )
                    }));
                groups.push(Level::HELP.secondary_title("replace with").element(snippet));
            }
            writeln!(writer, "{}", self.renderer.render(&groups))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ruff_diagnostics::{Edit, Fix};

    use super::*;
    use crate::diagnostics::Diagnostic;
    use crate::source::Source;
    use crate::testing::{format_diagnostic, parse, range};

    fn render_to_string(source: &Source, diag: &Diagnostic) -> String {
        let mut buf = Vec::<u8>::new();
        {
            let mut writer = anstream::AutoStream::never(&mut buf);
            Text::new()
                .emit(
                    &mut writer,
                    &[(source.source_file(), std::slice::from_ref(diag))],
                    &EmitterSummary::default(),
                )
                .expect("emits");
        }
        String::from_utf8(buf).expect("utf-8")
    }

    #[test]
    fn appends_help_block_when_fix_is_available() {
        let source = parse("x = 1\n");
        let rendered = render_to_string(&source, &format_diagnostic(range(0, 1)));
        assert!(rendered.contains("warning: rewrite x to y"));
        assert!(rendered.contains("help: replace with"));
        assert!(rendered.contains('y'));
    }

    #[test]
    fn help_block_renders_every_edit_in_a_group() {
        let source = parse("x = 1\ny = 2\n");
        let rendered = render_to_string(
            &source,
            &Diagnostic {
                fix: Some(Fix::safe_edits(
                    Edit::range_replacement("aaa".to_owned(), range(0, 1)),
                    [Edit::range_replacement("bbb".to_owned(), range(6, 7))],
                )),
                ..format_diagnostic(range(0, 7))
            },
        );
        assert!(rendered.contains("help: replace with"));
        assert!(rendered.contains("aaa"));
        assert!(rendered.contains("bbb"));
    }

    #[test]
    fn renders_path_line_column_message_and_caret() {
        let source = parse("x = 1\n");
        let rendered = render_to_string(
            &source,
            &Diagnostic {
                fix: None,
                ..format_diagnostic(range(0, 1))
            },
        );
        assert!(rendered.contains("warning: rewrite x to y"));
        assert!(rendered.contains("--> <source>:1:1"));
        assert!(rendered.contains("rewrite-x"));
        assert!(rendered.contains("x = 1"));
    }
}
