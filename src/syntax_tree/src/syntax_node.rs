use crate::{BareSyntaxKind, BareSyntaxNode};
use derive_more::From;
use std::{fmt::Debug, sync::Arc};
use text_edit::{TextPointRangeUtf16, TextPointUtf16};

#[derive(Debug)]
pub struct SyntaxNode {
    pub(crate) bare: Arc<BareSyntaxNode>,
    pub(crate) parent: Option<Arc<SyntaxNode>>,
    pub(crate) start: TextPointUtf16,
}

impl SyntaxNode {
    pub fn new(
        parent: Option<Arc<Self>>,
        bare: Arc<BareSyntaxNode>,
        start: TextPointUtf16,
    ) -> Arc<Self> {
        Arc::new(Self {
            parent,
            bare,
            start,
        })
    }

    pub fn bare(&self) -> &Arc<BareSyntaxNode> {
        &self.bare
    }

    pub fn parent(&self) -> Option<&Arc<Self>> {
        self.parent.as_ref()
    }

    pub fn text_range(&self) -> TextPointRangeUtf16 {
        TextPointRangeUtf16::new(self.start, self.start + self.bare.text_point_diff_utf16)
    }

    pub fn children(self: &Arc<Self>) -> impl Iterator<Item = Arc<Self>> {
        self.bare
            .children
            .iter()
            .scan(self.start, |next_start, child| {
                let start = *next_start;
                *next_start += child.text_point_diff_utf16;
                Some((start, child))
            })
            .map(|(start, child)| Self::new(Some(self.clone()), child.clone(), start))
    }

    pub fn dump(
        self: &Arc<Self>,
        w: &mut impl std::io::Write,
        depth: usize,
    ) -> std::io::Result<()> {
        let padding = " ".repeat(depth * 2);

        match &self.bare.text {
            Some(leaf) => match &self.bare.kind {
                BareSyntaxKind::ColumnSpacing(_) | BareSyntaxKind::LineSpacing(_) => {
                    writeln!(w, "{}{:?}", padding, self.bare.kind)?;
                }
                _ => {
                    writeln!(w, "{}{:?}: `{}`", padding, self.bare.kind, leaf)?;
                }
            },
            None => {
                writeln!(w, "{}{:?}", padding, self.bare.kind)?;
                for child in self.children() {
                    child.dump(w, depth + 1)?;
                }
            }
        }

        Ok(())
    }

    pub fn bindings(self: &Arc<Self>) -> impl Iterator<Item = Binding> {
        self.children()
            .filter(|child| matches!(child.bare.kind, BareSyntaxKind::Binding))
            .map(|binding| Binding {
                name: binding.find_name(),
                value: binding.find_term(),
            })
    }

    pub fn find_name(self: &Arc<Self>) -> Option<Arc<str>> {
        self.find(BareSyntaxKind::Name).and_then(|name| {
            name.children().find_map(|child| {
                if let BareSyntaxKind::Identifier(name) = child.bare.kind() {
                    Some(name.clone())
                } else {
                    None
                }
            })
        })
    }

    pub fn find_implicit_name(self: &Arc<Self>) -> Option<Arc<str>> {
        self.find(BareSyntaxKind::ImplicitName).and_then(|name| {
            name.children().find_map(|child| {
                if let BareSyntaxKind::Identifier(name) = child.bare.kind() {
                    Some(name.clone())
                } else {
                    None
                }
            })
        })
    }

    pub fn find_names(self: &Arc<Self>) -> impl Iterator<Item = Arc<str>> {
        self.children()
            .filter(|child| matches!(child.bare.kind(), BareSyntaxKind::Name))
            .flat_map(|name| {
                name.children().find_map(|child| {
                    if let BareSyntaxKind::Identifier(name) = child.bare.kind() {
                        Some(name.clone())
                    } else {
                        None
                    }
                })
            })
    }

