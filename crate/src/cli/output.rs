//! Human-readable run summary: section anchors and the Ube palette.
//!
//! A plain run writes the anchor and message untinted, matching the
//! emitters and the diff, so nothing downstream has to scan the stream
//! for escapes to remove. The 24-bit-versus-8-color choice is the one
//! this module owns, keyed on `anstyle_query::truecolor`.

use std::io::{self, Write};

use anstyle::{AnsiColor, Color, Reset, RgbColor};

use crate::unstable;

const APRICOT: (RgbColor, AnsiColor) = (RgbColor(0xe8, 0x87, 0x6f), AnsiColor::Red);
const CELADON: (RgbColor, AnsiColor) = (RgbColor(0x8c, 0xc5, 0xa3), AnsiColor::Green);
const UBE: (RgbColor, AnsiColor) = (RgbColor(0x8a, 0x80, 0xcb), AnsiColor::Magenta);

/// Stream-capability signals that gate framing and the diagnostic
/// renderer.
///
/// `color` is the choice stdout resolved to and gates the diagnostics
/// and diffs written there, whereas `stderr_color` is stderr's own and
/// gates the summary line, which is the one thing written to that
/// stream. `quiet` reduces the anchor emoji and color to a bare count
/// line, and a non-TTY stdout leaves `--diff` headers plain so the
/// output stays a valid patch.
pub(super) struct Presentation {
    pub(super) color: bool,
    pub(super) quiet: bool,
    pub(super) stderr_color: bool,
    pub(super) stdout_tty: bool,
}

impl Presentation {
    /// The bare-count shape a `--quiet` run resolves to, which the
    /// summary and notice tests write through.
    #[cfg(test)]
    pub(super) fn quieted() -> Self {
        Self {
            quiet: true,
            ..Self::windowed()
        }
    }

    /// The uncolored, non-quiet, non-TTY shape the runner and emitter
    /// tests write through.
    #[cfg(test)]
    pub(super) fn windowed() -> Self {
        Self {
            color: false,
            quiet: false,
            stderr_color: false,
            stdout_tty: false,
        }
    }

    pub(super) fn decorate_diff(&self) -> bool {
        self.stdout_tty && !self.quiet
    }
}

/// One run's outcome, resolved to a single anchored summary line.
#[derive(Debug)]
pub(super) enum Summary {
    Clean,
    Diagnostics { files: usize, total: usize },
    LintRemainder { total: usize },
    Reformatted { files: usize },
    Unstable { files: usize },
    UnstableRewrite { subject: String },
    WouldReformat { files: usize },
}

impl Summary {
    fn anchor(&self) -> &'static str {
        match self {
            Self::Clean => "🪻",
            Self::Diagnostics { .. } | Self::LintRemainder { .. } => "🔖",
            Self::Reformatted { .. } | Self::WouldReformat { .. } => "🗞️",
            Self::Unstable { .. } | Self::UnstableRewrite { .. } => "🐞",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Clean => "All clean.".to_owned(),
            Self::Diagnostics { files, total } => {
                format!(
                    "{} in {}.",
                    pluralize(*total, "diagnostic"),
                    pluralize(*files, "file")
                )
            }
            Self::LintRemainder { total } => format!(
                "{} not shown. Run `prose check` to see {} in full.",
                pluralize(*total, "lint diagnostic"),
                if *total == 1 { "it" } else { "them" },
            ),
            Self::Reformatted { files } => format!("Reformatted {}.", pluralize(*files, "file")),
            Self::Unstable { files } => format!(
                "{} would change on a second run.",
                pluralize(*files, "file"),
            ),
            Self::UnstableRewrite { subject } => format!("{}.", unstable::headline(subject)),
            Self::WouldReformat { files } => {
                format!("{} would be reformatted.", pluralize(*files, "file"))
            }
        }
    }

    fn tinted(&self) -> String {
        paint(
            &self.message(),
            if matches!(self, Self::Clean) {
                CELADON
            } else {
                APRICOT
            },
        )
    }
}

/// `count` prefixed to `noun`, the noun taking an `s` for any count
/// other than one.
pub(super) fn pluralize(count: usize, noun: &str) -> String {
    let suffix = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{suffix}")
}

/// Writes the closing summary line, tinted only where the run resolved
/// to color.
pub(super) fn report(
    writer: &mut dyn Write,
    present: &Presentation,
    summary: &Summary,
) -> io::Result<()> {
    if present.quiet {
        return writeln!(writer, "{}", summary.message());
    }
    if !present.stderr_color {
        return writeln!(writer, "{} {}", summary.anchor(), summary.message());
    }
    writeln!(writer, "{} {}", ube(summary.anchor()), summary.tinted())
}

pub(super) fn ube(text: &str) -> String {
    paint(text, UBE)
}

fn paint(text: &str, color: (RgbColor, AnsiColor)) -> String {
    paint_with(text, anstyle_query::truecolor(), color)
}

