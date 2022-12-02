// SPDX-FileCopyrightText: 2022 Klarälvdalens Datakonsult AB, a KDAB Group company <info@kdab.com>
// SPDX-FileContributor: Andrew Hayzen <andrew.hayzen@kdab.com>
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::parser::{qobject::ParsedQObject, signals::ParsedSignalsEnum};
use crate::syntax::attribute::attribute_tokens_to_value;
use crate::syntax::foreignmod::{foreign_mod_to_foreign_item_types, verbatim_to_foreign_mod};
use crate::syntax::{
    attribute::{attribute_find_path, attribute_tokens_to_ident},
    path::path_to_single_ident,
};
use std::collections::BTreeMap;
use syn::{
    spanned::Spanned, Error, Ident, Item, ItemEnum, ItemImpl, LitStr, Result, Type, TypePath,
};

#[derive(Default)]
pub struct ParsedCxxQtData {
    /// Map of the cxx_name of any types defined in CXX extern blocks
    ///
    /// This is used in the C++ generation to map the Rust type name to the C++ name
    pub cxx_names_map: BTreeMap<String, String>,
    /// Map of the QObjects defined in the module that will be used for code generation
    //
    // We have to use a BTreeMap here, instead of a HashMap, to keep the order of QObjects stable.
    // Otherwise, the output order would be different, depending on the environment, which makes it hard to test/debug.
    pub qobjects: BTreeMap<Ident, ParsedQObject>,
    /// The namespace of the CXX-Qt module
    pub namespace: String,
    /// Any `use` statements end up in the CXX-Qt generated module
    pub uses: Vec<Item>,
}

impl ParsedCxxQtData {
    /// Find the QObjects within the module and add into the qobjects BTreeMap
    pub fn find_qobject_structs(&mut self, items: &[Item]) -> Result<()> {
        for item in items {
            if let Item::Struct(qobject_struct) = item {
                if let Some(index) =
                    attribute_find_path(&qobject_struct.attrs, &["cxx_qt", "qobject"])
                {
                    // Note that we assume a compiler error will occur later
                    // if you had two structs with the same name
                    self.qobjects.insert(
                        qobject_struct.ident.clone(),
                        ParsedQObject::from_struct(qobject_struct, index)?,
                    );
                }
            }
        }

        Ok(())
    }

    /// Search any ForeignMod's and look for a cxx_name attribute on a type
    ///
    /// We need to know this as it affects the type name used in the C++ generation
    pub fn parse_cxx_names_map(&mut self, item: &Item, bridge_namespace: &str) -> Result<()> {
        // Extract the foreign mod (extern "ABI" { ... })
        let foreign_mod = match item {
            Item::ForeignMod(foreign_mod) => Some(foreign_mod.clone()),
            // Could be Verbatim TokenStream when it's an unsafe block, the remainder of the blocks are a normal ForeignMod though
            Item::Verbatim(tokens) => verbatim_to_foreign_mod(tokens)?,
            _others => None,
        };

        // If there is a foreign mod then process it
        if let Some(foreign_mod) = &foreign_mod {
            // Retrieve a namespace from the mod or the bridge
            let block_namespace =
                if let Some(index) = attribute_find_path(&foreign_mod.attrs, &["namespace"]) {
                    attribute_tokens_to_value::<LitStr>(&foreign_mod.attrs[index])?.value()
                } else {
                    bridge_namespace.to_owned()
                };

            // Read each of the types in the mod (type A;)
            for foreign_type in foreign_mod_to_foreign_item_types(foreign_mod)? {
                // Retrieve the namespace for the type itself if there is one
                let namespace =
                    if let Some(index) = attribute_find_path(&foreign_type.attrs, &["namespace"]) {
                        attribute_tokens_to_value::<LitStr>(&foreign_type.attrs[index])?.value()
                    } else {
                        block_namespace.clone()
                    };

                let type_with_namespace = |ty: String, namespace: &str| {
                    if namespace.is_empty() {
                        ty
                    } else {
                        format!("::{}::{}", namespace, ty)
                    }
                };

                // There is a cxx_name attribute
                if let Some(index) = attribute_find_path(&foreign_type.attrs, &["cxx_name"]) {
                    self.cxx_names_map.insert(
                        foreign_type.ident.to_string(),
                        type_with_namespace(
                            attribute_tokens_to_value::<LitStr>(&foreign_type.attrs[index])?
                                .value(),
                            &namespace,
                        ),
                    );
                // There is a namespace
                } else if !namespace.is_empty() {
                    self.cxx_names_map.insert(
                        foreign_type.ident.to_string(),
                        type_with_namespace(foreign_type.ident.to_string(), &namespace),
                    );
                }
            }
        }

        Ok(())
    }

