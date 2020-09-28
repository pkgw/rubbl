// Copyright 2017 Peter Williams and collaborators
// Licensed under the MIT License.

/*!
A framework for notifying users about what tools are doing.

This module provides a way for Rubbl programs to notify the user about actions
taken, problems, and so on. It is very narrowly targeted at the command-line
use case.

This module is ripped off from the `status` module used by the
[Tectonic](https://github.com/tectonic-typesetting/tectonic) typesetting
engine. (Which the author of this module also wrote.)

*/

#[macro_use]
pub mod termcolor;

use clap;
use failure::Error;
use std::cmp;
use std::fmt::Arguments;
use std::result::Result as StdResult;

/// How chatty the notification system should be.
#[repr(usize)]
#[derive(Clone, Copy, Eq, Debug)]
pub enum ChatterLevel {
    /// A minimal level of output — only warnings and errors will be reported.
    Minimal = 0,

    /// The normal level of output — informational messages will be reported.
    Normal,
}

impl PartialEq for ChatterLevel {
    #[inline]
    fn eq(&self, other: &ChatterLevel) -> bool {
        *self as usize == *other as usize
    }
}

impl PartialOrd for ChatterLevel {
    #[inline]
    fn partial_cmp(&self, other: &ChatterLevel) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChatterLevel {
    #[inline]
    fn cmp(&self, other: &ChatterLevel) -> cmp::Ordering {
        (*self as usize).cmp(&(*other as usize))
    }
}

/// The kind of notification that is being produced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationKind {
    /// An informational notice.
    Note,

    /// Warning of an unusual condition; the program will likely perform as intended.
    Warning,

    /// Notification of a severe problem; the program will likely fail but will attempt to contine.
    Severe,

    /// Notification of a fatal error; the program must give up.
    Fatal,
}

/// Trait for type that handle notifications to the user.
pub trait NotificationBackend {
    /// Notify the user about an event.
    ///
    /// If `err` is not `None`, the information contained in the object should
    /// be reported after the main message.
    fn notify(&mut self, kind: NotificationKind, args: Arguments, err: Option<Error>);
}

/// Send an informational notification to the user.
///
/// Standard usage looks like this:
///
/// ```rust,ignore
/// rn_note!(nb, "downloaded {} files", n_files);
/// ```
///
/// where `nb` is a type implementing the NotificationBackend trait. You may
/// also provide an Error value after a semicolon; the information it contains
/// will be printed after the informational message. This is not expected to
/// be common usage for this particular macro, but makes more sense for the
/// `rn_warning!`, `rn_severe!`, and `rn_fatal!` macros.
#[macro_export]
macro_rules! rn_note {
    ($dest:expr, $( $fmt_args:expr ),*) => {
        $dest.notify($crate::notify::NotificationKind::Note, format_args!($( $fmt_args ),*), None)
    };
    ($dest:expr, $( $fmt_args:expr ),* ; $err:expr) => {
        $dest.notify($crate::notify::NotificationKind::Note, format_args!($( $fmt_args ),*), Some($err))
    };
}

/// Warn the user of a problematic condition.
///
/// See the documentation of `rn_note!` for usage information. This macro
/// should be used when an unusual condition has been detected, but the task
/// at hand will likely succeed.
#[macro_export]
macro_rules! rn_warning {
    ($dest:expr, $( $fmt_args:expr ),*) => {
        $dest.notify($crate::notify::NotificationKind::Warning, format_args!($( $fmt_args ),*), None)
    };
    ($dest:expr, $( $fmt_args:expr ),* ; $err:expr) => {
        $dest.notify($crate::notify::NotificationKind::Warning, format_args!($( $fmt_args ),*), Some($err))
    };
}

/// Notify the user of a severe problem.
///
/// See the documentation of `rn_note!` for usage information. This macro
/// should be used when an issue has been detected that makes it likely that
/// the task at hand cannot be completed successfully; however, the program
/// will attempt to continue.
#[macro_export]
macro_rules! rn_severe {
    ($dest:expr, $( $fmt_args:expr ),*) => {
        $dest.notify($crate::notify::NotificationKind::Severe, format_args!($( $fmt_args ),*), None)
    };
    ($dest:expr, $( $fmt_args:expr ),* ; $err:expr) => {
        $dest.notify($crate::notify::NotificationKind::Severe, format_args!($( $fmt_args ),*), Some($err))
    };
}

