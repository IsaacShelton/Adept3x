use super::empty::{is_empty_record, IsEmptyRecordOptions};
use crate::{ir, target::type_layout::TypeLayoutCache};

#[derive(Copy, Clone, Debug)]
pub struct HomoAggregate<'a> {
    pub base: &'a ir::Type,
    pub num_members: u32,
}

pub trait HomoDecider {
    fn is_base_type(&self, ir_type: &ir::Type, type_layout_cache: &TypeLayoutCache) -> bool;
    fn is_small_enough(&self, homo_aggregate: &HomoAggregate<'_>) -> bool;
}

pub fn is_homo_aggregate<'a>(
    decider: &impl HomoDecider,
    ir_type: &'a ir::Type,
    ir_module: &'a ir::Module,
    existing_base: Option<&'a ir::Type>,
    type_layout_cache: &TypeLayoutCache,
) -> Option<HomoAggregate<'a>> {
    let homo_aggregate = match ir_type {
        ir::Type::FixedArray(fixed_array) => {
            if fixed_array.length == 0 {
                return None;
            }

            is_homo_aggregate(
                decider,
                &fixed_array.inner,
                ir_module,
                existing_base,
                type_layout_cache,
            )
            .map(|homo_aggregate| HomoAggregate {
                base: homo_aggregate.base,
                num_members: homo_aggregate.num_members
                    * u32::try_from(fixed_array.length).unwrap(),
            })
        }
        ir::Type::Structure(structure_ref) => {
            let structure = ir_module
                .structures
                .get(structure_ref)
                .expect("referenced structure to exist for is_homo_aggregate");

            is_homo_aggregate_record(
                decider,
                ir_type,
                ir_module,
                &structure.fields[..],
                existing_base,
                type_layout_cache,
            )
        }
        ir::Type::AnonymousComposite(anonymous_composite) => is_homo_aggregate_record(
            decider,
            ir_type,
            ir_module,
            &anonymous_composite.fields[..],
            existing_base,
            type_layout_cache,
        ),
        _ => {
            let (ir_type, num_members) = if let ir::Type::Complex(complex) = ir_type {
                (&complex.element_type, 2)
            } else {
                (ir_type, 1)
            };

            if !decider.is_base_type(ir_type, type_layout_cache) {
                return None;
            }

            let base = &ir_type;

            if existing_base.is_none() {
                if let ir::Type::Vector(vector) = base {
                    let element_type = &vector.element_type;
                    let num_elements = vector.num_elements;

                    assert_eq!(
                        num_elements,
                        type_layout_cache.get(base).width
                            / type_layout_cache.get(element_type).width,
                    );
                }

                if base.is_vector() != ir_type.is_vector()
                    || type_layout_cache.get(ir_type).width != type_layout_cache.get(base).width
                {
                    return None;
                }
            }

            Some(HomoAggregate { base, num_members })
        }
    };

    homo_aggregate.filter(|homo_aggregate| {
        homo_aggregate.num_members > 0 && decider.is_small_enough(homo_aggregate)
    })
}

fn is_homo_aggregate_record<'a>(
    decider: &impl HomoDecider,
    record_ir_type: &'a ir::Type,
    ir_module: &'a ir::Module,
    fields: &'a [ir::Field],
    existing_base: Option<&'a ir::Type>,
    type_layout_cache: &TypeLayoutCache,
) -> Option<HomoAggregate<'a>> {
    if record_ir_type.has_flexible_array_member() {
        return None;
    }

    let mut base = existing_base;
    let mut num_members = 0;

    // NOTE: We would need to check the bases as well if this was a C++ record type,
    // but we don't support those yet.

    for field in fields.iter() {
        let mut field = &field.ir_type;

        // Ignore fixed array modifiers
        while let ir::Type::FixedArray(fixed_array) = field {
            if fixed_array.length == 0 {
                return None;
            }

            field = &fixed_array.inner;
        }

        // Ignore empty records
        if is_empty_record(
            field,
            ir_module,
            IsEmptyRecordOptions {
                allow_arrays: true,
                ..Default::default()
            },
        ) {
            continue;
        }

        /*
        // NOTE: Once we support bit fields, we will need something like this:
        if is_zero_length_bitfield_allowed_in_homo_aggregrate() && is_zero_length_bitfield(field) {
            continue;
        }
        */

        if let Some(inner) = is_homo_aggregate(decider, field, ir_module, base, type_layout_cache) {
            // Update base type to new type found (it's the same as what we have, or we don't already have one)
            base = Some(inner.base);

            // Update minimum number of members required to store this record
            num_members = if record_ir_type.is_union() {
                num_members.max(inner.num_members)
            } else {
                num_members + inner.num_members
            };
        } else {
            return None;
        }
    }

    // If no base type found, then it's not a homogeneous aggregate record
    let base = base?;

    // If the record has tail padding, it's not a homogeneous aggregate record
    if type_layout_cache.get(base).width * num_members
        != type_layout_cache.get(record_ir_type).width
    {
        return None;
    }

    Some(HomoAggregate { base, num_members })
}
