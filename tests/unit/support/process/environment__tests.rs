use super::*;

fn entries(block: &[u16]) -> Vec<String> {
    assert_eq!(
        &block[block.len() - 2..],
        &[0, 0],
        "the block ends with an empty entry"
    );
    block[..block.len() - 2]
        .split(|unit| *unit == 0)
        .map(|entry| String::from_utf16(entry).unwrap())
        .collect()
}

#[test]
fn an_override_replaces_a_variable_whatever_its_case_and_new_ones_are_added() {
    let path = std::env::var_os("PATH").expect("the test process has a PATH");
    let block = environment_block(&[
        (OsString::from("path"), OsString::from(r"C:\only")),
        (
            OsString::from("GAMETOASTER_TEST_VARIABLE"),
            OsString::from("set"),
        ),
    ]);
    let entries = entries(&block);
    let paths: Vec<&String> = entries
        .iter()
        .filter(|entry| entry.to_ascii_uppercase().starts_with("PATH="))
        .collect();
    assert_eq!(paths, [&r"path=C:\only".to_string()]);
    assert_ne!(path, OsString::from(r"C:\only"));
    assert!(entries.contains(&"GAMETOASTER_TEST_VARIABLE=set".to_string()));
}

#[test]
fn entries_are_sorted_by_their_upper_case_names() {
    let entries = entries(&environment_block(&[]));

    let names: Vec<String> = entries
        .iter()
        .map(|entry| {
            let end = entry[1..].find('=').map_or(entry.len(), |at| at + 1);
            entry[..end].to_ascii_uppercase()
        })
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}
