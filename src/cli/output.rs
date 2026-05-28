//! Human-readable run summary: section anchors and the Ube palette.
//!
//! Color is emitted unconditionally and stripped downstream by the
//! `anstream::AutoStream` the summary writes through, so `--color
//! never` and non-TTY runs fall back to plain text without a branch
//! here. The 24-bit-versus-8-color choice is the one decision this
//! module owns, keyed on `anstyle_query::truecolor`.

use std::io::{self, Write};

use anstyle::{AnsiColor, Color, Reset, RgbColor};

const APRICOT: (RgbColor, AnsiColor) = (RgbColor(0xe8, 0x87, 0x6f), AnsiColor::Red);
const CELADON: (RgbColor, AnsiColor) = (RgbColor(0x8c, 0xc5, 0xa3), AnsiColor::Green);
const UBE: (RgbColor, AnsiColor) = (RgbColor(0x8a, 0x80, 0xcb), AnsiColor::Magenta);

/// Stream-capability signals that gate framing independently of color.
///
/// `quiet` strips the anchor emoji and color down to a bare count
/// line, and a non-TTY stdout leaves `--diff` headers plain so the
/// output stays a valid patch.
pub(crate) struct Presentation {
    pub(crate) quiet: bool,
    pub(crate) stdout_tty: bool,
}

impl Presentation {
    pub(crate) fn decorate_diff(&self) -> bool {
        self.stdout_tty && !self.quiet
    }
}

/// One run's outcome, resolved to a single anchored summary line.
#[derive(Debug)]
pub(crate) enum Summary {
    Clean,
    Diagnostics { files: usize, total: usize },
    Reformatted { files: usize },
    WouldReformat { files: usize },
}

impl Summary {
    fn anchor(&self) -> &'static str {
        match self {
            Self::Clean => "🪻",
            Self::Diagnostics { .. } => "☕",
            Self::Reformatted { .. } | Self::WouldReformat { .. } => "🗞️",
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
            Self::Reformatted { files } => format!("Reformatted {}.", pluralize(*files, "file")),
            Self::WouldReformat { files } => {
                format!("{} would be reformatted.", pluralize(*files, "file"))
            }
        }
    }

    fn tinted(&self) -> String {
        match self {
            Self::Clean => celadon(&self.message()),
            _ => apricot(&self.message()),
        }
    }
}

/// Writes the closing summary line. Color escapes are stripped
/// downstream when `writer` is a non-color `AutoStream`.
pub(crate) fn report(
    writer: &mut dyn Write,
    present: &Presentation,
    summary: &Summary,
) -> io::Result<()> {
    if present.quiet {
        return writeln!(writer, "{}", summary.message());
    }
    writeln!(writer, "{} {}", ube(summary.anchor()), summary.tinted())
}

pub(crate) fn ube(text: &str) -> String {
    paint(text, UBE)
}

fn apricot(text: &str) -> String {
    paint(text, APRICOT)
}

fn celadon(text: &str) -> String {
    paint(text, CELADON)
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

fn pluralize(count: usize, noun: &str) -> String {
    let suffix = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{suffix}")
}

#[cfg(test)]
mod tests {
    use anstream::AutoStream;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    fn plain(present: &Presentation, summary: &Summary) -> String {
        let mut buf = Vec::new();
        {
            let mut writer = AutoStream::never(&mut buf);
            report(&mut writer, present, summary).expect("reports");
        }
        String::from_utf8(buf).expect("utf-8")
    }

    fn quiet() -> Presentation {
        Presentation {
            quiet: true,
            stdout_tty: true,
        }
    }

    fn windowed() -> Presentation {
        Presentation {
            quiet: false,
            stdout_tty: false,
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
        assert_eq!(Presentation { quiet, stdout_tty }.decorate_diff(), expected);
    }

    #[rstest]
    #[case(Summary::Clean, "🪻 All clean.\n")]
    #[case(Summary::Diagnostics { files: 2, total: 5 }, "☕ 5 diagnostics in 2 files.\n")]
    #[case(Summary::Diagnostics { files: 1, total: 1 }, "☕ 1 diagnostic in 1 file.\n")]
    #[case(Summary::Reformatted { files: 4 }, "🗞️ Reformatted 4 files.\n")]
    #[case(Summary::Reformatted { files: 1 }, "🗞️ Reformatted 1 file.\n")]
    #[case(Summary::WouldReformat { files: 3 }, "🗞️ 3 files would be reformatted.\n")]
    fn each_outcome_renders_its_anchored_line(#[case] summary: Summary, #[case] expected: &str) {
        assert_eq!(plain(&windowed(), &summary), expected);
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
    fn quiet_strips_emoji_and_color() {
        let out = plain(&quiet(), &Summary::Diagnostics { files: 2, total: 5 });
        assert_eq!(out, "5 diagnostics in 2 files.\n");
    }
}
