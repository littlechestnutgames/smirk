use std::{
    time::SystemTime,
    collections::HashMap,
    any::{Any, type_name},
    net::{TcpListener,TcpStream},
    io::{Write, BufReader, BufRead}, sync::{Arc, Mutex, MutexGuard}, fmt::Display, str::FromStr
};

use regex::Regex;

#[derive(Debug)]
enum SmirkSearchMode {
    Glob,
    Regex
}

impl SmirkSearchMode {
    fn from_ref(mode: &SmirkSearchMode) -> SmirkSearchMode {
        match mode {
            Self::Glob => Self::Glob,
            Self::Regex => Self::Regex
        }
    }
}

struct SmirkMap {
    search_mode: SmirkSearchMode,
    map: HashMap<String, Record<Box<dyn Any + Send>>>
}

enum SmirkMessages {
    /// Positive Messages :)
    SetKey(String, String, String),
    /// Negative Messages :(

    /// The key wasn't found in the SmirkMap.
    ///
    /// `String` is the map key.
    KeyNotFound(String),

    /// This means the value stored in key `param1`
    ///
    ///
    TypeMismatch(String, String),

    ParseError(String, String, String)
}

impl ToString for SmirkMessages {
    fn to_string(&self) -> String {
        match self {
            SmirkMessages::SetKey(
                key,
                registered_type_name,
                desired_type_name
            ) => format!(
                "Set key \"{}\" successfully. Stored-Type: {}, User-Type: {}\n",
                key,
                registered_type_name,
                desired_type_name
            ),
            SmirkMessages::KeyNotFound(key) => format!(
                "Key \"{}\" not found.\n",
                key
            ),
            Self::TypeMismatch(key, desired_type) => format!(
                "Couldn't downcast the value stored in key \"{}\" to type \"{}\".\n",
                key,
                desired_type
            ),
            Self::ParseError(key, value, desired_type) => format!(
                "Setting key \"{}\" failed. Could not parse \"{}\" into \"{}\".\n",
                key,
                value,
                desired_type
            )
        }
    }
}

impl SmirkMap {
    /// Retrieves a value from the SmirkMap.
    ///
    /// # Arguments
    ///
    /// * `key`: A `&String` representing the key to be fetched.
    ///
    /// # Returns
    ///
    /// * `Ok(&T)`: Returns &T, if key exists and is able to be downcast as T
    ///
    /// * `Err(String)`: The error message.
    fn get<'a, T: 'static>(&'a self, key: &String) -> Result<&'a T, SmirkMessages> {
        if let Some(record) = self.map.get(key) {
            if let Some(real_value) = record.value.downcast_ref::<T>() {
                return Ok(real_value);
            }
            return Err(SmirkMessages::TypeMismatch(String::from(key), type_name::<T>().to_string()));
        }

        return Err(SmirkMessages::KeyNotFound(String::from(key)));
    }

    /// Sets a value in the SmirkMap at key.
    ///
    /// # Arguments
    ///
    /// * `key`: A `&String` representing the key to be fetched.
    ///
    /// * `value`: A `T` value to be stored in the map with `key`.
    fn set<'a, T: 'static + FromStr + Send>(
        &mut self,
        key: &String,
        value: String,
        desired_type_name: &String
    ) -> Result<SmirkMessages, SmirkMessages>{
        let parsed_value = value.parse::<T>();
        if let Ok(value) = parsed_value {
            let record: Record<Box<dyn Any + Send>> = Record {
                value: Box::new(value),
                ttl: None,
                ttl_start: SystemTime::now(),
                type_name: String::from(type_name::<T>()),
                desired_type_name: String::from(desired_type_name)
            };
            self.map.insert(key.to_owned(), record);
        } else if let Err(_) = parsed_value {
            return Err(SmirkMessages::ParseError(String::from(key), value, String::from(type_name::<T>())));
        }
        return Ok(
            SmirkMessages::SetKey(
                String::from(key),
                String::from(type_name::<T>()),
                String::from(desired_type_name)
            )
        );
    }
    fn exists(&self, key: &String) -> bool {
        return self.map.contains_key(key);
    }
    fn get_record(&self, key: &String) -> Result<&Record<Box<dyn Any + Send>>, SmirkMessages> {
        if self.exists(key) {
            return Ok(self.map.get(key).unwrap());
        }

        Err(SmirkMessages::KeyNotFound(key.clone()))
    }
    fn del(&mut self, key: &String) -> u64 {
        if self.map.contains_key(key) {
            self.map.remove(key);
            1
        } else {
            0
        }
    }
    fn ttl(&self, key: &String) -> Result<Option<u64>, String> {
        if let Some(record) = self.map.get(key) {
            return Ok(record.get_ttl());
        }
        Err(format!("Key \"{}\" was not found", key))
    }
    fn set_ttl(&mut self, key: &String, ttl: &Option<u64>) {
        if let Some(record) = self.map.get_mut(key) {
            record.ttl = *ttl;
        }
    }
    fn search_mode(&mut self, mode: SmirkSearchMode) {
        self.search_mode = mode;
    }
}

struct Record<T> {
    value: T,
    ttl: Option<u64>,
    ttl_start: SystemTime,
    type_name: String,
    desired_type_name: String
}

trait RecordLike<T> {
    fn is_expired(&self) -> bool;
    fn get_ttl(&self) -> Option<u64>;
}

impl<T> RecordLike<T> for Record<T> {
    fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl  {
            return SystemTime::now()
                .duration_since(self.ttl_start)
                .unwrap_or_default()
                .as_secs() >= ttl;
        }

