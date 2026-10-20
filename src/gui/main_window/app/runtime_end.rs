#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum RuntimeEnd {
    Clean,

    Cancelled,

    Failed { title: String, message: String },
}

pub(super) fn dialog_for(end: &RuntimeEnd, launch_confirmed: bool) -> Option<(String, String)> {
    match end {
        RuntimeEnd::Failed { title, message } if !launch_confirmed => {
            Some((title.clone(), message.clone()))
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/app/runtime_end__tests.rs"]
mod tests;
