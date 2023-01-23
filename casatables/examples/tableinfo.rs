// Copyright 2017-2023 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

//! Summarize the structure of a CASA table.

use anyhow::Error;
use clap::{Arg, Command};
use rubbl_casatables::{Table, TableOpenMode};
use rubbl_core::{ctry, notify::ClapNotificationArgsExt};
use std::{cmp::max, path::PathBuf, process};

fn main() {
    let matches = Command::new("tableinfo")
        .version("0.1.0")
        .rubbl_notify_args()
        .arg(
            Arg::new("IN-TABLE")
                .help("The path of the input data set")
                .required(true)
                .index(1),
        )
        .get_matches();

    process::exit(rubbl_core::notify::run_with_notifications(
        matches,
        |matches, _nbe| -> Result<i32, Error> {
            let inpath = matches.get_one::<PathBuf>("IN-TABLE").unwrap();

            let mut t = ctry!(
                Table::open(&inpath, TableOpenMode::Read);
                "failed to open input table \"{}\"", inpath.display()
            );

            println!("Table \"{}\":", inpath.display());
            println!("Number of rows: {}", t.n_rows());
            println!("Number of columns: {}", t.n_columns());
            println!("");

            let col_names = ctry!(
                t.column_names();
                "failed to get names of columns in \"{}\"", inpath.display()
            );

            let mut max_name_len = 0;
            let mut max_type_len = 0;
            let mut info: Vec<(&str, String, String)> = Vec::new();

            for n in &col_names {
                let desc = ctry!(
                    t.get_col_desc(&n);
                    "failed to query column \"{}\" in \"{}\"", n, inpath.display()
                );

                let type_text = format!("{}", desc.data_type());

                let multiplicity_text = if desc.is_scalar() {
                    "scalar".to_owned()
                } else if desc.is_fixed_shape() {
                    format!("vector of shape {:?}", desc.shape().unwrap())
                } else {
                    "variable-shape vector".to_owned()
                };

                max_name_len = max(max_name_len, n.len());
                max_type_len = max(max_type_len, type_text.len());

                info.push((&n, type_text, multiplicity_text));
            }

            for i in info {
                println!(
                    "{0:<1$}  {2:<3$}  {4}",
                    i.0, max_name_len, i.1, max_type_len, i.2
                );
            }

            let table_kw_names = ctry!(
                t.table_keyword_names();
                "failed to get keyword info in \"{}\"", inpath.display()
            );

            if table_kw_names.len() != 0 {
                println!();
                println!("Sub-tables (table-type \"keywords\"):");

                for n in &table_kw_names {
                    println!("  {}", n);
                }
            }

            Ok(0)
        },
    ));
}
