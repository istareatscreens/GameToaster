use serde::Deserialize;

pub const MAX_LINE_BYTES: usize = 65_536;

pub const MAX_CREDITS: u32 = 9_999;
const STAGE_CHARS: usize = 32;
const PROGRESS_TEXT_CHARS: usize = 256;
const RESULT_TEXT_CHARS: usize = 1_024;
const LOG_TEXT_CHARS: usize = 1_024;

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    Progress(Progress),
    GameState(GameState),
    Result(ResultMessage),
    Log(LogMessage),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub stage: Stage,
    pub percent: Option<f32>,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stage {
    Validating,
    BackingUp,
    Preparing,
    Patching,
    Verifying,
    Launching,
    Running,
    Restoring,
    Complete,
    Other(String),
}

impl Stage {
    pub fn parse(word: &str) -> Option<Self> {
        let well_formed = (1..=STAGE_CHARS).contains(&word.len())
            && word
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_');
        if !well_formed {
            return None;
        }
        Some(match word {
            "validating" => Self::Validating,
            "backing_up" => Self::BackingUp,
            "preparing" => Self::Preparing,
            "patching" => Self::Patching,
            "verifying" => Self::Verifying,
            "launching" => Self::Launching,
            "running" => Self::Running,
            "restoring" => Self::Restoring,
            "complete" => Self::Complete,
            other => Self::Other(other.to_owned()),
        })
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Validating => "validating",
            Self::BackingUp => "backing_up",
            Self::Preparing => "preparing",
            Self::Patching => "patching",
            Self::Verifying => "verifying",
            Self::Launching => "launching",
            Self::Running => "running",
            Self::Restoring => "restoring",
            Self::Complete => "complete",
            Self::Other(word) => word,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GameState {
    pub credits: Option<u32>,
    pub paid_play: Option<bool>,
    pub free_play: bool,
    pub mode: GameMode,
}

impl GameState {
    pub fn payment_known(&self) -> bool {
        self.credits.is_some() || self.paid_play.is_some()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GameMode {
    #[default]
    Unknown,
    Attract,
    Ready,
    ActivePlay,
    Results,
}

impl GameMode {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "unknown" => Self::Unknown,
            "attract" => Self::Attract,
            "ready" => Self::Ready,
            "active_play" => Self::ActivePlay,
            "results" => Self::Results,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultMessage {
    pub status: ResultStatus,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultStatus {
    Ok,
    Failed,
    Refused,
}

impl ResultStatus {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "ok" => Self::Ok,
            "failed" => Self::Failed,
            "refused" => Self::Refused,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogMessage {
    pub level: LogLevel,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "debug" => Self::Debug,
            "info" => Self::Info,
            "warn" => Self::Warn,
            "error" => Self::Error,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Invalid {
    Overlong,
    NotUtf8,
    NotJson,
    NotAnObject,
    MissingType,

    Field(&'static str),
}

#[derive(Deserialize)]
struct ProgressWire {
    stage: String,
    percent: Option<f64>,
    text: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameStateWire {
    credits: Option<u32>,
    paid_play: Option<bool>,
    free_play: Option<bool>,
    mode: Option<String>,
}

#[derive(Deserialize)]
struct ResultWire {
    status: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct LogWire {
    level: String,
    text: Option<String>,
}

pub fn parse_line(line: &[u8]) -> Result<Option<Message>, Invalid> {
    let text = std::str::from_utf8(line).map_err(|_| Invalid::NotUtf8)?;
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| Invalid::NotJson)?;
    if !value.is_object() {
        return Err(Invalid::NotAnObject);
    }
    let Some(kind) = value.get("type").and_then(serde_json::Value::as_str) else {
        return Err(Invalid::MissingType);
    };
    Ok(Some(match kind {
        "progress" => Message::Progress(progress(value)?),
        "game_state" => Message::GameState(game_state(value)?),
        "result" => Message::Result(result(value)?),
        "log" => Message::Log(log(value)?),
        _ => return Ok(None),
    }))
}

fn progress(value: serde_json::Value) -> Result<Progress, Invalid> {
    let wire: ProgressWire =
        serde_json::from_value(value).map_err(|_| Invalid::Field("progress"))?;
    let stage = Stage::parse(&wire.stage).ok_or(Invalid::Field("stage"))?;
    let percent = match wire.percent {
        None => None,
        Some(percent) if (0.0..=100.0).contains(&percent) => Some(percent as f32),
        Some(_) => return Err(Invalid::Field("percent")),
    };
    Ok(Progress {
        stage,
        percent,
        text: truncate(wire.text.unwrap_or_default(), PROGRESS_TEXT_CHARS),
    })
}

fn game_state(value: serde_json::Value) -> Result<GameState, Invalid> {
    let wire: GameStateWire =
        serde_json::from_value(value).map_err(|_| Invalid::Field("game_state"))?;
    if wire.credits.is_some_and(|credits| credits > MAX_CREDITS) {
        return Err(Invalid::Field("credits"));
    }
    let mode = match wire.mode.as_deref() {
        None => GameMode::Unknown,
        Some(word) => GameMode::parse(word).ok_or(Invalid::Field("mode"))?,
    };
    Ok(GameState {
        credits: wire.credits,
        paid_play: wire.paid_play,
        free_play: wire.free_play.unwrap_or(false),
        mode,
    })
}

fn result(value: serde_json::Value) -> Result<ResultMessage, Invalid> {
    let wire: ResultWire = serde_json::from_value(value).map_err(|_| Invalid::Field("result"))?;
    Ok(ResultMessage {
        status: ResultStatus::parse(&wire.status).ok_or(Invalid::Field("status"))?,
        text: truncate(wire.text.unwrap_or_default(), RESULT_TEXT_CHARS),
    })
}

fn log(value: serde_json::Value) -> Result<LogMessage, Invalid> {
    let wire: LogWire = serde_json::from_value(value).map_err(|_| Invalid::Field("log"))?;
    Ok(LogMessage {
        level: LogLevel::parse(&wire.level).ok_or(Invalid::Field("level"))?,
        text: truncate(wire.text.unwrap_or_default(), LOG_TEXT_CHARS),
    })
}

fn truncate(text: String, characters: usize) -> String {
    if text.chars().count() <= characters {
        text
    } else {
        text.chars().take(characters).collect()
    }
}

pub fn message_type(line: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(line)
        .ok()
        .and_then(|value| value.get("type")?.as_str().map(str::to_owned))
        .map(|word| word.chars().take(STAGE_CHARS).collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../../../../tests/unit/plugin/runtime/channel/messages__tests.rs"]
mod tests;
