pub enum SmirkMessages {
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
