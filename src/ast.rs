
#[derive(Clone, Debug)]
pub struct Ast {
    pub functions: Vec<Function>,
}

impl Ast {
    pub fn new() -> Ast {
        Ast {
            functions: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
}
