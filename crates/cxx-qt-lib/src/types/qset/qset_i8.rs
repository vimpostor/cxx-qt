// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cxx::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qset.h");
        type QSet_i8 = crate::QSet<i8>;
    }

    unsafe extern "C++" {
        #[rust_name = "cxx_clear"]
        fn clear(self: &mut QSet_i8);
        #[rust_name = "cxx_contains"]
        fn contains(self: &QSet_i8, _: &i8) -> bool;
        #[rust_name = "cxx_remove"]
        fn remove(self: &mut QSet_i8, _: &i8) -> bool;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        #[rust_name = "clone_i8"]
        fn qset_clone(_: &QSet_i8) -> QSet_i8;
        #[rust_name = "default_i8"]
        fn qset_default() -> QSet_i8;
        #[rust_name = "drop_i8"]
        fn qset_drop(_: &mut QSet_i8);
        #[rust_name = "get_unchecked_i8"]
        #[allow(clippy::needless_lifetimes)]
        unsafe fn qset_get_unchecked<'a>(set: &'a QSet_i8, pos: usize) -> &'a i8;
        #[rust_name = "insert_i8"]
        fn qset_insert(_: &mut QSet_i8, _: &i8);
        #[rust_name = "len_i8"]
        fn qset_len(_: &QSet_i8) -> usize;
    }
}

pub(crate) fn clone(s: &ffi::QSet_i8) -> ffi::QSet_i8 {
    ffi::clone_i8(s)
}

pub(crate) fn default() -> ffi::QSet_i8 {
    ffi::default_i8()
}

pub(crate) fn drop(s: &mut ffi::QSet_i8) {
    ffi::drop_i8(s);
}

pub(crate) unsafe fn get_unchecked(s: &ffi::QSet_i8, pos: usize) -> &i8 {
    ffi::get_unchecked_i8(s, pos)
}

pub(crate) fn insert(s: &mut ffi::QSet_i8, value: &i8) {
    ffi::insert_i8(s, value);
}

pub(crate) fn len(s: &ffi::QSet_i8) -> usize {
    ffi::len_i8(s)
}
