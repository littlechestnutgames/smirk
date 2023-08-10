use std::{
    collections::HashMap,
    net::{TcpListener,TcpStream},
    io::{Write, BufReader, BufRead}, sync::{Arc, Mutex, MutexGuard}, str::FromStr, fmt::Display
};

mod smirk_config;
use num::{CheckedAdd, BigInt};
use smirk::core::command::Command;
use smirk::core::trie::Trie;
use smirk::core::smirk_search_mode::SmirkSearchMode;
use smirk::core::smirk_map::SmirkMap;
use smirk_config::SmirkConfig;
use regex::Regex;


fn main() {
    let config: SmirkConfig = SmirkConfig::get_runtime_config();
    let server_data = SmirkMap {
        search_mode: config.default_key_search_method,
        map: HashMap::new(),
        trie: Trie::default()
    };

    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.port)).expect(format!("Failed to bind to port {}", config.port).as_str());
    println!("Server listening on port {}", config.port);
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

trait Streamable {
    fn write_to_stream(&self, stream: &mut TcpStream);
}

macro_rules! impl_streamable_for_display {
    ($($ty:ty),*) => {
        $(
            impl Streamable for $ty {
                fn write_to_stream(&self, stream: &mut TcpStream) {
                    write!(stream, "{}\n", self).unwrap();
                }
            }
        )*
    };
}

impl_streamable_for_display!(
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64, bool, char, String, BigInt
);

impl Streamable for Vec<u8> {
    fn write_to_stream(&self, stream: &mut TcpStream) {
        stream.write_all(self).unwrap();
        stream.write_all("\n".as_bytes()).unwrap();
    }
}

fn get_value_and_write_to_stream<T: Streamable + 'static>(
    stream: &mut TcpStream,
    smirk_map: &MutexGuard<'_, SmirkMap>,
    key: &String
) {
    let result = smirk_map.get::<T>(&key.to_owned());
    if let Ok(d) = result {
        d.write_to_stream(stream);
    } else if let Err(s) = result {
        stream.write_all(s.to_string().as_bytes()).unwrap();
    }
}

fn set_value_and_write_to_stream<T: Send + FromStr + 'static>(
    stream: &mut TcpStream,
    smirk_map: &mut MutexGuard<'_, SmirkMap>,
    key: &String,
    value: Vec<u8>,
    desired_type_name: &String
) {
    let result = smirk_map.set::<T>(key, value, desired_type_name);
    if let Ok(success) = result {
        smirk_map.trie.insert(key);
        stream.write_all(success.to_string().as_bytes()).unwrap();
    } else if let Err(e) = result {
        stream.write_all(e.to_string().as_bytes()).unwrap();
    }
}

fn set_binary_value_and_write_to_stream(
    stream: &mut TcpStream,
    smirk_map: &mut MutexGuard<'_, SmirkMap>,
    key: &String,
    value: Vec<u8>,
    desired_type_name: &String
) {
    let result = smirk_map.binary_set(key, value, desired_type_name);
    if let Ok(success) = result {
        smirk_map.trie.insert(key);
        stream.write_all(success.to_string().as_bytes()).unwrap();
    } else if let Err(e) = result {
        stream.write_all(e.to_string().as_bytes()).unwrap();
    }
}

fn add_float_and_write_to_stream<T: std::ops::Add<Output = T> + Default + Copy + Display + 'static>(
    stream: &mut TcpStream,
    smirk_map: &mut MutexGuard<'_, SmirkMap>,
    keys: Vec<String>
) {
    let total = smirk_map.add_float::<T>(keys);
    if let Ok(total) = total {
        stream.write_all(total.to_string().as_bytes()).unwrap();
    } else if let Err(e) = total {
        stream.write_all(e.to_string().as_bytes()).unwrap();
    }
}

fn add_and_write_to_stream<T: CheckedAdd<Output = T> + Default + Display + 'static>(
    stream: &mut TcpStream,
    smirk_map: &mut MutexGuard<'_, SmirkMap>,
    keys: Vec<String>
) {
    let total = smirk_map.add::<T>(keys);
    if let Ok(total) = total {
        stream.write_all(total.to_string().as_bytes()).unwrap();
    } else if let Err(e) = total {
        stream.write_all(e.to_string().as_bytes()).unwrap();
    }
}

