//! Módulo de corrección ortográfica
//!
//! Proporciona funcionalidades para detectar y sugerir correcciones ortográficas.

pub mod levenshtein;

use crate::dictionary::Trie;
use crate::languages::{Language, VerbFormRecognizer};

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
    language: &'a dyn Language,
    verb_recognizer: Option<&'a dyn VerbFormRecognizer>,
    max_distance: usize,
    max_suggestions: usize,
}

impl<'a> SpellingCorrector<'a> {
    pub fn new(dictionary: &'a Trie, language: &'a dyn Language) -> Self {
        Self {
            dictionary,
            language,
            verb_recognizer: None,
            max_distance: 2,
            max_suggestions: 5,
        }
    }

    /// Usa un reconocedor de verbos precalculado (más eficiente para uso repetido)
    pub fn with_verb_recognizer(mut self, recognizer: &'a dyn VerbFormRecognizer) -> Self {
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

        // Elisiones con apóstrofo (ej. catalán: l'home → l' + home)
        // Funciona para cualquier idioma cuyo diccionario contenga las formas elididas.
        if self.is_correct_elision(&word_lower) {
            return true;
        }

        // Abreviaturas convencionales del idioma
        if self.language.is_known_abbreviation(word) {
            return true;
        }

        // Fallback: derivar plurales ausentes (sustantivos/adjetivos)
        if self.dictionary.derive_plural_info(&word_lower).is_some() {
            return true;
        }

        // Luego verificar si es una forma verbal conjugada
        if let Some(ref recognizer) = self.verb_recognizer {
            if recognizer.is_valid_verb_form(word) {
                // Falso válido frecuente en -ger/-gir: "coje/proteje".
                // Si existe una variante en diccionario con g ante e/i,
                // forzar a tratarlo como typo.
                if self
                    .preferred_j_to_g_before_front_vowel(&word_lower)
                    .is_some()
                {
                    return false;
                }
                return true;
            }
        }

        false
    }

