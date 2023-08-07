use std::any::{Any, type_name};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::SystemTime;

use super::smirk_messages::SmirkMessages;
use super::smirk_search_mode::SmirkSearchMode;
use super::record::{ Record, RecordLike };
use super::trie::Trie;

pub struct SmirkMap {
    pub search_mode: SmirkSearchMode,
    pub map: HashMap<String, Record<Box<dyn Any + Send>>>,
    pub trie: Trie
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
    pub fn get<'a, T: 'static>(&'a self, key: &String) -> Result<&'a T, SmirkMessages> {
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
    pub fn set<'a, T: Send + 'static>(
        &mut self,
        key: &String,
        value: Vec<u8>,
        desired_type_name: &String
        ) -> Result<SmirkMessages, SmirkMessages>{
        // if let Ok(value) = parsed_value {
            let record: Record<Box<dyn Any + Send>> = Record {
                value: Box::new(value),
                ttl: None,
                ttl_start: SystemTime::now(),
                type_name: String::from(type_name::<T>()),
                desired_type_name: String::from(desired_type_name)
            };
            self.map.insert(key.to_owned(), record);
        // } else if let Err(_) = parsed_value {
        //     return Err(SmirkMessages::ParseError(String::from(key), value, String::from(type_name::<T>())));
        // }
        return Ok(
            SmirkMessages::SetKey(
                String::from(key),
                String::from(type_name::<T>()),
                String::from(desired_type_name)
                )
            );
    }
    pub fn exists(&self, key: &String) -> bool {
        return self.map.contains_key(key);
    }
    pub fn get_record(&self, key: &String) -> Result<&Record<Box<dyn Any + Send>>, SmirkMessages> {
        if self.exists(key) {
            return Ok(self.map.get(key).unwrap());
        }

        Err(SmirkMessages::KeyNotFound(key.clone()))
    }
    pub fn del(&mut self, key: &String) -> u64 {
        if self.map.contains_key(key) {
            self.map.remove(key);
            1
        } else {
            0
        }
    }
    pub fn ttl(&self, key: &String) -> Result<Option<u64>, String> {
        if let Some(record) = self.map.get(key) {
            return Ok(record.get_ttl());
        }
        Err(format!("Key \"{}\" was not found", key))
    }
    pub fn set_ttl(&mut self, key: &String, ttl: &Option<u64>) {
        if let Some(record) = self.map.get_mut(key) {
            record.ttl = *ttl;
        }
    }
    pub fn set_search_mode(&mut self, mode: SmirkSearchMode) {
        self.search_mode = mode;
    }
}
