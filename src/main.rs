use std::{
    time::SystemTime,
    collections::HashMap,
    any::Any,
    net::{TcpListener,TcpStream},
    io::{Read, Write}, sync::{Arc, Mutex}
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

impl SmirkMap {
    fn get<'a, T: 'static>(&'a self, key: &String) -> Option<&'a T> {
        println!("Hello");
        if let Some(record) = self.map.get(key) {
            if let Some(real_value) = record.value.downcast_ref::<T>() {
                return Some(real_value);
            }
        }
        None
    }
    fn set<'a, T: 'static + Send>(&mut self, key: &String, value: T) {
        let record: Record<Box<dyn Any + Send>> = Record {
            value: Box::new(value),
            ttl: None,
            ttl_start: SystemTime::now()
        };
        self.map.insert(key.to_owned(), record);
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
    ttl_start: SystemTime
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

struct SmirkCommand {
    command: String,
    args: Vec<String>
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
                                        if let Some(data) = smirk_map.get::<i8>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "i16" => {
                                        if let Some(data) = smirk_map.get::<i16>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "i32" => {
                                        if let Some(data) = smirk_map.get::<i32>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "i64" => {
                                        if let Some(data) = smirk_map.get::<i64>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "i128" => {
                                        if let Some(data) = smirk_map.get::<i128>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "u8" => {
                                        if let Some(data) = smirk_map.get::<u8>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "u16" => {
                                        if let Some(data) = smirk_map.get::<u16>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "u32" => {
                                        if let Some(data) = smirk_map.get::<u32>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "u64" => {
                                        if let Some(data) = smirk_map.get::<u64>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "u128" => {
                                        if let Some(data) = smirk_map.get::<u128>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "isize" => {
                                        if let Some(data) = smirk_map.get::<isize>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "usize" => {
                                        if let Some(data) = smirk_map.get::<usize>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "f32" => {
                                        if let Some(data) = smirk_map.get::<f32>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "f64" => {
                                        if let Some(data) = smirk_map.get::<f64>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "bool" => {
                                        if let Some(data) = smirk_map.get::<bool>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "char" => {
                                        if let Some(data) = smirk_map.get::<char>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    "String" => {
                                        if let Some(data) = smirk_map.get::<String>(&key.to_owned()) {
                                            stream.write_all(format!("{}\n", data).as_bytes()).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                    _ => {
                                        if let Some(data) = smirk_map.get::<Vec<u8>>(&key.to_owned()) {
                                            stream.write_all(data).unwrap();
                                        } else {
                                            stream.write_all(format!("Key \"{}\" not found.\n", &key.to_owned()).as_bytes()).unwrap();
                                        }
                                    }
                                };

                            } else {
                                stream.write_all(b"Usage: GET <TYPE> <KEY>").unwrap();
                            }
                        }
                        "SET" => {
                            let c_args_len = c.args.len();
                            if c.args.len() == 3 || c.args.len() == 4 {

                                let mut type_key = c.args.iter();
                                let t = type_key.next().unwrap().as_str();
                                let key = type_key.next().unwrap();
                                let value = type_key.next().unwrap();
                                match t {
                                    "i8" => {
                                        if let Ok(data) = value.parse::<i8>() {
                                            smirk_map.set::<i8>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as i8.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "i16" => {
                                        if let Ok(data) = value.parse::<i16>() {
                                            smirk_map.set::<i16>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as i16.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "i32" => {
                                        if let Ok(data) = value.parse::<i32>() {
                                            smirk_map.set::<i32>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as i32.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "i64" => {
                                        if let Ok(data) = value.parse::<i64>() {
                                            smirk_map.set::<i64>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as i64.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "i128" => {
                                        if let Ok(data) = value.parse::<i128>() {
                                            smirk_map.set::<i128>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as i128.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "u8" => {
                                        if let Ok(data) = value.parse::<u8>() {
                                            smirk_map.set::<u8>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as u8.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "u16" => {
                                        if let Ok(data) = value.parse::<u16>() {
                                            smirk_map.set::<u16>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as u16.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "u32" => {
                                        if let Ok(data) = value.parse::<u32>() {
                                            smirk_map.set::<u32>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as u32.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "u64" => {
                                        if let Ok(data) = value.parse::<u64>() {
                                            smirk_map.set::<u64>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as u64.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "u128" => {
                                        if let Ok(data) = value.parse::<u128>() {
                                            smirk_map.set::<u128>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as u128.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "isize" => {
                                        if let Ok(data) = value.parse::<isize>() {
                                            smirk_map.set::<isize>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as isize.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "usize" => {
                                        if let Ok(data) = value.parse::<usize>() {
                                            smirk_map.set::<usize>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as usize.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "f32" => {
                                        if let Ok(data) = value.parse::<f32>() {
                                            smirk_map.set::<f32>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as f32.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "f64" => {
                                        if let Ok(data) = value.parse::<f64>() {
                                            smirk_map.set::<f64>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as f64.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "bool" => {
                                        if let Ok(data) = value.parse::<f64>() {
                                            smirk_map.set::<f64>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as f64.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "char" => {
                                        if let Ok(data) = value.parse::<char>() {
                                            smirk_map.set::<char>(&key, data);
                                        } else {
                                            stream.write_all(format!("Can't cast value \"{}\" as char.\n", &value).as_bytes()).unwrap();
                                        }
                                    }
                                    "String" => {
                                        smirk_map.set::<String>(&key, value.to_owned());
                                    }
                                    _ => {
                                        smirk_map.set::<String>(&key, value.to_owned());
                                    }
                                };
                            } else {
                                stream.write_all(b"Usage: SET <TYPE> <KEY> <VALUE>").unwrap();
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

