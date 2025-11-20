mod afts;
mod attrs;
mod elt;
mod hooks;
mod pair;

pub(crate) use afts::*;
pub(crate) use attrs::*;
pub(crate) use elt::*;
pub(crate) use hooks::*;
use iter_ext::{IterTupleMutExt, IterTupleRefExt};
pub(crate) use pair::*;
use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use std::str::FromStr;
use syn::{Ident, Item, ItemMod, Type, parse, parse_macro_input};

#[proc_macro_attribute]
pub fn group(_attrs: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let input_mod = parse_macro_input!(input as ItemMod);
    let content = input_mod.content.unwrap().1;

    let mut elts = Vec::new();
    let mut rest = Vec::new();

    for item in content {
        elts.push(match item {
            Item::Enum(item_enum) => Elt::new(
                item_enum.ident.clone(),
                item_enum.generics.clone(),
                item_enum.into(),
            ),
            Item::Struct(item_struct) => Elt::new(
                item_struct.ident.clone(),
                item_struct.generics.clone(),
                item_struct.into(),
            ),
            _ => {
                rest.push(item);
                continue;
            }
        });
    }

    let mut pairs = pair_up(elts);
    let mut all_st = TokenStream::new();
    let mut st_wrap = TokenStream::new();
    let mut st_unwrap = TokenStream::new();

    let any_st_e = pairs
        .iter()
        .b()
        .map(|st| st.e())
        .find(|e| !e.is_empty())
        .unwrap_or_default();

    for (req, st) in pairs.iter() {
        let req_ident = &req.ident;
        let req_e = req.e();
        let st_ident = &st.ident;
        let st_e = st.e();
        let st_a = st.a();
        all_st.extend(quote! { #req_ident(#st_ident #st_e), });
        st_wrap.extend(quote! {
            impl #any_st_e From<#st_ident #st_e> for St #any_st_e {
                fn from(value: #st_ident #st_e) -> Self {
                    Self::#req_ident(value)
                }
            }
        });
        st_unwrap.extend(quote! {
            impl<'e> UnwrapSt<'e> for #req_ident #req_e {
                type St<'a> = #st_ident #st_a;
                fn unwrap_st(st: &mut St #any_st_e) -> &mut Self::St<'e> {
                    match st {
                        St::#req_ident(st) => st,
                        _ => panic!("unwrap_st failed!"),
                    }
                }
            }
        });
    }

    let reqs = TokenStream::from_iter(pairs.iter_mut().a().map(|req| {
        let mut item = req.item.clone();
        match &mut item {
            Item::Enum(item_enum) => {
                inspect_attrs(req, &item_enum.ident, &mut item_enum.attrs);
            }
            Item::Struct(item_struct) => {
                inspect_attrs(req, &item_struct.ident, &mut item_struct.attrs);
            }
            _ => (),
        }
        item.to_token_stream()
    }));

    let sts = TokenStream::from_iter(pairs.iter().b().map(|st| st.item.to_token_stream()));
    let rest = TokenStream::from_iter(rest.iter().map(ToTokens::to_token_stream));

    let afts = Afts::from_iter(pairs.iter().a().map(|req| match &req.aft {
        Some(aft) => aft.clone(),
        None => {
            assert!(!req.is_st);
            panic!("Missing aft for {}", req.ident)
        }
    }));

    let any_aft_e = &afts.e;

    let all_afts = TokenStream::from_iter(afts.by_tyn.values().map(|(name, ty)| {
        let name = Ident::new(name, Span::call_site());
        quote! { #name(#ty), }
    }));

    let aft_wrap = TokenStream::from_iter(afts.by_tyn.values().map(|(name, ty)| {
        let name = Ident::new(name, Span::call_site());
        quote! {
            impl From<#ty> for Aft {
                fn from(value: #ty) -> Self {
                    Self::#name(value)
                }
            }
        }
    }));

    let aft_unwrap = TokenStream::from_iter(pairs.iter().a().map(|req| {
        let req_ident = &req.ident;
        let req_e = req.e();

        let Some(aft_ty) = &req.aft else {
            panic!("Missing #[{}::returns(...)] for {}", ATTR_HOOK, req.ident);
        };

        let (aft_ident, tyn) = afts.by_ty(&aft_ty);
        let aft_ident = Ident::new(aft_ident, Span::call_site());
        let aft_ty_in_a = parse::<Type>(TokenStream1::from_str(&tyn.replace("'e", "'a")).unwrap())
            .expect("aft_ty_in_a");

        quote! {
            impl<'e> UnwrapAft<'e> for #req_ident #req_e {
                type Aft<'a> = #aft_ty_in_a;
                fn unwrap_aft(aft: Aft #any_aft_e) -> Self::Aft<'e> {
                    match aft {
                        Aft::#aft_ident(aft) => aft,
                        _ => panic!("unwrap_aft failed!"),
                    }
                }
            }
        }
    }));

    quote! {
        mod requests {
            #rest
            pub enum St #any_st_e {
                #all_st
            }
            pub trait UnwrapSt<'e> {
                type St<'a>;
                fn unwrap_st(st: &mut St #any_st_e) -> &mut Self::St<'e>;
            }
            #reqs
            #sts
            #st_wrap
            #st_unwrap
            pub enum Aft #any_aft_e {
                #all_afts
            }
            pub trait UnwrapAft<'e> {
                type Aft<'a>;
                fn unwrap_aft(aft: Aft #any_aft_e) -> Self::Aft<'e>;
            }
            #aft_wrap
            #aft_unwrap
            pub trait Run<'e>: UnwrapSt<'e> + UnwrapAft<'e> {
                fn run(&self, st: &mut Self::St<'e>, th: &mut impl Th) -> Result<Self::Aft<'e>, Suspend>;
            }
            pub trait RunA<'e>: Run<'e> where Aft: From<<Self as UnwrapAft<'e>>::Aft<'e>> {
                fn run_a(&self, st: &mut St #any_st_e, th: &mut impl Th) -> Result<Aft #any_aft_e, Suspend> {
                    self.run(Self::unwrap_st(st), th).map(|aft| aft.into())
                }
            }
            struct Suspend;
            trait Th {}
        }
    }
    .into()
}
