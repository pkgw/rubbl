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
        // Empirically, n_bytes = 0 can make us segfault without special
        // handling. std.assign() seems to barf with zero size inputs, but
        // other functions assume that the struct is reasonably initialized,
        // which isn't guaranteed to be the case for us. The code below
        // achieves the n_bytes = 0 outcome without crashing.
        if (n_bytes != 0)
            str.assign((const char *) data, n_bytes);
        else {
            str.assign(" ", 1);
            str = std::string();
        }
    }

    void
    string_get_buf(const GlueString &str, void const **data_ptr, unsigned long *n_bytes_ptr)
    {
        (*data_ptr) = str.data();
        (*n_bytes_ptr) = str.length();
    }

    void
    string_deinit(GlueString &str)
    {
        // See https://stackoverflow.com/questions/10978864/free-memory-used-by-a-stdstring
        std::string().swap(str);
    }

    // Data Types

    int
    data_type_get_element_size(const GlueDataType ty)
    {
        switch (ty) {
        case casa::TpBool: return sizeof(casa::Bool);
        case casa::TpChar: return sizeof(casa::Char);
        case casa::TpUChar: return sizeof(casa::uChar);
        case casa::TpShort: return sizeof(casa::Short);
        case casa::TpUShort: return sizeof(casa::uShort);
        case casa::TpInt: return sizeof(casa::Int);
        case casa::TpUInt: return sizeof(casa::uInt);
        case casa::TpFloat: return sizeof(float);
        case casa::TpDouble: return sizeof(double);
        case casa::TpComplex: return sizeof(casa::Complex);
        case casa::TpDComplex: return sizeof(casa::DComplex);
        case casa::TpString: return -1;
        case casa::TpTable: return -1;
        case casa::TpArrayBool: return sizeof(casa::Bool);
        case casa::TpArrayChar: return sizeof(casa::Char);
        case casa::TpArrayUChar: return sizeof(casa::uChar);
        case casa::TpArrayShort: return sizeof(casa::Short);
        case casa::TpArrayUShort: return sizeof(casa::uShort);
        case casa::TpArrayInt: return sizeof(casa::Int);
        case casa::TpArrayUInt: return sizeof(casa::uInt);
        case casa::TpArrayFloat: return sizeof(float);
        case casa::TpArrayDouble: return sizeof(double);
        case casa::TpArrayComplex: return sizeof(casa::Complex);
        case casa::TpArrayDComplex: return sizeof(casa::DComplex);
        case casa::TpArrayString: return -1;
        case casa::TpRecord: return -1;
        case casa::TpOther: return -1;
        case casa::TpQuantity: return -1;
        case casa::TpArrayQuantity: return -1;
        case casa::TpInt64: return sizeof(casa::Int64);
        case casa::TpArrayInt64: return sizeof(casa::Int64);
        case casa::TpNumberOfTypes: return -1; // shut up compiler warning
        }

        return -1;
    }

    // Tables

    GlueTable *
    table_alloc_and_open(const GlueString &path, const TableOpenMode mode, ExcInfo &exc)
    {
        GlueTable::TableOption option = GlueTable::Old;

        if (mode == TOM_OPEN_RW)
            option = GlueTable::Update;
        else if (mode == TOM_CREATE)
            option = GlueTable::NewNoReplace;

        try {
            return new GlueTable(path, option, casacore::TSMOption());
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
        // I *think* we can safely say that this code should never trigger an exception.
        return table.nrow();
    }

    unsigned long
    table_n_columns(const GlueTable &table)
    {
        // I *think* we can safely say that this code should never trigger an exception.
        return table.actualTableDesc().columnDescSet().ncolumn();
    }

    // We assume the caller has allocated col_names of sufficient size.
    int
    table_get_column_names(const GlueTable &table, GlueString *col_names, ExcInfo &exc)
    {
        try {
            casa::Vector<casa::String> cnames = table.actualTableDesc().columnNames();

            for (size_t i = 0; i < cnames.size(); i++)
                col_names[i] = cnames[i];
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
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
                          int *is_scalar, int *is_fixed_shape, int *n_dim,
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
            *n_dim = (int) desc.ndim();

            for (int i = 0; i < *n_dim; i++) // note: for empty cols, n_dim = -1; this is OK
                dims[i] = (unsigned long) shape[i];
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    // This function assumes that the caller has already vetted the types and
    // has figured how big `data` needs to be.
    int
    table_get_scalar_column_data(const GlueTable &table, const GlueString &col_name,
                                 void *data, ExcInfo &exc)
    {
        try {
            const casa::ColumnDesc &desc = casa::TableColumn(table, col_name).columnDesc();
            casa::IPosition shape(1, table.nrow());

            switch (desc.dataType()) {

#define CASE(DTYPE, CPPTYPE) \
            case GlueDataType::DTYPE: { \
                casa::ScalarColumn<CPPTYPE> col(table, col_name); \
                casa::Vector<CPPTYPE> vec(shape, (CPPTYPE *) data, casa::StorageInitPolicy::SHARE); \
                col.getColumn(vec); \
                break; \
            }

            CASE(TpBool, casa::Bool)
            CASE(TpChar, casa::Char)
            CASE(TpUChar, casa::uChar)
            CASE(TpShort, casa::Short)
            CASE(TpUShort, casa::uShort)
            CASE(TpInt, casa::Int)
            CASE(TpUInt, casa::uInt)
            CASE(TpFloat, float)
            CASE(TpDouble, double)
            CASE(TpComplex, casa::Complex)
            CASE(TpDComplex, casa::DComplex)
            CASE(TpString, casa::String)

#undef CASE

            default:
                throw std::runtime_error("unhandled scalar column data type");
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_get_cell_info(const GlueTable &table, const GlueString &col_name,
                        unsigned long row_number, GlueDataType *data_type,
                        int *n_dim, unsigned long dims[8], ExcInfo &exc)
    {
        try {
            casa::TableColumn col(table, col_name);
            const casa::ColumnDesc &desc = col.columnDesc();

            *data_type = desc.dataType();

            if (desc.isScalar())
                *n_dim = 0;
            else {
                *n_dim = (int) col.ndim(row_number);

                if (*n_dim > 8)
                    throw std::runtime_error("cannot handle cells with data of dimensionality greater than 8");

                const casa::IPosition shape = col.shape(row_number);

                for (int i = 0; i < *n_dim; i++)
                    dims[i] = (unsigned long) shape[i];
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    // This function assumes that the caller has already vetted the types and
    // has figured how big `data` needs to be.
    int
    table_get_cell(const GlueTable &table, const GlueString &col_name,
                   const unsigned long row_number, void *data, ExcInfo &exc)
    {
        try {
            casa::TableColumn col(table, col_name);
            const casa::ColumnDesc &desc = col.columnDesc();
            casa::IPosition shape;

            if (!desc.isScalar())
                shape = col.shape(row_number);

            switch (desc.trueDataType()) {

#define SCALAR_CASE(DTYPE, CPPTYPE) \
            case GlueDataType::DTYPE: { \
                casa::ScalarColumn<CPPTYPE> col(table, col_name); \
                *((CPPTYPE *) data) = col.get(row_number); \
                break; \
            }

#define VECTOR_CASE(DTYPE, CPPTYPE) \
            case GlueDataType::DTYPE: { \
                casa::ArrayColumn<CPPTYPE> col(table, col_name); \
                casa::Array<CPPTYPE> array(shape, (CPPTYPE *) data, casa::StorageInitPolicy::SHARE); \
                col.get(row_number, array, casa::False); \
                break; \
            }

            SCALAR_CASE(TpBool, casa::Bool)
            SCALAR_CASE(TpChar, casa::Char)
            SCALAR_CASE(TpUChar, casa::uChar)
            SCALAR_CASE(TpShort, casa::Short)
            SCALAR_CASE(TpUShort, casa::uShort)
            SCALAR_CASE(TpInt, casa::Int)
            SCALAR_CASE(TpUInt, casa::uInt)
            SCALAR_CASE(TpFloat, float)
            SCALAR_CASE(TpDouble, double)
            SCALAR_CASE(TpComplex, casa::Complex)
            SCALAR_CASE(TpDComplex, casa::DComplex)
            SCALAR_CASE(TpString, casa::String)

            VECTOR_CASE(TpArrayBool, casa::Bool)
            VECTOR_CASE(TpArrayChar, casa::Char)
            VECTOR_CASE(TpArrayUChar, casa::uChar)
            VECTOR_CASE(TpArrayShort, casa::Short)
            VECTOR_CASE(TpArrayUShort, casa::uShort)
            VECTOR_CASE(TpArrayInt, casa::Int)
            VECTOR_CASE(TpArrayUInt, casa::uInt)
            VECTOR_CASE(TpArrayFloat, float)
            VECTOR_CASE(TpArrayDouble, double)
            VECTOR_CASE(TpArrayComplex, casa::Complex)
            VECTOR_CASE(TpArrayDComplex, casa::DComplex)
            VECTOR_CASE(TpArrayString, casa::String)

#undef SCALAR_CASE
#undef VECTOR_CASE

            default:
                throw std::runtime_error("unhandled cell data type");
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }
}
