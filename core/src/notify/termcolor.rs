// Copyright 2017 Peter William and collaborators
// Licensed under the MIT License.

/*!
A notification backend that sends colorized output to the terminal.

This module is ripped off from the `status.termcolor` module used by the
[Tectonic](https://github.com/tectonic-typesetting/tectonic) typesetting
engine. (Which the author of this module also wrote.)

*/

// TODO: make this module a feature that can be disabled if the user doesn't want to
// link with termcolor

use anyhow::Error;
use std::fmt::Arguments;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use super::{ChatterLevel, NotificationBackend, NotificationKind};

/// A notification backend that writes colorized output to the terminal.
///
/// This struct implements the NotificationBackend trait, and emits
/// notifications to standard output and standard error with colorized
/// prefixes.
pub struct TermcolorNotificationBackend {
    chatter: ChatterLevel,
    stdout: StandardStream,
    stderr: StandardStream,
    note_spec: ColorSpec,
    //highlight_spec: ColorSpec,
    warning_spec: ColorSpec,
    severe_spec: ColorSpec,
    fatal_spec: ColorSpec,
}

impl TermcolorNotificationBackend {
    /// Create a new TermcolorNotificationBackend.
    pub fn new(chatter: ChatterLevel) -> TermcolorNotificationBackend {
        let mut note_spec = ColorSpec::new();
        note_spec.set_fg(Some(Color::Green)).set_bold(true);

        //let mut highlight_spec = ColorSpec::new();
        //highlight_spec.set_bold(true);

        let mut warning_spec = ColorSpec::new();
        warning_spec.set_fg(Some(Color::Yellow)).set_bold(true);

        let mut severe_spec = ColorSpec::new();
        severe_spec.set_fg(Some(Color::Red)).set_bold(true);

        let mut fatal_spec = ColorSpec::new();
        fatal_spec.set_fg(Some(Color::Red)).set_bold(true);

        TermcolorNotificationBackend {
            chatter,
            stdout: StandardStream::stdout(ColorChoice::Auto),
            stderr: StandardStream::stderr(ColorChoice::Auto),
            note_spec,
            //highlight_spec: highlight_spec,
            warning_spec,
            severe_spec,
            fatal_spec,
        }
    }

    fn styled<F>(&mut self, kind: NotificationKind, f: F)
    where
        F: FnOnce(&mut StandardStream),
    {
        if kind == NotificationKind::Note && self.chatter <= ChatterLevel::Minimal {
            return;
        }

        let (spec, stream) = match kind {
            NotificationKind::Note => (&self.note_spec, &mut self.stdout),
            NotificationKind::Warning => (&self.warning_spec, &mut self.stderr),
            NotificationKind::Severe => (&self.severe_spec, &mut self.stderr),
            NotificationKind::Fatal => (&self.fatal_spec, &mut self.stderr),
        };

        stream.set_color(spec).expect("failed to set color");
        f(stream);
        stream.reset().expect("failed to clear color");
    }

    fn with_stream<F>(&mut self, kind: NotificationKind, f: F)
    where
        F: FnOnce(&mut StandardStream),
    {
        if kind == NotificationKind::Note && self.chatter <= ChatterLevel::Minimal {
            return;
        }

        let stream = match kind {
            NotificationKind::Note => &mut self.stdout,
            NotificationKind::Warning => &mut self.stderr,
            NotificationKind::Severe => &mut self.stderr,
            NotificationKind::Fatal => &mut self.stderr,
        };

        f(stream);
    }

    fn generic_message(&mut self, kind: NotificationKind, prefix: Option<&str>, args: Arguments) {
        let text = match prefix {
            Some(s) => s,
            None => match kind {
                NotificationKind::Note => "note:",
                NotificationKind::Warning => "warning:",
                NotificationKind::Severe => "severe:",
                NotificationKind::Fatal => "fatal:",
            },
        };

        self.styled(kind, |s| {
            write!(s, "{text}").expect("failed to write to standard stream");
        });
        self.with_stream(kind, |s| {
            writeln!(s, " {args}").expect("failed to write to standard stream");
        });
    }

    // Helpers for the CLI program that aren't needed by the internal bits,
    // so we put them here to minimize the cross-section of the NotificationBackend
    // trait.

    /// Print the information contained in an Error object.
    ///
    /// This function prints out the error, the sub-errors that caused it, and
    /// its associated backtrace if available, with colorization.
    pub fn bare_error<E: Into<Error>>(&mut self, err: E) {
        let mut prefix = "error:";
        let err = err.into();

        for fail in err.chain() {
            self.generic_message(
                NotificationKind::Severe,
                Some(prefix),
                format_args!("{fail}"),
            );
            prefix = "caused by:";
        }

        let backtrace = err.backtrace();
        self.generic_message(
            NotificationKind::Severe,
            Some("debugging:"),
            format_args!("backtrace follows:"),
        );
        self.with_stream(NotificationKind::Severe, |s| {
            writeln!(s, "{backtrace:?}").expect("backtrace dump failed");
        });
    }
}

impl NotificationBackend for TermcolorNotificationBackend {
    fn notify(&mut self, kind: NotificationKind, args: Arguments, err: Option<Error>) {
        self.generic_message(kind, None, args);

        if let Some(e) = err {
            for fail in e.chain() {
                self.generic_message(kind, Some("caused by:"), format_args!("{fail}"));
            }

            let backtrace = e.backtrace();
            self.generic_message(kind, Some("debugging:"), format_args!("backtrace follows:"));
            self.with_stream(kind, |s| {
                writeln!(s, "{backtrace:?}").expect("backtrace dump failed");
            });
        }
    }
}
