use std::env;

use crate::lib::smirk_search_mode::SmirkSearchMode;

#[derive(Debug)]
pub struct SmirkConfig {
    pub port: u16,
    pub number_of_dbs: u8,
    pub max_threads: usize,
    pub default_key_search_method: SmirkSearchMode
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

impl SmirkConfig {
    pub fn get_runtime_config() -> SmirkConfig {
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
}
