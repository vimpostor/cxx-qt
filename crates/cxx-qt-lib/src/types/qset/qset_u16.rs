// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cxx::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qset.h");
        type QSet_u16 = crate::QSet<u16>;
    }

    unsafe extern "C++" {
        #[rust_name = "cxx_clear"]
        fn clear(self: &mut QSet_u16);
        #[rust_name = "cxx_contains"]
        fn contains(self: &QSet_u16, _: &u16) -> bool;
        #[rust_name = "cxx_remove"]
        fn remove(self: &mut QSet_u16, _: &u16) -> bool;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        #[rust_name = "clone_u16"]
        fn qset_clone(_: &QSet_u16) -> QSet_u16;
        #[rust_name = "default_u16"]
        fn qset_default() -> QSet_u16;
        #[rust_name = "drop_u16"]
        fn qset_drop(_: &mut QSet_u16);
        #[rust_name = "get_unchecked_u16"]
        #[allow(clippy::needless_lifetimes)]
        unsafe fn qset_get_unchecked<'a>(set: &'a QSet_u16, pos: usize) -> &'a u16;
        #[rust_name = "insert_u16"]
        fn qset_insert(_: &mut QSet_u16, _: &u16);
        #[rust_name = "len_u16"]
        fn qset_len(_: &QSet_u16) -> usize;
    }
}

pub(crate) fn clone(s: &ffi::QSet_u16) -> ffi::QSet_u16 {
    ffi::clone_u16(s)
}

pub(crate) fn default() -> ffi::QSet_u16 {
    ffi::default_u16()
}

pub(crate) fn drop(s: &mut ffi::QSet_u16) {
    ffi::drop_u16(s);
}

pub(crate) unsafe fn get_unchecked(s: &ffi::QSet_u16, pos: usize) -> &u16 {
    ffi::get_unchecked_u16(s, pos)
}

pub(crate) fn insert(s: &mut ffi::QSet_u16, value: &u16) {
    ffi::insert_u16(s, value);
}

pub(crate) fn len(s: &ffi::QSet_u16) -> usize {
    ffi::len_u16(s)
}
