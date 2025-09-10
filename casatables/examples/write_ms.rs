use anyhow::Error;
use clap::{Arg, Command};
use ndarray::Array2;
use rubbl_casatables::{
    Complex, GlueDataType, TSMOption, Table, TableCreateMode, TableDesc, TableDescCreateMode,
};
use rubbl_core::{ctry, notify::ClapNotificationArgsExt};
use std::{path::PathBuf, process, str::FromStr};

/// Perform basic casatables operations on a mock Measurement Set
///
/// Example usage with strace:
/// ```bash
/// cd casatables/examples
/// cargo build --example write_ms
/// rm -rf /tmp/test.ms; strace -f -s 256 -k -o write_ms_rust_create_init.strace ../../target/debug/examples/write_ms /tmp/test.ms -w create_only -i true
/// rm -rf /tmp/test.ms; strace -f -s 256 -k -o write_ms_rust_create_noinit.strace ../../target/debug/examples/write_ms /tmp/test.ms -w create_only -i false
/// wc -l *.strace
/// ```
/// ```txt
///    7949 write_ms_rust_create_init.strace
///    5871 write_ms_rust_create_noinit.strace
/// ```
///
fn main() {
    let matches = Command::new("benchmark")
        .rubbl_notify_args()
        .arg(
            Arg::new("TABLE-PATH")
                .value_parser(clap::value_parser!(PathBuf))
                .help("The path where the benchmark table will be created")
                .default_value("/tmp/write_ms_rust.ms")
                .index(1),
        )
        .arg(
            Arg::new("rows")
                .short('r')
                .long("rows")
                .value_parser(clap::value_parser!(usize))
                .help("Number of rows to create")
                .default_value("100"),
        )
        .arg(
            Arg::new("tsm_option")
                .short('t')
                .long("tsm_option")
                .value_parser(["CACHE", "BUFFER", "MMAP", "DEFAULT", "AIPSRC"])
                .help("TSM option to use")
                .default_value("DEFAULT"),
        )
        .arg(
            Arg::new("initialize")
                .short('i')
                .long("initialize")
                .help("Initialize the table")
                .value_parser(clap::value_parser!(bool))
                .default_value("false"),
        )
        .arg(
            Arg::new("write_mode")
                .short('w')
                .long("write_mode")
                .value_parser(["create_only", "table_put_cell"])
                .help("Write mode to use")
                .default_value("table_put_cell"),
        )
        .arg(
            Arg::new("data_shape")
                .short('d')
                .long("data_shape")
                .value_parser(|s: &str| {
                    s.split(',')
                        .map(|x| x.trim().parse::<u64>())
                        .collect::<Result<Vec<_>, _>>()
                })
                .help("Data shape to use (comma-separated, e.g. 32,4)")
                .default_value("32,4"),
        )
        .get_matches();

    process::exit(rubbl_core::notify::run_with_notifications(
        matches,
        |matches, _nbe| -> Result<i32, Error> {
            let table_path = matches.get_one::<PathBuf>("TABLE-PATH").unwrap();
            let n_rows = *matches.get_one::<usize>("rows").unwrap();
            let tsm_option =
                TSMOption::from_str(matches.get_one::<String>("tsm_option").unwrap()).unwrap();
            let initialize = matches.get_one::<bool>("initialize").unwrap();
            let write_mode = matches.get_one::<String>("write_mode").unwrap();
            let data_shape = matches.get_one::<Vec<u64>>("data_shape").unwrap();

            let mut table: Table = create_test_table(
                table_path,
                n_rows,
                data_shape.as_slice(),
                *initialize,
                Some(tsm_option),
            )?;
            let data_template = create_test_data(data_shape.as_slice());
            let flags_template = create_test_flags(data_shape.as_slice());

            match write_mode.as_str() {
                "create_only" => {
                    // Do nothing
                }
                "table_put_cell" => {
                    write_test_data_table_put_cell(&mut table, &data_template, &flags_template)?;
                }
                _ => {
                    eprintln!("Unknown write mode '{}'", write_mode);
                    std::process::exit(1);
                }
            }

            Ok(0)
        },
    ));
}

