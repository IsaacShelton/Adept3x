use interpreter_api::{ConstantValue, ConstantValueSchema};
use primitives::IntegerSign;

#[derive(Debug)]
pub struct Evaluated {
    pub schema: ConstantValueSchema,
    pub value: ConstantValue,
}

impl Evaluated {
    pub fn new_boolean(whether: bool) -> Self {
        Self {
            schema: ConstantValueSchema::Boolean,
            value: ConstantValue::SmallData(whether as u64),
        }
    }

    pub fn new_unsigned(value: u64) -> Self {
        Self {
            schema: ConstantValueSchema::Integer(IntegerSign::Unsigned),
            value: ConstantValue::SmallData(value),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match (&self.schema, &self.value) {
            (ConstantValueSchema::Boolean, ConstantValue::SmallData(value)) => Some(*value != 0),
            _ => None,
        }
    }
}
