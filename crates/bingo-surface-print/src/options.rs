//! Options owned by the headless surface, including its CLI vocabulary.

use std::str::FromStr;

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub output_format: OutputFormat,
    pub input_format: InputFormat,
    pub permission_prompt_tool: Option<PromptTool>,
    pub images: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    StreamJson,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputFormat {
    #[default]
    Text,
    StreamJson,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptTool {
    Stdio,
}

macro_rules! words {
    ($kind:ident, $($variant:ident => $word:literal),+ $(,)?) => {
        impl $kind {
            pub const VALUES: &'static [&'static str] = &[$($word),+];
        }

        impl FromStr for $kind {
            type Err = String;

            fn from_str(word: &str) -> Result<Self, Self::Err> {
                match word {
                    $($word => Ok(Self::$variant),)+
                    _ => Err(format!("unknown value {word:?}; expected {}", Self::VALUES.join(", "))),
                }
            }
        }
    };
}

words!(OutputFormat, Text => "text", Json => "json", StreamJson => "stream-json");
words!(InputFormat, Text => "text", StreamJson => "stream-json");
words!(PromptTool, Stdio => "stdio");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_advertised_word_parses_and_unknown_words_fail() {
        for word in OutputFormat::VALUES {
            assert!(word.parse::<OutputFormat>().is_ok());
        }
        for word in InputFormat::VALUES {
            assert!(word.parse::<InputFormat>().is_ok());
        }
        for word in PromptTool::VALUES {
            assert!(word.parse::<PromptTool>().is_ok());
        }
        assert!("yaml".parse::<OutputFormat>().is_err());
        assert!("json".parse::<InputFormat>().is_err());
        assert!("mcp__auth".parse::<PromptTool>().is_err());
    }
}
