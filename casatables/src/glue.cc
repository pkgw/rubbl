// Copyright 2017-2020 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

// Note that in CASA, an array shape like (4, 64) means that the most
// rapidly-varying axis is 4, i.e. Fortran array ordering. Rust's ndarray uses
// C ordering instead. So we must take care to reverse array shapes when
// translating from C++-land to Rust-land.

#include <stdexcept>
#include <casacore/casa/BasicSL.h>
#include <casacore/tables/Tables.h>

#define CASA_TYPES_ALREADY_DECLARED
#define GlueTable casacore::Table
#define GlueTableDesc casacore::TableDesc
#define GlueTableRow casacore::ROTableRow
#define GlueDataType casacore::DataType
#define GlueUInt casacore::uInt

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

    // StringBridge

    casacore::String
    bridge_string(const StringBridge &input)
    {
        casacore::String result((const char *) input.data, input.n_bytes);
        return result;
    }

    // To pass strings from C++ to Rust, we *always* have to copy the data. The
    // only time that could safely avoid copying would be if we were absolutely
    // sure that the string buffer pointed into a data structure whose lifetime
    // was longer than that of the calling Rust code ... but even then, we would
    // need to have Rust-side code to check that the C++ string data are valid
    // UTF-8, so that there's always a potential need to allocate anyway. The
    // only efficient way that I can come up with that will work 100% reliably
    // is to pass a Rust callback into the C++ code, so that the Rust code can
    // do its memory allocation inside a stack frame where we are *sure* that
    // the C++ pointer is still valid. So, that's what this function enforces.
    void
    unbridge_string(const casacore::String &input, StringBridgeCallback callback, void *ctxt)
    {
        StringBridge bridge;
        bridge.data = input.data();
        bridge.n_bytes = input.length();
        callback(&bridge, ctxt);
    }

    casacore::Array<casacore::String>
    bridge_string_array(const StringBridge *source, const casacore::IPosition &shape)
    {
        casacore::Array<casacore::String> array(shape);
        unsigned int n = 0;
        casacore::Array<casacore::String>::iterator end = array.end();

        for (casacore::Array<casacore::String>::iterator i = array.begin(); i != end; i++, n++)
            *i = bridge_string(source[n]);

        return array;
    }

    void
    unbridge_string_array(const casacore::Array<casacore::String> &input,
                          StringBridgeCallback callback, void *ctxt)
    {
        StringBridge bridge;
        casacore::Array<casacore::String>::const_iterator end = input.end();

        for (casacore::Array<casacore::String>::const_iterator i = input.begin(); i != end; i++) {
            bridge.data = (*i).data();
            bridge.n_bytes = (*i).length();
            callback(&bridge, ctxt);
        }
    }

    // Data Types

    int
    data_type_get_element_size(const GlueDataType ty)
    {
        switch (ty) {
        case casacore::TpBool: return sizeof(casacore::Bool);
        case casacore::TpChar: return sizeof(casacore::Char);
        case casacore::TpUChar: return sizeof(casacore::uChar);
        case casacore::TpShort: return sizeof(casacore::Short);
        case casacore::TpUShort: return sizeof(casacore::uShort);
        case casacore::TpInt: return sizeof(casacore::Int);
        case casacore::TpUInt: return sizeof(casacore::uInt);
        case casacore::TpFloat: return sizeof(float);
        case casacore::TpDouble: return sizeof(double);
        case casacore::TpComplex: return sizeof(casacore::Complex);
        case casacore::TpDComplex: return sizeof(casacore::DComplex);
        case casacore::TpString: return -1;
        case casacore::TpTable: return -1;
        case casacore::TpArrayBool: return sizeof(casacore::Bool);
        case casacore::TpArrayChar: return sizeof(casacore::Char);
        case casacore::TpArrayUChar: return sizeof(casacore::uChar);
        case casacore::TpArrayShort: return sizeof(casacore::Short);
        case casacore::TpArrayUShort: return sizeof(casacore::uShort);
        case casacore::TpArrayInt: return sizeof(casacore::Int);
        case casacore::TpArrayUInt: return sizeof(casacore::uInt);
        case casacore::TpArrayFloat: return sizeof(float);
        case casacore::TpArrayDouble: return sizeof(double);
        case casacore::TpArrayComplex: return sizeof(casacore::Complex);
        case casacore::TpArrayDComplex: return sizeof(casacore::DComplex);
        case casacore::TpArrayString: return -1;
        case casacore::TpRecord: return -1;
        case casacore::TpOther: return -1;
        case casacore::TpQuantity: return -1;
        case casacore::TpArrayQuantity: return -1;
        case casacore::TpInt64: return sizeof(casacore::Int64);
        case casacore::TpArrayInt64: return sizeof(casacore::Int64);
        case casacore::TpNumberOfTypes: return -1; // shut up compiler warning
        }

        return -1;
    }

    // TableDesc

    GlueTableDesc *
    tabledesc_create(
        const StringBridge &type
    )
    {
        // TODO: expose this?
        GlueTableDesc::TDOption td_option = GlueTableDesc::TDOption::New;
        return new GlueTableDesc(bridge_string(type), td_option);
    }

    GlueTableDesc *
    tabledesc_add_scalar_column(
        GlueTableDesc &table_desc,
        GlueDataType data_type,
        const StringBridge &col_name,
        const StringBridge &comment,
        // see casacore::ColumnDesc::Direct
        bool direct,
        // undefined values are possible, see casacore::ColumnDesc::Direct
        bool undefined,
        ExcInfo &exc
    )
    {
        // scalar columns are never fixed.
        int opt = 0;
        if (direct) {
            opt |= casacore::ColumnDesc::Direct;
        }
        if (undefined) {
            opt |= casacore::ColumnDesc::Undefined;
        }

        try {
            switch (data_type) {

#define CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                table_desc.addColumn(casacore::ScalarColumnDesc<CPPTYPE>( \
                    bridge_string(col_name), \
                    bridge_string(comment), \
                    opt \
                )); \
                break; \
            }

            CASE(TpBool, casacore::Bool)
            CASE(TpChar, casacore::Char)
            CASE(TpUChar, casacore::uChar)
            CASE(TpShort, casacore::Short)
            CASE(TpUShort, casacore::uShort)
            CASE(TpInt, casacore::Int)
            CASE(TpUInt, casacore::uInt)
            CASE(TpFloat, float)
            CASE(TpDouble, double)
            CASE(TpComplex, casacore::Complex)
            CASE(TpDComplex, casacore::DComplex)
            CASE(TpString, casacore::String)
