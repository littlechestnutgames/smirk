use std::{
    time::SystemTime,
    collections::HashMap,
    any::{Any, type_name},
    net::{TcpListener,TcpStream},
    io::{Read, Write}, sync::{Arc, Mutex, MutexGuard}, fmt::Display, str::FromStr
};

use regex::Regex;

enum SmirkSearchMode {
    Glob,
    Regex
}

struct SmirkMap {
    search_mode: SmirkSearchMode,
    map: HashMap<String, Record<Box<dyn Any + Send>>>
}

enum SmirkError {
    KeyNotFound(String),
    TypeMistmatch(String),

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
    fn get<'a, T: 'static>(&'a self, key: &String) -> Result<&'a T, String> {
        if let Some(record) = self.map.get(key) {
            if let Some(real_value) = record.value.downcast_ref::<T>() {
                return Ok(real_value);
            }
            return Err(format!("Key \"{}\" found, but can not be downcast to \"{}\".\n", key, type_name::<T>()));
        }

        return Err(format!("Key not found: \"{}\"\n", key));
    }

    /// Sets a value in the SmirkMap at key.
    ///
    /// # Arguments
    ///
    /// * `key`: A `&String` representing the key to be fetched.
    ///
    /// * `value`: A `T` value to be stored in the map with `key`.
    fn set<'a, T: 'static + Send>(&mut self, key: &String, value: T, requested_type_name: &String) {
        let record: Record<Box<dyn Any + Send>> = Record {
            value: Box::new(value),
            ttl: None,
            ttl_start: SystemTime::now(),
            type_name: String::from(type_name::<T>()),
            requested_type_name: String::from(requested_type_name)
        };
        self.map.insert(key.to_owned(), record);
    }
    fn exists(&self, key: &String) -> bool {
        return self.map.contains_key(key);
    }
    fn get_record(&self, key: &String) -> Result<&Record<Box<dyn Any + Send>>, SmirkError> {
        if self.exists(key) {
            return Ok(self.map.get(key).unwrap());
        }
        Err(SmirkError::KeyNotFound(key.clone()))
    }
    fn del(&mut self, key: &String) {
        self.map.remove(key);
    }
    fn ttl(&self, key: &String) -> Result<Option<u64>, String> {
        if let Some(record) = self.map.get(key) {
            return Ok(record.get_ttl());
        }
        Err(format!("Key \"{}\" was not found", key))
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
    requested_type_name: String
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

struct SmirkCommand {
    command: String,
    args: Vec<String>
}

fn main() {
    let server_data = SmirkMap {
        search_mode: SmirkSearchMode::Glob,
        map: HashMap::new()
    };

    let listener = TcpListener::bind("127.0.0.1:2873").expect("Failed to bind to port 2873");
    println!("Server listening on port 2873");

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
        stream.write_all(s.as_bytes()).unwrap();
    }
}

fn set_value_and_write_to_stream<T: Display + Send + FromStr + 'static>(
    stream: &mut TcpStream,
    smirk_map: &mut MutexGuard<'_, SmirkMap>,
    key: &String,
    value: &String,
    requested_type_name: &String
) {
    let parsed_value = value.parse::<T>();
    if let Ok(v) = parsed_value {
        smirk_map.set::<T>(key, v, requested_type_name);
        stream.write_all(format!("Set \"{}\" successfully. Stored-Type: {}, User-Type: {}\n", key, std::any::type_name::<T>(), requested_type_name).as_bytes()).unwrap();
    } else if let Err(_) = parsed_value {
        stream.write_all(format!("Failed to parse value for key \"{}\" as \"{}\"", key, std::any::type_name::<T>()).as_bytes()).unwrap();
    }
}

fn handle_client(mut stream: TcpStream, threadsafe_server_data: &Arc<Mutex<SmirkMap>>) {
    // Buffer to store incoming data from the client
    let mut buffer = [0; 512];

    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read == 0 {
                    break;
                }

                let received_data = String::from_utf8_lossy(&buffer[..bytes_read]);
                let commands = received_data.lines().map(|m| {
                    let mut items = m.split_whitespace();
                    let mut command = "";
                    let mut args = Vec::<String>::new();

                    if let Some(c) = items.next() {
                        command = c;
                        args = items.map(|i| { i.to_owned() }).collect();
                    }
                    return SmirkCommand {
                        command: command.to_owned(),
                        args
                    }
                });

                commands.for_each(|c| {
                    let mut smirk_map = threadsafe_server_data.lock().unwrap();
                    match c.command.to_uppercase().as_str() {
                        "SAVE" => {
                            stream.write_all("Saving a dump of all keys.".as_bytes()).unwrap();
                            todo!();
                        }
                        "QUIT" => {
                            stream.write_all("Bye.\n".as_bytes()).unwrap();
                            let shutdown = stream.shutdown(std::net::Shutdown::Both);
                            if let Err(e) = shutdown {
                                stream.write_all(format!("Hmm. It seems like we're having problems shutting down the stream. {}", e).as_bytes()).unwrap();
                            }
                        }
                        "TTL" => {
                            // Get TTL
                            if c.args.len() == 1 {
                                let mut cmd_str = c.args.iter();
                                let k = cmd_str.next().unwrap().as_str();
                                let smttl = smirk_map.ttl(&String::from(k));
                                match smttl {
                                    Ok(option) => {
                                        if let Some(o) = option {
                                            stream.write_all(format!("{}\n", o).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" does not expire.\n", k).as_bytes()).unwrap();
                                        }
                                    }
                                    Err(_) => {
                                        stream.write_all(format!("Key \"{}\" does not exist.\n", k).as_bytes()).unwrap();
                                    }
                                }
                            }

                            // Set TTL
                            if c.args.len() == 2 {

                            }
                        }
                        "GET" => {
                            if c.args.len() == 2 {
                                let mut type_key = c.args.iter();
                                let t = type_key.next().unwrap().as_str();
                                let key = type_key.next().unwrap();

                                match t {
                                    "i8" => {
                                        get_value_and_write_to_stream::<i8>(&mut stream, &smirk_map, key);
                                    }
                                    "i16" => {
                                        get_value_and_write_to_stream::<i16>(&mut stream, &smirk_map, key);
                                    }
                                    "i32" => {
                                        get_value_and_write_to_stream::<i32>(&mut stream, &smirk_map, key);
                                    }
                                    "i64" => {
                                        get_value_and_write_to_stream::<i64>(&mut stream, &smirk_map, key);
                                    }
                                    "i128" => {
                                        get_value_and_write_to_stream::<i128>(&mut stream, &smirk_map, key);
                                    }
                                    "u8" => {
                                        get_value_and_write_to_stream::<u8>(&mut stream, &smirk_map, key);
                                    }
                                    "u16" => {
                                        get_value_and_write_to_stream::<u16>(&mut stream, &smirk_map, key);
                                    }
                                    "u32" => {
                                        get_value_and_write_to_stream::<u32>(&mut stream, &smirk_map, key);
                                    }
                                    "u64" => {
                                        get_value_and_write_to_stream::<u64>(&mut stream, &smirk_map, key);
                                    }
                                    "u128" => {
                                        get_value_and_write_to_stream::<u128>(&mut stream, &smirk_map, key);
                                    }
                                    "isize" => {
                                        get_value_and_write_to_stream::<isize>(&mut stream, &smirk_map, key);
                                    }
                                    "usize" => {
                                        get_value_and_write_to_stream::<usize>(&mut stream, &smirk_map, key);
                                    }
                                    "f32" => {
                                        get_value_and_write_to_stream::<f32>(&mut stream, &smirk_map, key);
                                    }
                                    "f64" => {
                                        get_value_and_write_to_stream::<f64>(&mut stream, &smirk_map, key);
                                    }
                                    "bool" => {
                                        get_value_and_write_to_stream::<bool>(&mut stream, &smirk_map, key);
                                    }
                                    "char" => {
                                        get_value_and_write_to_stream::<char>(&mut stream, &smirk_map, key);
                                    }
                                    _ => {
                                        get_value_and_write_to_stream::<String>(&mut stream, &smirk_map, key);
                                    }
                                };

                            } else {
                                stream.write_all(b"Usage: GET <TYPE> <KEY>").unwrap();
                            }
                        }
                        "SET" => {
                            if c.args.len() == 3 {
                                let mut type_key = c.args.into_iter();
                                let t = type_key.next().unwrap();
                                let key = type_key.next().unwrap();
                                let value = type_key.next().unwrap();
                                match t.as_str() {
                                    "i8" => {
                                        set_value_and_write_to_stream::<i8>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "i16" => {
                                        set_value_and_write_to_stream::<i16>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "i32" => {
                                        set_value_and_write_to_stream::<i32>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "i64" => {
                                        set_value_and_write_to_stream::<i64>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "i128" => {
                                        set_value_and_write_to_stream::<i128>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "u8" => {
                                        set_value_and_write_to_stream::<u8>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "u16" => {
                                        set_value_and_write_to_stream::<u16>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "u32" => {
                                        set_value_and_write_to_stream::<u32>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "u64" => {
                                        set_value_and_write_to_stream::<u64>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "u128" => {
                                        set_value_and_write_to_stream::<u128>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "isize" => {
                                        set_value_and_write_to_stream::<isize>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "usize" => {
                                        set_value_and_write_to_stream::<usize>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "f32" => {
                                        set_value_and_write_to_stream::<f32>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "f64" => {
                                        set_value_and_write_to_stream::<f64>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "bool" => {
                                        set_value_and_write_to_stream::<bool>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    "char" => {
                                        set_value_and_write_to_stream::<char>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                    _ => {
                                        set_value_and_write_to_stream::<String>(&mut stream, &mut smirk_map, &key, &value, &t);
                                    }
                                };
                            } else {
                                stream.write_all(b"Usage: SET <TYPE> <KEY> <VALUE>\n").unwrap();
                            }
                        }
                        "DEL" => {
                            c.args.into_iter().for_each(|k| { smirk_map.del(&k); });
                        }
                        "MODE" => {
                            let mut type_key = c.args.iter();
                            let mode = type_key.next().unwrap().as_str().to_uppercase();
                            let message: Result<SmirkSearchMode, String> = match mode.as_str() {
                                "GLOB" => Ok(SmirkSearchMode::Glob),
                                "REGEX" => Ok(SmirkSearchMode::Regex),
                                _ => Err("Usage: MODE Regex or MODE Glob. Glob is the default.".to_owned())
                            };

                            if let Ok(m) = message {
                                smirk_map.search_mode(m);
                                stream.write_all(format!("Search mode updated to \"{}\".", mode).as_bytes()).unwrap();
                            }
                            else if let Err(m) = message {
                                stream.write_all(format!("{}\n", m).as_bytes()).unwrap();
                            }
                        }
                        "EXISTS" => {

                            let mut type_key = c.args.iter();
                            let key = type_key.next().unwrap().as_str();
                            let exists = smirk_map.exists(&String::from(key));
                            stream.write_all(format!("{}\n", exists).as_bytes()).unwrap();
                        }
                        "TYPE" => {
                            let mut type_key = c.args.iter();
                            let key = type_key.next().unwrap().as_str();
                            let result = smirk_map.get_record(&String::from(key));
                            if let Ok(record) = result {
                                stream.write_all(
                                    format!(
                                        "Stored-Type: {}, User-Type: {}\n",
                                        record.type_name.clone(),
                                        record.requested_type_name.clone()
                                    ).as_bytes()
                                ).unwrap();
                            } else if let Err(s) = result {
                                match s {
                                    SmirkError::KeyNotFound(err) => {
                                        stream.write_all(format!("{}\n", err).as_bytes()).unwrap();
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "KEYS" => {
                            let mut type_key = c.args.iter();
                            let key = type_key.next().unwrap().as_str();
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
                        },
                        _ => {
                            stream.write_all(format!("Command \"{}\" not recognized.\n", c.command).as_bytes()).unwrap()
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("Error reading from socket: {}", e);
                break;
            }
        }
    }
}

