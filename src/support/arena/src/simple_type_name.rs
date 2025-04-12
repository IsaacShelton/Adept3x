pub(crate) fn simple_type_name<T>() -> &'static str {
    let mut type_name = core::any::type_name::<T>();
    if let Some(idx) = type_name.rfind(':') {
        type_name = &type_name[idx + 1..]
    }
    type_name
}
