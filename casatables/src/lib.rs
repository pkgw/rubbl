extern crate rubbl_casatables_impl;

mod glue;

/// OMG. Strings were incredibly painful.
///
/// One important piece of context: `casacore::String` is a subclass of C++'s
/// `std::string`. Rust strings can contain interior NUL bytes. Fortunately,
/// `std::string` can as well, so we don't need to worry about the C string
/// convention.
///
/// My understanding is that C++'s `std::string` always allocates its own
/// buffer. So we can't try to be clever about lifetimes and borrowing: every
/// time we bridge to C++ there's going to be a copy.
///
/// Then I ran into problems essentially because of the following bindgen
/// problem: https://github.com/rust-lang-nursery/rust-bindgen/issues/778 . On
/// Linux small classes, such as String, have special ABI conventions, and
/// bindgen does not represent them properly to Rust at the moment (Sep 2017).
/// The String class is a victim of this problem, which led to completely
/// bizarre failures in my code when the small-string optimization was kicking
/// in. It seems that if we only interact with the C++ through pointers and
/// references to Strings, things remain OK.
///
/// Finally, as best I understand it, we need to manually ensure that the C++
/// destructor for the String class is run. I have done this with a little
/// trick off of StackExchange.

impl glue::GlueString {
    fn from_rust(s: &str) -> Self {
        unsafe {
            let mut cs = ::std::mem::zeroed::<glue::GlueString>();
            glue::string_init(&mut cs, s.as_ptr() as _, s.len() as u64);
            cs
        }
    }
}

impl Drop for glue::GlueString {
    fn drop(&mut self) {
        unsafe { glue::string_deinit(self) };
    }
}

// Exceptions

impl glue::ExcInfo {
    fn as_error(&self) -> ::std::io::Error {
        let c_str = unsafe { ::std::ffi::CStr::from_ptr(self.message.as_ptr()) };

        let msg = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => "[un-translatable C++ exception]",
        };

        ::std::io::Error::new(::std::io::ErrorKind::Other, msg)
    }

    fn as_err<T>(&self) -> Result<T,::std::io::Error> {
        Err(self.as_error())
    }
}


// Tables

pub struct Table {
    handle: *mut glue::GlueTable,
    exc_info: glue::ExcInfo,
}

impl Table {
    pub fn open(name: &str) -> Result<Self,::std::io::Error> {
        let cname = glue::GlueString::from_rust(name);
        let mut exc_info = unsafe { ::std::mem::zeroed::<glue::ExcInfo>() };

        let handle = unsafe { glue::table_alloc_and_open(&cname, &mut exc_info) };
        if handle.is_null() {
            return exc_info.as_err();
        }

        Ok(Table {
            handle: handle,
            exc_info: exc_info,
        })
    }

    pub fn n_rows(&self) -> usize {
        unsafe { glue::table_n_rows(self.handle) as usize }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        // FIXME: not sure if this function can actually produce useful
        // exceptions anyway, but we can't do anything if it does!
        unsafe { glue::table_close_and_free(self.handle, &mut self.exc_info) }
    }
}


/*
use bindings::root::casacore;

// Tables

pub use casacore::Table_TableOption as TableOpenMode;
pub use casacore::TSMOption as TsmConfig;
pub use casacore::TSMOption_Option as TsmMode;

impl TsmConfig {
    pub fn default() -> Self {
        unsafe { Self::new(TsmMode::Default, 65536, 32) }
    }
}

pub use casacore::TableDesc;

impl TableDesc {
    pub fn n_cols(&self) -> usize {
        unsafe { self.ncolumn() as usize }
    }

    /// Panics if `col_name` cannot be converted to a C string.
    pub fn has_column(&self, col_name: &str) -> bool {
        let cname = casacore::String::from_rust(col_name);
        unsafe { self.isColumn(&cname) }
    }
}

pub struct Table {
    handle: casacore::Table,
}

impl Table {
    pub fn open(name: &str, mode: TableOpenMode, storage: &TsmConfig) -> Self {
        let cname = casacore::String::from_rust(name);

        Table {
            handle: unsafe { casacore::Table::openTable(&cname, mode, storage) },
        }
    }

    pub fn n_rows(&self) -> usize {
        unsafe { self.handle.nrow() as usize }
    }

    // TODO: understand difference between this and "actualTableDesc"
    pub fn desc(&self) -> &TableDesc {
        unsafe { &*self.handle.tableDesc() }
    }
}
 */

#[cfg(test)]
mod tests {
    use super::glue;

    #[test]
    fn check_string_size() {
        let cpp_size = unsafe { glue::string_check_size() } as usize;
        let rust_size = ::std::mem::size_of::<glue::GlueString>();
        assert_eq!(cpp_size, rust_size);
    }
}
