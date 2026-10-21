use mdfmt::{Position, Source, utf16_len};

#[test]
fn positions_use_utf16() {
    let source = Source::new("a😀b\r\nz".into());
    assert_eq!(
        source.position(5).unwrap(),
        Position {
            line: 1,
            column_utf16: 4
        }
    );
    assert_eq!(
        source.position(8).unwrap(),
        Position {
            line: 2,
            column_utf16: 1
        }
    );
    assert!(source.position(2).is_err());
    assert!(source.position(7).is_err());
    assert!(source.position(10).is_err());
    assert_eq!(utf16_len("a😀\t"), 4);
}

#[test]
fn preserves_mixed_terminators_bom_and_eof() {
    let text = "\u{feff}a\rb\r\nc\ne\u{301}\t";
    let source = Source::new(text.into());
    assert_eq!(source.text(), text);
    assert_eq!(
        source.position(5).unwrap(),
        Position {
            line: 2,
            column_utf16: 1
        }
    );
    assert_eq!(
        source.position(8).unwrap(),
        Position {
            line: 3,
            column_utf16: 1
        }
    );
    assert_eq!(
        source.position(text.len()).unwrap(),
        Position {
            line: 4,
            column_utf16: 4
        }
    );
    assert_eq!(
        Source::new(String::new()).position(0).unwrap(),
        Position {
            line: 1,
            column_utf16: 1
        }
    );
}
