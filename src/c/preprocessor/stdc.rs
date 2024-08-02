use super::{ast::PlaceholderAffinity, expand::Environment, Define, DefineKind, PreTokenKind};
use crate::source_files::Source;

pub fn stdc() -> Environment {
    let mut stdc = Environment::default();

    stdc.add_define(Define {
        name: "__STDC__".into(),
        source: Source::internal(),
        kind: DefineKind::ObjectMacro(vec![], PlaceholderAffinity::Discard),
    });

    stdc.add_define(Define {
        name: "__STDC_VERSION__".into(),
        source: Source::internal(),
        kind: DefineKind::ObjectMacro(
            vec![PreTokenKind::Number("202311L".into()).at(Source::internal())],
            PlaceholderAffinity::Discard,
        ),
    });

    stdc
}
