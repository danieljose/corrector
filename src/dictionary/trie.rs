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
pub struct Trie {
    root: TrieNode,
    word_count: usize,
    depluralize_fn: Option<fn(&str) -> Vec<String>>,
}

impl Trie {
    pub fn new() -> Self {
        Self {
            root: TrieNode::default(),
            word_count: 0,
            depluralize_fn: None,
        }
    }

    /// Inyecta la función de despluralización específica del idioma.
    pub fn set_depluralize_fn(&mut self, f: fn(&str) -> Vec<String>) {
        self.depluralize_fn = Some(f);
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

    /// Actualiza `WordInfo` de una palabra existente, sin lógica de frecuencia.
    /// Devuelve `true` si la palabra existía y se actualizó.
    pub fn set_word_info(&mut self, word: &str, info: WordInfo) -> bool {
        let word_lower = word.to_lowercase();
        let mut node = &mut self.root;

        for ch in word_lower.chars() {
            match node.children.get_mut(&ch) {
                Some(child) => node = child,
                None => return false,
            }
        }

        if node.is_word {
            node.word_info = Some(info);
            true
        } else {
            false
        }
    }

    fn node_for_lower(&self, word_lower: &str) -> Option<&TrieNode> {
        let mut node = &self.root;

        for ch in word_lower.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return None,
            }
        }