    /// Verifica si una palabra con apóstrofo es una elisión válida (prefijo + sufijo en diccionario)
    fn is_correct_elision(&self, word_lower: &str) -> bool {
        for apos in ['\'', '\u{2019}'] {
            if let Some(pos) = word_lower.find(apos) {
                let prefix = &word_lower[..pos + apos.len_utf8()]; // "l'" con apóstrofo incluido
                let suffix = &word_lower[pos + apos.len_utf8()..];
                if !suffix.is_empty()
                    && self.dictionary.contains(prefix)
                    && (self.dictionary.contains(suffix)
                        || self.dictionary.derive_plural_info(suffix).is_some())
                {
                    return true;
                }
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

        // Sugerencias para elisiones: buscar solo la parte tras el apóstrofo
        for apos in ['\'', '\u{2019}'] {
            if let Some(pos) = word_lower.find(apos) {
                let prefix = &word_lower[..pos + apos.len_utf8()];
                let suffix = &word_lower[pos + apos.len_utf8()..];
                if !suffix.is_empty() && self.dictionary.contains(prefix) {
                    let mut suggestions: Vec<SpellingSuggestion> = self
                        .dictionary
                        .search_within_distance(suffix, self.max_distance)
                        .into_iter()
                        // En elisiones, priorizar candidatos léxicos reales y evitar ruido
                        // técnico (p. ej., símbolos de unidades).
                        .filter(|(w, _, _)| {
                            w.chars().all(|c| {
                                c.is_alphabetic() || self.language.is_word_internal_char(c)
                            })
                        })
                        .map(|(w, info, dist)| SpellingSuggestion {
                            word: format!("{}{}", prefix, w),
                            distance: dist,
                            frequency: info.frequency,
                        })
                        .collect();
                    suggestions.sort_by(|a, b| {
                        let a_strict = levenshtein_distance(&word_lower, &a.word);
                        let b_strict = levenshtein_distance(&word_lower, &b.word);
                        let a_diacritic_equivalent = Self::is_high_confidence_diacritic_equivalent(
                            &word_lower,
                            &a.word,
                            a.frequency,
                        );
                        let b_diacritic_equivalent = Self::is_high_confidence_diacritic_equivalent(
                            &word_lower,
                            &b.word,
                            b.frequency,
                        );
                        let a_same_initial = Self::shares_initial_char(&word_lower, &a.word);
                        let b_same_initial = Self::shares_initial_char(&word_lower, &b.word);
                        let a_len_diff = word_lower.chars().count().abs_diff(a.word.chars().count());
                        let b_len_diff = word_lower.chars().count().abs_diff(b.word.chars().count());
                        a_strict
                            .cmp(&b_strict)
                            .then_with(|| b_diacritic_equivalent.cmp(&a_diacritic_equivalent))
                            .then_with(|| b_same_initial.cmp(&a_same_initial))
                            .then_with(|| b.frequency.cmp(&a.frequency))
                            .then_with(|| a.distance.cmp(&b.distance))
                            .then_with(|| a_len_diff.cmp(&b_len_diff))
                            .then_with(|| a.word.cmp(&b.word))
                    });
                    suggestions.truncate(self.max_suggestions);
                    return suggestions;
                }
            }
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
            let a_strict = levenshtein_distance(&word_lower, &a.word);
            let b_strict = levenshtein_distance(&word_lower, &b.word);
            let a_diacritic_equivalent = Self::is_high_confidence_diacritic_equivalent(
                &word_lower,
                &a.word,
                a.frequency,
            );
            let b_diacritic_equivalent = Self::is_high_confidence_diacritic_equivalent(
                &word_lower,
                &b.word,
                b.frequency,
            );
            let a_same_initial = Self::shares_initial_char(&word_lower, &a.word);
            let b_same_initial = Self::shares_initial_char(&word_lower, &b.word);
            let a_len_diff = word_lower.chars().count().abs_diff(a.word.chars().count());
            let b_len_diff = word_lower.chars().count().abs_diff(b.word.chars().count());
            a_strict
                .cmp(&b_strict)
                .then_with(|| b_diacritic_equivalent.cmp(&a_diacritic_equivalent))
                .then_with(|| b_same_initial.cmp(&a_same_initial))
                .then_with(|| b.frequency.cmp(&a.frequency))
                .then_with(|| a.distance.cmp(&b.distance))
                .then_with(|| a_len_diff.cmp(&b_len_diff))
                .then_with(|| a.word.cmp(&b.word))
        });

        let mut boosted_candidates = self.dialect_asin_candidates(&word_lower);
        for candidate in self.j_to_g_before_front_vowel_candidates(&word_lower) {
            if !boosted_candidates.iter().any(|c| c == &candidate) {
                boosted_candidates.push(candidate);
            }
        }
        for candidate in self.initial_h_omission_candidates(&word_lower) {
            if !boosted_candidates.iter().any(|c| c == &candidate) {
                boosted_candidates.push(candidate);
            }
        }
        for candidate in self.missing_accent_candidates(&word_lower) {
            if !boosted_candidates.iter().any(|c| c == &candidate) {
                boosted_candidates.push(candidate);
            }
        }
        if !boosted_candidates.is_empty() {
            suggestions.retain(|s| !boosted_candidates.iter().any(|c| c == &s.word));
            let mut boosted: Vec<SpellingSuggestion> = boosted_candidates
                .into_iter()
                .map(|word| SpellingSuggestion {
                    word,
                    distance: 1,
                    frequency: u32::MAX,
                })
                .collect();
            boosted.extend(suggestions);
            boosted.truncate(self.max_suggestions);
            return boosted;
        }

        suggestions.truncate(self.max_suggestions);
        suggestions
    }

    fn dialect_asin_candidates(&self, word_lower: &str) -> Vec<String> {
        if matches!(word_lower, "asin" | "asín" | "asina") && self.dictionary.contains("así") {
            return vec!["así".to_string()];
        }
        if word_lower == "quizas" && self.dictionary.contains("quizás") {
            return vec!["quizás".to_string()];
        }
        if word_lower == "quiza" && self.dictionary.contains("quizá") {
            return vec!["quizá".to_string()];
        }
        Vec::new()
    }

    fn preferred_j_to_g_before_front_vowel(&self, word_lower: &str) -> Option<String> {
        self.j_to_g_before_front_vowel_candidates(word_lower)
            .into_iter()
            .next()
    }

    fn j_to_g_before_front_vowel_candidates(&self, word_lower: &str) -> Vec<String> {
        let chars: Vec<char> = word_lower.chars().collect();
        if chars.len() < 2 {
            return Vec::new();
        }

        let mut out = Vec::new();
        for i in 0..(chars.len() - 1) {
            if chars[i] == 'j' && matches!(chars[i + 1], 'e' | 'i' | 'é' | 'í') {
                let mut candidate_chars = chars.clone();
                candidate_chars[i] = 'g';
                let candidate: String = candidate_chars.into_iter().collect();
                if candidate != word_lower
                    && self.dictionary.contains(&candidate)
                    && !out.iter().any(|c| c == &candidate)
                {
                    out.push(candidate);
                }
            }
        }
        out
    }

    fn initial_h_omission_candidates(&self, word_lower: &str) -> Vec<String> {
        if word_lower.len() < 3 {
            return Vec::new();
        }
        let mut chars = word_lower.chars();
        let Some(first) = chars.next() else {
            return Vec::new();
        };
        if matches!(first, 'h' | 'H')
            || !matches!(
                first,
                'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú' | 'A' | 'E' | 'I' | 'O'
                    | 'U' | 'Á' | 'É' | 'Í' | 'Ó' | 'Ú'
            )
        {
            return Vec::new();
        }

        let prefixed = format!("h{}", word_lower);
        let folded_input = Self::fold_spanish_diacritics(word_lower);
        let input_plural_like = Self::looks_plural(word_lower);

        let mut out: Vec<(String, u32, usize, usize, bool)> = self
            .dictionary
            .search_within_distance(&prefixed, 1)
            .into_iter()
            .filter_map(|(candidate, info, dist)| {
                if !candidate.starts_with('h') {
                    return None;
                }
                let candidate_without_h = candidate.strip_prefix('h').unwrap_or(&candidate);
                let folded_candidate = Self::fold_spanish_diacritics(candidate_without_h);
                let folded_distance = levenshtein_distance(&folded_input, &folded_candidate);
                if folded_distance <= 1 {
                    let same_plurality = input_plural_like == Self::looks_plural(candidate_without_h);
                    Some((candidate, info.frequency, dist, folded_distance, same_plurality))
                } else {
                    None
                }
            })
            .collect();

        out.sort_by(|a, b| {
            a.3.cmp(&b.3)
                .then_with(|| b.4.cmp(&a.4))
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| a.0.cmp(&b.0))
        });
        out.into_iter().map(|(candidate, _, _, _, _)| candidate).collect()
    }

