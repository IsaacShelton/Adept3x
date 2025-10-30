use interpreter_api::{ConstantValue, ConstantValueSchema};

#[derive(Debug)]
pub struct Evaluated {
    schema: ConstantValueSchema,
    value: ConstantValue,
}

impl Evaluated {
    pub fn new_boolean(whether: bool) -> Self {
        Self {
            schema: ConstantValueSchema::Boolean,
            value: ConstantValue::SmallData(whether as u64),
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match (&self.schema, &self.value) {
            (ConstantValueSchema::Boolean, ConstantValue::SmallData(value)) => Some(*value != 0),
            _ => None,
        }
    }
}
