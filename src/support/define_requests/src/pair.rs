use crate::Elt;
use std::collections::HashMap;

pub fn pair_up(elts: Vec<Elt>) -> Vec<(Elt, Elt)> {
    let mut pairs = Vec::with_capacity(elts.len());
    let mut needs_st = HashMap::new();
    let mut needs_req = HashMap::new();

    for item in elts {
        if item.is_st {
            if let Some(req) = needs_req.remove(&item.req_name) {
                pairs.push((req, item));
            } else {
                needs_st.insert(item.req_name.clone(), item);
            }
        } else {
            if let Some(st) = needs_st.remove(&item.req_name) {
                pairs.push((item, st));
            } else {
                needs_req.insert(item.req_name.clone(), item);
            }
        }
    }

    let missing = Vec::from_iter(needs_st.values().chain(needs_req.values()));

    if !missing.is_empty() {
        let mut missing = Vec::from_iter(missing.iter().map(|elt| &elt.req_name));
        missing.sort();
        panic!("Incomplete pairs - {:?}", missing);
    }

    pairs
}
