use crate::{ATTR_HOOK, Elt};
use syn::{Attribute, Ident, Meta, Type, parse};

pub fn inspect_attrs<'a, 'b, 'c>(
    req: &'a mut Elt,
    item_ident: &'b Ident,
    item_attrs: &'c mut Vec<Attribute>,
) {
    let attrs = std::mem::take(item_attrs);
    let mut left = Vec::new();

    for attr in attrs {
        let segments = &attr.meta.path().segments;
        let Some((ns, dtv)) = segments.get(0).zip(segments.get(1)) else {
            left.push(attr);
            continue;
        };

        if format!("{}", ns.ident) != ATTR_HOOK {
            left.push(attr);
            continue;
        }

        match format!("{}", dtv.ident).as_str() {
            "returns" => {
                let Meta::List(ml) = &attr.meta else {
                    panic!(
                        "Expected type for #[{}::{}(...)] on {}",
                        ATTR_HOOK, dtv.ident, item_ident
                    );
                };

                let Ok(ty) = parse::<Type>(ml.tokens.clone().into()) else {
                    panic!(
                        "Failed to parse type for #[{}::{}(...)] on {}",
                        ATTR_HOOK, dtv.ident, item_ident
                    );
                };

                req.aft = Some(ty);
            }
            "impure" => {
                let Meta::Path(_) = attr.meta else {
                    panic!(
                        "Extra data cannot be specified for #[{}::{}] on {}",
                        ATTR_HOOK, dtv.ident, item_ident
                    );
                };
                req.pure = false;
            }
            "never_persist" => {
                let Meta::Path(_) = attr.meta else {
                    panic!(
                        "Extra data cannot be specified for #[{}::{}] on {}",
                        ATTR_HOOK, dtv.ident, item_ident
                    );
                };
                req.persist = false;
            }
            _ => panic!("Unrecognized directive {} in {}", dtv.ident, ATTR_HOOK),
        }
    }

    *item_attrs = left;
}
