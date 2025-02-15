// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cxx::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qdatetime.h");
        type QDateTime = crate::QDateTime;

        include!("cxx-qt-lib/qset.h");
        type QSet_QDateTime = crate::QSet<QDateTime>;
    }

    unsafe extern "C++" {
        #[rust_name = "cxx_clear"]
        fn clear(self: &mut QSet_QDateTime);
        #[rust_name = "cxx_contains"]
        fn contains(self: &QSet_QDateTime, _: &QDateTime) -> bool;
        #[rust_name = "cxx_remove"]
        fn remove(self: &mut QSet_QDateTime, _: &QDateTime) -> bool;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        #[rust_name = "clone_QDateTime"]
        fn qset_clone(_: &QSet_QDateTime) -> QSet_QDateTime;
        #[rust_name = "default_QDateTime"]
        fn qset_default() -> QSet_QDateTime;
        #[rust_name = "drop_QDateTime"]
        fn qset_drop(_: &mut QSet_QDateTime);
        #[rust_name = "get_unchecked_QDateTime"]
        unsafe fn qset_get_unchecked(set: &QSet_QDateTime, pos: usize) -> &QDateTime;
        #[rust_name = "insert_QDateTime"]
        fn qset_insert(_: &mut QSet_QDateTime, _: &QDateTime);
        #[rust_name = "len_QDateTime"]
        fn qset_len(_: &QSet_QDateTime) -> usize;
    }
}

pub(crate) fn clone(s: &ffi::QSet_QDateTime) -> ffi::QSet_QDateTime {
    ffi::clone_QDateTime(s)
}

pub(crate) fn default() -> ffi::QSet_QDateTime {
    ffi::default_QDateTime()
}

pub(crate) fn drop(s: &mut ffi::QSet_QDateTime) {
    ffi::drop_QDateTime(s);
}

pub(crate) unsafe fn get_unchecked(s: &ffi::QSet_QDateTime, pos: usize) -> &ffi::QDateTime {
    ffi::get_unchecked_QDateTime(s, pos)
}

pub(crate) fn insert(s: &mut ffi::QSet_QDateTime, value: &ffi::QDateTime) {
    ffi::insert_QDateTime(s, value);
}

pub(crate) fn len(s: &ffi::QSet_QDateTime) -> usize {
    ffi::len_QDateTime(s)
}