    /// Determine if the given [syn::Item] is a CXX-Qt related item
    /// If it is then add the [syn::Item] into qobjects BTreeMap
    /// Otherwise return the [syn::Item] to pass through to CXX
    pub fn parse_cxx_qt_item(&mut self, item: Item) -> Result<Option<Item>> {
        match item {
            Item::Enum(item_enum) => self.parse_enum(item_enum),
            Item::Impl(imp) => self.parse_impl(imp),
            // Ignore structs which are qobjects
            Item::Struct(s) if self.qobjects.contains_key(&s.ident) => Ok(None),
            Item::Use(_) => {
                // Any use statements go into the CXX-Qt generated block
                self.uses.push(item);
                Ok(None)
            }
            _ => Ok(Some(item)),
        }
    }

    /// Parse a [syn::ItemEnum] into the qobjects if it's a CXX-Qt signal
    /// otherwise return as a [syn::Item] to pass through.
    fn parse_enum(&mut self, item_enum: ItemEnum) -> Result<Option<Item>> {
        // Check if the enum has cxx_qt::qsignals(T)
        if let Some(index) = attribute_find_path(&item_enum.attrs, &["cxx_qt", "qsignals"]) {
            let ident = attribute_tokens_to_ident(&item_enum.attrs[index])?;
            // Find the matching QObject for the enum
            if let Some(qobject) = self.qobjects.get_mut(&ident) {
                qobject.signals = Some(ParsedSignalsEnum::from(&item_enum, index)?);
                return Ok(None);
            } else {
                return Err(Error::new(
                    item_enum.span(),
                    "No matching QObject found for the given cxx_qt::qsignals<T> enum.",
                ));
            }
        }

        // Passthrough this unknown enum
        Ok(Some(Item::Enum(item_enum)))
    }