#undef CASE

            default:
                throw std::runtime_error("unhandled scalar column data type");
            }
        } catch (...) {
            handle_exception(exc);
        }

        return &table_desc;
    }

    GlueTableDesc *
    tabledesc_add_array_column(
        GlueTableDesc &table_desc,
        GlueDataType data_type,
        const StringBridge &col_name,
        const StringBridge &comment,
        // see casacore::ColumnDesc::Direct
        bool direct,
        // undefined values are possible, see casacore::ColumnDesc::Direct
        bool undefined,
        ExcInfo &exc
    )
    {
        int opt = 0;
        if (direct) {
            opt |= casacore::ColumnDesc::Direct;
        }
        if (undefined) {
            opt |= casacore::ColumnDesc::Undefined;
        }

        try {
            switch (data_type) {

#define CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                table_desc.addColumn(casacore::ArrayColumnDesc<CPPTYPE>( \
                    bridge_string(col_name), \
                    bridge_string(comment), \
                    -1, \
                    opt \
                )); \
                break; \
            }

            CASE(TpBool, casacore::Bool)
            CASE(TpChar, casacore::Char)
            CASE(TpUChar, casacore::uChar)
            CASE(TpShort, casacore::Short)
            CASE(TpUShort, casacore::uShort)
            CASE(TpInt, casacore::Int)
            CASE(TpUInt, casacore::uInt)
            CASE(TpFloat, float)
            CASE(TpDouble, double)
            CASE(TpComplex, casacore::Complex)
            CASE(TpDComplex, casacore::DComplex)
            CASE(TpString, casacore::String)
