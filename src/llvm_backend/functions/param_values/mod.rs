mod helpers;
mod ignore;
mod inalloca;
mod indirect;
mod value;

use self::value::ParamValue;

pub struct ParamValues {
    values: Vec<ParamValue>,
}

impl ParamValues {
    pub fn new() -> Self {
        Self {
            values: Vec::<ParamValue>::with_capacity(16),
        }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ParamValue> {
        self.values.iter()
    }
}