fn process_command(stream: &mut TcpStream, command: &Command, smirk_map: &mut MutexGuard<SmirkMap>) {
    match command {
        Command::Set(t, k, v) => {
            match t.as_str() {
                "i8" => { set_value_and_write_to_stream::<i8>(stream, smirk_map, k, v.to_vec(), t); }
                "i16" => { set_value_and_write_to_stream::<i16>(stream, smirk_map, k, v.to_vec(), t); }
                "i32" => { set_value_and_write_to_stream::<i32>(stream, smirk_map, k, v.to_vec(), t); }
                "i64" => { set_value_and_write_to_stream::<i64>(stream, smirk_map, k, v.to_vec(), t); }
                "i128" => { set_value_and_write_to_stream::<i128>(stream, smirk_map, k, v.to_vec(), t); }
                "u8" => { set_value_and_write_to_stream::<u8>(stream, smirk_map, k, v.to_vec(), t); }
                "u16" => { set_value_and_write_to_stream::<u16>(stream, smirk_map, k, v.to_vec(), t); }
                "u32" => { set_value_and_write_to_stream::<u32>(stream, smirk_map, k, v.to_vec(), t); }
                "u64" => { set_value_and_write_to_stream::<u64>(stream, smirk_map, k, v.to_vec(), t); }
                "u128" => { set_value_and_write_to_stream::<u128>(stream, smirk_map, k, v.to_vec(), t); }
                "isize" => { set_value_and_write_to_stream::<isize>(stream, smirk_map, k, v.to_vec(), t); }
                "usize" => { set_value_and_write_to_stream::<usize>(stream, smirk_map, k, v.to_vec(), t); }
                "BigInt" => { set_value_and_write_to_stream::<BigInt>(stream, smirk_map, k, v.to_vec(), t); }
                "f32" => { set_value_and_write_to_stream::<f32>(stream, smirk_map, k, v.to_vec(), t); }
                "f64" => { set_value_and_write_to_stream::<f64>(stream, smirk_map, k, v.to_vec(), t); }
                "bool" => { set_value_and_write_to_stream::<bool>(stream, smirk_map, k, v.to_vec(), t); }
                "char" => { set_value_and_write_to_stream::<char>(stream, smirk_map, k, v.to_vec(), t); }
                "String" => { set_value_and_write_to_stream::<String>(stream, smirk_map, k, v.to_vec(), t); }
                _ => { set_binary_value_and_write_to_stream(stream, smirk_map, k, v.to_vec(), t); }
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
                "BigInt" => { get_value_and_write_to_stream::<BigInt>(stream, &smirk_map, k); }
                "f32" => { get_value_and_write_to_stream::<f32>(stream, &smirk_map, k); }
                "f64" => { get_value_and_write_to_stream::<f64>(stream, &smirk_map, k); }
                "bool" => { get_value_and_write_to_stream::<bool>(stream, &smirk_map, k); }
                "char" => { get_value_and_write_to_stream::<char>(stream, &smirk_map, k); }
                "String" => { get_value_and_write_to_stream::<String>(stream, &smirk_map, k); }
                _ => { get_value_and_write_to_stream::<Vec<u8>>(stream, &smirk_map, k); }
            }
        }
        Command::Del(keys) => {
            let deleted: u64 = keys.into_iter().map(|k| smirk_map.del(k)).sum();
            for key in keys {
                smirk_map.trie.remove(key);
            }
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
                },
                SmirkSearchMode::Trie => {
                    let results = smirk_map.trie.get_keys_with_prefix(key.as_str());
                    if results.len() == 0 {
                        stream.write_all(format!("No matches for key query \"{}\" were found.\n", key).as_bytes()).unwrap();
                    } else {
                        for result in results {
                            stream.write_all(format!("{}\n", result).as_bytes()).unwrap();
                        }
                    }
                }
            }
        }
        Command::Mode(mode) => {
            smirk_map.set_search_mode(match mode {
                SmirkSearchMode::Glob => SmirkSearchMode::Glob,
                SmirkSearchMode::Regex => SmirkSearchMode::Regex,
                SmirkSearchMode::Trie => SmirkSearchMode::Trie
            });
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
        Command::Add(t, k) => {
            match t.as_str() {
                "i8" => { add_and_write_to_stream::<i8>(stream, smirk_map, k.clone()) }
                "i16" => { add_and_write_to_stream::<i16>(stream, smirk_map, k.clone())  }
                "i32" => { add_and_write_to_stream::<i32>(stream, smirk_map, k.clone())  }
                "i64" => { add_and_write_to_stream::<i64>(stream, smirk_map, k.clone())  }
                "i128" => { add_and_write_to_stream::<i128>(stream, smirk_map, k.clone())  }
                "isize" => { add_and_write_to_stream::<isize>(stream, smirk_map, k.clone())  }
                "u8" => { add_and_write_to_stream::<u8>(stream, smirk_map, k.clone())  }
                "u16" => { add_and_write_to_stream::<u16>(stream, smirk_map, k.clone())  }
                "u32" => { add_and_write_to_stream::<u32>(stream, smirk_map, k.clone())  }
                "u64" => { add_and_write_to_stream::<u64>(stream, smirk_map, k.clone())  }
                "u128" => { add_and_write_to_stream::<u128>(stream, smirk_map, k.clone())  }
                "usize" => { add_and_write_to_stream::<usize>(stream, smirk_map, k.clone())  }
                "BigInt" => { add_and_write_to_stream::<BigInt>(stream, smirk_map, k.clone())  }
                "f32" => { add_float_and_write_to_stream::<f32>(stream, smirk_map, k.clone())  }
                "f64" => { add_float_and_write_to_stream::<f64>(stream, smirk_map, k.clone())  }
                _ => { }
            }
        }
    }
}

fn handle_client(stream: TcpStream, threadsafe_server_data: &Arc<Mutex<SmirkMap>>) {
    let mut bufreader = BufReader::new(&stream);

    let mut smirk_map = threadsafe_server_data.lock().unwrap();
    loop {
        let mut line: Vec<u8> = Vec::new();

        match bufreader.read_until(b'\n', &mut line) {
            Ok(0) => {
                break;
            }
            Ok(_) => {
                let cmd = Command::from_vec(line);

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

