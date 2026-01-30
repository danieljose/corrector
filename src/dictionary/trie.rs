//! Estructura Trie para búsqueda eficiente de palabras

use std::collections::HashMap;

/// Categoría gramatical de una palabra
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WordCategory {
    Sustantivo,
    Verbo,
    Adjetivo,
    Adverbio,
    Articulo,
    Preposicion,
    Conjuncion,
    Pronombre,
    Determinante,
    Otro,
}

impl WordCategory {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sustantivo" | "noun" | "n" => WordCategory::Sustantivo,
            "verbo" | "verb" | "v" => WordCategory::Verbo,
            "adjetivo" | "adjective" | "adj" => WordCategory::Adjetivo,
            "adverbio" | "adverb" | "adv" => WordCategory::Adverbio,
            "articulo" | "article" | "art" => WordCategory::Articulo,
            "preposicion" | "preposition" | "prep" => WordCategory::Preposicion,
            "conjuncion" | "conjunction" | "conj" => WordCategory::Conjuncion,
            "pronombre" | "pronoun" | "pron" => WordCategory::Pronombre,
            "determinante" | "determiner" | "det" => WordCategory::Determinante,
            _ => WordCategory::Otro,
        }
    }
}

/// Género gramatical
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Gender {
    Masculine,
    Feminine,
    None,
}

impl Gender {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "m" | "masc" | "masculine" | "masculino" => Gender::Masculine,
            "f" | "fem" | "feminine" | "femenino" => Gender::Feminine,
            _ => Gender::None,
        }
    }
}

/// Número gramatical
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Number {
    Singular,
    Plural,
    None,
}

impl Number {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "s" | "sing" | "singular" => Number::Singular,
            "p" | "pl" | "plural" => Number::Plural,
            _ => Number::None,
        }
    }
}

/// Información asociada a una palabra
#[derive(Debug, Clone)]
pub struct WordInfo {
    pub category: WordCategory,
    pub gender: Gender,
    pub number: Number,
    pub extra: String,
    pub frequency: u32,
}

impl Default for WordInfo {
    fn default() -> Self {
        Self {
            category: WordCategory::Otro,
            gender: Gender::None,
            number: Number::None,
            extra: String::new(),
            frequency: 1,
        }
    }
}

/// Nodo del Trie
#[derive(Debug, Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    word_info: Option<WordInfo>,
    is_word: bool,
}

/// Estructura Trie para almacenamiento y búsqueda de palabras
#[derive(Debug, Default)]
pub struct Trie {
    root: TrieNode,
    word_count: usize,
}

impl Trie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
            word_count: 0,
        }
    }

    /// Inserta una palabra en el Trie
    /// Si la palabra ya existe, solo sobrescribe si la nueva entrada tiene mayor frecuencia
    pub fn insert(&mut self, word: &str, info: WordInfo) {
        let word_lower = word.to_lowercase();
        let mut node = &mut self.root;

        for ch in word_lower.chars() {
            node = node.children.entry(ch).or_default();
        }

        if !node.is_word {
            self.word_count += 1;
            node.is_word = true;
            node.word_info = Some(info);
        } else {
            // Solo sobrescribir si la nueva entrada tiene mayor frecuencia
            if let Some(ref existing) = node.word_info {
                if info.frequency > existing.frequency {
                    node.word_info = Some(info);
                }
            } else {
                node.word_info = Some(info);
            }
        }
    }

    /// Inserta una palabra con información por defecto
    pub fn insert_word(&mut self, word: &str) {
        self.insert(word, WordInfo::default());
    }

    /// Verifica si una palabra existe en el Trie
    pub fn contains(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();
        let mut node = &self.root;

        for ch in word_lower.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return false,
            }
        }

        node.is_word
    }

    /// Obtiene la información de una palabra
    pub fn get(&self, word: &str) -> Option<&WordInfo> {
        let word_lower = word.to_lowercase();
        let mut node = &self.root;

        for ch in word_lower.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return None,
            }
        }

        if node.is_word {
            node.word_info.as_ref()
        } else {
            None
        }
    }

    /// Obtiene todas las palabras del Trie con su información
    pub fn get_all_words(&self) -> Vec<(String, WordInfo)> {
        let mut words = Vec::with_capacity(self.word_count);
        self.collect_words(&self.root, String::new(), &mut words);
        words
    }

    fn collect_words(&self, node: &TrieNode, prefix: String, words: &mut Vec<(String, WordInfo)>) {
        if node.is_word {
            if let Some(ref info) = node.word_info {
                words.push((prefix.clone(), info.clone()));
            }
        }

        for (ch, child) in &node.children {
            let mut new_prefix = prefix.clone();
            new_prefix.push(*ch);
            self.collect_words(child, new_prefix, words);
        }
    }

    /// Obtiene palabras que empiezan con un prefijo
    pub fn get_words_with_prefix(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let mut node = &self.root;

        for ch in prefix_lower.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return vec![],
            }
        }

        let mut words = Vec::new();
        self.collect_word_strings(node, prefix_lower, &mut words);
        words
    }

    fn collect_word_strings(&self, node: &TrieNode, prefix: String, words: &mut Vec<String>) {
        if node.is_word {
            words.push(prefix.clone());
        }

        for (ch, child) in &node.children {
            let mut new_prefix = prefix.clone();
            new_prefix.push(*ch);
            self.collect_word_strings(child, new_prefix, words);
        }
    }

    /// Número de palabras en el Trie
    pub fn len(&self) -> usize {
        self.word_count
    }

    /// Verifica si el Trie está vacío
    pub fn is_empty(&self) -> bool {
        self.word_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_contains() {
        let mut trie = Trie::new();
        trie.insert_word("hola");
        trie.insert_word("mundo");

        assert!(trie.contains("hola"));
        assert!(trie.contains("mundo"));
        assert!(!trie.contains("adios"));
    }

    #[test]
    fn test_case_insensitive() {
        let mut trie = Trie::new();
        trie.insert_word("Hola");

        assert!(trie.contains("hola"));
        assert!(trie.contains("HOLA"));
        assert!(trie.contains("HoLa"));
    }

    #[test]
    fn test_word_info() {
        let mut trie = Trie::new();
        trie.insert(
            "casa",
            WordInfo {
                category: WordCategory::Sustantivo,
                gender: Gender::Feminine,
                number: Number::Singular,
                extra: String::new(),
                frequency: 100,
            },
        );

        let info = trie.get("casa").unwrap();
        assert_eq!(info.category, WordCategory::Sustantivo);
        assert_eq!(info.gender, Gender::Feminine);
        assert_eq!(info.number, Number::Singular);
    }

    #[test]
    fn test_prefix_search() {
        let mut trie = Trie::new();
        trie.insert_word("casa");
        trie.insert_word("casas");
        trie.insert_word("casero");
        trie.insert_word("perro");

        let words = trie.get_words_with_prefix("cas");
        assert_eq!(words.len(), 3);
        assert!(words.contains(&"casa".to_string()));
        assert!(words.contains(&"casas".to_string()));
        assert!(words.contains(&"casero".to_string()));
    }
}
