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
use syn::{Ident, Item, ItemMod, Type, parse, parse_macro_input, parse_quote};

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
    let mut all_req = TokenStream::new();
    let mut req_wrap = TokenStream::new();
    let mut st_wrap = TokenStream::new();
    let mut st_unwrap = TokenStream::new();

    let any_st_e = pairs
        .iter()
        .b()
        .map(|st| st.e())
        .find(|e| !e.is_empty())
        .unwrap_or_default();

    let any_req_e = pairs
        .iter()
        .a()
        .map(|req| req.e())
        .find(|e| !e.is_empty())
        .unwrap_or_default();

    for (req, st) in pairs.iter() {
        let req_ident = &req.ident;
        let req_e = req.e();
        let st_ident = &st.ident;
        let st_e = st.e();
        let st_a = st.a();
        all_st.extend(quote! { #req_ident(#st_ident #st_e), });
        all_req.extend(quote! { #req_ident(#req_ident #req_e), });
        req_wrap.extend(quote! {
            impl #any_req_e From<#req_ident #req_e> for Req #any_req_e {
                fn from(value: #req_ident #req_e) -> Self {
                    Self::#req_ident(value)
                }
            }
        });
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
                    if st.is_default() {
                        *st = St::#req_ident(Default::default());
                    }

                    match st {
                        St::#req_ident(st) => st,
                        _ => panic!("unwrap_st failed!"),
                    }
                }
            }
        });
    }

    let reqs = TokenStream::from_iter(pairs.iter_mut().a().flat_map(|req| {
        let mut item = req.item.clone();

        match &mut item {
            Item::Enum(item_enum) => {
                inspect_attrs(req, &item_enum.ident, &mut item_enum.attrs);
                item_enum
                    .attrs
                    .push(parse_quote!(#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]));
                item.to_token_stream()
            }
            Item::Struct(item_struct) => {
                inspect_attrs(req, &item_struct.ident, &mut item_struct.attrs);
                item_struct
                    .attrs
                    .push(parse_quote!(#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]));
                item.to_token_stream()
            }
            _ => item.to_token_stream(),
        }
    }));

    let sts = TokenStream::from_iter(pairs.iter_mut().b().map(|st| match &mut st.item {
        Item::Enum(item_enum) => {
            item_enum.attrs.push(parse_quote!(#[derive(Debug)]));
            st.item.to_token_stream()
        }
        Item::Struct(item_struct) => {
            item_struct.attrs.push(parse_quote!(#[derive(Debug)]));
            st.item.to_token_stream()
        }
        _ => st.item.to_token_stream(),
    }));
    let rest = TokenStream::from_iter(rest.iter().map(ToTokens::to_token_stream));

    let afts = Afts::from_iter(pairs.iter().a().map(|req| match &req.aft {
        Some(aft) => aft.clone(),
        None => panic!("Missing aft for {}", req.ident),
    }));

    let any_aft_short_e = if afts.e.is_empty() {
        afts.e.clone()
    } else {
        quote! { , 'e }
    };

    let all_afts = TokenStream::from_iter(afts.by_tyn.values().map(|(name, ty)| {
        let name = Ident::new(name, Span::call_site());
        quote! { #name(#ty), }
    }));

    let aft_wrap = TokenStream::from_iter(afts.by_tyn.values().map(|(name, ty)| {
        let name = Ident::new(name, Span::call_site());
        quote! {
            impl<P: Pf> From<#ty> for Aft<P #any_aft_short_e> {
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
            impl<'e, P: Pf> UnwrapAft<'e, P> for #req_ident #req_e {
                type Aft<'a> = #aft_ty_in_a;
                fn unwrap_aft(aft: Aft<P #any_aft_short_e>) -> Self::Aft<'e> {
                    match aft {
                        Aft::#aft_ident(aft) => aft,
                        _ => panic!("unwrap_aft failed!"),
                    }
                }
                fn as_aft(aft: &Aft<P #any_aft_short_e>) -> Option<&Self::Aft<'e>> {
                    match aft {
                        Aft::#aft_ident(aft) => Some(aft),
                        _ => None,
                    }
                }
            }
        }
    }));

    let mut impure_arms = TokenStream::new();
    let mut should_persist_arms = TokenStream::new();
    let mut run_dispatch_arms = TokenStream::new();
    for req in pairs.iter().a() {
        let boolean = |condition| {
            if condition {
                quote! { true }
            } else {
                quote! { false }
            }
        };

        let ident = &req.ident;

        let value = boolean(req.pure);
        impure_arms.extend(quote! {
            Self::#ident(..) => #value,
        });

        let value = boolean(req.persist);
        should_persist_arms.extend(quote! {
            Self::#ident(..) => #value,
        });

        run_dispatch_arms.extend(quote! {
            Req::#ident(req) => req.run(
                aft.and_then(|aft| #ident::as_aft(aft)),
                st,
                th
            ).map(|aft| P::Aft::un_like(Aft::from(aft))),
        });
    }

    quote! {
        mod requests {
            use ::serde::{Serialize, Deserialize};
            #rest
            #[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
            pub enum Req #any_req_e {
                #all_req
            }
            #[derive(Debug, Default)]
            pub enum St #any_st_e {
                #[default]
                Initial,
                #all_st
            }
            pub trait IsDefault {
                fn is_default(&self) -> bool;
            }
            impl<#any_st_e> IsDefault for St #any_st_e {
                fn is_default(&self) -> bool {
                    matches!(self, Self::Initial)
                }
            }
            pub trait UnwrapSt<'e> {
                type St<'a>: Default;
                fn unwrap_st(st: &mut St #any_st_e) -> &mut Self::St<'e>;
            }
            #sts
            #st_wrap
            #st_unwrap
            #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
            pub enum Aft<P: Pf #any_aft_short_e> {
                #all_afts
            }
            pub trait UnwrapAft<'e, P: Pf> {
                type Aft<'a>;
                fn unwrap_aft(aft: Aft<P #any_aft_short_e>) -> Self::Aft<'e>;
                fn as_aft(aft: &Aft<P #any_aft_short_e>) -> Option<&Self::Aft<'e>>;
            }
            #aft_wrap
            #aft_unwrap
            pub trait RunDispatch<'e, P: Pf> {
                fn run_dispath(&self, aft: Option<&Aft<P>>, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<P::Aft<'e>, Suspend>;
            }
            pub trait Run<'e, P: Pf>: UnwrapSt<'e> + UnwrapAft<'e, P> {
                fn run(&self, aft: Option<&Self::Aft<'e>>, st: &mut P::St<'e>, th: &mut impl Th<'e, P>) -> Result<Self::Aft<'e>, Suspend>;
            }
            pub struct Suspend;
            #[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
            pub struct PfIn;
            impl Pf for PfIn {
                type Rev = Rev;
                type Req<'e> = Req #any_req_e;
                type Aft<'e> = Aft<Self #any_aft_short_e>;
                type St<'e> = St #any_st_e;
            }
            #reqs
            #req_wrap
            pub trait IsImpure {
                fn is_impure(&self) -> bool;
            }
            pub trait ShouldPersist {
                fn should_persist(&self) -> bool;
            }
            impl #any_req_e IsImpure for Req #any_req_e {
                fn is_impure(&self) -> bool {
                    match self {
                        #impure_arms
                    }
                }
            }
            impl #any_req_e ShouldPersist for Req #any_req_e {
                fn should_persist(&self) -> bool {
                    match self {
                        #should_persist_arms
                    }
                }
            }
            impl<'e, P: Pf> RunDispatch<'e, P> for P::Req<'e>
            where
                P::Req<'e>: Like<Req #any_req_e>,
                P::Aft<'e>: UnLike<Aft<P>>,
            {
                fn run_dispath(
                    &self,
                    aft: Option<&Aft<P>>,
                    st: &mut P::St<'e>,
                    th: &mut impl Th<'e, P>,
                ) -> Result<P::Aft<'e>, Suspend> {
                    match self.like_ref() {
                        #run_dispatch_arms
                    }
                }
            }
        }
    }
    .into()
}
