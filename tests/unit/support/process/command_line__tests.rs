use super::*;

#[test]
fn quotes_spaces_quotes_and_trailing_slashes() {
    assert_eq!(quote_argument(OsStr::new("plain")), "plain");
    assert_eq!(quote_argument(OsStr::new("two words")), "\"two words\"");
    assert_eq!(quote_argument(OsStr::new("a\\\"b")), "\"a\\\\\\\"b\"");
}

#[test]
fn a_command_line_quotes_only_what_needs_it_and_ends_with_a_nul() {
    let line = command_line(
        Path::new(r"C:\Program Files\Game\game.exe"),
        &[
            OsString::from("--fast"),
            OsString::from("two words"),
            OsString::new(),
        ],
    );
    assert_eq!(line.last(), Some(&0));
    assert_eq!(
        String::from_utf16(&line[..line.len() - 1]).unwrap(),
        r#""C:\Program Files\Game\game.exe" --fast "two words" """#
    );
}
