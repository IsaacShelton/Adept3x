use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use std::collections::HashMap;
use syn::{Generics, Ident, Item};

const STATE_SUFFIX: &'static str = "State";
const ATTRIBUTE_HOOK: &'static str = "define_requests";

struct SpecialItem {
    request_name: String,
    ident: Ident,
    generics: Generics,
    is_state: bool,
    item: Item,
}

impl SpecialItem {
    pub fn new(ident: Ident, generics: Generics, item: Item) -> Self {
        let name = format!("{}", ident);
        Self {
            request_name: name.strip_suffix(STATE_SUFFIX).unwrap_or(&name).into(),
            is_state: name.ends_with(STATE_SUFFIX),
            ident,
            generics,
            item,
        }
    }

    pub fn env(&self) -> TokenStream {
        self.generics
            .lt_token
            .map(|_| quote! { <'env> })
            .unwrap_or_default()
    }

    pub fn a(&self) -> TokenStream {
        self.generics
            .lt_token
            .map(|_| quote! { <'a> })
            .unwrap_or_default()
    }
}

#[proc_macro_attribute]
pub fn group(_attrs: TokenStream1, input: TokenStream1) -> TokenStream1 {
    let input_mod = syn::parse_macro_input!(input as syn::ItemMod);
    let content = input_mod.content.unwrap().1;

    let mut special_items = Vec::new();
    let mut rest = Vec::new();

    for item in content {
        special_items.push(match item {
            syn::Item::Enum(item_enum) => SpecialItem::new(
                item_enum.ident.clone(),
                item_enum.generics.clone(),
                item_enum.into(),
            ),
            syn::Item::Struct(item_struct) => SpecialItem::new(
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

    let pairs: Vec<(SpecialItem, SpecialItem)> = pair_up(special_items);
    let any_state_env = pairs
        .iter()
        .flat_map(|(_, state)| state.generics.lt_token.map(|_| quote! { <'env> }))
        .next()
        .unwrap_or_default();

    let mut all_states = TokenStream::new();
    let mut req_state_wrap = TokenStream::new();
    let mut req_state_unwrap = TokenStream::new();

    for (req, state) in pairs.iter() {
        let req_ident = &req.ident;
        let req_env = req.env();
        let state_ident = &state.ident;
        let state_env = state.env();
        let state_a = state.a();
        all_states.extend(quote! { #req_ident(#state_ident #state_env), });
        req_state_wrap.extend(quote! {
            impl #any_state_env From<#state_ident #state_env> for ReqState #any_state_env {
                fn from(value: #state_ident #state_env) -> Self {
                    Self::#req_ident(value)
                }
            }
        });
        req_state_unwrap.extend(quote! {
            impl<'env> UnwrapReqState<'env> for #req_ident #req_env {
                type Unwrapped<'a> = #state_ident #state_a;
                fn unwrap_req_state(req_state: ReqState #any_state_env) -> Self::Unwrapped<'env> {
                    match req_state {
                        ReqState::#req_ident(state) => state,
                        _ => panic!("unwrap_req_state failed!"),
                    }
                }
            }
        });
    }

    let mut returns = Vec::new();
    let requests = TokenStream::from_iter(pairs.iter().map(|(req, _)| {
        let mut skinned_item = req.item.clone();
        match &mut skinned_item {
            Item::Enum(item_enum) => {
                process_attrs(req, &item_enum.ident, &mut item_enum.attrs, &mut returns)
            }
            Item::Struct(item_struct) => process_attrs(
                req,
                &item_struct.ident,
                &mut item_struct.attrs,
                &mut returns,
            ),
            _ => (),
        }
        skinned_item.to_token_stream()
    }));

    let states = TokenStream::from_iter(pairs.iter().map(|(_, st)| st.item.to_token_stream()));
    let rest = TokenStream::from_iter(rest.iter().map(ToTokens::to_token_stream));

    let mut unique_types = Vec::from_iter(
        returns
            .iter()
            .map(|(_item, ty)| (format!("{}", ty.to_token_stream()), ty)),
    );
    unique_types.sort_by(|a, b| a.0.cmp(&b.0));
    unique_types.dedup_by(|a, b| a.0 == b.0);

    let any_artifact_env = unique_types
        .iter()
        .any(|(name, _)| name.contains("'"))
        .then(|| quote! { <'env> })
        .unwrap_or_default();

    let artifacts = Vec::from_iter(
        unique_types
            .into_iter()
            .enumerate()
            .map(|(i, (_, ty))| (ty, format!("_Artifact{}", i))),
    );
    let artifact_to_type =
        HashMap::<&str, &syn::Type>::from_iter(artifacts.iter().map(|(a, b)| (b.as_str(), *a)));

    let mut all_artifacts = TokenStream::new();
    for (ty, name) in artifacts.iter() {
        let name = Ident::new(name, Span::call_site());
        all_artifacts.extend(quote! {
            #name(#ty),
        });
    }

    quote! {
        mod requests {
            #rest
            pub enum ReqState #any_state_env {
                #all_states
            }
            pub enum Artifact #any_artifact_env {
                #all_artifacts
            }
            pub trait UnwrapReqState<'env> {
                type Unwrapped<'a>;
                fn unwrap_req_state(req_state: ReqState #any_state_env) -> Self::Unwrapped<'env>;
            }
            #requests
            #states
            #req_state_wrap
            #req_state_unwrap
        }
    }
    .into()
}

fn pair_up(special_items: Vec<SpecialItem>) -> Vec<(SpecialItem, SpecialItem)> {
    let mut pairs = Vec::with_capacity(special_items.len());
    let mut needs_state = HashMap::new();
    let mut needs_request = HashMap::new();

    for item in special_items {
        if item.is_state {
            if let Some(request) = needs_request.remove(&item.request_name) {
                pairs.push((request, item));
            } else {
                needs_state.insert(item.request_name.clone(), item);
            }
        } else {
            if let Some(state) = needs_state.remove(&item.request_name) {
                pairs.push((item, state));
            } else {
                needs_request.insert(item.request_name.clone(), item);
            }
        }
    }

    let missing = Vec::from_iter(needs_state.values().chain(needs_request.values()));
    if !missing.is_empty() {
        let mut missing = Vec::from_iter(
            missing
                .iter()
                .map(|special_item| &special_item.request_name),
        );
        missing.sort();
        panic!("Incomplete pairs - {:?}", missing);
    }

    pairs
}

fn process_attrs<'a, 'b, 'c>(
    request: &'a SpecialItem,
    item_ident: &'b Ident,
    item_attrs: &'c mut Vec<syn::Attribute>,
    returns: &mut Vec<(&'a SpecialItem, syn::Type)>,
) {
    let attrs = std::mem::take(item_attrs);
    let mut untouched = Vec::new();

    for attr in attrs {
        match attr.meta {
            syn::Meta::List(meta_list) => {
                let segments = meta_list.path.segments;
                let Some((namespace, directive)) = segments.get(0).zip(segments.get(1)) else {
                    continue;
                };

                if format!("{}", namespace.ident) != ATTRIBUTE_HOOK {
                    continue;
                }

                match format!("{}", directive.ident).as_str() {
                    "returns" => {
                        let Ok(ty) = syn::parse::<syn::Type>(meta_list.tokens.into()) else {
                            panic!(
                                "Failed to parse type for {}::returns on {}",
                                ATTRIBUTE_HOOK, item_ident
                            );
                        };

                        returns.push((request, ty));
                    }
                    _ => panic!("Unrecognized directive in {}", ATTRIBUTE_HOOK),
                }
            }
            _ => untouched.push(attr),
        }
    }

    *item_attrs = untouched;
}
