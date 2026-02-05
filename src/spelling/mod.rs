//! Módulo de corrección ortográfica
//!
//! Proporciona funcionalidades para detectar y sugerir correcciones ortográficas.

pub mod levenshtein;

use crate::dictionary::Trie;
use crate::languages::spanish::VerbRecognizer;

pub use levenshtein::levenshtein_distance;

/// Resultado de una corrección ortográfica
#[derive(Debug, Clone)]
pub struct SpellingSuggestion {
    pub word: String,
    pub distance: usize,
    pub frequency: u32,
}

/// Motor de corrección ortográfica
pub struct SpellingCorrector<'a> {
    dictionary: &'a Trie,
    verb_recognizer: Option<&'a VerbRecognizer>,
    max_distance: usize,
    max_suggestions: usize,
}

impl<'a> SpellingCorrector<'a> {
    pub fn new(dictionary: &'a Trie) -> Self {
        Self {
            dictionary,
            verb_recognizer: None,
            max_distance: 2,
            max_suggestions: 5,
        }
    }

    /// Usa un reconocedor de verbos precalculado (más eficiente para uso repetido)
    pub fn with_verb_recognizer(mut self, recognizer: &'a VerbRecognizer) -> Self {
        self.verb_recognizer = Some(recognizer);
        self
    }

    pub fn with_max_distance(mut self, distance: usize) -> Self {
        self.max_distance = distance;
        self
    }

    pub fn with_max_suggestions(mut self, count: usize) -> Self {
        self.max_suggestions = count;
        self
    }

    /// Verifica si una palabra está en el diccionario o es una forma verbal válida
    pub fn is_correct(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();

        // Primero buscar en el diccionario
        if self.dictionary.contains(&word_lower) {
            return true;
        }

        // Abreviaturas de número: N.º, n.º, N.ª, n.ª
        if word_lower == "n.º" || word_lower == "n.ª" {
            return true;
        }

        // Fallback: derivar plurales ausentes (sustantivos/adjetivos)
        if self.dictionary.derive_plural_info(&word_lower).is_some() {
            return true;
        }

        // Luego verificar si es una forma verbal conjugada
        if let Some(ref recognizer) = self.verb_recognizer {
            if recognizer.is_valid_verb_form(word) {
                return true;
            }
        }

        false
    }

    /// Obtiene sugerencias para una palabra incorrecta
    /// Usa búsqueda acotada sobre el trie (no recorre todo el diccionario)
    pub fn get_suggestions(&self, word: &str) -> Vec<SpellingSuggestion> {
        let word_lower = word.to_lowercase();

        // Si la palabra está en el diccionario, no hay sugerencias
        if self.dictionary.contains(&word_lower) {
            return vec![];
        }

        // Búsqueda acotada: solo encuentra palabras dentro de max_distance
        // Complejidad O(k * n) en lugar de O(N * m * n)
        let mut suggestions: Vec<SpellingSuggestion> = self
            .dictionary
            .search_within_distance(&word_lower, self.max_distance)
            .into_iter()
            .map(|(dict_word, info, distance)| SpellingSuggestion {
                word: dict_word,
                distance,
                frequency: info.frequency,
            })
            .collect();

        // Ordenar por distancia (menor primero), luego por frecuencia (mayor primero)
        suggestions.sort_by(|a, b| {
            a.distance
                .cmp(&b.distance)
                .then_with(|| b.frequency.cmp(&a.frequency))
        });

        suggestions.truncate(self.max_suggestions);
        suggestions
    }
}
