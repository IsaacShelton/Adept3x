use crate::PAIR_SUFFIX;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Generics, Item, Type};

pub struct Elt {
    pub req_name: String,
    pub ident: Ident,
    pub generics: Generics,
    pub is_st: bool,
    pub item: Item,
    pub aft: Option<Type>,
    pub pure: bool,
}

impl Elt {
    pub fn new(ident: Ident, generics: Generics, item: Item) -> Self {
        let name = format!("{}", ident);
        Self {
            req_name: name.strip_suffix(PAIR_SUFFIX).unwrap_or(&name).into(),
            is_st: name.ends_with(PAIR_SUFFIX),
            ident,
            generics,
            item,
            aft: None,
            pure: true,
        }
    }

    pub fn e(&self) -> TokenStream {
        self.generics
            .lt_token
            .map(|_| quote! { <'e> })
            .unwrap_or_default()
    }

    pub fn a(&self) -> TokenStream {
        self.generics
            .lt_token
            .map(|_| quote! { <'a> })
            .unwrap_or_default()
    }
}
