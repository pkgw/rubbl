// Eqivalent C++ code for the write_ms.rs example
//
// e.g.
// ```bash
// cd casatables/examples
// make
// rm -rf /tmp/test.ms; strace -f -s 256 -k -o write_ms_cpp_create_init.strace ./write_ms -path /tmp/test.ms -write_mode create_only -initialize true
// rm -rf /tmp/test.ms; strace -f -s 256 -k -o write_ms_cpp_create_noinit.strace ./write_ms -path /tmp/test.ms -write_mode create_only -initialize false
// ```
// ```txt
//    5176 write_ms_cpp_create_init.strace
//    3848 write_ms_cpp_create_noinit.strace
// ```

#include <casacore/casa/Exceptions/Error.h>
#include <casacore/tables/Tables/Table.h>
#include <casacore/tables/Tables/TableDesc.h>
#include <casacore/tables/Tables/SetupNewTab.h>
#include <casacore/tables/Tables/ScalarColumn.h>
#include <casacore/tables/Tables/ArrayColumn.h>
#include <casacore/tables/Tables/ScaColDesc.h>
#include <casacore/tables/Tables/ArrColDesc.h>
#include <casacore/casa/Arrays/Vector.h>
#include <casacore/casa/Arrays/Matrix.h>
#include <casacore/casa/Arrays/IPosition.h>
#include <casacore/casa/BasicSL/Complex.h>
#include <casacore/tables/DataMan/TSMOption.h>
#include <vector>
#include <complex>
#include <string>
#include <map>

// Function to parse command-line arguments
std::map<std::string, std::string> parse_args(int argc, char* argv[]) {
    std::map<std::string, std::string> args;
    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];
        if (arg[0] == '-') {
            std::string key = arg.substr(1);
            std::string value = "";
            if ((i + 1) < argc && argv[i + 1][0] != '-') {
                value = argv[++i];
            }
            args[key] = value;
        }
    }
    return args;
}

int main(int argc, char* argv[]) {
    try {
        // Parse command-line arguments
        auto args = parse_args(argc, argv);

        // Extract options with defaults
        std::string table_path = args.count("path") ? args["path"] : "/tmp/write_ms_cpp.ms";
        casacore::uInt n_rows = args.count("rows") ? std::stoi(args["rows"]) : 100;
        std::string tsm_option_str = args.count("tsm_option") ? args["tsm_option"] : "DEFAULT";
        bool initialize = args.count("initialize") ? (args["initialize"] == "true") : false;
        std::string write_mode = args.count("write_mode") ? args["write_mode"] : "create_only";
        std::string data_shape_str = args.count("data_shape") ? args["data_shape"] : "32,4";

        // Parse data shape
        std::vector<casacore::uInt> data_shape;
        std::stringstream ss(data_shape_str);
        std::string item;
        while (std::getline(ss, item, ',')) {
            data_shape.push_back(std::stoi(item));
        }
        casacore::IPosition data_shape_ipos(2, data_shape[0], data_shape[1]);

        // Map TSM option
        casacore::TSMOption::Option opt = casacore::TSMOption::Default;
        if (tsm_option_str == "MMAP") opt = casacore::TSMOption::MMap;
        else if (tsm_option_str == "BUFFER") opt = casacore::TSMOption::Buffer;
        else if (tsm_option_str == "CACHE") opt = casacore::TSMOption::Cache;
        else if (tsm_option_str == "AIPSRC") opt = casacore::TSMOption::Aipsrc;
        else if (tsm_option_str == "DEFAULT") opt = casacore::TSMOption::Default;
        else {
            std::cerr << "Unknown TSM option: " << tsm_option_str << std::endl;
            return 1;
        }

        // print all the args
        std::cout << "table_path: " << table_path << std::endl;
        std::cout << "n_rows: " << n_rows << std::endl;
        std::cout << "tsm_option_str: " << tsm_option_str << std::endl;
        std::cout << "initialize: " << initialize << std::endl;
        std::cout << "write_mode: " << write_mode << std::endl;
        std::cout << "data_shape_str: " << data_shape_str << std::endl;

        casacore::TSMOption tsmOpt(opt);

        // Create table description
        casacore::TableDesc td("test", "1", casacore::TableDesc::Scratch);
        td.addColumn(casacore::ScalarColumnDesc<casacore::Double>("TIME", "Observation time"));
        td.addColumn(casacore::ScalarColumnDesc<casacore::Int>("ANTENNA1", "First antenna"));
        td.addColumn(casacore::ScalarColumnDesc<casacore::Int>("ANTENNA2", "Second antenna"));
        td.addColumn(casacore::ScalarColumnDesc<casacore::Bool>("FLAG_ROW", "Row flag"));
        td.addColumn(casacore::ArrayColumnDesc<casacore::Complex>("DATA", "Visibility data", data_shape_ipos, casacore::ColumnDesc::FixedShape));
        td.addColumn(casacore::ArrayColumnDesc<casacore::Bool>("FLAG", "Data flags", data_shape_ipos, casacore::ColumnDesc::FixedShape));

        // Create the table
        std::cout << "Creating SetupNewTable..." << std::endl;
        casacore::SetupNewTable setup(table_path, td, casacore::Table::New);
        std::cout << "Creating Table..." << std::endl;
        casacore::Table table(setup, casacore::Table::Plain, n_rows, initialize, casacore::Table::LocalEndian, tsmOpt);

        // Write data based on mode
        if (write_mode == "create_only") {
            // Do nothing
        } else if (write_mode == "table_put_cell") {
            casacore::ScalarColumn<casacore::Double> time_col(table, "TIME");
            casacore::ScalarColumn<casacore::Int> ant1_col(table, "ANTENNA1");
            casacore::ScalarColumn<casacore::Int> ant2_col(table, "ANTENNA2");
            casacore::ScalarColumn<casacore::Bool> flag_row_col(table, "FLAG_ROW");
            casacore::ArrayColumn<casacore::Complex> data_col(table, "DATA");
            casacore::ArrayColumn<casacore::Bool> flag_col(table, "FLAG");

            casacore::Matrix<casacore::Complex> data_matrix(data_shape_ipos);
            casacore::Matrix<casacore::Bool> flag_matrix(data_shape_ipos, false);
            for (casacore::uInt i = 0; i < data_shape_ipos[0]; ++i) {
                for (casacore::uInt j = 0; j < data_shape_ipos[1]; ++j) {
                    casacore::uInt idx = i * data_shape_ipos[1] + j;
                    data_matrix(i, j) = casacore::Complex(static_cast<float>(idx), 0.0f);
                    flag_matrix(i, j) = (idx % 13 == 0);
                }
            }

            for (casacore::uInt row_idx = 0; row_idx < n_rows; ++row_idx) {
                time_col.put(row_idx, static_cast<casacore::Double>(row_idx));
                ant1_col.put(row_idx, static_cast<casacore::Int>(row_idx % 128));
                ant2_col.put(row_idx, static_cast<casacore::Int>((row_idx + 1) % 128));
                flag_row_col.put(row_idx, (row_idx % 2 == 0));

                data_col.put(row_idx, data_matrix);
                flag_col.put(row_idx, flag_matrix);
            }
        } else {
            std::cerr << "Unknown write mode: " << write_mode << std::endl;
            return 1;
        }

        // Clean up
        // table.markForDelete();
        // std::filesystem::remove_all(std::filesystem::path(table_path));

        std::cout << "C++ syscall tracer with casacore completed successfully." << std::endl;

    } catch (const casacore::AipsError& e) {
        std::cerr << "CasaCore error: " << e.getMesg() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Standard error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
