//! Motor principal de corrección

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::config::Config;
use crate::dictionary::{DictionaryLoader, Gender, Number, ProperNames, Trie, WordCategory};
use crate::grammar::{GrammarAnalyzer, Tokenizer};
use crate::languages::{get_language, Language, VerbFormRecognizer};
use crate::spelling::SpellingCorrector;
use crate::units;

/// Motor principal del corrector
pub struct Corrector {
    dictionary: Trie,
    proper_names: ProperNames,
    verb_recognizer: Option<Box<dyn VerbFormRecognizer>>,
    tokenizer: Tokenizer,
    grammar_analyzer: GrammarAnalyzer,
    language: Box<dyn Language>,
    config: Config,
    custom_dict_path: PathBuf,
}

impl Corrector {
    /// Crea una nueva instancia del corrector
    pub fn new(config: &Config) -> Result<Self, String> {
        // Obtener implementación del idioma
        let language = get_language(&config.language)
            .ok_or_else(|| format!("Idioma no soportado: {}", config.language))?;
        let language_code = language.code().to_string();

        // Cargar diccionario principal
        let dict_path = config.data_dir.join(&language_code).join("words.txt");
        let mut dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(&dict_path)?
        } else {
            // Si no existe el archivo, crear un Trie vacío y emitir advertencia
            eprintln!(
                "Advertencia: No se encontró diccionario en '{}'. Usando diccionario vacío.",
                dict_path.display()
            );
            Trie::new()
        };

        // Cargar diccionario custom del usuario
        let custom_dict_path = config.data_dir.join(&language_code).join("custom.txt");
        if custom_dict_path.exists() {
            if let Err(e) = DictionaryLoader::append_from_file(&mut dictionary, &custom_dict_path) {
                eprintln!("Advertencia: Error cargando diccionario custom: {}", e);
            }
        }

        // Cargar diccionario adicional si se especificó
        if let Some(ref custom) = config.custom_dict {
            if let Err(e) = DictionaryLoader::append_from_file(&mut dictionary, custom) {
                return Err(format!("Error cargando diccionario '{}': {}", custom, e));
            }
        }

        if language_code == "es" {
            Self::sanitize_spanish_verb_like_noun_entries(&mut dictionary);
        }

        // Cargar nombres propios (compartidos entre todos los idiomas)
        let names_path = config.data_dir.join("names.txt");
        let proper_names = if names_path.exists() {
            match ProperNames::load_from_file(&names_path) {
                Ok(names) => names,
                Err(e) => {
                    eprintln!("Advertencia: Error cargando nombres propios: {}", e);
                    ProperNames::new()
                }
            }
        } else {
            ProperNames::new()
        };

