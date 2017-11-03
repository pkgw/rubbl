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
    void string_deinit(GlueString &str);

    GlueTable *table_alloc_and_open(const GlueString &path, ExcInfo &exc);
    void table_close_and_free(GlueTable *table, ExcInfo &exc);
    unsigned long table_n_rows(const GlueTable &table);
}
