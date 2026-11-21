use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FolderPatch {
    Selected,

    AddedVersion,

    Unknown,
}

pub fn classify<'a>(
    files: impl IntoIterator<Item = (Option<&'a str>, &'a str)>,
    known: impl FnOnce() -> BTreeSet<String>,
) -> FolderPatch {
    let differing: Vec<String> = files
        .into_iter()
        .filter_map(|(current, selected)| {
            current
                .filter(|current| !current.eq_ignore_ascii_case(selected))
                .map(str::to_ascii_lowercase)
        })
        .collect();
    if differing.is_empty() {
        return FolderPatch::Selected;
    }
    let known = known();
    if differing.iter().all(|current| known.contains(current)) {
        FolderPatch::AddedVersion
    } else {
        FolderPatch::Unknown
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/lifecycle/folder_patch__tests.rs"]
mod tests;