        // Crear analizador gramatical con reglas del idioma
        let grammar_analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());

        // Inyectar despluralización específica del idioma
        language.configure_dictionary(&mut dictionary);
        let verb_recognizer = language.build_verb_recognizer(&dictionary);

        // Configurar tokenizador con caracteres internos de palabra del idioma
        let tokenizer = Tokenizer::new().with_word_internal_chars(language.word_internal_chars());

        Ok(Self {
            dictionary,
            proper_names,
            verb_recognizer,
            tokenizer,
            grammar_analyzer,
            language,
            config: config.clone(),
            custom_dict_path,
        })
    }

    /// Sanitiza entradas espurias del diccionario español:
    /// formas verbales en -ía/-ío/-ría/-río etiquetadas como sustantivo con lema vacío.
    fn sanitize_spanish_verb_like_noun_entries(dictionary: &mut Trie) {
        let mut updates: Vec<(String, crate::dictionary::WordInfo)> = Vec::new();

        for (word, mut info) in dictionary.get_all_words() {
            if Self::is_spanish_spurious_verb_like_noun(&word, &info.extra, info.category)
                && Self::is_likely_spurious_spanish_noun_verb_form(&word, dictionary)
            {
                info.category = WordCategory::Verbo;
                info.gender = Gender::None;
                info.number = Number::None;
                updates.push((word, info));
            }
        }

        for (word, info) in updates {
            let _ = dictionary.set_word_info(&word, info);
        }
    }

    fn is_spanish_spurious_verb_like_noun(word: &str, extra: &str, category: WordCategory) -> bool {
        if category != WordCategory::Sustantivo || !extra.trim().is_empty() {
            return false;
        }

        let lower = word.to_lowercase();
        lower.ends_with("\u{00ED}a")
            || lower.ends_with("\u{00ED}o")
            || lower.ends_with("r\u{00ED}a")
            || lower.ends_with("r\u{00ED}o")
    }

    fn is_likely_spurious_spanish_noun_verb_form(word: &str, dictionary: &Trie) -> bool {
        let lower = word.to_lowercase();

        // Condicional: "beneficiaría" -> "beneficiar"
        if let Some(stem) = lower.strip_suffix("r\u{00ED}a") {
            if Self::is_dictionary_verb(dictionary, &format!("{}r", stem)) {
                return true;
            }
        }

        // Formas en "-ía":
        // - imperfecto de -er/-ir: "comía", "vivía"
        // - presente de verbos en -iar: "enfría", "desvía", "amplía"
        if let Some(stem) = lower.strip_suffix("\u{00ED}a") {
            return Self::is_dictionary_verb(dictionary, &format!("{}er", stem))
                || Self::is_dictionary_verb(dictionary, &format!("{}ir", stem))
                || Self::is_dictionary_verb(dictionary, &format!("{}\u{00ED}r", stem))
                || Self::is_dictionary_verb(dictionary, &format!("{}iar", stem));
        }

        // Primera persona singular de verbos en -iar: "amplío", "ansío"
        if let Some(stem) = lower.strip_suffix("\u{00ED}o") {
            if Self::is_dictionary_verb(dictionary, &format!("{}iar", stem)) {
                return true;
            }
        }

        // Si no podemos inferir un infinitivo plausible, no recategorizar.
        false
    }

    fn is_dictionary_verb(dictionary: &Trie, word: &str) -> bool {
        dictionary
            .get(word)
            .map(|info| info.category == WordCategory::Verbo)
            .unwrap_or(false)
    }

    /// Corrige el texto proporcionado
    pub fn correct(&self, text: &str) -> String {
        let mut tokens = self.tokenizer.tokenize(text);
        let mut spelling_corrector =
            SpellingCorrector::new(&self.dictionary, self.language.as_ref());
        if let Some(vr) = self.verb_recognizer.as_deref() {
            spelling_corrector = spelling_corrector.with_verb_recognizer(vr);
        }
        let mut proper_name_cache: HashMap<String, bool> = HashMap::with_capacity(tokens.len());
        let mut spelling_ok_cache: HashMap<String, bool> = HashMap::with_capacity(tokens.len());
        let mut spelling_suggestion_cache: HashMap<String, String> =
            HashMap::with_capacity(tokens.len());
        let mut valid_compound_cache: HashMap<String, bool> = HashMap::with_capacity(tokens.len());
        let mut valid_verb_form_cache: HashMap<String, bool> = HashMap::with_capacity(tokens.len());
        let url_token_mask = Self::compute_url_token_mask(&tokens);

        // Fase 1: Corrección ortográfica
        for i in 0..tokens.len() {
            if !tokens[i].is_word() {
                continue;
            }
            let token_text = tokens[i].text.as_str();

            // Verificar si la palabra es una excepción conocida
            if self.language.is_exception(token_text) {
                continue;
            }

            // Verificar si es un nombre propio (empieza con mayúscula y está en la lista)
            let is_proper_name = if let Some(cached) = proper_name_cache.get(token_text) {
                *cached
            } else {
                let value = self.proper_names.is_proper_name(token_text);
                proper_name_cache.insert(token_text.to_string(), value);
                value
            };
            if is_proper_name {
                continue;
            }

            // Verificar si es una palabra compuesta con guión donde cada parte es válida
            if token_text.contains('-') {
                let is_valid_compound = if let Some(cached) = valid_compound_cache.get(token_text) {
                    *cached
                } else {
                    let value = self.is_valid_compound_word(token_text, &spelling_corrector);
                    valid_compound_cache.insert(token_text.to_string(), value);
                    value
                };
                if is_valid_compound {
                    continue;
                }
            }

            // Skip technical measurements: number + unit abbreviation (500W, 100km, etc.)
            // Pattern: starts with digit(s), ends with letter(s)
            if Self::is_technical_measurement(token_text) {
                continue;
            }

            // Skip uppercase codes/acronyms in mixed text: BB, BBB, UK, DD, HH, BBB-, BB+, etc.
            // In ALL-CAPS sentences (headlines), keep spelling corrections active.
            if Self::is_uppercase_code(token_text) && !Self::is_all_caps_sentence(&tokens, i) {
                continue;
            }

            // Skip compact dotted abbreviations (e.g. "i.e.", "U.S.A.").
            if Self::is_compact_dotted_abbreviation(token_text) {
                continue;
            }

            // Skip tokens that are part of URLs: https://es.wikipedia.org/wiki/...
            if url_token_mask[i] {
                continue;
            }

            // Skip unit-like words when preceded by a number: "100 kWh", "5000 mAh", "100 Mbps"
            if Self::is_unit_like(token_text) && Self::is_preceded_by_number(&tokens, i) {
                continue;
            }
            // Skip name/acronym compounds with slash: "SEO/BirdLife", "WWF/España".
            if Self::is_slash_name_or_acronym_context(&tokens, i) {
                continue;
            }

            let is_correct = if let Some(cached) = spelling_ok_cache.get(token_text) {
                *cached
            } else {
                let value = spelling_corrector.is_correct(token_text);
                spelling_ok_cache.insert(token_text.to_string(), value);
                value
            };
            if !is_correct {
                // En español, si el VerbRecognizer reconoce la forma verbal, no debe
                // entrar en el corrector ortográfico aunque la forma no exista en el diccionario
                // (ej: "cuecen" → no sugerir "crecen").
                if self.verb_recognizer.as_ref().map_or(false, |vr| {
                    if let Some(cached) = valid_verb_form_cache.get(token_text) {
                        *cached
                    } else {
                        let value = vr.is_valid_verb_form(token_text);
                        valid_verb_form_cache.insert(token_text.to_string(), value);
                        value
                    }
                }) {
                    continue;
                }

                // Fallback: si parece forma verbal y el contexto es verbal,
                // no marcar como error aunque el infinitivo no esté en diccionario
                if self
                    .language
                    .is_likely_verb_form_in_context(token_text, &tokens, i)
                {
                    continue;
                }

                let suggestion_text =
                    if let Some(cached) = spelling_suggestion_cache.get(token_text) {
                        cached.clone()
                    } else {
                        let computed: Vec<String> = spelling_corrector
                            .get_suggestions(token_text)
                            .into_iter()
                            .map(|s| s.word)
                            .collect();
                        let value = if computed.is_empty() {
                            "?".to_string()
                        } else {
                            computed.join(",")
                        };
                        spelling_suggestion_cache.insert(token_text.to_string(), value.clone());
                        value
                    };
                // Evitar correcciones idempotentes (ej: "además [además]").
                let is_identity_suggestion = suggestion_text != "?"
                    && !suggestion_text.contains(',')
                    && suggestion_text.to_lowercase() == token_text.to_lowercase();
                if is_identity_suggestion {
                    continue;
                }
                tokens[i].corrected_spelling = Some(suggestion_text);
            }
        }

        // Fase 2: Corrección gramatical
        // Trabajamos con las palabras corregidas ortográficamente
        let corrections = self.grammar_analyzer.analyze(
            &mut tokens,
            &self.dictionary,
            self.language.as_ref(),
            self.verb_recognizer.as_deref(),
        );

        // Aplicar correcciones gramaticales a los tokens
        for correction in corrections {
            if correction.token_index < tokens.len() {
                tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
            }
        }

        self.language.apply_language_specific_corrections(
            &mut tokens,
            &self.dictionary,
            &self.proper_names,
            self.verb_recognizer.as_deref(),
        );

        Self::suppress_redundant_spelling_with_grammar(&mut tokens);

        self.reconstruct_with_markers(&tokens)
    }

    fn is_non_replacement_grammar_message(text: &str) -> bool {
        text.starts_with("falta")
            || text.starts_with("sobra")
            || text == "desbalanceado"
            || text == "?"
    }

    fn is_missing_opening_punctuation_marker(text: &str) -> bool {
        text == "falta ¿" || text == "falta ¡"
    }

    fn is_missing_closing_punctuation_marker(text: &str) -> bool {
        text == "falta ?" || text == "falta !"
    }

    fn missing_punctuation_sign_from_marker(text: &str) -> Option<&'static str> {
        match text {
            "falta ¿" => Some("¿"),
            "falta ¡" => Some("¡"),
            "falta ?" => Some("?"),
            "falta !" => Some("!"),
            _ => None,
        }
    }

    fn is_case_only_grammar_change(token: &crate::grammar::Token, grammar: &str) -> bool {
        token.text.to_lowercase() == grammar.to_lowercase()
    }

    fn suppress_redundant_spelling_with_grammar(tokens: &mut [crate::grammar::Token]) {
        for token in tokens {
            let Some(grammar) = token.corrected_grammar.as_deref() else {
                continue;
            };
            if Self::is_non_replacement_grammar_message(grammar) {
                continue;
            }
            if token.corrected_spelling.is_some()
                && Self::is_case_only_grammar_change(token, grammar)
            {
                continue;
            }
            if token.corrected_spelling.is_some() {
                token.corrected_spelling = None;
            }
        }
    }

    /// Reconstruye el texto con marcadores de corrección
    fn reconstruct_with_markers(&self, tokens: &[crate::grammar::Token]) -> String {
        use crate::grammar::tokenizer::TokenType;

        let mut result = String::new();
        let sep = &self.config.spelling_separator;
        let (gram_open, gram_close) = &self.config.grammar_separator;
        let mut i = 0usize;
        while i < tokens.len() {
            let token = &tokens[i];
            // Si este token es whitespace y el anterior tenía corrección o tachado, saltarlo
            // (el whitespace se añadirá después del marcador de corrección)
            if token.token_type == TokenType::Whitespace && i > 0 {
                let prev = &tokens[i - 1];
                if prev.corrected_spelling.is_some()
                    || prev.corrected_grammar.is_some()
                    || prev.strikethrough
                {
                    i += 1;
                    continue;
                }
            }

            // Colapsar secuencias de reemplazo frasal:
            // token corregido + uno o más tokens tachados contiguos.
            // Ej.: "si [sino] ~~no~~" -> "si no [sino]".
            let mut collapse_end = i;
            let mut collapse_text = String::new();
            let mut collapsed = false;
            if token.corrected_grammar.is_some()
                && token.corrected_spelling.is_none()
                && !token.strikethrough
            {
                collapse_text.push_str(&token.text);
                let mut j = i + 1;
                let mut saw_strikethrough_word = false;
                while j < tokens.len() {
                    let t = &tokens[j];
                    if t.token_type == TokenType::Whitespace {
                        collapse_text.push_str(&t.text);
                        collapse_end = j;
                        j += 1;
                        continue;
                    }
                    let is_collapsible_removed_word = t.strikethrough
                        && t.corrected_spelling.is_none()
                        && t.corrected_grammar.is_none()
                        && t.token_type == TokenType::Word;
                    if is_collapsible_removed_word {
                        collapse_text.push_str(&t.text);
                        saw_strikethrough_word = true;
                        collapse_end = j;
                        j += 1;
                        continue;
                    }
                    break;
                }

                if saw_strikethrough_word {
                    let collapsed_phrase = collapse_text.trim_end();
                    // Para reemplazos frasales (p. ej., "a nivel de" -> "en cuanto a"),
                    // mostrar explícitamente la frase sustituida para mejorar legibilidad.
                    let should_strike_collapsed_phrase = token
                        .corrected_grammar
                        .as_deref()
                        .is_some_and(|g| g.contains(' '));
                    if should_strike_collapsed_phrase {
                        result.push_str("~~");
                        result.push_str(collapsed_phrase);
                        result.push_str("~~");
                    } else {
                        result.push_str(collapsed_phrase);
                    }
                    collapsed = true;
                }
            }

            // Si no colapsó, render normal de token.
            if !collapsed {
                if token.strikethrough {
                    result.push_str("~~");
                    result.push_str(&token.text);
                    result.push_str("~~");
                } else if token
                    .corrected_grammar
                    .as_deref()
                    .is_some_and(Self::is_missing_opening_punctuation_marker)
                {
                    // Para signos de apertura faltantes, mostrar la anotación
                    // antes del inicio de la cláusula.
                    result.push_str(gram_open);
                    result.push_str(token.corrected_grammar.as_deref().unwrap_or_default());
                    result.push_str(gram_close);
                    result.push(' ');
                    if let Some(sign) = token
                        .corrected_grammar
                        .as_deref()
                        .and_then(Self::missing_punctuation_sign_from_marker)
                    {
                        result.push_str(sign);
                    }
                    result.push_str(&token.text);
                } else {
                    result.push_str(&token.text);
                }
            }

            let has_correction = token.corrected_spelling.is_some()
                || token.corrected_grammar.is_some()
                || token.strikethrough;

            // Obtener el whitespace original que sigue (si existe)
            let next_whitespace = tokens
                .get(i + 1)
                .filter(|t| t.token_type == TokenType::Whitespace)
                .map(|t| t.text.as_str());

            // Añadir corrección ortográfica si existe
            if let Some(ref spelling) = token.corrected_spelling {
                result.push(' ');
                result.push_str(sep);
                result.push_str(spelling);
                result.push_str(sep);
            }

            // Añadir corrección gramatical si existe
            if let Some(ref grammar) = token.corrected_grammar {
                if token.corrected_spelling.is_some()
                    && Self::is_case_only_grammar_change(token, grammar)
                {
                    // Priorizar ortografía útil sobre cambio de mayúscula redundante.
                } else if !Self::is_missing_opening_punctuation_marker(grammar) {
                    if Self::is_missing_closing_punctuation_marker(grammar) {
                        if let Some(sign) = Self::missing_punctuation_sign_from_marker(grammar) {
                            result.push_str(sign);
                        }
                    }
                    result.push(' ');
                    result.push_str(gram_open);
                    result.push_str(grammar);
                    result.push_str(gram_close);
                }
            }

            // Si hubo corrección y hay whitespace después, preservar el whitespace original
            // (en lugar de reemplazarlo con un espacio fijo)
            if has_correction {
                if let Some(ws) = next_whitespace {
                    result.push_str(ws);
                }
            }

            if collapsed {
                i = collapse_end + 1;
            } else {
                i += 1;
            }
        }

        result
    }

    /// Añade una palabra al diccionario personalizado
    pub fn add_custom_word(&mut self, word: &str) -> Result<(), String> {
        // Crear directorio si no existe
        if let Some(parent) = self.custom_dict_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Error creando directorio: {}", e))?;
        }

        // Añadir al archivo
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.custom_dict_path)
            .map_err(|e| format!("Error abriendo archivo: {}", e))?;

        writeln!(file, "{}", word).map_err(|e| format!("Error escribiendo: {}", e))?;

        // Añadir al diccionario en memoria
        self.dictionary.insert_word(word);

        Ok(())
    }

    /// Verifica si una palabra está en el diccionario o es una forma verbal válida
    pub fn is_word_known(&self, word: &str) -> bool {
        let mut corrector = SpellingCorrector::new(&self.dictionary, self.language.as_ref());
        if let Some(vr) = self.verb_recognizer.as_deref() {
            corrector = corrector.with_verb_recognizer(vr);
        }
        corrector.is_correct(word)
    }

    /// Verifica si una palabra compuesta con guión es válida
    /// (cada parte debe ser un nombre propio, palabra del diccionario, o forma verbal)
    fn is_valid_compound_word(&self, word: &str, spelling_corrector: &SpellingCorrector) -> bool {
        let parts: Vec<&str> = word.split('-').collect();

        // Debe tener al menos 2 partes
        if parts.len() < 2 {
            return false;
        }

        // Cada parte debe ser válida
        for part in parts {
            // Parte vacía no es válida (ej: "Madrid--Sevilla" o "-Madrid")
            if part.is_empty() {
                return false;
            }

            // Verificar si es nombre propio
            if self.proper_names.is_proper_name(part) {
                continue;
            }

            // Verificar si está en el diccionario o es forma verbal
            if spelling_corrector.is_correct(part) {
                continue;
            }

            // Verificar si es excepción del idioma
            if self.language.is_exception(part) {
                continue;
            }

            // Si ninguna condición se cumple, la parte no es válida
            return false;
        }

        true
    }

    /// Verifica si una palabra es una medida técnica (número + unidad)
    /// Ejemplos: 500W, 100km, 13.6kWh, 17kWh, 100m², 10m^2
    fn is_technical_measurement(word: &str) -> bool {
        Self::technical_measurement_unit_start(word).is_some()
    }

    fn technical_measurement_unit_start(word: &str) -> Option<usize> {
        if word.is_empty() {
            return None;
        }

        let first_char = word.chars().next()?;
        if !first_char.is_ascii_digit() {
            return None;
        }

        let mut found_digit = false;
        let mut unit_start = None;

        for (byte_idx, ch) in word.char_indices() {
            if ch.is_ascii_digit() || ch == '.' || ch == ',' {
                found_digit = true;
            } else if ch.is_alphabetic() || Self::is_unit_suffix_char(ch) {
                if unit_start.is_none() {
                    unit_start = Some(byte_idx);
                }
            } else {
                return None;
            }
        }

        if found_digit {
            unit_start
        } else {
            None
        }
    }

    fn is_unit_suffix_char(ch: char) -> bool {
        matches!(
            ch,
            '²' | '³' | '⁻' | '¹' | '⁰' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '^' | '-'
        )
    }

    /// Extrae el sufijo de unidad de una medición técnica (ej: "100km" → "km", "10m" → "m")
    /// Retorna None si no es una medición técnica válida
    fn extract_unit_suffix_slice(word: &str) -> Option<&str> {
        let suffix_start = Self::technical_measurement_unit_start(word)?;
        word.get(suffix_start..)
    }

    #[cfg(test)]
    fn extract_unit_suffix(word: &str) -> Option<String> {
        Self::extract_unit_suffix_slice(word).map(|s| s.to_string())
    }

    fn is_uppercase_code(word: &str) -> bool {
        if word.is_empty() || word.len() > 6 {
            return false;
        }

        let mut alpha_end = None;
        for (byte_idx, ch) in word.char_indices() {
            if ch.is_alphabetic() {
                if !ch.is_uppercase() {
                    return false;
                }
                alpha_end = Some(byte_idx + ch.len_utf8());
            } else {
                break;
            }
        }

        let alpha_end = match alpha_end {
            Some(end) => end,
            None => return false,
        };

        word[alpha_end..]
            .chars()
            .all(|c| c == '+' || c == '-' || c.is_numeric())
    }

    fn is_compact_dotted_abbreviation(word: &str) -> bool {
        if !word.ends_with('.') {
            return false;
        }

        let mut chars = word.chars().peekable();
        let mut chunks = 0usize;

        loop {
            let Some(letter) = chars.next() else {
                break;
            };
            if !letter.is_alphabetic() {
                return false;
            }

            let Some(dot) = chars.next() else {
                return false;
            };
            if dot != '.' {
                return false;
            }
            chunks += 1;

            if chars.peek().is_none() {
                break;
            }
        }

        chunks >= 2
    }

    fn is_all_caps_sentence(tokens: &[crate::grammar::Token], token_idx: usize) -> bool {
        use crate::grammar::tokenizer::TokenType;

        if token_idx >= tokens.len() {
            return false;
        }

        let mut start = 0usize;
        for i in (0..=token_idx).rev() {
            if tokens[i].is_sentence_boundary() {
                start = i + 1;
                break;
            }
        }

        let mut end = tokens.len();
        for (i, token) in tokens.iter().enumerate().skip(token_idx + 1) {
            if token.is_sentence_boundary() {
                end = i;
                break;
            }
        }

        let mut total_alpha_words = 0usize;
        let mut uppercase_alpha_words = 0usize;
        for token in &tokens[start..end] {
            if token.token_type != TokenType::Word {
                continue;
            }
            let text = token.text.as_str();
            if !text.chars().any(|c| c.is_alphabetic()) {
                continue;
            }
            total_alpha_words += 1;
            if text.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
                uppercase_alpha_words += 1;
            }
        }

        // Evitar tratar una sola sigla como "oración ALL-CAPS".
        total_alpha_words >= 2 && uppercase_alpha_words * 100 >= total_alpha_words * 60
    }

    fn is_url_protocol_or_prefix(word_lower: &str) -> bool {
        matches!(word_lower, "http" | "https" | "ftp" | "www" | "mailto")
    }

    fn looks_like_name_or_acronym(word: &str) -> bool {
        if word.is_empty() || !word.chars().any(|c| c.is_alphabetic()) {
            return false;
        }
        let is_all_caps = word.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
        if is_all_caps {
            return true;
        }
        let has_upper = word.chars().any(|c| c.is_uppercase());
        let has_lower = word.chars().any(|c| c.is_lowercase());
        has_upper && has_lower
    }

    fn non_whitespace_token_index_before(tokens: &[crate::grammar::Token], idx: usize) -> Option<usize> {
        let mut i = idx;
        while i > 0 {
            i -= 1;
            if !matches!(tokens[i].token_type, crate::grammar::tokenizer::TokenType::Whitespace) {
                return Some(i);
            }
        }
        None
    }

    fn non_whitespace_token_index_after(tokens: &[crate::grammar::Token], idx: usize) -> Option<usize> {
        let mut i = idx + 1;
        while i < tokens.len() {
            if !matches!(tokens[i].token_type, crate::grammar::tokenizer::TokenType::Whitespace) {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn is_slash_name_or_acronym_context(tokens: &[crate::grammar::Token], idx: usize) -> bool {
        if idx >= tokens.len() || !tokens[idx].is_word() {
            return false;
        }
        let this_word = tokens[idx].text.as_str();
        if !Self::looks_like_name_or_acronym(this_word) {
            return false;
        }

        let prev_idx = Self::non_whitespace_token_index_before(tokens, idx);
        if let Some(slash_idx) = prev_idx {
            if tokens[slash_idx].text == "/" {
                if let Some(left_idx) = Self::non_whitespace_token_index_before(tokens, slash_idx) {
                    if tokens[left_idx].is_word()
                        && Self::looks_like_name_or_acronym(tokens[left_idx].text.as_str())
                    {
                        return true;
                    }
                }
            }
        }

        let next_idx = Self::non_whitespace_token_index_after(tokens, idx);
        if let Some(slash_idx) = next_idx {
            if tokens[slash_idx].text == "/" {
                if let Some(right_idx) = Self::non_whitespace_token_index_after(tokens, slash_idx) {
                    if tokens[right_idx].is_word()
                        && Self::looks_like_name_or_acronym(tokens[right_idx].text.as_str())
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn lowercase_if_needed(word: &str) -> Cow<'_, str> {
        if word.chars().any(|c| c.is_uppercase()) {
            Cow::Owned(word.to_lowercase())
        } else {
            Cow::Borrowed(word)
        }
    }

    fn is_common_url_tld(word_lower: &str) -> bool {
        const COMMON_TLDS: &[&str] = &[
            "com", "org", "net", "edu", "gov", "io", "co", "es", "mx", "ar", "cl", "pe", "ve",
            "ec", "bo", "py", "uy", "br", "uk", "de", "fr", "it", "pt", "ru", "cn", "jp", "kr",
            "au", "nz", "ca", "us", "info", "biz", "tv", "me", "app", "dev", "wiki", "html", "htm",
            "php", "asp", "jsp", "xml", "json", "css", "js",
        ];
        COMMON_TLDS.contains(&word_lower)
    }

    fn is_email_like(word_lower: &str) -> bool {
        let (local, domain) = match word_lower.split_once('@') {
            Some(parts) => parts,
            None => return false,
        };

        if local.is_empty() || domain.is_empty() {
            return false;
        }
        if local.starts_with('.') || local.ends_with('.') {
            return false;
        }
        if !local
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-'))
        {
            return false;
        }

        let mut labels = domain.split('.');
        let Some(first_label) = labels.next() else {
            return false;
        };
        if first_label.is_empty() {
            return false;
        }
        let mut saw_dot = false;
        let mut last_label = first_label;
        for label in labels {
            saw_dot = true;
            if label.is_empty() {
                return false;
            }
            if label.starts_with('-') || label.ends_with('-') {
                return false;
            }
            if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return false;
            }
            last_label = label;
        }

        saw_dot && last_label.len() >= 2
    }

    fn compute_url_token_mask(tokens: &[crate::grammar::Token]) -> Vec<bool> {
        use crate::grammar::tokenizer::TokenType;

        let len = tokens.len();
        let mut is_direct_url_token = vec![false; len];
        let mut has_url_anchor = vec![false; len];

        for (i, token) in tokens.iter().enumerate() {
            match token.token_type {
                TokenType::Word => {
                    let lower = Self::lowercase_if_needed(token.text.as_str());
                    let lower = lower.as_ref();
                    if Self::is_url_protocol_or_prefix(lower)
                        || Self::is_common_url_tld(lower)
                        || Self::is_email_like(lower)
                    {
                        is_direct_url_token[i] = true;
                    }
                    if lower == "http" || lower == "https" || lower == "www" {
                        has_url_anchor[i] = true;
                    }
                }
                TokenType::Punctuation => {
                    if token.text == ":"
                        && i + 2 < len
                        && tokens[i + 1].text == "/"
                        && tokens[i + 2].text == "/"
                    {
                        has_url_anchor[i] = true;
                    }
                }
                _ => {}
            }
        }

        let mut anchor_prefix = vec![0usize; len + 1];
        for i in 0..len {
            anchor_prefix[i + 1] = anchor_prefix[i] + usize::from(has_url_anchor[i]);
        }

        let context_range = 10usize;
        let mut is_url_token = is_direct_url_token;
        for (i, flag) in is_url_token.iter_mut().enumerate() {
            if *flag {
                continue;
            }
            let start = i.saturating_sub(context_range);
            let end = (i + context_range).min(len);
            *flag = anchor_prefix[end] > anchor_prefix[start];
        }

        is_url_token
    }

    /// Verifica si un token está en contexto de unidad numérica
    /// Detecta: número + unidad, número + unidad + / + unidad, número + ° + C/F
    fn is_preceded_by_number(tokens: &[crate::grammar::Token], idx: usize) -> bool {
        use crate::grammar::tokenizer::TokenType;

        let mut prev_non_ws: [usize; 4] = [usize::MAX; 4];
        let mut prev_count = 0usize;
        let mut i = idx;
        while i > 0 && prev_count < prev_non_ws.len() {
            i -= 1;
            if tokens[i].token_type == TokenType::Whitespace {
                continue;
            }
            prev_non_ws[prev_count] = i;
            prev_count += 1;
        }

        if prev_count == 0 {
            return false;
        }

        if tokens[prev_non_ws[0]].token_type == TokenType::Number {
            return true;
        }

        if tokens[prev_non_ws[0]].text == "\u{00B0}" || tokens[prev_non_ws[0]].text == "\u{00BA}" {
            if prev_count >= 2 && tokens[prev_non_ws[1]].token_type == TokenType::Number {
                return true;
            }
        }

        if tokens[prev_non_ws[0]].text == "/" {
            if prev_count >= 2 {
                let prev_word = &tokens[prev_non_ws[1]].text;

                if units::is_unit_like(prev_word) {
                    if prev_count >= 3 && tokens[prev_non_ws[2]].token_type == TokenType::Number {
                        return true;
                    }
                    if prev_count >= 3 {
                        for &prev_idx in prev_non_ws.iter().take(prev_count).skip(2) {
                            let prev_token = &tokens[prev_idx];
                            if prev_token.token_type == TokenType::Number {
                                return true;
                            }
                            if prev_token.token_type != TokenType::Whitespace {
                                break;
                            }
                        }
                    }
                }

                if Self::extract_unit_suffix_slice(prev_word).map_or(false, units::is_unit_like) {
                    return true;
                }
            }
        }

        false
    }

    pub fn is_unit_like(word: &str) -> bool {
        units::is_unit_like(word)
    }

    /// Obtiene sugerencias para una palabra
    pub fn get_suggestions(&self, word: &str) -> Vec<String> {
        let mut corrector = SpellingCorrector::new(&self.dictionary, self.language.as_ref());
        if let Some(vr) = self.verb_recognizer.as_deref() {
            corrector = corrector.with_verb_recognizer(vr);
        }
        corrector
            .get_suggestions(word)
            .into_iter()
            .map(|s| s.word)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_unit_suffix() {
        assert_eq!(
            Corrector::extract_unit_suffix("100km"),
            Some("km".to_string())
        );
        assert_eq!(Corrector::extract_unit_suffix("10m"), Some("m".to_string()));
        assert_eq!(
            Corrector::extract_unit_suffix("9.8m"),
            Some("m".to_string())
        );
        assert_eq!(
            Corrector::extract_unit_suffix("3,14rad"),
            Some("rad".to_string())
        );
        assert_eq!(Corrector::extract_unit_suffix("km"), None);
        assert_eq!(Corrector::extract_unit_suffix("100"), None);
        assert_eq!(
            Corrector::extract_unit_suffix("100m²"),
            Some("m²".to_string())
        );
        assert_eq!(
            Corrector::extract_unit_suffix("50km²"),
            Some("km²".to_string())
        );
        assert_eq!(
            Corrector::extract_unit_suffix("100m^2"),
            Some("m^2".to_string())
        );
    }
}



