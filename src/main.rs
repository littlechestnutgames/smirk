use std::{
    time::SystemTime,
    collections::HashMap,
    any::Any,
    net::{TcpListener,TcpStream},
    io::{Read, Write}, sync::{Arc, Mutex}
};

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
        if let Some(record) = self.map.get(key) {
            if let Some(real_value) = record.value.downcast_ref::<T>() {
                return Some(real_value);
            }
        }
        None
    }
    fn set<'a, T: 'static + Send>(&mut self, key: &String, value: T, ttl: Option<u64>) {
        let record: Record<Box<dyn Any + Send>> = Record {
            value: Box::new(value),
            ttl,
            ttl_start: SystemTime::now()
        };
        self.map.insert(key.to_owned(), record);
    }
    fn del(&mut self, key: &String) {
        self.map.remove(key);
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
            return Some(ttl - duration);
        }
        None
    }
}

fn main() {
    let mut server_data = SmirkMap {
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
    let allowed_commands = vec![
        String::from("GET"),
        String::from("SET"),
        String::from("DEL"),
        String::from("KEYS")
    ];
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
                    match c.command.as_str() {
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
                                    "i64" => { let data = smirk_map.get::<i64>(&key.to_owned()); }
                                    "i128" => { let data = smirk_map.get::<i128>(&key.to_owned()); }
                                    "u8" => { let data = smirk_map.get::<u8>(&key.to_owned()); }
                                    "u16" => { let data = smirk_map.get::<u16>(&key.to_owned()); }
                                    "u32" => { let data = smirk_map.get::<u32>(&key.to_owned()); }
                                    "u64" => {
                                        let data = smirk_map.get::<u64>(&key.to_owned());
                                    }
                                    "u128" => {
                                        let data = smirk_map.get::<u128>(&key.to_owned());
                                    }
                                    "isize" => {
                                        let data = smirk_map.get::<isize>(&key.to_owned());
                                        if let Some(d) = data {

                                        } else {
                                        }
                                    }
                                    "usize" => {
                                        let data = smirk_map.get::<usize>(&key.to_owned());
                                    }
                                    "f32" => {
                                        let data = smirk_map.get::<f32>(&key.to_owned());
                                    }
                                    "f64" => {
                                        let data = smirk_map.get::<f64>(&key.to_owned());
                                    }
                                    "bool" => {
                                        let data = smirk_map.get::<bool>(&key.to_owned());
                                    }
                                    "char" => {
                                        let data = smirk_map.get::<char>(&key.to_owned());
                                    }
                                    "String" => {
                                        let data = smirk_map.get::<String>(&key.to_owned());
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
                        "DEL" => {
                            c.args.into_iter().for_each(|k| { smirk_map.del(&k); });
                        }
                        "KEYS" => todo!("Add KEYS implementation."),
                        _ => {
                            let mut smirk_map = threadsafe_server_data.lock().unwrap();
                            if let Some(data) = smirk_map.get::<i32>(&"custom2".to_owned()) {
                                stream.write_all(format!("{}", data).as_bytes()).unwrap();
                            } else {
                                smirk_map.set::<i32>(&"custom2".to_owned(), 33, None);
                            }
                            stream.write_all(format!("Command \"{}\" not recognized.", c.command).as_bytes()).unwrap()
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

