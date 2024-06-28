use crate::ir;
use std::marker::PhantomData;

pub trait FieldsIter<'t>: ExactSizeIterator<Item = &'t ir::Field> + Clone + Sized {}
impl<'t, I> FieldsIter<'t> for I where I: ExactSizeIterator<Item = &'t ir::Field> + Clone + Sized {}

#[derive(Clone, Debug)]
pub struct RecordInfo<'t, F: FieldsIter<'t>> {
    pub fields_iter: F,
    pub is_packed: bool,
    phantom: PhantomData<&'t ()>,
}

pub fn info_from_structure<'t>(
    structure: &'t ir::Structure,
) -> RecordInfo<'t, impl FieldsIter<'t>> {
    let fields_iter = structure.fields.iter();

    RecordInfo {
        fields_iter,
        is_packed: structure.is_packed,
        phantom: PhantomData,
    }
}

pub fn info_from_composite<'t>(
    composite: &'t ir::TypeComposite,
) -> RecordInfo<'t, impl FieldsIter<'t>> {
    let fields_iter = composite.fields.iter();

    RecordInfo {
        fields_iter,
        is_packed: composite.is_packed,
        phantom: PhantomData,
    }
}

impl<'t, F: FieldsIter<'t>> RecordInfo<'t, F> {
    pub fn iter(&'t self) -> impl FieldsIter {
        self.fields_iter.clone()
    }

    pub fn len(&self) -> usize {
        self.fields_iter.len()
    }

    pub fn may_insert_extra_padding(&self, _thing: bool) -> bool {
        todo!("may_insert_extra_padding")
    }
}
