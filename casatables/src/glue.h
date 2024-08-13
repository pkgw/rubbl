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
/**Different data types supported by the CASA tables format.*/
typedef enum GlueDataType
{
    /**A boolean value.*/
    TpBool,
    /**A signed 8-bit integer value.*/
    TpChar,
    /**An unsigned 8-bit integer value.*/
    TpUChar,
    /**A signed 16-bit integer value.*/
    TpShort,
    /**An unsigned 16-bit integer value.*/
    TpUShort,
    /**A signed 32-bit integer value.*/
    TpInt,
    /**An unsigned 32-bit integer value.*/
    TpUInt,
    /**A 32-bit IEEE754 floating-point value.*/
    TpFloat,
    /**A 64-bit IEEE754 double-precision floating-point value.*/
    TpDouble,
    /**A complex number composed of two single-precision floating-point values.*/
    TpComplex,
    /**A complex number composed of two double-precision floating-point values.*/
    TpDComplex,
    /**A string value. **Todo:** encoding???*/
    TpString,
    /**A value that is its own CASA table.*/
    TpTable,
    /**A value that is an array of booleans.*/
    TpArrayBool,
    /**A value that is an array of signed 8-bit integers.*/
    TpArrayChar,
    /**A value that is an array of unsigned 8-bit integers.*/
    TpArrayUChar,
    /**A value that is an array of signed 16-bit integers.*/
    TpArrayShort,
    /**A value that is an array of unsigned 16-bit integers.*/
    TpArrayUShort,
    /**A value that is an array of signed 32-bit integers.*/
    TpArrayInt,
    /**A value that is an array of unsigned 32-bit integers.*/
    TpArrayUInt,
    /**A value that is an array of 32-bit single-precision floating-point numbers.*/
    TpArrayFloat,
    /**A value that is an array of 64-bit double-precision floating-point numbers.*/
    TpArrayDouble,
    /**A value that is an array of complex numbers with single-precision components.*/
    TpArrayComplex,
    /**A value that is an array of complex numbers with double-precision components.*/
    TpArrayDComplex,
    /**A value that is an array of strings. **Todo:** encoding???*/
    TpArrayString,
    /**A value that is a dictionary of name-value pairs.*/
    TpRecord,
    /**A value of some other type.*/
    TpOther,
    /**A value that is a physical quantity with associated dimensions.*/
    TpQuantity,
    /**A value that is an array of physical quantities with associated dimensions.*/
    TpArrayQuantity,
    /**A signed 64-bit integer value.*/
    TpInt64,
    /**A value that is an array of unsigned 8-bit integers.*/
    TpArrayInt64,
} GlueDataType;

typedef struct GlueTable GlueTable;
typedef struct GlueTableRow GlueTableRow;
typedef struct GlueTableDesc GlueTableDesc;
typedef struct GlueTableRecord GlueTableRecord;

#endif

// OMG, strings. First of all: casacore::String is a subclass of std::string,
// and the two are interchangeable for our purposes. I first tried to send
// std::strings directly into the Rust layer, but different versions of the
// STL have different semantics that I could never make work consistently.
// Note also that, to the best of my knowledge, there is no reliable mechanism
// to take ownership of a std::string's underlying buffer, which is what would
// be necessary for zero-copy transfer of strings from C++ to Rust. (Some STLs
// use a "small string optimization" that means that for short strings there
// *is no* underlying buffer anyway.) So we have to copy data, and often need
// to use C++->Rust callbacks to be able to copy string contents before they
// are deallocated at the C++ layer.

typedef struct StringBridge
{
    const void *data;
    unsigned long n_bytes;
} StringBridge;

typedef struct ExcInfo
{
    char message[512];
} ExcInfo;

// Generic callback prototype when handing off owned strings from C++ to Rust.
// See, e.g., table_get_column_names.
typedef void (*StringBridgeCallback)(const StringBridge *name, void *ctxt);

// A more specific callback type for table_get_keyword_info, in which there is
// additional information we'd like to to transfer.
typedef void (*KeywordInfoCallback)(const StringBridge *name, GlueDataType dtype, void *ctxt);
typedef void (*KeywordReprCallback)(const StringBridge *name, GlueDataType dtype, const StringBridge *repr, void *ctxt);

typedef enum TableOpenMode
{
    TOM_OPEN_READONLY = 1,
    TOM_OPEN_RW = 2,
    TOM_CREATE = 3,
} TableOpenMode;

typedef enum TableCreateMode
{
    // create table
    TCM_NEW = 1,
    // create table (may not exist)
    TCM_NEW_NO_REPLACE = 2,
    // New table, which gets marked for delete"
    TCM_SCRATCH = 3,
} TableCreateMode;

