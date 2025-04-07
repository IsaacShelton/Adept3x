use super::expand::Environment;
use pp_ast::{Define, DefineKind, ObjMacro, PlaceholderAffinity};
use pp_token::PreTokenKind;
use source_files::Source;

pub fn stdc() -> Environment {
    let mut stdc = Environment::default();

    stdc.add_define(Define {
        name: "__STDC__".into(),
        source: Source::internal(),
        kind: DefineKind::ObjMacro(ObjMacro::new(vec![], PlaceholderAffinity::Discard)),
        is_file_local_only: true,
    });

    stdc.add_define(Define {
        name: "__STDC_VERSION__".into(),
        source: Source::internal(),
        kind: DefineKind::ObjMacro(ObjMacro::new(
            vec![PreTokenKind::Number("202311L".into()).at(Source::internal())],
            PlaceholderAffinity::Discard,
        )),
        is_file_local_only: true,
    });

    stdc
}
