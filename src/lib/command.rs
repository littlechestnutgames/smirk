use super::smirk_search_mode::SmirkSearchMode;

use super::command_error::CommandError;

#[derive(Debug)]
pub enum Command {
    Set(String, String, Vec<u8>),
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

impl Command {
    pub fn from_vec(v: Vec<u8>) -> Result<Self, CommandError> {
        let mut trimmed_v = v;
        if trimmed_v.last() == Some(&b'\n') {
            trimmed_v.pop();
        }

        let mut tokens: Vec<&[u8]> = trimmed_v.split(|&x| x == b' ').collect();

        let mut cmd = tokens[0].to_ascii_uppercase();
        if cmd.last() == Some(&b'\n') {
            cmd.pop();
        }
        tokens = tokens.split_off(1);
        let tok_len = tokens.len();
        match cmd.as_slice() {
            b"SET" => {
                if tok_len < 3 {
                    return Err(CommandError::ArgumentMismatch)
                }

                let data_tokens = tokens[2..].to_vec();
                let data_with_spaces = data_tokens.join(&b' ');

                Ok(
                    Command::Set(
                        String::from_utf8_lossy(tokens[0]).to_string(),
                        String::from_utf8_lossy(tokens[1]).to_string(),
                        data_with_spaces
                    )
                )
            },
            b"GET" => {
                if tok_len != 2 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(
                    Command::Get(
                        String::from_utf8_lossy(tokens[0])
                            .to_string(),
                        String::from_utf8_lossy(tokens[1])
                            .to_string()
                    )
                )
            },
            b"DEL" => {
                if tok_len < 1 {
                    return Err(CommandError::ArgumentMismatch);
                }
                let keys = tokens
                            .into_iter()
                            .map(|x| String::from_utf8_lossy(x).to_string())
                            .collect();
                Ok(
                    Command::Del(
                        keys
                    )
                )
            }
            b"KEYS" => {
                if tok_len != 1 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Keys(String::from_utf8_lossy(tokens[0]).to_string()))
            }
            b"MODE" => {
                if tok_len != 1 {
                    return Err(CommandError::ArgumentMismatch);
                }
                let mut mode = tokens[0].to_ascii_uppercase();
                mode.pop();
                match mode.as_slice() {
                    b"GLOB" => Ok(Command::Mode(SmirkSearchMode::Glob)),
                    b"REGEX" => Ok(Command::Mode(SmirkSearchMode::Regex)),
                    _ => Err(CommandError::NoValidModeSpecified)
                }
            }
            b"TTL" => {
                match tok_len {
                    1 => Ok(Command::TtlGet(String::from_utf8_lossy(tokens[0]).to_string())),
                    2 => {
                        let ttl = String::from_utf8_lossy(tokens[1]).to_string().parse::<u64>();
                        if let Ok(ttl) = ttl {
                            Ok(Command::TtlSet(String::from_utf8_lossy(tokens[0]).to_string(), Some(ttl)))
                        } else {
                            Err(CommandError::InvalidTtlSpecified)
                        }
                    }
                    _ => Err(CommandError::ArgumentMismatch)
                }
            }
            b"DELTTL" => {
                match tok_len {
                    1 => Ok(Command::TtlSet(String::from_utf8_lossy(tokens[0]).to_string(), None)),
                    _ => Err(CommandError::ArgumentMismatch)
                }
            }
            b"EXISTS" => {
                if tok_len != 1 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Exists(String::from_utf8_lossy(tokens[0]).to_string()))
            }
            b"TYPE" => {
                if tok_len != 1 {
                    return Err(CommandError::ArgumentMismatch);
                }
                Ok(Command::Type(String::from_utf8_lossy(tokens[0]).to_string()))
            }
            b"QUIT" => {
                Ok(Command::Quit)
            }
            b"SAVE" => {
                Ok(Command::Save)
            }
            _ => Err(CommandError::Unknown)
        }
    }
}
