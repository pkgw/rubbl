// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

#include <casacore/casa/BasicSL.h>
#include <casacore/tables/Tables.h>

#define CASA_TYPES_ALREADY_DECLARED
#define GlueString casacore::String
#define GlueTable casacore::Table

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
}