        false
    }
    fn get_ttl(&self) -> Option<u64> {

        if let Some(ttl) = self.ttl  {
            let duration = SystemTime::now()
                .duration_since(self.ttl_start)
                .unwrap_or_default()
                .as_secs();
            if duration >= ttl {
                return Some(0);
            }
            return Some(ttl - duration);
        }
        None
    }
}

#[derive(Debug)]
enum Command {
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

#[derive(Debug)]
enum CommandError {
    NoInput,
    ArgumentMismatch,
    Unknown,
    NoValidModeSpecified,
    InvalidTtlSpecified
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

fn main() {
    let server_data = SmirkMap {
        search_mode: SmirkSearchMode::Glob,
        map: HashMap::new()
    };

    let listener = TcpListener::bind("127.0.0.1:53173").expect("Failed to bind to port 53173");
    println!("Server listening on port 53173");

    let threadsafe_server_data = Arc::new(Mutex::new(server_data));

    for stream in listener.incoming() {

        match stream {
            Ok(stream) => {
                println!("New client connected: {:?}", stream.peer_addr());
                let threadsafe_server_data = threadsafe_server_data.clone();
                std::thread::spawn(move || {
                    handle_client(stream, &threadsafe_server_data);
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

fn get_value_and_write_to_stream<T: Display + 'static>(
    stream: &mut TcpStream,
    smirk_map: &MutexGuard<'_, SmirkMap>,
    key: &String
) {
    let result = smirk_map.get::<T>(&key.to_owned());
    if let Ok(d) = result {
        stream.write_all(format!("{}\n", d).as_bytes()).unwrap();
    } else if let Err(s) = result {
        stream.write_all(s.to_string().as_bytes()).unwrap();
    }
}

fn set_value_and_write_to_stream<T: Display + Send + FromStr + 'static>(
    stream: &mut TcpStream,
    smirk_map: &mut MutexGuard<'_, SmirkMap>,
    key: &String,
    value: &String,
    desired_type_name: &String
) {
    let result = smirk_map.set::<T>(key, String::from(value), desired_type_name);
    if let Ok(success) = result {
        stream.write_all(success.to_string().as_bytes()).unwrap();
    } else if let Err(e) = result {
        stream.write_all(e.to_string().as_bytes()).unwrap();
    }
}

fn process_command(stream: &mut TcpStream, command: &Command, smirk_map: &mut MutexGuard<SmirkMap>) {
    match command {
        Command::Set(t, k, v) => {
            match t.as_str() {
                "i8" => { set_value_and_write_to_stream::<i8>(stream, smirk_map, k, v, t); }
                "i16" => { set_value_and_write_to_stream::<i16>(stream, smirk_map, k, v, t); }
                "i32" => { set_value_and_write_to_stream::<i32>(stream, smirk_map, k, v, t); }
                "i64" => { set_value_and_write_to_stream::<i64>(stream, smirk_map, k, v, t); }
                "i128" => { set_value_and_write_to_stream::<i128>(stream, smirk_map, k, v, t); }
                "u8" => { set_value_and_write_to_stream::<u8>(stream, smirk_map, k, v, t); }
                "u16" => { set_value_and_write_to_stream::<u16>(stream, smirk_map, k, v, t); }
                "u32" => { set_value_and_write_to_stream::<u32>(stream, smirk_map, k, v, t); }
                "u64" => { set_value_and_write_to_stream::<u64>(stream, smirk_map, k, v, t); }
                "u128" => { set_value_and_write_to_stream::<u128>(stream, smirk_map, k, v, t); }
                "isize" => { set_value_and_write_to_stream::<isize>(stream, smirk_map, k, v, t); }
                "usize" => { set_value_and_write_to_stream::<usize>(stream, smirk_map, k, v, t); }
                "f32" => { set_value_and_write_to_stream::<f32>(stream, smirk_map, k, v, t); }
                "f64" => { set_value_and_write_to_stream::<f64>(stream, smirk_map, k, v, t); }
                "bool" => { set_value_and_write_to_stream::<bool>(stream, smirk_map, k, v, t); }
                "char" => { set_value_and_write_to_stream::<char>(stream, smirk_map, k, v, t); }
                _ => { set_value_and_write_to_stream::<String>(stream, smirk_map, k, v, t); }
            }
        }
        Command::Get(t, k) => {
            match t.as_str() {
                "i8" => { get_value_and_write_to_stream::<i8>(stream, &smirk_map, k); }
                "i16" => { get_value_and_write_to_stream::<i16>(stream, &smirk_map, k); }
                "i32" => { get_value_and_write_to_stream::<i32>(stream, &smirk_map, k); }
                "i64" => { get_value_and_write_to_stream::<i64>(stream, &smirk_map, k); }
                "i128" => { get_value_and_write_to_stream::<i128>(stream, &smirk_map, k); }
                "u8" => { get_value_and_write_to_stream::<u8>(stream, &smirk_map, k); }
                "u16" => { get_value_and_write_to_stream::<u16>(stream, &smirk_map, k); }
                "u32" => { get_value_and_write_to_stream::<u32>(stream, &smirk_map, k); }
                "u64" => { get_value_and_write_to_stream::<u64>(stream, &smirk_map, k); }
                "u128" => { get_value_and_write_to_stream::<u128>(stream, &smirk_map, k); }
                "isize" => { get_value_and_write_to_stream::<isize>(stream, &smirk_map, k); }
                "usize" => { get_value_and_write_to_stream::<usize>(stream, &smirk_map, k); }
                "f32" => { get_value_and_write_to_stream::<f32>(stream, &smirk_map, k); }
                "f64" => { get_value_and_write_to_stream::<f64>(stream, &smirk_map, k); }
                "bool" => { get_value_and_write_to_stream::<bool>(stream, &smirk_map, k); }
                "char" => { get_value_and_write_to_stream::<char>(stream, &smirk_map, k); }
                _ => { get_value_and_write_to_stream::<String>(stream, &smirk_map, k); }
            }
        }
        Command::Del(keys) => {
            let deleted: u64 = keys.into_iter().map(|k| smirk_map.del(k)).sum();
            stream.write_all(format!("{}", deleted).as_bytes()).unwrap();
        }
        Command::Keys(key) => {
            match smirk_map.search_mode {
                SmirkSearchMode::Glob => {
                    let pattern = glob::Pattern::new(key).unwrap();
                    let matching_keys: Vec<String> = smirk_map
                        .map.keys().into_iter()
                        .filter(|k| pattern.matches(k))
                        .cloned()
                        .collect();
                    if matching_keys.len() == 0 {
                        stream.write_all(format!("No matches for key query \"{}\" were found.\n", key).as_bytes()).unwrap();
                    } else {
                        let matched = matching_keys.join("\n");
                        stream.write_all(format!("{}\n", matched).as_bytes()).unwrap();
                    }
                }
                SmirkSearchMode::Regex => {
                    let pattern = Regex::new(key).unwrap();
                    let matching_keys: Vec<String> = smirk_map
                        .map.keys().into_iter()
                        .filter(|k| pattern.is_match(k))
                        .cloned()
                        .collect();
                    if matching_keys.len() == 0 {
                        stream.write_all(format!("No matches for key query \"{}\" were found.\n", key).as_bytes()).unwrap();
                    } else {
                        let matched = matching_keys.join("\n");
                        stream.write_all(format!("{}\n", matched).as_bytes()).unwrap();
                    }
                }
            }
        }
        Command::Mode(mode) => {
            smirk_map.search_mode(SmirkSearchMode::from_ref(mode));
        }
        Command::TtlSet(key, ttl) => {
            smirk_map.set_ttl(key, ttl);
        }
        Command::TtlGet(key) => {
            let smttl = smirk_map.ttl(&String::from(key));
            match smttl {
                Ok(option) => {
                    if let Some(o) = option {
                        stream.write_all(format!("{}\n", o).as_bytes()).unwrap();
                    } else {
                        stream.write_all(format!("Key \"{}\" does not expire.\n", key).as_bytes()).unwrap();
                    }
                }
                Err(_) => {
                    stream.write_all(format!("Key \"{}\" does not exist.\n", key).as_bytes()).unwrap();
                }
            }
        }
        Command::Exists(key) => {
            let exists = smirk_map.exists(key);
            stream.write_all(format!("{}\n", exists).as_bytes()).unwrap();
        }
        Command::Type(key) => {
            let result = smirk_map.get_record(&String::from(key));
            if let Ok(record) = result {
                stream.write_all(
                    format!(
                        "Stored-Type: {}, User-Type: {}\n",
                        record.type_name.clone(),
                        record.desired_type_name.clone()
                        ).as_bytes()
                    ).unwrap();
            } else if let Err(s) = result {
                stream.write_all(s.to_string().as_bytes()).unwrap();
            }
        }
        Command::Save => {
            stream.write_all("Saving a dump of all keys.".as_bytes()).unwrap();
            todo!();
        }
        Command::Quit => {
            stream.write_all("Bye.\n".as_bytes()).unwrap();
            let shutdown = stream.shutdown(std::net::Shutdown::Both);
            if let Err(e) = shutdown {
                stream.write_all(format!("Hmm. It seems like we're having problems shutting down the stream. {}", e).as_bytes()).unwrap();
            }
        }
    }
}

fn handle_client(stream: TcpStream, threadsafe_server_data: &Arc<Mutex<SmirkMap>>) {
    let mut bufreader = BufReader::new(&stream);

    let mut smirk_map = threadsafe_server_data.lock().unwrap();
    loop {
        let mut line = String::new();

        match bufreader.read_line(&mut line) {
            Ok(0) => {
                break;
            }
            Ok(_) => {
                let cmd = Command::from_str(line.as_str());

                if let Ok(cmd) = cmd {
                    let mut sclone = stream.try_clone().unwrap();
                    process_command(&mut sclone, &cmd, &mut smirk_map);
                } else if let Err(cmd_err) = cmd {
                    println!("{:?}", cmd_err);
                }
            }
            Err(e) => {
                eprintln!("Error reading from socket: {}", e);
                break;
            }
        }
    }
}

