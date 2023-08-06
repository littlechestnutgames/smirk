#[derive(Debug)]
pub enum SmirkSearchMode {
    Glob,
    Regex,
    Trie
}
impl SmirkSearchMode {
    fn from_ref(mode: SmirkSearchMode) -> SmirkSearchMode {
        match mode {
            Self::Glob => Self::Glob,
            Self::Regex => Self::Regex,
            Self::Trie => Self::Trie
        }
    }
}
