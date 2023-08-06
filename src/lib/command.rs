use std::str::FromStr;

use super::smirk_search_mode::SmirkSearchMode;

use super::command_error::CommandError;

#[derive(Debug)]
pub enum Command {
    Set(String, String, String),
    Get(String, String),
    Del(Vec<String>),
    Keys(String),
    Mode(SmirkSearchMode),
    TtlGet(String),
    TtlSet(String, Option<u64>),
    Exists(String),
    Type(String),
    Quit,
    Save
}

impl FromStr for Command {
    type Err = CommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<&str> = s.trim().split_whitespace().collect();
        if tokens.is_empty() {
            return Err(CommandError::NoInput);
        }

        let tok_len = tokens.len();
        match tokens[0].to_uppercase().as_str() {
            "SET" => {
                if tok_len < 4 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Set(tokens[1].to_string(), tokens[2].to_string(), tokens[3..].join(" ").to_string()))
            }
            "GET" => {
                if tok_len != 3 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Get(tokens[1].to_string(), tokens[2].to_string()))
            }
            "DEL" => {
                if tok_len < 2 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Del(tokens[1..].into_iter().map(|a| {a.to_string()}).collect()))
            }
            "KEYS" => {
                if tok_len != 2 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Keys(tokens[1].to_string()))
            }
            "MODE" => {
                if tok_len != 2 {
                    return Err(CommandError::ArgumentMismatch);
                }
                match tokens[1].to_uppercase().as_str() {
                    "GLOB" => Ok(Command::Mode(SmirkSearchMode::Glob)),
                    "REGEX" => Ok(Command::Mode(SmirkSearchMode::Regex)),
                    _ => Err(CommandError::NoValidModeSpecified)
                }
            }
            "TTL" => {
                match tok_len {
                    2 => Ok(Command::TtlGet(tokens[1].to_string())),
                    3 => {
                        let ttl = tokens[2].to_owned().parse::<u64>();
                        if let Ok(ttl) = ttl {
                            Ok(Command::TtlSet(tokens[1].to_string(), Some(ttl)))
                        } else {
                            Err(CommandError::InvalidTtlSpecified)
                        }
                    }
                    _ => Err(CommandError::ArgumentMismatch)
                }
            }
            "DELTTL" => {
                match tok_len {
                    2 => Ok(Command::TtlSet(tokens[1].to_string(), None)),
                    _ => Err(CommandError::ArgumentMismatch)
                }
            }
            "EXISTS" => {
                if tok_len != 2 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Exists(tokens[1].to_string()))
            }
            "TYPE" => {
                if tok_len != 2 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Type(tokens[1].to_string()))
            }
            "QUIT" => {
                Ok(Command::Quit)
            }
            "SAVE" => {
                Ok(Command::Save)
            }
            _ => Err(CommandError::Unknown)
        }
    }
}

