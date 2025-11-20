use crate::{AFT_DTV, ATTR_HOOK, Elt};
use syn::{Attribute, Ident, Meta, Type, parse};

pub fn inspect_attrs<'a, 'b, 'c>(
    req: &'a mut Elt,
    item_ident: &'b Ident,
    item_attrs: &'c mut Vec<Attribute>,
) {
    let attrs = std::mem::take(item_attrs);
    let mut left = Vec::new();

    for attr in attrs {
        let Meta::List(ml) = attr.meta else {
            left.push(attr);
            continue;
        };

        let segments = &ml.path.segments;
        let Some((ns, dtv)) = segments.get(0).zip(segments.get(1)) else {
            continue;
        };

        if format!("{}", ns.ident) != ATTR_HOOK {
            continue;
        }

        match format!("{}", dtv.ident).as_str() {
            AFT_DTV => {
                let Ok(ty) = parse::<Type>(ml.tokens.into()) else {
                    panic!(
                        "Failed to parse type for #[{}::{}(...)] on {}",
                        ATTR_HOOK, AFT_DTV, item_ident
                    );
                };

                req.aft = Some(ty);
            }
            _ => panic!("Unrecognized directive in {}", ATTR_HOOK),
        }
    }

    *item_attrs = left;
}