/// Notify the user of a fatal problem.
///
/// See the documentation of `rn_note!` for usage information. This macro
/// should be used when an issue has been detected that forces the program to
/// give up on the task at hand. If the command-line interface is being used,
/// it will probably exit almost immediately after a fatal notification is
/// issued.
#[macro_export]
macro_rules! rn_fatal {
    ($dest:expr, $( $fmt_args:expr ),*) => {
        $dest.notify($crate::notify::NotificationKind::Fatal, format_args!($( $fmt_args ),*), None)
    };
    ($dest:expr, $( $fmt_args:expr ),* ; $err:expr) => {
        $dest.notify($crate::notify::NotificationKind::Fatal, format_args!($( $fmt_args ),*), Some($err))
    };
}

/// A no-op notification backend.
///
/// This empty structure implements the NotificationBackend trait. Its
/// `notify()` function does nothing.
#[derive(Clone, Copy, Debug)]
pub struct NoopNotificationBackend {}

impl NoopNotificationBackend {
    /// Create a new NoopNotificationBackend object.
    pub fn new() -> NoopNotificationBackend {
        NoopNotificationBackend {}
    }
}

impl NotificationBackend for NoopNotificationBackend {
    fn notify(&mut self, _kind: NotificationKind, _args: Arguments, _err: Option<Error>) {}
}

#[derive(Debug)]
struct NotificationData {
    kind: NotificationKind,
    text: String,
    err: Option<Error>,
}

/// A notification backend that buffers notifications and emits them later.
#[derive(Debug)]
pub struct BufferingNotificationBackend {
    buf: Vec<NotificationData>,
}

impl BufferingNotificationBackend {
    /// Create and return a new BufferingNotificationBackend.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Empty the buffered notifications into a different notification backend.
    ///
    /// This function consumes the object.
    pub fn drain<B: NotificationBackend>(mut self, other: &mut B) {
        for info in self.buf.drain(..) {
            other.notify(info.kind, format_args!("{}", info.text), info.err);
        }
    }
}

impl NotificationBackend for BufferingNotificationBackend {
    fn notify(&mut self, kind: NotificationKind, args: Arguments, err: Option<Error>) {
        self.buf.push(NotificationData {
            kind: kind,
            text: format!("{}", args),
            err: err,
        });
    }
}

/// An extension trait for adding standard notification arguments to a clap
/// App object.
pub trait ClapNotificationArgsExt {
    /// Add standard Rubbl notification-related arguments to this App.
    fn rubbl_notify_args(self) -> Self;
}

impl<'a, 'b> ClapNotificationArgsExt for clap::App<'a, 'b> {
    fn rubbl_notify_args(self) -> Self {
        self.arg(
            clap::Arg::with_name("chatter_level")
                .long("chatter")
                .short("c")
                .value_name("LEVEL")
                .help("How much chatter to print when running")
                .possible_values(&["default", "minimal"])
                .default_value("default"),
        )
    }
}

/// Run a function with colorized reporting of errors.
pub fn run_with_notifications<'a, E, F>(matches: clap::ArgMatches<'a>, inner: F) -> i32
where
    E: Into<Error>,
    F: FnOnce(clap::ArgMatches<'a>, &mut dyn NotificationBackend) -> StdResult<i32, E>,
{
    let chatter = match matches.value_of("chatter_level").unwrap() {
        "default" => ChatterLevel::Normal,
        "minimal" => ChatterLevel::Minimal,
        _ => unreachable!(),
    };

    // Set up colorized output. At one point we might add an option to disable
    // this, which is why the inner function takes a boxed trait object.

    let mut tnb = termcolor::TermcolorNotificationBackend::new(chatter);

    // Now that we've got colorized output, we're ready to pass off to the
    // inner function ... all so that we can print out the word "error:" in
    // red.

    match inner(matches, &mut tnb) {
        Ok(ret) => ret,

        Err(e) => {
            tnb.bare_error(e);
            1
        }
    }
}