        Some(node)
    }

    fn contains_lower(&self, word_lower: &str) -> bool {
        self.node_for_lower(word_lower)
            .map(|node| node.is_word)
            .unwrap_or(false)
    }

    fn get_lower(&self, word_lower: &str) -> Option<&WordInfo> {
        self.node_for_lower(word_lower)
            .and_then(|node| if node.is_word { node.word_info.as_ref() } else { None })
    }

    /// Verifica si una palabra existe en el Trie
    pub fn contains(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();
        self.contains_lower(&word_lower)
    }

    /// Obtiene la información de una palabra
    pub fn get(&self, word: &str) -> Option<&WordInfo> {
        let word_lower = word.to_lowercase();
        self.get_lower(&word_lower)
    }

    /// Intenta derivar `WordInfo` para un plural no presente en el diccionario,
    /// buscando un singular conocido y devolviendo una entrada con `number=Plural`.
    ///
    /// Solo deriva desde sustantivos y adjetivos, y solo si el singular es
    /// `Singular` o `None` (no deriva desde entradas marcadas como plurales).
    pub fn derive_plural_info(&self, word: &str) -> Option<WordInfo> {
        let depluralize = self.depluralize_fn?;

        let word_lower = word.to_lowercase();

        // Solo interesa como fallback cuando no existe ya en diccionario.
        if self.contains_lower(&word_lower) {
            return None;
        }

        // Heurística rápida: la mayoría de plurales terminan en 's'.
        if !word_lower.ends_with('s') {
            return None;
        }

        for singular in depluralize(&word_lower) {
            if singular.is_empty() {
                continue;
            }
            let info = match self.get_lower(&singular) {
                Some(i) => i,
                None => continue,
            };

            if !matches!(
                info.category,
                WordCategory::Sustantivo | WordCategory::Adjetivo
            ) {
                continue;
            }

            // Evitar derivar desde entradas ya marcadas como plural.
            if info.number == Number::Plural {
                continue;
            }

            let mut derived = info.clone();
            derived.number = Number::Plural;
            derived.frequency = (derived.frequency / 2).max(1);
            return Some(derived);
        }

        None
    }

    /// Obtiene información de la palabra, con fallback a plural derivado.
    pub fn get_or_derive(&self, word: &str) -> Option<WordInfo> {
        if let Some(info) = self.get(word) {
            return Some(info.clone());
        }
        self.derive_plural_info(word)
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

    /// Busca palabras dentro de una distancia Levenshtein máxima
    /// Usa búsqueda acotada sobre el trie para evitar recorrer todo el diccionario
    /// Complejidad: O(k * n) donde k es max_distance y n es la longitud de la palabra
    /// en lugar de O(N * m * n) donde N es el tamaño del diccionario
    pub fn search_within_distance(
        &self,
        word: &str,
        max_distance: usize,
    ) -> Vec<(String, WordInfo, usize)> {
        let word_lower: Vec<char> = word.to_lowercase().chars().collect();
        let word_len = word_lower.len();

        // Fila inicial de la matriz de Levenshtein: [0, 1, 2, ..., word_len]
        let initial_row: Vec<usize> = (0..=word_len).collect();

        let mut results = Vec::new();
        self.search_recursive(
            &self.root,
            &word_lower,
            String::new(),
            initial_row,
            max_distance,
            &mut results,
        );

        results
    }

    fn search_recursive(
        &self,
        node: &TrieNode,
        word: &[char],
        prefix: String,
        prev_row: Vec<usize>,
        max_distance: usize,
        results: &mut Vec<(String, WordInfo, usize)>,
    ) {
        let word_len = word.len();

        // Si este nodo es una palabra completa, verificar si está dentro de la distancia
        if node.is_word {
            // La distancia final es el último elemento de prev_row
            let distance = prev_row[word_len];
            if distance <= max_distance {
                if let Some(ref info) = node.word_info {
                    results.push((prefix.clone(), info.clone(), distance));
                }
            }
        }

        // Explorar hijos
        for (&ch, child) in &node.children {
            // Calcular nueva fila de Levenshtein para este carácter
            let mut current_row = Vec::with_capacity(word_len + 1);

            // Primera celda: distancia desde cadena vacía = longitud del prefijo actual + 1
            current_row.push(prev_row[0] + 1);

            for i in 1..=word_len {
                let insert_cost = current_row[i - 1] + 1;
                let delete_cost = prev_row[i] + 1;
                let replace_cost = if word[i - 1] == ch {
                    prev_row[i - 1] // Sin costo si los caracteres coinciden
                } else {
                    prev_row[i - 1] + 1
                };

                current_row.push(insert_cost.min(delete_cost).min(replace_cost));
            }

            // Poda: solo continuar si el mínimo de la fila <= max_distance
            // (Si todos los valores exceden max_distance, no hay forma de encontrar
            // una palabra dentro del límite en este subárbol)
            let min_in_row = *current_row.iter().min().unwrap_or(&(max_distance + 1));
            if min_in_row <= max_distance {
                let mut new_prefix = prefix.clone();
                new_prefix.push(ch);
                self.search_recursive(child, word, new_prefix, current_row, max_distance, results);
            }
        }
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
    fn test_set_word_info_updates_existing_entry() {
        let mut trie = Trie::new();
        trie.insert(
            "enfría",
            WordInfo {
                category: WordCategory::Sustantivo,
                gender: Gender::Feminine,
                number: Number::Singular,
                extra: String::new(),
                frequency: 10,
            },
        );

        let updated = trie.set_word_info(
            "enfría",
            WordInfo {
                category: WordCategory::Verbo,
                gender: Gender::None,
                number: Number::None,
                extra: String::new(),
                frequency: 10,
            },
        );

        assert!(updated, "set_word_info debería actualizar palabra existente");
        let info = trie.get("enfría").expect("la palabra debería existir");
        assert_eq!(info.category, WordCategory::Verbo);
        assert_eq!(info.gender, Gender::None);
        assert_eq!(info.number, Number::None);
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

    #[test]
    fn test_contains_numeric_prefix() {
        let mut trie = Trie::new();
        trie.insert_word("6K");
        trie.insert_word("4K");

        // After insert with "6K", it should be stored as "6k" and found
        assert!(trie.contains("6K"), "Should find 6K (uppercase)");
        assert!(trie.contains("6k"), "Should find 6k (lowercase)");
        assert!(trie.contains("4K"), "Should find 4K (uppercase)");
        assert!(trie.contains("4k"), "Should find 4k (lowercase)");
    }

    #[test]
    fn test_search_within_distance() {
        let mut trie = Trie::new();
        trie.insert_word("casa");
        trie.insert_word("casas");
        trie.insert_word("cama");
        trie.insert_word("perro");
        trie.insert_word("gato");

        // Buscar palabras a distancia 1 de "casa"
        let results = trie.search_within_distance("casa", 1);
        let words: Vec<&str> = results.iter().map(|(w, _, _)| w.as_str()).collect();

        // "casa" está a distancia 0
        assert!(
            words.contains(&"casa"),
            "Debe encontrar 'casa' (distancia 0)"
        );
        // "casas" está a distancia 1 (inserción de 's')
        assert!(
            words.contains(&"casas"),
            "Debe encontrar 'casas' (distancia 1)"
        );
        // "cama" está a distancia 1 (sustitución s->m)
        assert!(
            words.contains(&"cama"),
            "Debe encontrar 'cama' (distancia 1)"
        );
        // "perro" y "gato" están a distancia > 1
        assert!(!words.contains(&"perro"), "No debe encontrar 'perro'");
        assert!(!words.contains(&"gato"), "No debe encontrar 'gato'");
    }

    #[test]
    fn test_search_within_distance_returns_correct_distances() {
        let mut trie = Trie::new();
        trie.insert_word("casa");
        trie.insert_word("casas");
        trie.insert_word("cosa");

        let results = trie.search_within_distance("cassa", 2);

        // Verificar que las distancias son correctas
        for (word, _, distance) in &results {
            match word.as_str() {
                "casa" => assert_eq!(*distance, 1, "casa debería estar a distancia 1"),
                "casas" => assert_eq!(*distance, 2, "casas debería estar a distancia 2"),
                "cosa" => assert_eq!(*distance, 2, "cosa debería estar a distancia 2"),
                _ => {}
            }
        }
    }

    #[test]
    fn test_derive_plural_info_and_get_or_derive() {
        use crate::languages::spanish::plurals::depluralize_candidates;

        let mut trie = Trie::new();
        trie.set_depluralize_fn(depluralize_candidates);
        trie.insert(
            "abuela",
            WordInfo {
                category: WordCategory::Sustantivo,
                gender: Gender::Feminine,
                number: Number::Singular,
                extra: String::new(),
                frequency: 40,
            },
        );
        trie.insert(
            "común",
            WordInfo {
                category: WordCategory::Adjetivo,
                gender: Gender::None,
                number: Number::None,
                extra: String::new(),
                frequency: 10,
            },
        );
        trie.insert(
            "come",
            WordInfo {
                category: WordCategory::Verbo,
                gender: Gender::None,
                number: Number::None,
                extra: String::new(),
                frequency: 5,
            },
        );

        let info = trie
            .derive_plural_info("abuelas")
            .expect("Should derive from 'abuela'");
        assert_eq!(info.category, WordCategory::Sustantivo);
        assert_eq!(info.gender, Gender::Feminine);
        assert_eq!(info.number, Number::Plural);
        assert_eq!(
            info.frequency, 20,
            "Frequency should be reduced for derived forms"
        );

        let info = trie
            .get_or_derive("comunes")
            .expect("Should derive from 'común'");
        assert_eq!(info.category, WordCategory::Adjetivo);
        assert_eq!(info.number, Number::Plural);

        // No derivar desde verbos
        assert!(
            trie.derive_plural_info("comes").is_none(),
            "Should not derive noun/adj info from verb base"
        );

        // Si ya existe, no derivar
        trie.insert(
            "abuelas",
            WordInfo {
                category: WordCategory::Sustantivo,
                gender: Gender::Feminine,
                number: Number::Plural,
                extra: String::new(),
                frequency: 1,
            },
        );
        assert!(
            trie.derive_plural_info("abuelas").is_none(),
            "Should not derive when the word already exists"
        );
    }
}