#undef CASE

            default:
                throw std::runtime_error("unhandled array column data type");
            }
        } catch (...) {
            handle_exception(exc);
        }

        return &table_desc;
    }

    GlueTableDesc *
    tabledesc_add_fixed_array_column(
        GlueTableDesc &table_desc,
        GlueDataType data_type,
        const StringBridge &col_name,
        const StringBridge &comment,
        // number of dimensions
        const unsigned long n_dims,
        // dimensions array
        const unsigned long *dims,
        // see casacore::ColumnDesc::Direct
        bool direct,
        // undefined values are possible, see casacore::ColumnDesc::Direct
        bool undefined,
        ExcInfo &exc
    )
    {
        int opt = casacore::ColumnDesc::FixedShape;
        if (direct) {
            opt |= casacore::ColumnDesc::Direct;
        }
        if (undefined) {
            opt |= casacore::ColumnDesc::Undefined;
        }

        casacore::IPosition shape(n_dims); \
        for (casacore::uInt i = 0; i < n_dims; i++) \
            shape[i] = dims[n_dims - 1 - i]; \

        try {
            switch (data_type) {

#define CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                table_desc.addColumn(casacore::ArrayColumnDesc<CPPTYPE>( \
                    bridge_string(col_name), \
                    bridge_string(comment), \
                    shape, \
                    opt \
                )); \
                break; \
            }

            CASE(TpBool, casacore::Bool)
            CASE(TpChar, casacore::Char)
            CASE(TpUChar, casacore::uChar)
            CASE(TpShort, casacore::Short)
            CASE(TpUShort, casacore::uShort)
            CASE(TpInt, casacore::Int)
            CASE(TpUInt, casacore::uInt)
            CASE(TpFloat, float)
            CASE(TpDouble, double)
            CASE(TpComplex, casacore::Complex)
            CASE(TpDComplex, casacore::DComplex)
            CASE(TpString, casacore::String)
