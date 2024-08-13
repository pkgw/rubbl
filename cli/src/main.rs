// Copyright 2022 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

/*! The main rubbl driver command

This just provides swiss-army-knife access to commands installed by other
Rubbl modules. Not 100% sure this is the way to got but we'll see.

Heavily modeled on Cargo's implementation of the same sort of functionality.

*/

use anyhow::Result;
use clap::{crate_version, Arg, ArgMatches, Command};
use rubbl_core::{
    notify::{ClapNotificationArgsExt, NotificationBackend},
    rn_warning,
};
use std::{
    collections::BTreeSet,
    env, fs,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process,
};

// Some error help.

#[derive(thiserror::Error, Debug)]
#[error("no such sub-command `{0}`")]
pub struct NoSuchSubcommandError(String);

fn main() {
    let matches = make_command().get_matches();

    process::exit(rubbl_core::notify::run_with_notifications(
        matches,
        |matches, nbe| -> Result<i32> {
            match matches.subcommand() {
                Some(("help", m)) => do_help(m, nbe),
                Some(("list", m)) => do_list(m, nbe),
                Some(("show", m)) => do_show(m, nbe),
                Some((external, m)) => do_external(external, m, nbe),
                None => {
                    // No sub-command provided; can't use do_help() since it wants sub-matches.
                    make_command().print_long_help()?;
                    Ok(0)
                }
            }
        },
    ));
}

/// It seems that the best way to re-print the help in the "help" subcommand
/// is to be able to make multiple Command objects.
fn make_show_command() -> Command {
    Command::new("show")
        .about("Show various useful metadata")
        .disable_help_subcommand(true)
        .subcommand(
            Command::new("concept-doi").about("Show the Zenodo concept DOI of the Rubbl CLI"),
        )
        .subcommand(
            Command::new("version-doi").about("Show the DOI of this version of the Rubbl CLI"),
        )
}

fn make_command() -> Command {
    Command::new("rubbl")
        .version(crate_version!())
        .allow_external_subcommands(true)
        .disable_help_subcommand(true)
        .rubbl_notify_args()
        .subcommand(
            Command::new("help")
                .about("Get help information for sub-commands")
                .arg(Arg::new("command").help("The name of a sub-command to get help for")),
        )
        .subcommand(Command::new("list").about("List the available sub-commands"))
        .subcommand(make_show_command())
        .help_template(
            r#"rubbl -- dispatcher for command-line access to Rubbl tools

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
    show    Show various useful metadata
"#,
        )
}

/// Get help on a subcommand, or on the main program.
fn do_help(matches: &ArgMatches, _nbe: &mut dyn NotificationBackend) -> Result<i32> {
    match matches.get_one::<String>("command").map(|s| s.as_ref()) {
        None | Some("help") | Some("list") => {
            make_command().print_long_help()?;
            Ok(0)
        }

        Some("show") => {
            make_show_command().print_long_help()?;
            Ok(0)
        }

        Some(cmd) => {
            // If the function returns, something went wrong by definition.
            Err(try_exec_subcommand(cmd, &["--help"]))
        }
    }
}

/// Print out a list of the available sub-commands.
fn do_list(_matches: &ArgMatches, _nbe: &mut dyn NotificationBackend) -> Result<i32> {
    println!("Currently available \"rubbl\" sub-commands:");

    for command in list_commands() {
        println!("    {}", command);
    }

    Ok(0)
}

/// Print useful quantities
fn do_show(matches: &ArgMatches, nbe: &mut dyn NotificationBackend) -> Result<i32> {
    match matches.subcommand() {
        Some(("concept-doi", _)) => {
            // For releases, this will be rewritten to the real DOI:
            let doi = "10.5281/zenodo.7563019";

            if doi.starts_with("xx.") {
                rn_warning!(
                    nbe,
                    "you are running a development build; the printed value is not a real DOI"
                );
            }

            println!("{}", doi);
        }

        Some(("version-doi", _)) => {
            // For releases, this will be rewritten to the real DOI:
            let doi = "10.5281/zenodo.13315460";

            if doi.starts_with("xx.") {
                rn_warning!(
                    nbe,
                    "you are running a development build; the printed value is not a real DOI"
                );
            }

            println!("{}", doi);
        }

        Some(_) | None => {
            make_show_command().print_long_help()?;
        }
    }

    Ok(0)
}

/// Run an external command by executing a subprocess
fn do_external(cmd: &str, matches: &ArgMatches, _nbe: &mut dyn NotificationBackend) -> Result<i32> {
    // TODO: propagate chatter settings downstream.
    let args: Vec<&str> = match matches.get_many::<String>("") {
        Some(v) => v.map(|s| s.as_ref()).collect(),
        None => Vec::new(),
    };

    Err(try_exec_subcommand(cmd, &args))
}

/// Try to re-execute the process using the executable corresponding to the
/// named sub-command. If this function returns, something went wrong.
fn try_exec_subcommand(cmd: &str, args: &[&str]) -> anyhow::Error {
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

    process::Command::new(command).args(args).exec().into()
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
    commands.insert("show".to_owned());

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
    fs::metadata(path)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
}

fn search_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(val) = env::var_os("PATH") {
        dirs.extend(env::split_paths(&val));
    }
    dirs
}