    /// Parse a [syn::ItemImpl] into the qobjects if it's a CXX-Qt implementation
    /// otherwise return as a [syn::Item] to pass through.
    fn parse_impl(&mut self, imp: ItemImpl) -> Result<Option<Item>> {
        // If the implementation has a qobject::T
        // then this is the block of methods to be implemented on the C++ object
        if let Type::Path(TypePath { path, .. }) = imp.self_ty.as_ref() {
            // Find if we are a impl qobject::T
            if path.segments.len() == 2 && path.segments[0].ident == "qobject" {
                if let Some(qobject) = self.qobjects.get_mut(&path.segments[1].ident) {
                    // Extract the ImplItem's from each Impl block
                    qobject.parse_impl_items(&imp.items)?;
                } else {
                    return Err(Error::new(
                        imp.span(),
                        "No matching QObject found for the given qobject::T impl block.",
                    ));
                }

                return Ok(None);
            // Find if we are an impl block for a qobject
            } else if let Some(qobject) = self.qobjects.get_mut(&path_to_single_ident(path)?) {
                qobject.others.push(Item::Impl(imp));
                return Ok(None);
            }
        }

        Ok(Some(Item::Impl(imp)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parser::qobject::tests::create_parsed_qobject;
    use crate::tests::tokens_to_syn;
    use quote::{format_ident, quote};
    use syn::ItemMod;

    /// The QObject ident used in these tests as the ident that already
    /// has been found.
    fn qobject_ident() -> Ident {
        format_ident!("MyObject")
    }

    /// Creates a ParsedCxxQtData with a QObject definition already found
    fn create_parsed_cxx_qt_data() -> ParsedCxxQtData {
        let mut cxx_qt_data = ParsedCxxQtData::default();
        cxx_qt_data
            .qobjects
            .insert(qobject_ident(), create_parsed_qobject());
        cxx_qt_data
    }

    #[test]
    fn test_find_qobjects_one_qobject() {
        let mut cxx_qt_data = ParsedCxxQtData::default();

        let module: ItemMod = tokens_to_syn(quote! {
            mod module {
                struct Other;
                #[cxx_qt::qobject]
                struct MyObject;
            }
        });
        let result = cxx_qt_data.find_qobject_structs(&module.content.unwrap().1);
        assert!(result.is_ok());
        assert_eq!(cxx_qt_data.qobjects.len(), 1);
        assert!(cxx_qt_data.qobjects.contains_key(&qobject_ident()));
    }

    #[test]
    fn test_find_qobjects_multiple_qobject() {
        let mut cxx_qt_data = ParsedCxxQtData::default();

        let module: ItemMod = tokens_to_syn(quote! {
            mod module {
                struct Other;
                #[cxx_qt::qobject]
                struct MyObject;
                #[cxx_qt::qobject]
                struct SecondObject;
            }
        });
        let result = cxx_qt_data.find_qobject_structs(&module.content.unwrap().1);
        assert!(result.is_ok());
        assert_eq!(cxx_qt_data.qobjects.len(), 2);
        assert!(cxx_qt_data.qobjects.contains_key(&qobject_ident()));
        assert!(cxx_qt_data
            .qobjects
            .contains_key(&format_ident!("SecondObject")));
    }

    #[test]
    fn test_find_qobjects_no_macro() {
        let mut cxx_qt_data = ParsedCxxQtData::default();

        let module: ItemMod = tokens_to_syn(quote! {
            mod module {
                struct Other;
                struct MyObject;
            }
        });
        let result = cxx_qt_data.find_qobject_structs(&module.content.unwrap().1);
        assert!(result.is_ok());
        assert_eq!(cxx_qt_data.qobjects.len(), 0);
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_enum_valid_signals() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            #[cxx_qt::qsignals(MyObject)]
            enum MySignals {
                Ready,
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_none());
        assert!(cxx_qt_data.qobjects[&qobject_ident()].signals.is_some());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_enum_unknown_qobject() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        // Valid signals enum but missing QObject
        let item: Item = tokens_to_syn(quote! {
            #[cxx_qt::qsignals(UnknownObj)]
            enum MySignals {
                Ready,
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_enum_passthrough() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            enum MySignals {
                Ready,
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_enum_error() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            #[cxx_qt::qsignals]
            enum MySignals {
                Ready,
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_struct_qobject() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            #[cxx_qt::qobject]
            struct MyObject;
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_struct_passthrough() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            struct Unknown;
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_impl_valid_qobject() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            impl qobject::MyObject {
                #[qinvokable]
                fn invokable() {}

                fn cpp_context() {}
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_none());
        assert_eq!(cxx_qt_data.qobjects[&qobject_ident()].invokables.len(), 1);
        assert_eq!(cxx_qt_data.qobjects[&qobject_ident()].methods.len(), 1);
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_impl_invalid_qobject() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            impl qobject::MyObject::Bad {
                #[qinvokable]
                fn invokable() {}
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_impl_unknown_qobject() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            impl qobject::UnknownObj {
                #[qinvokable]
                fn invokable() {}
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_impl_valid_rustobj() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            impl MyObject {
                fn method() {}
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_none());
        assert_eq!(cxx_qt_data.qobjects[&qobject_ident()].others.len(), 1);
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_uses() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            use std::collections::HashMap;
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_none());
        assert_eq!(cxx_qt_data.uses.len(), 1);
    }

    #[test]
    fn test_find_and_merge_cxx_qt_item_passthrough() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            extern "Rust" {
                fn test();
            }
        });
        let result = cxx_qt_data.parse_cxx_qt_item(item).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_cxx_names_map_empty() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            extern "C++" {
                type A;
            }
        });
        assert!(cxx_qt_data.parse_cxx_names_map(&item, "").is_ok());
        assert!(cxx_qt_data.cxx_names_map.is_empty());
    }

    #[test]
    fn test_cxx_names_map_normal() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            extern "C++" {
                #[cxx_name = "B"]
                type A;
            }
        });
        assert!(cxx_qt_data.parse_cxx_names_map(&item, "").is_ok());
        assert_eq!(cxx_qt_data.cxx_names_map.len(), 1);
        assert_eq!(cxx_qt_data.cxx_names_map.get("A").unwrap(), "B");
    }

    #[test]
    fn test_cxx_names_map_verbatim() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            unsafe extern "C++" {
                #[cxx_name = "B"]
                type A = C;
            }
        });
        assert!(cxx_qt_data.parse_cxx_names_map(&item, "").is_ok());
        assert_eq!(cxx_qt_data.cxx_names_map.len(), 1);
        assert_eq!(cxx_qt_data.cxx_names_map.get("A").unwrap(), "B");
    }

    #[test]
    fn test_cxx_names_map_namespace_bridge() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            extern "C++" {
                type A;

                #[cxx_name = "C"]
                type B;
            }
        });
        assert!(cxx_qt_data
            .parse_cxx_names_map(&item, "bridge_namespace")
            .is_ok());
        assert_eq!(cxx_qt_data.cxx_names_map.len(), 2);
        assert_eq!(
            cxx_qt_data.cxx_names_map.get("A").unwrap(),
            "::bridge_namespace::A"
        );
        assert_eq!(
            cxx_qt_data.cxx_names_map.get("B").unwrap(),
            "::bridge_namespace::C"
        );
    }

    #[test]
    fn test_cxx_names_map_namespace_items() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            #[namespace = "extern_namespace"]
            extern "C++" {
                type A;

                #[namespace = "type_namespace"]
                type B;
            }
        });
        // Also ensure item namespace is chosen instead of bridge namespace
        assert!(cxx_qt_data.parse_cxx_names_map(&item, "namespace").is_ok());
        assert_eq!(cxx_qt_data.cxx_names_map.len(), 2);
        assert_eq!(
            cxx_qt_data.cxx_names_map.get("A").unwrap(),
            "::extern_namespace::A"
        );
        assert_eq!(
            cxx_qt_data.cxx_names_map.get("B").unwrap(),
            "::type_namespace::B"
        );
    }

    #[test]
    fn test_cxx_names_map_normal_namespace_cxx_name() {
        let mut cxx_qt_data = create_parsed_cxx_qt_data();

        let item: Item = tokens_to_syn(quote! {
            #[namespace = "extern_namespace"]
            extern "C++" {
                #[cxx_name = "B"]
                #[namespace = "type_namespace"]
                type A;

                #[cxx_name = "D"]
                type C;
            }
        });
        assert!(cxx_qt_data.parse_cxx_names_map(&item, "").is_ok());
        assert_eq!(cxx_qt_data.cxx_names_map.len(), 2);
        assert_eq!(
            cxx_qt_data.cxx_names_map.get("A").unwrap(),
            "::type_namespace::B"
        );
        assert_eq!(
            cxx_qt_data.cxx_names_map.get("C").unwrap(),
            "::extern_namespace::D"
        );
    }
}