    pub fn find_param_heads(self: &Arc<Self>) -> impl Iterator<Item = ParamHead> {
        self.children()
            .filter(|child| matches!(child.bare.kind(), BareSyntaxKind::ParamHead))
            .flat_map(|param_head| {
                let mut name = param_head.find_name();
                let mut implicit_name = param_head.find_implicit_name();
                let mut implicitness = Implicitness::Explicit.into();

                if name.is_none() {
                    if let Some(implicit_name) = implicit_name.take() {
                        name = Some(implicit_name);
                        implicitness = Implicitness::Implicit.into();
                    }
                }

                if let Some(implicit_name) = implicit_name {
                    implicitness = NamedImplicitness::ImplicitWithName(implicit_name);
                }

                Some(ParamHead { name, implicitness })
            })
    }

    pub fn find_eval(self: &Arc<Self>) -> Option<Eval> {
        self.find(BareSyntaxKind::Eval).map(|eval| {
            let value = eval.find_term();
            Eval { value }
        })
    }

    pub fn find_term(self: &Arc<Self>) -> Option<Arc<Self>> {
        self.find(BareSyntaxKind::Term)
    }

    pub fn find(self: &Arc<Self>, kind: BareSyntaxKind) -> Option<Arc<Self>> {
        self.children().find(|child| child.bare.kind == kind)
    }

    pub fn find_var(self: &Arc<Self>) -> Option<Arc<str>> {
        self.children().find_map(|child| {
            if let BareSyntaxKind::Variable(name) = child.bare.kind() {
                Some(name.clone())
            } else {
                None
            }
        })
    }

    pub fn find_fn(self: &Arc<Self>) -> Option<Func> {
        self.find(BareSyntaxKind::FnValue).map(|func| {
            let param_list = func.find(BareSyntaxKind::ParamList);
            let return_type_annotation = func.find(BareSyntaxKind::TypeAnnotation);
            let block = func.find(BareSyntaxKind::Block);

            Func {
                param_list,
                return_type_annotation,
                block,
            }
        })
    }

    pub fn param_list_params(self: &Arc<Self>) -> impl Iterator<Item = Param> {
        self.children()
            .filter(|param| matches!(param.bare.kind(), BareSyntaxKind::Param))
            .map(|param| {
                let type_annotation = param.find(BareSyntaxKind::TypeAnnotation);

                Param {
                    param,
                    type_annotation,
                }
            })
    }
}

#[derive(Clone, Debug)]
pub struct Binding {
    pub name: Option<Arc<str>>,
    pub value: Option<Arc<SyntaxNode>>,
}

#[derive(Clone, Debug)]
pub struct Eval {
    pub value: Option<Arc<SyntaxNode>>,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub param_list: Option<Arc<SyntaxNode>>,
    pub return_type_annotation: Option<Arc<SyntaxNode>>,
    pub block: Option<Arc<SyntaxNode>>,
}

impl Func {
    pub fn params(&self) -> impl Iterator<Item = Param> {
        self.param_list
            .as_ref()
            .map(|list| list.param_list_params())
            .into_iter()
            .flatten()
    }

    pub fn body(&self) -> impl Iterator<Item = Arc<SyntaxNode>> {
        self.block.as_ref().into_iter().flat_map(|block| {
            block
                .children()
                .filter(|child| matches!(child.bare.kind, BareSyntaxKind::Term))
        })
    }
}

#[derive(Clone, Debug)]
pub struct Param {
    pub param: Arc<SyntaxNode>,
    pub type_annotation: Option<Arc<SyntaxNode>>,
}

impl Param {
    pub fn param_heads(&self) -> impl Iterator<Item = ParamHead> {
        self.param.find_param_heads()
    }
}

#[derive(Clone, Debug)]
pub struct ParamHead {
    pub name: Option<Arc<str>>,
    pub implicitness: NamedImplicitness,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Implicitness {
    Explicit,
    Implicit,
}

#[derive(Clone, Debug, From)]
pub enum NamedImplicitness {
    ImplicitWithName(Arc<str>),
    Implicitness(Implicitness),
}

impl NamedImplicitness {
    pub fn matches_param(&self, param_name: &str, param_implicitness: Implicitness) -> bool {
        match self {
            NamedImplicitness::ImplicitWithName(name) => {
                param_name == name.as_ref() && param_implicitness == Implicitness::Implicit
            }
            NamedImplicitness::Implicitness(lambda_implicitness) => {
                param_implicitness == *lambda_implicitness
            }
        }
    }
}
