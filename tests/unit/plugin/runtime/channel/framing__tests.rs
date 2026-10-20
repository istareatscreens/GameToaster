use super::*;

#[test]
fn lines_split_across_writes_arrive_whole_and_without_their_terminator() {
    let mut splitter = LineSplitter::new(64);
    assert_eq!(splitter.push(b"{\"a\":"), Vec::<Frame>::new());
    assert_eq!(
        splitter.push(b"1}\n{\"b\":2}\r\n{\"c"),
        [
            Frame::Line(b"{\"a\":1}".to_vec()),
            Frame::Line(b"{\"b\":2}".to_vec())
        ]
    );
    assert_eq!(
        splitter.push(b"\":3}\n\n"),
        [Frame::Line(b"{\"c\":3}".to_vec()), Frame::Line(Vec::new())]
    );
}

#[test]
fn an_overlong_line_is_discarded_whole_and_the_next_line_is_read() {
    let mut splitter = LineSplitter::new(8);
    let mut frames = splitter.push(b"12345678\n");
    assert_eq!(
        frames,
        [Frame::Line(b"12345678".to_vec())],
        "a line of exactly the limit fits"
    );
    frames = splitter.push(b"123456789");
    assert_eq!(frames, Vec::<Frame>::new());
    frames = splitter.push(b"0123456789\nok\n");
    assert_eq!(frames, [Frame::Overlong, Frame::Line(b"ok".to_vec())]);
}
