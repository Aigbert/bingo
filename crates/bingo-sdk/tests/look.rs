use bingo_sdk::{
    ToolLook,
    look::{self, RowLook},
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    input: Value,
    declared: Option<ToolLook>,
    ascii: bool,
    expected: RowLook,
}

macro_rules! case {
    ($name:ident) => {
        #[test]
        fn $name() {
            let fixture: Fixture = serde_json::from_str(include_str!(concat!(
                "../fixtures/look/",
                stringify!($name),
                ".json"
            )))
            .unwrap();
            assert_eq!(
                look::row(
                    &fixture.name,
                    &fixture.input,
                    fixture.declared.as_ref(),
                    fixture.ascii
                ),
                fixture.expected
            );
        }
    };
}

case!(declared);
case!(ascii_twin);
case!(ascii_mark);
case!(wide_mark);
case!(zero_width);
case!(control);
case!(non_ascii);
case!(invalid_twin);
case!(unknown);
case!(unknown_ascii);
case!(missing_target);
case!(non_string_target);
case!(short_string);
case!(no_string);
case!(command);
case!(text);
case!(acp);
case!(acp_ascii);
case!(colon_name);

#[test]
fn standalone_marks_follow_the_row_contract() {
    for (mark, ascii, expected) in [
        (Some('¶'), false, '¶'),
        (Some('¶'), true, '-'),
        (Some('界'), false, '•'),
        (Some('\0'), false, '•'),
        (None, false, '•'),
        (Some('/'), true, '/'),
    ] {
        assert_eq!(look::mark(mark, ascii), expected);
    }
}
