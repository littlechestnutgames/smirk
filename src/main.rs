use std::{
    collections::HashMap,
    net::{TcpListener,TcpStream},
    io::{Write, BufReader, BufRead}, sync::{Arc, Mutex, MutexGuard}, fmt::Display, str::FromStr, os, env
};

mod lib;

use lib::smirk_map::SmirkMap;
use lib::trie::Trie;
use lib::smirk_search_mode::SmirkSearchMode;
use regex::Regex;
use lib::command::Command;

#[derive(Debug)]
struct SmirkConfig {
    port: u16,
    number_of_dbs: u8,
    max_threads: usize,
    default_key_search_method: SmirkSearchMode
}

impl Default for SmirkConfig {
    fn default() -> Self {
        SmirkConfig {
            port: 53173,
            number_of_dbs: 1,
            max_threads: num_cpus::get(),
            default_key_search_method: SmirkSearchMode::Glob
        }
    }
}

fn get_config() -> SmirkConfig {
    let args: Vec<String> = env::args().collect();
    let mut config = SmirkConfig::default();

    if args.len() > 1 {
        for i in 1..args.len() {
            if args[i] == "--port" && i + 1 < args.len() {
                config.port = args[i+1].parse().unwrap_or(config.port);
            }
            else if args[i] == "--number-of-dbs" && i + 1 < args.len() {
                config.number_of_dbs = args[i+1].parse().unwrap_or(config.number_of_dbs);
            }
            else if args[i] == "--max-threads" && i + 1 < args.len() {
                config.max_threads = args[i+1].parse().unwrap_or(config.max_threads);
            }
            else if args[i] == "--default-key-search-type" && i + 1 < args.len() {
                config.default_key_search_method = match args[i+1].to_uppercase().as_str() {
                    "REGEX" => SmirkSearchMode::Regex,
                    _ => SmirkSearchMode::Glob
                }
            }
        }
    }
    config
}

fn main() {
    let config: SmirkConfig = get_config();
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
                },
                SmirkSearchMode::Trie => {

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

