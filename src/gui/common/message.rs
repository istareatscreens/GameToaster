use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Message {
    pub shown: String,

    pub logged: String,
}

impl Message {
    pub(crate) fn detail(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            shown: text.clone(),
            logged: text,
        }
    }
}

impl From<String> for Message {
    fn from(detail: String) -> Self {
        Self::detail(detail)
    }
}

impl From<Message> for String {
    fn from(message: Message) -> Self {
        message.shown
    }
}

impl fmt::Display for Message {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.shown)
    }
}

macro_rules! message {
    ($key:literal $(, $name:ident => $value:expr)* $(,)?) => {
        $crate::gui::common::message::Message {
            shown: rust_i18n::t!($key $(, $name => $value)*).to_string(),
            logged: rust_i18n::t!($key, locale = "en" $(, $name => $value)*).to_string(),
        }
    };
}
pub(crate) use message;

#[cfg(test)]
#[path = "../../../tests/unit/gui/common/message__tests.rs"]
mod tests;
