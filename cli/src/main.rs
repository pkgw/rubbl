// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

/*! The main rubbl driver command

This just provides swiss-army-knife access to commands installed by other
Rubbl modules. Not 100% sure this is the way to got but we'll see.

Heavily modeled on Cargo's implementation of the same sort of functionality.

*/

#[macro_use] extern crate clap;
#[macro_use] extern crate failure_derive;
extern crate failure;
extern crate rubbl_core;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use failure::Error;
use rubbl_core::Result;
use rubbl_core::notify::{ClapNotificationArgsExt, NotificationBackend};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process;


// Some error help.

#[derive(Fail, Debug)]
#[fail(display = "no such sub-command `{}`", _0)]
pub struct NoSuchSubcommandError(String);


fn main() {
    let matches = make_app().get_matches();

    process::exit(rubbl_core::notify::run_with_notifications(matches, |matches, nbe| -> Result<i32> {
        match matches.subcommand() {
            ("help", Some(m)) => do_help(m, nbe),
            ("list", Some(m)) => do_list(m, nbe),
            (external, Some(m)) => do_external(external, m, nbe),
            (_, None) => {
                // No sub-command provided; can't use do_help() since it wants sub-matches.
                make_app().print_long_help()?;
                Ok(0)
            }
        }
    }));
}


/// It seems that the best way to re-print the help in the "help" subcommand
/// is to be able to make multiple App objects.
fn make_app<'a, 'b>() -> App<'a, 'b> {
    App::new("rubbl")
        .version(crate_version!())
        .setting(AppSettings::AllowExternalSubcommands)
        .setting(AppSettings::DisableHelpSubcommand)
        .rubbl_notify_args()
        .subcommand(SubCommand::with_name("help")
                    .about("Get help information for sub-commands")
                    .arg(Arg::with_name("command")
                         .help("The name of a sub-command to get help for")))
        .subcommand(SubCommand::with_name("list")
                    .about("List the available sub-commands"))
        .help(r#"rubbl -- dispatcher for command-line access to Rubbl tools

USAGE:
    rubbl [GLOBAL-OPTIONS] [SUBCOMMAND] [SUBCOMMAND arguments ...]

GLOBAL OPTIONS:
    -h, --help     Print help information
    -V, --version  Print version information

SUBCOMMANDS:
    Available sub-commands depend on which Rubbl tools you have installed.
    Use "rubbl list" to see what is available and "rubbl help" to get help
    on their usage. Built-in sub-commands are:

    help    Get help on sub-command usage
    list    List the available sub-commands
"#)
}


/// Get help on a subcommand, or on the main program.
fn do_help(matches: &ArgMatches, _nbe: &mut NotificationBackend) -> Result<i32> {
    match matches.value_of("command") {
        None | Some("help") | Some("list") => {
            make_app().print_long_help()?;
            Ok(0)
        },

        Some(cmd) => {
            // If the function returns, something went wrong by definition.
            Err(try_exec_subcommand(cmd, &["--help"]))
        }
    }
}


/// Print out a list of the available sub-commands.
fn do_list(_matches: &ArgMatches, _nbe: &mut NotificationBackend) -> Result<i32> {
    println!("Currently available \"rubbl\" sub-commands:");

    for command in list_commands() {
        println!("    {}", command);
    }

    Ok(0)
}


/// Run an external command by executing a subprocess
fn do_external(cmd: &str, matches: &ArgMatches, _nbe: &mut NotificationBackend) -> Result<i32> {
    // TODO: propagate chatter settings downstream.
    let args: Vec<&str> = match matches.values_of("") {
        Some(v) => v.collect(),
        None => Vec::new(),
    };

    Err(try_exec_subcommand(cmd, &args))
}


/// Try to re-execute the process using the executable corresponding to the
/// named sub-command. If this function returns, something went wrong.
fn try_exec_subcommand(cmd: &str, args: &[&str]) -> Error {
    let command_exe = format!("rubbl-{}{}", cmd, env::consts::EXE_SUFFIX);
    let path = search_directories()
        .iter()
        .map(|dir| dir.join(&command_exe))
        .find(|file| is_executable(file));

    let command = match path {
        Some(command) => command,
        None => {
            return NoSuchSubcommandError(cmd.to_owned()).into();
        }
    };

    process::Command::new(command)
        .args(args)
        .exec()
        .into()
}


// Lots of copy/paste from cargo:

fn list_commands() -> BTreeSet<String> {
    let prefix = "rubbl-";
    let suffix = env::consts::EXE_SUFFIX;
    let mut commands = BTreeSet::new();

    for dir in search_directories() {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            _ => continue,
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let filename = match path.file_name().and_then(|s| s.to_str()) {
                Some(filename) => filename,
                _ => continue,
            };
            if !filename.starts_with(prefix) || !filename.ends_with(suffix) {
                continue;
            }
            if is_executable(entry.path()) {
                let end = filename.len() - suffix.len();
                commands.insert(filename[prefix.len()..end].to_string());
            }
        }
    }

    commands.insert("help".to_owned());
    commands.insert("list".to_owned());

    commands
}

#[cfg(unix)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::prelude::*;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    fs::metadata(path).map(|metadata| metadata.is_file()).unwrap_or(false)
}

fn search_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(val) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&val));
    }
    dirs
}
