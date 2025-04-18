#[derive(Copy, Clone, Debug, Default)]
pub struct IsEmptyRecordOptions {
    pub allow_arrays: bool,
    pub as_if_no_unique_addr: bool,
}

pub fn is_empty_record(
    ty: &ir::Type,
    ir_module: &ir::Module,
    options: IsEmptyRecordOptions,
) -> bool {
    let fields = match ty.struct_fields(ir_module) {
        Some(fields) => fields,
        None => return false,
    };

    for field in fields.iter() {
        let occupied = !is_empty_field(&field.ir_type, ir_module, options);

        if occupied {
            return false;
        }
    }

    true
}

pub fn is_empty_field(
    field: &ir::Type,
    ir_module: &ir::Module,
    options: IsEmptyRecordOptions,
) -> bool {
    /*
    // NOTE: TODO: Once we add bitfields, we need to keep this in mind
    if is_unnamed_bit_field() {
        return true;
    }
    */

    let has_no_unique_address = false;
    let mut field = field;
    let mut was_array = false;

    // Strip off arrays if applicable
    if options.allow_arrays {
        while let ir::Type::FixedArray(fixed_array) = field {
            if fixed_array.length == 0 {
                return true;
            }
            was_array = true;
            field = &fixed_array.inner;
        }
    }

    if !(field.is_struct() || field.is_anonymous_composite()) {
        return false;
    }

    // NOTE: According to the Itanium ABI, C++ record fields are never empty,
    // unless they are also marked as [[no_unique_address]]
    if was_array || (!options.as_if_no_unique_addr && !has_no_unique_address) {
        return false;
    }

    is_empty_record(field, ir_module, options)
}