/**Different modes for creating a CASA table description.*/
typedef enum TableDescCreateMode
{
    // NB: bindgenc can't handle multiline docstrings here.
    /** Create a new table description file.*/
    // "The TableDesc destructor will write the table description into the file.""
    TDM_NEW,

    /** Create a new file, raising an error if it already exists.*/
    TDM_NEW_NO_REPLACE,

    /** Create a table description without an associated file on disk.*/
    // "The table description will be lost when the TableDesc object is
    // destructed. This is useful to create a Table object without storing the
    // description separately. Note that the Table object maintains its own
    // description (i.e. it copies the description when being constructed)."
    TDM_SCRATCH,

    // Note, there are other options here which could be enabled if needed.
    //  <li> Old
    //    Open an existing table description file as readonly.
    //  <li> Update
    //    Open an existing table description file as read/write
    //    The TableDesc destructor will rewrite the possibly changed
    //    description.
    //  <li> Delete
    //    Delete the table description file. This gets done by the destructor.
} TableDescOption;

extern "C"
{
    int data_type_get_element_size(const GlueDataType ty);

    // Table Records

    GlueTableRecord *tablerec_create(ExcInfo &exc);
    GlueTableRecord *tablerec_copy(const GlueTableRecord &other, ExcInfo &exc);
    bool tablerec_eq(const GlueTableRecord &rec, const GlueTableRecord &other);
    int tablerec_get_keyword_info(
        const GlueTableRecord &rec,
        KeywordInfoCallback callback,
        void *ctxt,
        ExcInfo &exc);
    int
    tablerec_get_keyword_repr(
        const GlueTableRecord &rec,
        KeywordReprCallback callback,
        void *ctxt,
        ExcInfo &exc
    );
    int tablerec_get_field_info(
        const GlueTableRecord &rec,
        const StringBridge &col_name,
        GlueDataType *data_type,
        int *n_dim,
        unsigned long dims[8],
        ExcInfo &exc);
    int tablerec_get_field(
        const GlueTableRecord &rec,
        const StringBridge &field_name,
        void *data,
        ExcInfo &exc);
    int tablerec_get_field_string(
        const GlueTableRecord &rec,
        const StringBridge &col_name,
        StringBridgeCallback callback,
        void *ctxt,
        ExcInfo &exc);
    int tablerec_get_field_string_array(
        const GlueTableRecord &rec,
        const StringBridge &col_name,
        StringBridgeCallback callback,
        void *ctxt,
        ExcInfo &exc);
    int tablerec_get_field_subrecord(
        const GlueTableRecord &rec,
        const StringBridge &col_name,
        GlueTableRecord &sub_rec,
        ExcInfo &exc);
    int tablerec_put_field(
        GlueTableRecord &rec,
        const StringBridge &field_name,
        const GlueDataType data_type,
        const unsigned long n_dims,
        const unsigned long *dims,
        void *data,
        ExcInfo &exc);
    int tablerec_free(GlueTableRecord *rec, ExcInfo &exc);

    // Table Description

    GlueTableDesc *tabledesc_create(
        const StringBridge &type,
        const TableDescCreateMode mode,
        ExcInfo &exc);
    int tabledesc_add_scalar_column(
        GlueTableDesc &table_desc,
        GlueDataType data_type,
        const StringBridge &col_name,
        const StringBridge &comment,
        bool direct,
        bool undefined,
        ExcInfo &exc);
    int tabledesc_add_array_column(
        GlueTableDesc &table_desc,
        GlueDataType data_type,
        const StringBridge &col_name,
        const StringBridge &comment,
        bool direct,
        bool undefined,
        ExcInfo &exc);
    int tabledesc_add_fixed_array_column(
        GlueTableDesc &table_desc,
        GlueDataType data_type,
        const StringBridge &col_name,
        const StringBridge &comment,
        const unsigned long n_dims,
        const unsigned long *dims,
        bool direct,
        bool undefined,
        ExcInfo &exc);
    int tabledesc_set_ndims(
        GlueTableDesc &table_desc,
        const StringBridge &col_name,
        const unsigned long n_dims,
        ExcInfo &exc);
    const GlueTableRecord * tabledesc_get_keywords(GlueTableDesc &table_desc, ExcInfo &exc);
    const GlueTableRecord * tabledesc_get_column_keywords(
        GlueTableDesc &table_desc, const StringBridge &col_name, ExcInfo &exc);
    int tabledesc_put_keyword(
        GlueTableDesc &table_desc,
        const StringBridge &kw_name,
        const GlueDataType data_type,
        const unsigned long n_dims,
        const unsigned long *dims,
        void *data,
        ExcInfo &exc
    );
    int tabledesc_put_column_keyword(
        GlueTableDesc &table_desc,
        const StringBridge &col_name,
        const StringBridge &kw_name,
        const GlueDataType data_type,
        const unsigned long n_dims,
        const unsigned long *dims,
        void *data,
        ExcInfo &exc
    );

    // Table

    GlueTable *table_create(const StringBridge &path, GlueTableDesc &table_desc,
                            unsigned long n_rows, const TableCreateMode mode, ExcInfo &exc);
    GlueTable *table_alloc_and_open(const StringBridge &path, const TableOpenMode mode, ExcInfo &exc);
    void table_close_and_free(GlueTable *table, ExcInfo &exc);
    unsigned long table_n_rows(const GlueTable &table);
    unsigned long table_n_columns(const GlueTable &table);
    int table_get_file_name(const GlueTable &table, StringBridgeCallback callback, void *ctxt, ExcInfo &exc);
    int table_get_column_names(const GlueTable &table, StringBridgeCallback callback,
                               void *ctxt, ExcInfo &exc);
    unsigned long table_n_keywords(const GlueTable &table);
    int table_get_keyword_info(const GlueTable &table, KeywordInfoCallback callback,
                               void *ctxt, ExcInfo &exc);
    int table_get_column_keyword_info(const GlueTable &table, const StringBridge &col_name,
                                      KeywordInfoCallback callback, void *ctxt, ExcInfo &exc);
    const GlueTableRecord *table_get_keywords(GlueTable &table, ExcInfo &exc);
    const GlueTableRecord *table_get_column_keywords(
        GlueTable &table,
        const StringBridge &col_name,
        ExcInfo &exc);
    int table_put_keyword(
        GlueTable &table,
        const StringBridge &kw_name,
        const GlueDataType data_type,
        const unsigned long n_dims,
        const unsigned long *dims,
        void *data,
        ExcInfo &exc);
    int table_put_column_keyword(
        GlueTable &table,
        const StringBridge &col_name,
        const StringBridge &kw_name,
        const GlueDataType data_type,
        const unsigned long n_dims, const unsigned long *dims, void *data,
        ExcInfo &exc);
    int table_copy_rows(const GlueTable &source, GlueTable &dest, ExcInfo &exc);
    int table_deep_copy_no_rows(const GlueTable &table, const StringBridge &dest_path, ExcInfo &exc);
    int table_get_column_info(const GlueTable &table, const StringBridge &col_name,
                              unsigned long *n_rows, GlueDataType *data_type,
                              int *is_scalar, int *is_fixed_shape, int *n_dim,
                              unsigned long dims[8], ExcInfo &exc);
    int table_remove_column(GlueTable &table, const StringBridge &col_name, ExcInfo &exc);
    int table_add_scalar_column(GlueTable &table, GlueDataType data_type, const StringBridge &col_name,
                                const StringBridge &comment, bool direct, bool undefined, ExcInfo &exc);
    int table_add_array_column(GlueTable &table, GlueDataType data_type, const StringBridge &col_name,
                               const StringBridge &comment, bool direct, bool undefined, ExcInfo &exc);
    int table_add_fixed_array_column(GlueTable &table, GlueDataType data_type, const StringBridge &col_name,
                                     const StringBridge &comment, const unsigned long n_dims,
                                     const unsigned long *dims, bool direct, bool undefined, ExcInfo &exc);
    int table_get_scalar_column_data(const GlueTable &table, const StringBridge &col_name,
                                     void *data, ExcInfo &exc);
    int table_get_scalar_column_data_string(const GlueTable &table, const StringBridge &col_name,
                                            StringBridgeCallback callback, void *ctxt,
                                            ExcInfo &exc);
    int table_get_cell_info(const GlueTable &table, const StringBridge &col_name,
                            unsigned long row_number, GlueDataType *data_type,
                            int *n_dim, unsigned long dims[8], ExcInfo &exc);
    int table_get_cell(const GlueTable &table, const StringBridge &col_name,
                       const unsigned long row_number, void *data, ExcInfo &exc);
    int table_get_cell_string(const GlueTable &table, const StringBridge &col_name,
                              const unsigned long row_number, StringBridgeCallback callback,
                              void *ctxt, ExcInfo &exc);
    int table_get_cell_string_array(const GlueTable &table, const StringBridge &col_name,
                                    const unsigned long row_number, StringBridgeCallback callback,
                                    void *ctxt, ExcInfo &exc);
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
    int table_row_get_cell_string(const GlueTableRow &row, const StringBridge &col_name,
                                  StringBridgeCallback callback, void *ctxt,
                                  ExcInfo &exc);
    int table_row_get_cell_string_array(const GlueTableRow &row, const StringBridge &col_name,
                                        StringBridgeCallback callback, void *ctxt,
                                        ExcInfo &exc);
    int table_row_put_cell(GlueTableRow &row, const StringBridge &col_name,
                           const GlueDataType data_type, const unsigned long n_dims,
                           const unsigned long *dims, void *data, ExcInfo &exc);
    int table_row_write(GlueTableRow &row, const unsigned long dest_row_number, ExcInfo &exc);
}
