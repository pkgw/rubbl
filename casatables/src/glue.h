// Copyright 2017 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

/* We process this header with bindgen to create Rust prototypes for our glue
 * to the casacore library. We need manually-written glue to deal with things
 * like exceptions safely.
 *
 * As such, this file is designed to declare a very minimal set of types and
 * functions; just enough to let Rust do its job without having to worry about
 * the whole C++ mess lying underneath.
 *
 * We use some silly preprocessor futzing so that the same prototypes can
 * either use opaque struct pointers or the actual C++ types known to glue.cc.
 */

#ifndef CASA_TYPES_ALREADY_DECLARED

// copied from casa/Utilities/DataType.h:
typedef enum GlueDataType {
    TpBool,
    TpChar,
    TpUChar,
    TpShort,
    TpUShort,
    TpInt,
    TpUInt,
    TpFloat,
    TpDouble,
    TpComplex,
    TpDComplex,
    TpString,
    TpTable,
    TpArrayBool,
    TpArrayChar,
    TpArrayUChar,
    TpArrayShort,
    TpArrayUShort,
    TpArrayInt,
    TpArrayUInt,
    TpArrayFloat,
    TpArrayDouble,
    TpArrayComplex,
    TpArrayDComplex,
    TpArrayString,
    TpRecord,
    TpOther,
    TpQuantity,
    TpArrayQuantity,
    TpInt64,
    TpArrayInt64,
} GlueDataType;

typedef struct GlueTable GlueTable;
typedef struct GlueTableRow GlueTableRow;
#endif

// OMG, strings. First of all: casacore::String is a subclass of std::string,
// and the two are interchangeable for our purposes. I first tried to send
// std::strings directly into the Rust layer, but different versions of the
// STL have different semantics that I could never make work consistently.
// Note also that, to the best of my knowledge, there is no reliable mechanism
// to take ownership of a std::string's underlying buffer, which is what would
// be necessary for zero-copy transfer of strings from C++ to Rust. (Some STLs
// use a "small string optimization" that means that for short strings there
// *is no* underyling buffer anyway.) So we have to copy data, and often need
// to use C++->Rust callbacks to be able to copy string contents before they
// are deallocated at the C++ layer.

typedef struct StringBridge {
    const void *data;
    unsigned long n_bytes;
} StringBridge;

typedef struct ExcInfo {
    char message[512];
} ExcInfo;

// Generic callback prototype when handing off owned strings from C++ to Rust.
// See, e.g., table_get_column_names.
typedef void (*StringBridgeCallback)(const StringBridge *name, void *ctxt);

// A more specific callback type for table_get_keyword_info, in which there is
// additional information we'd like to to transfer.
typedef void (*KeywordInfoCallback)(const StringBridge *name, GlueDataType dtype, void *ctxt);

typedef enum TableOpenMode {
    TOM_OPEN_READONLY = 1,
    TOM_OPEN_RW = 2,
    TOM_CREATE = 3,
} TableOpenMode;

extern "C" {
    int data_type_get_element_size(const GlueDataType ty);

    GlueTable *table_alloc_and_open(const StringBridge &path, const TableOpenMode mode, ExcInfo &exc);
    void table_close_and_free(GlueTable *table, ExcInfo &exc);
    unsigned long table_n_rows(const GlueTable &table);
    unsigned long table_n_columns(const GlueTable &table);
    int table_get_column_names(const GlueTable &table, StringBridgeCallback callback,
                               void *ctxt, ExcInfo &exc);
    unsigned long table_n_keywords(const GlueTable &table);
    int table_get_keyword_info(const GlueTable &table, KeywordInfoCallback callback,
                               void *ctxt, ExcInfo &exc);
    int table_copy_rows(const GlueTable &source, GlueTable &dest, ExcInfo &exc);
    int table_deep_copy_no_rows(const GlueTable &table, const StringBridge &dest_path, ExcInfo &exc);
    int table_get_column_info(const GlueTable &table, const StringBridge &col_name,
                              unsigned long *n_rows, GlueDataType *data_type,
                              int *is_scalar, int *is_fixed_shape, int *n_dim,
                              unsigned long dims[8], ExcInfo &exc);
    int table_remove_column(GlueTable &table, const StringBridge &col_name, ExcInfo &exc);
    int table_get_scalar_column_data(const GlueTable &table, const StringBridge &col_name,
                                     void *data, ExcInfo &exc);
    int table_get_cell_info(const GlueTable &table, const StringBridge &col_name,
                            unsigned long row_number, GlueDataType *data_type,
                            int *n_dim, unsigned long dims[8], ExcInfo &exc);
    int table_get_cell(const GlueTable &table, const StringBridge &col_name,
                       const unsigned long row_number, void *data, ExcInfo &exc);
    int table_put_cell(GlueTable &table, const StringBridge &col_name,
                       const unsigned long row_number, const GlueDataType data_type,
                       const unsigned long n_dims, const unsigned long *dims,
                       void *data, ExcInfo &exc);
    int table_add_rows(GlueTable &table, const unsigned long n_rows, ExcInfo &exc);

    GlueTableRow *table_row_alloc(const GlueTable &table, const unsigned char is_read_only, ExcInfo &exc);
    int table_row_free(GlueTableRow *row, ExcInfo &exc);
    int table_row_read(GlueTableRow &row, const unsigned long row_number, ExcInfo &exc);
    int table_row_copy_and_put(GlueTableRow &src_row, const unsigned long dest_row_number,
                               GlueTableRow &dest_row, ExcInfo &exc);
    int table_row_get_cell_info(const GlueTableRow &row, const StringBridge &col_name,
                                GlueDataType *data_type, int *n_dim,
                                unsigned long dims[8], ExcInfo &exc);
    int table_row_get_cell(const GlueTableRow &row, const StringBridge &col_name,
                           void *data, ExcInfo &exc);
    int table_row_put_cell(GlueTableRow &row, const StringBridge &col_name,
                           const GlueDataType data_type, const unsigned long n_dims,
                           const unsigned long *dims, void *data, ExcInfo &exc);
    int table_row_write(GlueTableRow &row, const unsigned long dest_row_number, ExcInfo &exc);
}
