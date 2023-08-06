use std::collections::HashMap;

pub struct Trie {
    has_key_ending_here: bool,
    counter: u64,
    children: HashMap<char, Trie>
}

impl Default for Trie {
    fn default() -> Self {
        Trie {
            has_key_ending_here: false,
            counter: 1,
            children: HashMap::new()
        }
    }
}

impl Trie {
    fn insert(&mut self, input: String) {
        let mut remaining_string = input.clone();
        let mut trie = self;
        let working_char: char = remaining_string.remove(0);
        if let Some(mut child) = trie.children.get_mut(&working_char) {
            child.counter += 1;
            child.has_key_ending_here = remaining_string.is_empty();
            trie = child;
        } else {
            let t = Trie {
                has_key_ending_here: remaining_string.is_empty(),
                counter: 1,
                children: HashMap::new()
            };
            // WIP. trie.children.insert(working_char, t);
            // if let Some(child) = trie.children.get_mut(&working_char) {
            //     trie = child;
            // }
        }
    }
    // TODO fn remove
    // TODO fn get_keys_with_prefix
}