    fn missing_accent_candidates(&self, word_lower: &str) -> Vec<String> {
        if word_lower.is_empty()
            || Self::has_spanish_diacritic(word_lower)
            || !word_lower.chars().all(|c| c.is_alphabetic())
        {
            return Vec::new();
        }

        let chars: Vec<char> = word_lower.chars().collect();
        let mut lexical: Vec<(String, u32)> = Vec::new();
        let mut verbal_fallback = Vec::new();
        for i in 0..chars.len() {
            let accented = match chars[i] {
                'a' => Some('á'),
                'e' => Some('é'),
                'i' => Some('í'),
                'o' => Some('ó'),
                'u' => Some('ú'),
                _ => None,
            };
            let Some(accented_char) = accented else {
                continue;
            };
            let mut candidate_chars = chars.clone();
            candidate_chars[i] = accented_char;
            let candidate: String = candidate_chars.into_iter().collect();
            if candidate == word_lower {
                continue;
            }
            let in_dictionary = self.dictionary.contains(&candidate);
            if in_dictionary {
                if let Some(info) = self.dictionary.get(&candidate) {
                    if Self::is_high_confidence_diacritic_equivalent(
                        word_lower,
                        &candidate,
                        info.frequency,
                    ) && !lexical.iter().any(|(c, _)| c == &candidate)
                    {
                        lexical.push((candidate.clone(), info.frequency));
                    }
                }
            } else {
                let is_valid_verb = self
                    .verb_recognizer
                    .as_ref()
                    .is_some_and(|r| r.is_valid_verb_form(&candidate));
                if is_valid_verb && !verbal_fallback.iter().any(|c| c == &candidate) {
                    verbal_fallback.push(candidate);
                }
            }
        }

        if !lexical.is_empty() {
            lexical.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            return lexical.into_iter().map(|(candidate, _)| candidate).collect();
        }

        verbal_fallback
    }

    fn shares_initial_char(input: &str, candidate: &str) -> bool {
        input.chars().next() == candidate.chars().next()
    }

    fn looks_plural(word: &str) -> bool {
        let len = word.chars().count();
        if len < 3 {
            return false;
        }
        word.ends_with('s')
    }

    fn is_high_confidence_diacritic_equivalent(
        input: &str,
        candidate: &str,
        candidate_frequency: u32,
    ) -> bool {
        !Self::has_spanish_diacritic(input)
            && Self::has_spanish_diacritic(candidate)
            && Self::fold_spanish_diacritics(input) == Self::fold_spanish_diacritics(candidate)
            && candidate_frequency >= 300
    }

    fn has_spanish_diacritic(text: &str) -> bool {
        text.chars()
            .any(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ü' | 'Á' | 'É' | 'Í' | 'Ó' | 'Ú' | 'Ü'))
    }

    fn fold_spanish_diacritics(text: &str) -> String {
        text.chars()
            .map(|c| match c {
                'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
                'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
                'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
                'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
                'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
                _ => c.to_ascii_lowercase(),
            })
            .collect()
    }
}
