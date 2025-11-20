use iter_ext::IterTupleRefExt;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::HashMap;
use syn::Type;

pub struct Afts {
    pub by_tyn: HashMap<String, (String, Type)>,
    pub e: TokenStream,
}

impl Afts {
    pub fn by_ty(&self, ty: &Type) -> (&str, String) {
        let name = format!("{}", ty.to_token_stream());
        let found = self.by_tyn.get(&name).unwrap();
        (found.0.as_str(), name)
    }
}

impl FromIterator<Type> for Afts {
    fn from_iter<I: IntoIterator<Item = Type>>(iter: I) -> Self {
        let tys = iter.into_iter();

        let mut utys = Vec::from_iter(tys.map(|ty| (format!("{}", ty.to_token_stream()), ty)));
        utys.sort_by(|a, b| a.0.cmp(&b.0));
        utys.dedup_by(|a, b| a.0 == b.0);

        let e = utys
            .iter()
            .a()
            .any(|tyn| tyn.contains("'"))
            .then(|| quote! { <'e> })
            .unwrap_or_default();

        let by_tyn = utys
            .into_iter()
            .enumerate()
            .map(|(i, (tyn, ty))| (tyn, (format!("_A{}", i), ty)))
            .collect();

        Self { by_tyn, e }
    }
}