fn create_test_table(
    table_path: &PathBuf,
    n_rows: usize,
    data_shape: &[u64],
    initialize: bool,
    tsm_option: Option<TSMOption>,
) -> Result<Table, Error> {
    // Create a fresh table using the rubbl API that mirrors the C++ demo
    // Build the same schema as the C++ example using existing rubbl APIs
    let mut table_desc = ctry!(TableDesc::new("syscall_test", TableDescCreateMode::TDM_NEW);
        "failed to create test table description"
    );
    // Scalars - match C++ exactly
    table_desc.add_scalar_column(
        GlueDataType::TpDouble,
        "TIME",
        Some("Observation time"),
        false,
        false,
    )?;
    table_desc.add_scalar_column(
        GlueDataType::TpInt,
        "ANTENNA1",
        Some("First antenna"),
        false,
        false,
    )?;
    table_desc.add_scalar_column(
        GlueDataType::TpInt,
        "ANTENNA2",
        Some("Second antenna"),
        false,
        false,
    )?;
    table_desc.add_scalar_column(
        GlueDataType::TpBool,
        "FLAG_ROW",
        Some("Row flag"),
        false,
        false,
    )?;
    // Fixed-shape arrays - match C++ exactly (no comments for arrays)
    // For now we always add arrays the same way; backend SM selection is handled by TSMOption
    table_desc.add_array_column(
        GlueDataType::TpComplex,
        "DATA",
        None,
        Some(data_shape),
        false,
        false,
    )?;
    table_desc.add_array_column(
        GlueDataType::TpBool,
        "FLAG",
        None,
        Some(data_shape),
        false,
        false,
    )?;

    let table = Table::new(
        table_path,
        table_desc,
        n_rows,
        TableCreateMode::New,
        initialize,
        tsm_option,
    )?;
    Ok(table)
}

fn create_test_data(data_shape: &[u64]) -> Array2<Complex<f32>> {
    let shape = (data_shape[0] as usize, data_shape[1] as usize);
    let mut data = Array2::<Complex<f32>>::zeros(shape);

    // Fill with some pattern to make it interesting for analysis
    for ((i, j), elem) in data.indexed_iter_mut() {
        *elem = Complex::new(
            (i as f32 * 0.1).sin() * (j as f32 * 0.2).cos(),
            (i as f32 * 0.15).cos() * (j as f32 * 0.25).sin(),
        );
    }

    data
}

fn create_test_flags(data_shape: &[u64]) -> ndarray::Array2<bool> {
    let shape = (data_shape[0] as usize, data_shape[1] as usize);
    let mut flags = ndarray::Array2::<bool>::from_elem(shape, false);

    // Set some flags to create realistic patterns
    for ((i, j), elem) in flags.indexed_iter_mut() {
        *elem = (i + j) % 17 == 0; // Arbitrary pattern
    }

    flags
}

fn write_test_data_table_put_cell(
    table: &mut Table,
    data_template: &Array2<Complex<f32>>,
    flags_template: &ndarray::Array2<bool>,
) -> Result<(), Error> {
    let rows_to_write = table.n_rows();
    for row_u64 in 0..rows_to_write {
        // Scalars
        table.put_cell("TIME", row_u64, &(row_u64 as f64))?;
        table.put_cell("ANTENNA1", row_u64, &((row_u64 as i32) % 128))?;
        table.put_cell("ANTENNA2", row_u64, &((row_u64 as i32 + 1) % 128))?;
        table.put_cell("FLAG_ROW", row_u64, &((row_u64 % 2) == 0))?;

        // Arrays
        table.put_cell("DATA", row_u64, data_template)?;
        table.put_cell("FLAG", row_u64, flags_template)?;
    }
    Ok(())
}

// No readback: benchmark only creates and writes the table