#undef CASE

            default:
                throw std::runtime_error("unhandled scalar column data type");
            }
        } catch (...) {
            handle_exception(exc);
        }

        return &table_desc;
    }

    // Tables

    GlueTable *
    table_create(
        const StringBridge &path, 
        // Description of columns and keys in the table
        GlueTableDesc &table_desc,
        // number of rows
        unsigned long n_rows,
        const TableCreateMode mode,
        ExcInfo &exc
    )
    {
        // TODO: expose this as an argument?
        // I think the only options here are New, NewNoReplace and Scratch, but 
        // not sure whether it's worth extending the TableOpenMode enum or 
        // doing something else. Open to suggestions
        GlueTable::TableOption table_option = GlueTable::TableOption::New;
        if (mode == TCM_NEW_NO_REPLACE) {
            table_option = GlueTable::TableOption::NewNoReplace;
        }

        // TOOD: expose this as an argument?
        // the enum is either either `Plain` or `Memory`
        GlueTable::TableType type = GlueTable::TableType::Plain;

        // TODO: expose this as an argument?
        // const casacore::TSMOption tsmOption();

        // TODO: expose this as an argument?
        casacore::Bool initialize = true;

        // always use the local endianness
        GlueTable::EndianFormat endian_format = GlueTable::EndianFormat::LocalEndian;

        try {
            // create a an object containing some information about the table we're creating
            casacore::SetupNewTable newTable(
                bridge_string(path),
                table_desc,
                table_option
            );
            return new GlueTable(newTable, type, n_rows, initialize, endian_format, casacore::TSMOption());
        } catch (...) {
            handle_exception(exc);
            return NULL;
        }
    }

    GlueTable *
    table_alloc_and_open(const StringBridge &path, const TableOpenMode mode, ExcInfo &exc)
    {
        GlueTable::TableOption option = GlueTable::Old;

        if (mode == TOM_OPEN_RW)
            option = GlueTable::Update;
        else if (mode == TOM_CREATE)
            option = GlueTable::NewNoReplace;

        try {
            return new GlueTable(bridge_string(path), option, casacore::TSMOption());
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

    int
    table_get_column_names(const GlueTable &table, StringBridgeCallback callback,
                           void *ctxt, ExcInfo &exc)
    {
        try {
            unbridge_string_array(table.actualTableDesc().columnNames(), callback, ctxt);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_remove_column(GlueTable &table, const StringBridge &col_name, ExcInfo &exc)
    {
        try {
            table.removeColumn(bridge_string(col_name));
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    unsigned long
    table_n_keywords(const GlueTable &table)
    {
        return table.keywordSet().nfields();
    }

    int
    table_get_keyword_info(const GlueTable &table, KeywordInfoCallback callback, void *ctxt, ExcInfo &exc)
    {
        try {
            StringBridge name;
            const casacore::TableRecord &rec = table.keywordSet();
            casacore::uInt n_kws = rec.nfields();

            for (casacore::uInt i = 0; i < n_kws; i++) {
                // Note: must preserve string variable as a local until after
                // the callback is called; otherwise it can be deleted before
                // we copy its data.
                const casacore::String n = rec.name(i);
                name.data = n.data();
                name.n_bytes = n.length();
                callback(&name, rec.type(i), ctxt);
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_copy_rows(const GlueTable &source, GlueTable &dest, ExcInfo &exc)
    {
        try {
            casacore::TableCopy::copyRows(dest, source);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_deep_copy_no_rows(const GlueTable &table, const StringBridge &dest_path, ExcInfo &exc)
    {
        try {
            table.deepCopy(
                bridge_string(dest_path),
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
    table_get_column_info(const GlueTable &table, const StringBridge &col_name,
                          unsigned long *n_rows, GlueDataType *data_type,
                          int *is_scalar, int *is_fixed_shape, int *n_dim,
                          unsigned long dims[8], ExcInfo &exc)
    {
        try {
            casacore::TableColumn col(table, bridge_string(col_name));
            const casacore::ColumnDesc &desc = col.columnDesc();
            const casacore::IPosition &shape = desc.shape();

            if (shape.size() > 8)
                throw std::runtime_error("cannot handle columns with data of dimensionality greater than 8");

            *n_rows = table.nrow();
            *data_type = desc.dataType();
            *is_scalar = (int) desc.isScalar();
            *is_fixed_shape = (int) desc.isFixedShape();
            *n_dim = (int) desc.ndim();

            for (int i = 0; i < *n_dim; i++) // note: for empty cols, n_dim = -1; this is OK
                dims[*n_dim - 1 - i] = (unsigned long) shape[i];
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    // This function assumes that the caller has already vetted the types and
    // has figured how big `data` needs to be.
    int
    table_get_scalar_column_data(const GlueTable &table, const StringBridge &col_name,
                                 void *data, ExcInfo &exc)
    {
        try {
            const casacore::ColumnDesc &desc = casacore::TableColumn(table, bridge_string(col_name)).columnDesc();
            casacore::IPosition shape(1, table.nrow());

            switch (desc.dataType()) {

#define CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                casacore::ScalarColumn<CPPTYPE> col(table, bridge_string(col_name)); \
                casacore::Vector<CPPTYPE> vec(shape, (CPPTYPE *) data, casacore::SHARE); \
                col.getColumn(vec); \
                break; \
            }

            CASE(TpBool, casacore::Bool)
            CASE(TpChar, casacore::Char)
            CASE(TpUChar, casacore::uChar)
            CASE(TpShort, casacore::Short)
            CASE(TpUShort, casacore::uShort)
            CASE(TpInt, casacore::Int)
            CASE(TpUInt, casacore::uInt)
            CASE(TpFloat, float)
            CASE(TpDouble, double)
            CASE(TpComplex, casacore::Complex)
            CASE(TpDComplex, casacore::DComplex)

#undef CASE

            case casacore::TpString:
                throw std::runtime_error("use table_get_scalar_column_data_string for TpString columns");

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
    table_get_scalar_column_data_string(const GlueTable &table, const StringBridge &col_name,
                                        StringBridgeCallback callback, void *ctxt, ExcInfo &exc)
    {
        try {
            casacore::ScalarColumn<casacore::String> col(table, bridge_string(col_name));
            casacore::IPosition shape(1, table.nrow());
            casacore::Vector<casacore::String> vec(shape);

            col.getColumn(vec);
            unbridge_string_array(vec, callback, ctxt);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_get_cell_info(const GlueTable &table, const StringBridge &col_name,
                        unsigned long row_number, GlueDataType *data_type,
                        int *n_dim, unsigned long dims[8], ExcInfo &exc)
    {
        try {
            casacore::TableColumn col(table, bridge_string(col_name));
            const casacore::ColumnDesc &desc = col.columnDesc();

            *data_type = desc.dataType();

            if (desc.isScalar())
                *n_dim = 0;
            else {
                *n_dim = (int) col.ndim(row_number);

                if (*n_dim > 8)
                    throw std::runtime_error("cannot handle cells with data of dimensionality greater than 8");

                const casacore::IPosition shape = col.shape(row_number);

                for (int i = 0; i < *n_dim; i++)
                    dims[*n_dim - 1 - i] = (unsigned long) shape[i];
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
    table_get_cell(const GlueTable &table, const StringBridge &col_name,
                   const unsigned long row_number, void *data, ExcInfo &exc)
    {
        try {
            casacore::TableColumn col(table, bridge_string(col_name));
            const casacore::ColumnDesc &desc = col.columnDesc();
            casacore::IPosition shape;

            if (!desc.isScalar())
                shape = col.shape(row_number);

            switch (desc.trueDataType()) {

#define SCALAR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                casacore::ScalarColumn<CPPTYPE> col(table, bridge_string(col_name)); \
                *((CPPTYPE *) data) = col.get(row_number); \
                break; \
            }

#define VECTOR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                casacore::ArrayColumn<CPPTYPE> col(table, bridge_string(col_name)); \
                casacore::Array<CPPTYPE> array(shape, (CPPTYPE *) data, casacore::SHARE); \
                col.get(row_number, array, casacore::False); \
                break; \
            }

            SCALAR_CASE(TpBool, casacore::Bool)
            SCALAR_CASE(TpChar, casacore::Char)
            SCALAR_CASE(TpUChar, casacore::uChar)
            SCALAR_CASE(TpShort, casacore::Short)
            SCALAR_CASE(TpUShort, casacore::uShort)
            SCALAR_CASE(TpInt, casacore::Int)
            SCALAR_CASE(TpUInt, casacore::uInt)
            SCALAR_CASE(TpFloat, float)
            SCALAR_CASE(TpDouble, double)
            SCALAR_CASE(TpComplex, casacore::Complex)
            SCALAR_CASE(TpDComplex, casacore::DComplex)

            VECTOR_CASE(TpArrayBool, casacore::Bool)
            VECTOR_CASE(TpArrayChar, casacore::Char)
            VECTOR_CASE(TpArrayUChar, casacore::uChar)
            VECTOR_CASE(TpArrayShort, casacore::Short)
            VECTOR_CASE(TpArrayUShort, casacore::uShort)
            VECTOR_CASE(TpArrayInt, casacore::Int)
            VECTOR_CASE(TpArrayUInt, casacore::uInt)
            VECTOR_CASE(TpArrayFloat, float)
            VECTOR_CASE(TpArrayDouble, double)
            VECTOR_CASE(TpArrayComplex, casacore::Complex)
            VECTOR_CASE(TpArrayDComplex, casacore::DComplex)

#undef SCALAR_CASE
#undef VECTOR_CASE

            case casacore::TpString:
                throw std::runtime_error("you must use table_get_cell_string() for string cells");

            case casacore::TpArrayString:
                throw std::runtime_error("you must use table_get_cell_string_array() for string-array cells");

            default:
                throw std::runtime_error("unhandled cell data type");
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_get_cell_string(const GlueTable &table, const StringBridge &col_name,
                          const unsigned long row_number, StringBridgeCallback callback,
                          void *ctxt, ExcInfo &exc)
    {
        try {
            casacore::ScalarColumn<casacore::String> col(table, bridge_string(col_name));
            unbridge_string(col.get(row_number), callback, ctxt);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_get_cell_string_array(const GlueTable &table, const StringBridge &col_name,
                                const unsigned long row_number, StringBridgeCallback callback,
                                void *ctxt, ExcInfo &exc)
    {
        try {
            casacore::ArrayColumn<casacore::String> col(table, bridge_string(col_name));
            casacore::IPosition shape = col.shape(row_number);
            casacore::Array<casacore::String> array(shape);
            col.get(row_number, array, casacore::False);
            unbridge_string_array(array, callback, ctxt);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_put_cell(GlueTable &table, const StringBridge &col_name,
                   const unsigned long row_number, const GlueDataType data_type,
                   const unsigned long n_dims, const unsigned long *dims,
                   void *data, ExcInfo &exc)
    {
        try {
            switch (data_type) {

#define SCALAR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                casacore::ScalarColumn<CPPTYPE> col(table, bridge_string(col_name)); \
                col.put(row_number, *(CPPTYPE *) data); \
                break; \
            }

#define VECTOR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                casacore::ArrayColumn<CPPTYPE> col(table, bridge_string(col_name)); \
                casacore::IPosition shape(n_dims); \
                for (casacore::uInt i = 0; i < n_dims; i++) \
                    shape[i] = dims[n_dims - 1 - i]; \
                casacore::Array<CPPTYPE> array(shape, (CPPTYPE *) data, casacore::SHARE); \
                col.put(row_number, array); \
                break; \
            }

            SCALAR_CASE(TpBool, casacore::Bool)
            SCALAR_CASE(TpChar, casacore::Char)
            SCALAR_CASE(TpUChar, casacore::uChar)
            SCALAR_CASE(TpShort, casacore::Short)
            SCALAR_CASE(TpUShort, casacore::uShort)
            SCALAR_CASE(TpInt, casacore::Int)
            SCALAR_CASE(TpUInt, casacore::uInt)
            SCALAR_CASE(TpFloat, float)
            SCALAR_CASE(TpDouble, double)
            SCALAR_CASE(TpComplex, casacore::Complex)
            SCALAR_CASE(TpDComplex, casacore::DComplex)

            VECTOR_CASE(TpArrayBool, casacore::Bool)
            VECTOR_CASE(TpArrayChar, casacore::Char)
            VECTOR_CASE(TpArrayUChar, casacore::uChar)
            VECTOR_CASE(TpArrayShort, casacore::Short)
            VECTOR_CASE(TpArrayUShort, casacore::uShort)
            VECTOR_CASE(TpArrayInt, casacore::Int)
            VECTOR_CASE(TpArrayUInt, casacore::uInt)
            VECTOR_CASE(TpArrayFloat, float)
            VECTOR_CASE(TpArrayDouble, double)
            VECTOR_CASE(TpArrayComplex, casacore::Complex)
            VECTOR_CASE(TpArrayDComplex, casacore::DComplex)

#undef SCALAR_CASE
#undef VECTOR_CASE

            case casacore::TpString: {
                casacore::ScalarColumn<casacore::String> col(table, bridge_string(col_name));
                col.put(row_number, bridge_string(*((StringBridge *) data)));
                break;
            }

            case casacore::TpArrayString: {
                casacore::ArrayColumn<casacore::String> col(table, bridge_string(col_name));
                casacore::IPosition shape(n_dims);
                for (casacore::uInt i = 0; i < n_dims; i++)
                    shape[i] = dims[n_dims - 1 - i];
                col.put(row_number, bridge_string_array((const StringBridge *) data, shape));
                break;
            }

            default:
                throw std::runtime_error("unhandled cell data type");
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_add_rows(GlueTable &table, const unsigned long n_rows, ExcInfo &exc)
    {
        try {
            table.addRow(n_rows);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    // Rows

    GlueTableRow *
    table_row_alloc(const GlueTable &table, const unsigned char is_read_only, ExcInfo &exc)
    {
        try {
            if (is_read_only)
                return new casacore::ROTableRow(table);
            else
                return new casacore::TableRow(table);
        } catch (...) {
            handle_exception(exc);
            return NULL;
        }
    }

    int
    table_row_free(GlueTableRow *row, ExcInfo &exc)
    {
        try {
            delete row;
            return 0;
        } catch (...) {
            handle_exception(exc);
            return 1;
        }
    }

    int
    table_row_read(GlueTableRow &row, const unsigned long row_number, ExcInfo &exc)
    {
        try {
            row.get(row_number);
            return 0;
        } catch (...) {
            handle_exception(exc);
            return 1;
        }
    }

    int
    table_row_copy_and_put(GlueTableRow &src_row, const unsigned long dest_row_number,
                           GlueTableRow &wrap_dest_row, ExcInfo &exc)
    {
        casacore::TableRow &dest_row = (casacore::TableRow &) wrap_dest_row;

        try {
            dest_row.put(dest_row_number, src_row.record(), src_row.getDefined());
            return 0;
        } catch (...) {
            handle_exception(exc);
            return 1;
        }
    }

    int
    table_row_get_cell_info(const GlueTableRow &row, const StringBridge &col_name,
                            GlueDataType *data_type, int *n_dim,
                            unsigned long dims[8], ExcInfo &exc)
    {
        try {
            const casacore::TableRecord &rec = row.record();
            const casacore::RecordDesc &desc = rec.description();
            casacore::Int field_num = rec.fieldNumber(bridge_string(col_name));

            if (field_num < 0)
                throw std::runtime_error("unrecognized column name");

            *data_type = rec.type(field_num);

            if (desc.isScalar(field_num))
                *n_dim = 0;
            else {
                // desc.shape() is generic, not specific to the row we're
                // looking at, so we have to create a TableColumn to get the
                // cell's shape.
                casacore::TableColumn col(row.table(), bridge_string(col_name));
                *n_dim = (int) col.ndim(row.rowNumber());

                if (*n_dim > 8)
                    throw std::runtime_error("cannot handle cells with data of dimensionality greater than 8");

                const casacore::IPosition shape = col.shape(row.rowNumber());

                for (int i = 0; i < *n_dim; i++)
                    dims[*n_dim - 1 - i] = (unsigned long) shape[i];
            }

            return 0;
        } catch (...) {
            handle_exception(exc);
            return 1;
        }
    }

    // This function assumes that the caller has already vetted the types and
    // has figured how big `data` needs to be.
    int
    table_row_get_cell(const GlueTableRow &row, const StringBridge &col_name,
                       void *data, ExcInfo &exc)
    {
        try {
            const casacore::TableRecord &rec = row.record();
            const casacore::RecordDesc &desc = rec.description();
            casacore::Int field_num = rec.fieldNumber(bridge_string(col_name));
            casacore::IPosition shape;

            if (field_num < 0)
                throw std::runtime_error("unrecognized column name");

            if (!desc.isScalar(field_num)) {
                casacore::TableColumn col(row.table(), bridge_string(col_name));
                shape = col.shape(row.rowNumber());
            }

            switch (rec.type(field_num)) {

#define SCALAR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                CPPTYPE datum; \
                rec.get(field_num, datum); \
                *((CPPTYPE *) data) = datum; \
                break; \
            }

#define VECTOR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                casacore::Array<CPPTYPE> array(shape, (CPPTYPE *) data, casacore::SHARE); \
                rec.get(field_num, array); \
                break; \
            }

            SCALAR_CASE(TpBool, casacore::Bool)
            //SCALAR_CASE(TpChar, casacore::Char)
            SCALAR_CASE(TpUChar, casacore::uChar)
            SCALAR_CASE(TpShort, casacore::Short)
            //SCALAR_CASE(TpUShort, casacore::uShort)
            SCALAR_CASE(TpInt, casacore::Int)
            SCALAR_CASE(TpUInt, casacore::uInt)
            SCALAR_CASE(TpFloat, float)
            SCALAR_CASE(TpDouble, double)
            SCALAR_CASE(TpComplex, casacore::Complex)
            SCALAR_CASE(TpDComplex, casacore::DComplex)

            VECTOR_CASE(TpArrayBool, casacore::Bool)
            //VECTOR_CASE(TpArrayChar, casacore::Char)
            VECTOR_CASE(TpArrayUChar, casacore::uChar)
            VECTOR_CASE(TpArrayShort, casacore::Short)
            //VECTOR_CASE(TpArrayUShort, casacore::uShort)
            VECTOR_CASE(TpArrayInt, casacore::Int)
            VECTOR_CASE(TpArrayUInt, casacore::uInt)
            VECTOR_CASE(TpArrayFloat, float)
            VECTOR_CASE(TpArrayDouble, double)
            VECTOR_CASE(TpArrayComplex, casacore::Complex)
            VECTOR_CASE(TpArrayDComplex, casacore::DComplex)

#undef SCALAR_CASE
#undef VECTOR_CASE

            case casacore::TpString:
                throw std::runtime_error("you must use table_get_cell_string() for string cells");

            case casacore::TpArrayString:
                throw std::runtime_error("you must use table_get_cell_string_array() for string-array cells");

            default:
                throw std::runtime_error("unhandled cell data type");
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_row_get_cell_string(const GlueTableRow &row, const StringBridge &col_name,
                              StringBridgeCallback callback, void *ctxt,
                              ExcInfo &exc)
    {
        try {
            const casacore::TableRecord &rec = row.record();
            casacore::Int field_num = rec.fieldNumber(bridge_string(col_name));

            if (field_num < 0)
                throw std::runtime_error("unrecognized column name");

            if (rec.type(field_num) != casacore::TpString)
                throw std::runtime_error("row cell must be of TpString type");

            casacore::String datum;
            rec.get(field_num, datum);
            unbridge_string(datum, callback, ctxt);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_row_get_cell_string_array(const GlueTableRow &row, const StringBridge &col_name,
                                    StringBridgeCallback callback, void *ctxt,
                                    ExcInfo &exc)
    {
        try {
            const casacore::TableRecord &rec = row.record();
            casacore::Int field_num = rec.fieldNumber(bridge_string(col_name));

            if (field_num < 0)
                throw std::runtime_error("unrecognized column name");

            casacore::TableColumn col(row.table(), bridge_string(col_name));
            casacore::IPosition shape = col.shape(row.rowNumber());

            if (rec.type(field_num) != casacore::TpArrayString)
                throw std::runtime_error("row cell must be of TpStringArray type");

            casacore::Array<casacore::String> array(shape);
            rec.get(field_num, array);
            unbridge_string_array(array, callback, ctxt);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_row_put_cell(GlueTableRow &wrap_row, const StringBridge &col_name,
                       const GlueDataType data_type, const unsigned long n_dims,
                       const unsigned long *dims, void *data, ExcInfo &exc)
    {
        casacore::TableRow &row = (casacore::TableRow &) wrap_row;

        try {
            casacore::TableRecord &rec = row.record();
            casacore::Int field_num = rec.fieldNumber(bridge_string(col_name));

            switch (data_type) {

#define SCALAR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                rec.define(field_num, *(CPPTYPE *) data); \
                break; \
            }

#define VECTOR_CASE(DTYPE, CPPTYPE) \
            case casacore::DTYPE: { \
                casacore::IPosition shape(n_dims); \
                for (casacore::uInt i = 0; i < n_dims; i++) \
                    shape[i] = dims[n_dims - 1 - i]; \
                casacore::Array<CPPTYPE> array(shape, (CPPTYPE *) data, casacore::SHARE); \
                rec.define(field_num, array); \
                break; \
            }

            SCALAR_CASE(TpBool, casacore::Bool)
            //SCALAR_CASE(TpChar, casacore::Char)
            SCALAR_CASE(TpUChar, casacore::uChar)
            SCALAR_CASE(TpShort, casacore::Short)
            //SCALAR_CASE(TpUShort, casacore::uShort)
            SCALAR_CASE(TpInt, casacore::Int)
            SCALAR_CASE(TpUInt, casacore::uInt)
            SCALAR_CASE(TpFloat, float)
            SCALAR_CASE(TpDouble, double)
            SCALAR_CASE(TpComplex, casacore::Complex)
            SCALAR_CASE(TpDComplex, casacore::DComplex)

            VECTOR_CASE(TpArrayBool, casacore::Bool)
            //VECTOR_CASE(TpArrayChar, casacore::Char)
            VECTOR_CASE(TpArrayUChar, casacore::uChar)
            VECTOR_CASE(TpArrayShort, casacore::Short)
            //VECTOR_CASE(TpArrayUShort, casacore::uShort)
            VECTOR_CASE(TpArrayInt, casacore::Int)
            VECTOR_CASE(TpArrayUInt, casacore::uInt)
            VECTOR_CASE(TpArrayFloat, float)
            VECTOR_CASE(TpArrayDouble, double)
            VECTOR_CASE(TpArrayComplex, casacore::Complex)
            VECTOR_CASE(TpArrayDComplex, casacore::DComplex)

#undef SCALAR_CASE
#undef VECTOR_CASE

            case casacore::TpString: {
                rec.define(field_num, bridge_string(*(StringBridge *) data));
                break;
            }

            case casacore::TpArrayString: {
                casacore::IPosition shape(n_dims);
                for (casacore::uInt i = 0; i < n_dims; i++)
                    shape[i] = dims[n_dims - 1 - i];
                rec.define(field_num, bridge_string_array((const StringBridge *) data, shape));
                break;
            }

            default:
                throw std::runtime_error("unhandled cell data type");
            }
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }

    int
    table_row_write(GlueTableRow &wrap_row, const unsigned long dest_row_number, ExcInfo &exc)
    {
        casacore::TableRow &row = (casacore::TableRow &) wrap_row;

        try {
            row.put(dest_row_number);
        } catch (...) {
            handle_exception(exc);
            return 1;
        }

        return 0;
    }
}