fn paint_with(text: &str, truecolor: bool, (rgb, fallback): (RgbColor, AnsiColor)) -> String {
    let color = if truecolor {
        Color::Rgb(rgb)
    } else {
        Color::Ansi(fallback)
    };
    format!("{}{text}{}", color.render_fg(), Reset.render())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// The summary line as it reaches the stream, which strips nothing,
    /// so an escape here is one the caller would see.
    fn rendered(present: &Presentation, summary: &Summary) -> String {
        let mut buf = Vec::new();
        report(&mut buf, present, summary).expect("reports");
        String::from_utf8(buf).expect("utf-8")
    }

    fn colored() -> Presentation {
        Presentation {
            color: true,
            quiet: false,
            stderr_color: true,
            stdout_tty: true,
        }
    }

    #[rstest]
    #[case(true, false, true)]
    #[case(true, true, false)]
    #[case(false, false, false)]
    #[case(false, true, false)]
    fn decorate_diff_requires_a_tty_without_quiet(
        #[case] stdout_tty: bool,
        #[case] quiet: bool,
        #[case] expected: bool,
    ) {
        let present = Presentation {
            color: true,
            quiet,
            stderr_color: true,
            stdout_tty,
        };

        assert_eq!(present.decorate_diff(), expected);
    }

    #[rstest]
    #[case(Summary::Clean, "🪻 All clean.\n")]
    #[case(Summary::Diagnostics { files: 2, total: 5 }, "🔖 5 diagnostics in 2 files.\n")]
    #[case(Summary::Diagnostics { files: 1, total: 1 }, "🔖 1 diagnostic in 1 file.\n")]
    #[case(
        Summary::LintRemainder { total: 1 },
        "🔖 1 lint diagnostic not shown. Run `prose check` to see it in full.\n"
    )]
    #[case(
        Summary::LintRemainder { total: 3 },
        "🔖 3 lint diagnostics not shown. Run `prose check` to see them in full.\n"
    )]
    #[case(Summary::Reformatted { files: 4 }, "🗞️ Reformatted 4 files.\n")]
    #[case(
        Summary::Unstable { files: 1 },
        "🐞 1 file would change on a second run.\n"
    )]
    #[case(
        Summary::Unstable { files: 3 },
        "🐞 3 files would change on a second run.\n"
    )]
    #[case(
        Summary::UnstableRewrite { subject: "src/a.py".to_owned() },
        "🐞 prose rewrote src/a.py to output a second run would change.\n"
    )]
    #[case(
        Summary::UnstableRewrite { subject: "4 files".to_owned() },
        "🐞 prose rewrote 4 files to output a second run would change.\n"
    )]
    #[case(Summary::Reformatted { files: 1 }, "🗞️ Reformatted 1 file.\n")]
    #[case(Summary::WouldReformat { files: 3 }, "🗞️ 3 files would be reformatted.\n")]
    fn each_outcome_renders_its_anchored_line(#[case] summary: Summary, #[case] expected: &str) {
        assert_eq!(rendered(&Presentation::windowed(), &summary), expected);
    }

    #[test]
    fn paint_emits_rgb_under_truecolor() {
        let painted = paint_with("x", true, UBE);
        assert!(painted.contains("\u{1b}[38;2;138;128;203m"));
        assert!(painted.ends_with("\u{1b}[0m"));
    }

    #[test]
    fn paint_falls_back_to_ansi_without_truecolor() {
        let painted = paint_with("x", false, UBE);
        assert!(painted.contains("\u{1b}[35m"));
        assert!(!painted.contains("38;2;"));
    }

    #[test]
    fn a_color_run_tints_the_anchor_and_the_message() {
        let line = rendered(&colored(), &Summary::Clean);

        assert!(line.contains("\u{1b}["), "line was {line:?}");
        assert!(line.contains("All clean."));
    }

    #[test]
    fn a_plain_run_writes_no_escape_for_the_stream_to_carry() {
        let line = rendered(&Presentation::windowed(), &Summary::Clean);

        assert!(!line.contains('\u{1b}'), "line was {line:?}");
    }

    #[rstest]
    #[case::stderr_carries_the_color(false, true, true)]
    #[case::stdout_carries_it_alone(true, false, false)]
    fn the_summary_follows_the_stream_it_is_written_to(
        #[case] color: bool,
        #[case] stderr_color: bool,
        #[case] tinted: bool,
    ) {
        let present = Presentation {
            color,
            quiet: false,
            stderr_color,
            stdout_tty: true,
        };

        let line = rendered(&present, &Summary::Clean);

        assert_eq!(line.contains('\u{1b}'), tinted, "line was {line:?}");
    }

    #[test]
    fn quiet_strips_emoji_and_color() {
        let out = rendered(
            &Presentation::quieted(),
            &Summary::Diagnostics { files: 2, total: 5 },
        );
        assert_eq!(out, "5 diagnostics in 2 files.\n");
    }
}
