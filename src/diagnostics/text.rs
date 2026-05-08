//! Text emitter: rustc-style snippet rendering with carets and fix
//! suggestions.

use std::io::{self, Write};

use annotate_snippets::{AnnotationKind, Level, Patch, Renderer, Snippet};
use ruff_text_size::Ranged;

use crate::diagnostics::{Emitter, Run};

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
    fn emit(&self, writer: &mut dyn Write, runs: &[Run<'_>]) -> io::Result<()> {
        for (source, diagnostics) in runs {
            for diag in *diagnostics {
                let warning = Level::WARNING.primary_title(diag.message.as_str()).element(
                    Snippet::source(source.text())
                        .line_start(1)
                        .path(source.filename())
                        .annotation(
                            AnnotationKind::Primary
                                .span(diag.range.to_std_range())
                                .label(diag.rule.as_str()),
                        ),
                );
                let mut groups = vec![warning];
                if let Some(edit) = &diag.fix {
                    groups.push(
                        Level::HELP.secondary_title("replace with").element(
                            Snippet::source(source.text())
                                .line_start(1)
                                .path(source.filename())
                                .patch(Patch::new(
                                    edit.range().to_std_range(),
                                    edit.content().unwrap_or_default(),
                                )),
                        ),
                    );
                }
                writeln!(writer, "{}", self.renderer.render(&groups))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ruff_diagnostics::Edit;
    use ruff_text_size::TextRange;

    use super::*;
    use crate::diagnostics::{Diagnostic, Severity};
    use crate::rule::RuleId;
    use crate::source::Source;

    fn diag(range: TextRange, fix: Option<Edit>) -> Diagnostic {
        Diagnostic {
            fix,
            message: "rewrite x to y".to_owned(),
            range,
            rule: RuleId::from("rewrite-x"),
            severity: Severity::Format,
        }
    }

    fn render_to_string(source: &Source, diag: &Diagnostic) -> String {
        let mut buf = Vec::<u8>::new();
        {
            let mut writer = anstream::AutoStream::never(&mut buf);
            Text::new()
                .emit(&mut writer, &[(source, std::slice::from_ref(diag))])
                .expect("emits");
        }
        String::from_utf8(buf).expect("utf-8")
    }

    #[test]
    fn renders_path_line_column_message_and_caret() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let range = TextRange::new(0.into(), 1.into());
        let rendered = render_to_string(&source, &diag(range, None));
        assert!(rendered.contains("warning: rewrite x to y"));
        assert!(rendered.contains("--> <source>:1:1"));
        assert!(rendered.contains("rewrite-x"));
        assert!(rendered.contains("x = 1"));
    }

    #[test]
    fn appends_help_block_when_fix_is_available() {
        let source: Source = "x = 1\n".parse().expect("parses");
        let range = TextRange::new(0.into(), 1.into());
        let rendered = render_to_string(
            &source,
            &diag(range, Some(Edit::range_replacement("y".to_owned(), range))),
        );
        assert!(rendered.contains("warning: rewrite x to y"));
        assert!(rendered.contains("help: replace with"));
        assert!(rendered.contains('y'));
    }
}
