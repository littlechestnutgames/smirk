use std::collections::HashMap;

#[derive(Debug)]
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
    pub fn insert(&mut self, input: &String) {
        let mut remaining_string = input.clone();
        let working_char = remaining_string.remove(0);
        let child = self.children.entry(working_char).or_insert_with(|| {
            Trie {
                children: HashMap::new(),
                counter: 1,
                has_key_ending_here: remaining_string.is_empty()
            }
        });

        child.counter += 1;
        child.has_key_ending_here = remaining_string.is_empty();

        if remaining_string.is_empty() {
            child.insert(&remaining_string);
        }
    }

    pub fn remove(&mut self, input: &String) {
        let mut remaining_string = input.clone();
        let mut trie = self;

        while let Some(working_char) = remaining_string.chars().next() {
            remaining_string.remove(0);
            if let Some(child) = trie.children.get_mut(&working_char) {
                trie = child;
            } else {
                return;
            }
        }

        trie.counter -= 1;

        let children_to_remove: Vec<_> = trie.children.keys()
            .filter_map(|&key| {
                if trie.children[&key].counter == 0 {
                    Some(key)
                } else {
                    None
                }
            })
        .collect();

        for key in children_to_remove {
            trie.children.remove(&key);
        }
    }
    pub fn get_keys_with_prefix(&self, prefix: &str) -> Vec<String> {
        let mut keys = Vec::new();
        let mut stack = vec![(prefix.to_string(), self)];

        while let Some((current_prefix, current_trie)) = stack.pop() {
            if current_prefix.is_empty() {
                keys.extend(current_trie.children.keys().map(|c| c.to_string()));
            } else if let Some(next_char) = current_prefix.chars().next() {
                if let Some(next_trie) = current_trie.children.get(&next_char) {
                    let next_prefix = current_prefix.chars().skip(1).collect::<String>();
                    stack.push((next_prefix, next_trie));
                }
            }
        }

        keys
    }
}

