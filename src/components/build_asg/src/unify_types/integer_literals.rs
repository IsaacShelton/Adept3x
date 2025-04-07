use asg::{Type, TypeKind};
use data_units::BitUnits;
use primitives::IntegerSign;

pub fn integer_literals_all_fit<'a>(
    preferred_type: Option<&Type>,
    mut types: impl Iterator<Item = &'a Type>,
) -> bool {
    let Some(Type {
        kind: TypeKind::Integer(preferred_bits, preferred_sign),
        ..
    }) = preferred_type
    else {
        return false;
    };

    types.all(|ty| match &ty.kind {
        TypeKind::IntegerLiteral(value) => {
            let literal_sign = IntegerSign::from(value);

            let literal_bits = BitUnits::of(match literal_sign {
                IntegerSign::Unsigned => value.bits(),
                IntegerSign::Signed => value.bits() + 1,
            });

            (preferred_sign.is_signed() || literal_sign.is_unsigned())
                && literal_bits <= preferred_bits.bits()
        }
        _ => false,
    })
}
