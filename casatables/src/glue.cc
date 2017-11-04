// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

#include <casacore/casa/BasicSL.h>
#include <casacore/tables/Tables.h>

#define CASA_TYPES_ALREADY_DECLARED
#define GlueString casacore::String
#define GlueTable casacore::Table
#define GlueDataType casacore::DataType

#include "glue.h"

#include <string.h>

extern "C" {
    void
    handle_exception(ExcInfo &exc)
    {
        try {
            throw;
        } catch (const std::exception &e) {
            strncpy(exc.message, e.what(), sizeof(exc.message) - 1);
            exc.message[sizeof(exc.message) - 1] = '\0';
        } catch (...) {
            strcpy(exc.message, "unidentifiable C++ exception occurred");
        }
    }

    // Strings

    unsigned long
    string_check_size(void)
    {
        return sizeof(GlueString);
    }

    void
    string_init(GlueString &str, const void *data, const unsigned long n_bytes)
    {
        str.assign((const char *) data, n_bytes);
    }

    void
    string_deinit(GlueString &str)
    {
        // See https://stackoverflow.com/questions/10978864/free-memory-used-by-a-stdstring
        std::string().swap(str);
    }

    // Tables

    GlueTable *
    table_alloc_and_open(const GlueString &path, ExcInfo &exc)
    {
        try {
            return new GlueTable(path, GlueTable::Old, casacore::TSMOption());
        } catch (...) {
            handle_exception(exc);
            return NULL;
        }
    }

    void
    table_close_and_free(GlueTable *table, ExcInfo &exc)
    {
        try {
            delete table;
        } catch (...) {
            handle_exception(exc);
        }
    }

    unsigned long
    table_n_rows(const GlueTable &table)
    {
        return table.nrow();
    }

    int
    table_deep_copy_no_rows(const GlueTable &table, const GlueString &dest_path, ExcInfo &exc)
    {
        try {
            table.deepCopy(
                dest_path,
                GlueTable::NewNoReplace,
                casacore::True, // "valueCopy"
                GlueTable::LocalEndian,
                casacore::True // "noRows"
            );
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_get_column_info(const GlueTable &table, const GlueString &col_name,
                          unsigned long *n_rows, GlueDataType *data_type,
                          int *is_scalar, int *is_fixed_shape, unsigned int *n_dim,
                          unsigned long dims[8], ExcInfo &exc)
    {
        try {
            casa::TableColumn col(table, col_name);
            const casa::ColumnDesc &desc = col.columnDesc();
            const casa::IPosition &shape = desc.shape();

            if (shape.size() > 8)
                throw std::runtime_error("cannot handle columns with data of dimensionality greater than 8");

            *n_rows = table.nrow();
            *data_type = desc.dataType();
            *is_scalar = (int) desc.isScalar();
            *is_fixed_shape = (int) desc.isFixedShape();
            *n_dim = (unsigned int) desc.ndim();

            for (unsigned int i = 0; i < *n_dim; i++)
                dims[i] = (unsigned long) shape[i];
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }
}
