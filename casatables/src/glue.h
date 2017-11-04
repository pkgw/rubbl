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
#endif

typedef struct _ExcInfo {
    char message[512];
} ExcInfo;

extern "C" {
    unsigned long string_check_size(void);
    void string_init(GlueString &str, const void *data, const unsigned long n_bytes);
    void string_get_buf(const GlueString &str, void const **data_ptr, unsigned long *n_bytes_ptr);
    void string_deinit(GlueString &str);

    int data_type_get_element_size(const GlueDataType ty);

    GlueTable *table_alloc_and_open(const GlueString &path, ExcInfo &exc);
    void table_close_and_free(GlueTable *table, ExcInfo &exc);
    unsigned long table_n_rows(const GlueTable &table);
    unsigned long table_n_columns(const GlueTable &table);
    int table_get_column_names(const GlueTable &table, GlueString *col_names, ExcInfo &exc);
    int table_deep_copy_no_rows(const GlueTable &table, const GlueString &dest_path, ExcInfo &exc);
    int table_get_column_info(const GlueTable &table, const GlueString &col_name,
                              unsigned long *n_rows, GlueDataType *data_type,
                              int *is_scalar, int *is_fixed_shape, unsigned int *n_dim,
                              unsigned long dims[8], ExcInfo &exc);
    int table_get_scalar_column_data(const GlueTable &table, const GlueString &col_name,
                                     void *data, ExcInfo &exc);
}
