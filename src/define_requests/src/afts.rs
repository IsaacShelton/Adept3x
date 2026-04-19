use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::HashMap;
use syn::Type;

pub struct Afts {
    pub by_tyn: HashMap<String, (bool, String, Type)>,
    pub e: TokenStream,
    pub cached_e: TokenStream,
}

impl Afts {
    pub fn by_ty(&self, ty: &Type) -> (&str, String) {
        let name = format!("{}", ty.to_token_stream());
        let Some(found) = self.by_tyn.get(&name) else {
            panic!("by_ty expect found {} - {:?}", &name, &self.by_tyn);
        };

        (found.1.as_str(), name)
    }
}

impl FromIterator<(bool, Type)> for Afts {
    fn from_iter<I: IntoIterator<Item = (bool, Type)>>(iter: I) -> Self {
        let tys = iter.into_iter();

        let mut utys = Vec::from_iter(
            tys.map(|(persist, ty)| (persist, format!("{}", ty.to_token_stream()), ty)),
        );
        utys.sort_by(|a, b| a.1.cmp(&b.1));
        utys.dedup_by(|a, b| a.1 == b.1);

        let e = utys
            .iter()
            .map(|(_, name, _)| name)
            .any(|tyn| tyn.contains("'"))
            .then(|| quote! { <'e> })
            .unwrap_or_default();

        let cached_e = utys
            .iter()
            .filter(|(persist, _, _)| *persist)
            .map(|(_, name, _)| name)
            .any(|tyn| tyn.contains("'"))
            .then(|| quote! { <'e> })
            .unwrap_or_default();

        let by_tyn = utys
            .into_iter()
            .enumerate()
            .map(|(i, (persist, tyn, ty))| (tyn, (persist, format!("_A{}", i), ty)))
            .collect();

        Self {
            by_tyn,
            e,
            cached_e,
        }
    }
}
