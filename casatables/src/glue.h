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
 * We further elaborate by stubbing out the GlueString type so that those
 * values can be stack-allocated by Rust, since it's common to pass strings
 * around and we want to be able to centralize the code that passes Rust
 * strings through without having to jump through the hoops needed for C
 * string conversion.
 */

#ifndef CASA_TYPES_ALREADY_DECLARED

// copied from casa/Utilities/DataType.h:
typedef enum _GlueDataType {
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

/**
 * <div rustbindgen nocopy></div>
 */
typedef struct _GlueString { void *a, *b, *c, *d; } GlueString;
typedef struct _GlueTable GlueTable;
typedef struct _GlueTableRow GlueTableRow;
#endif

typedef struct _ExcInfo {
    char message[512];
} ExcInfo;

typedef enum _TableOpenMode {
    TOM_OPEN_READONLY = 1,
    TOM_OPEN_RW = 2,
    TOM_CREATE = 3,
} TableOpenMode;

extern "C" {
    unsigned long string_check_size(void);
    void string_init(GlueString &str, const void *data, const unsigned long n_bytes);
    void string_get_buf(const GlueString &str, void const **data_ptr, unsigned long *n_bytes_ptr);
    void string_deinit(GlueString &str);

    int data_type_get_element_size(const GlueDataType ty);

    GlueTable *table_alloc_and_open(const GlueString &path, const TableOpenMode mode, ExcInfo &exc);
    void table_close_and_free(GlueTable *table, ExcInfo &exc);
    unsigned long table_n_rows(const GlueTable &table);
    unsigned long table_n_columns(const GlueTable &table);
    int table_get_column_names(const GlueTable &table, GlueString *col_names, ExcInfo &exc);
    unsigned long table_n_keywords(const GlueTable &table);
    int table_get_keyword_info(const GlueTable &table, GlueString *names, GlueDataType *types, ExcInfo &exc);
    int table_copy_rows(const GlueTable &source, GlueTable &dest, ExcInfo &exc);
    int table_deep_copy_no_rows(const GlueTable &table, const GlueString &dest_path, ExcInfo &exc);
    int table_get_column_info(const GlueTable &table, const GlueString &col_name,
                              unsigned long *n_rows, GlueDataType *data_type,
                              int *is_scalar, int *is_fixed_shape, int *n_dim,
                              unsigned long dims[8], ExcInfo &exc);
    int table_get_scalar_column_data(const GlueTable &table, const GlueString &col_name,
                                     void *data, ExcInfo &exc);
    int table_get_cell_info(const GlueTable &table, const GlueString &col_name,
                            unsigned long row_number, GlueDataType *data_type,
                            int *n_dim, unsigned long dims[8], ExcInfo &exc);
    int table_get_cell(const GlueTable &table, const GlueString &col_name,
                       const unsigned long row_number, void *data, ExcInfo &exc);
    int table_put_cell(GlueTable &table, const GlueString &col_name,
                       const unsigned long row_number, const GlueDataType data_type,
                       const unsigned long n_dims, const unsigned long *dims,
                       void *data, ExcInfo &exc);
    int table_add_rows(GlueTable &table, const unsigned long n_rows, ExcInfo &exc);

    GlueTableRow *table_row_alloc(const GlueTable &table, const unsigned char is_read_only, ExcInfo &exc);
    int table_row_free(GlueTableRow *row, ExcInfo &exc);
    int table_row_read(GlueTableRow &row, const unsigned long row_number, ExcInfo &exc);
    int table_row_copy_and_put(GlueTableRow &src_row, const unsigned long dest_row_number,
                               GlueTableRow &dest_row, ExcInfo &exc);
    int table_row_get_cell_info(const GlueTableRow &row, const GlueString &col_name,
                                GlueDataType *data_type, int *n_dim,
                                unsigned long dims[8], ExcInfo &exc);
    int table_row_get_cell(const GlueTableRow &row, const GlueString &col_name,
                           void *data, ExcInfo &exc);
    int table_row_put_cell(GlueTableRow &row, const GlueString &col_name,
                           const GlueDataType data_type, const unsigned long n_dims,
                           const unsigned long *dims, void *data, ExcInfo &exc);
    int table_row_write(GlueTableRow &row, const unsigned long dest_row_number, ExcInfo &exc);
}
