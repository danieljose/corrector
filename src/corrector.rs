//! Motor principal de corrección

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::config::Config;
use crate::dictionary::{DictionaryLoader, ProperNames, Trie};
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

    /// Corrige el texto proporcionado
    pub fn correct(&self, text: &str) -> String {
        let mut tokens = self.tokenizer.tokenize(text);
        let mut spelling_corrector =
            SpellingCorrector::new(&self.dictionary, self.language.as_ref());
        if let Some(vr) = self.verb_recognizer.as_deref() {
            spelling_corrector = spelling_corrector.with_verb_recognizer(vr);
        }

        // Fase 1: Corrección ortográfica
        for i in 0..tokens.len() {
            if !tokens[i].is_word() {
                continue;
            }

            // Verificar si la palabra es una excepción conocida
            if self.language.is_exception(&tokens[i].text) {
                continue;
            }

            // Verificar si es un nombre propio (empieza con mayúscula y está en la lista)
            if self.proper_names.is_proper_name(&tokens[i].text) {
                continue;
            }

            // Verificar si es una palabra compuesta con guión donde cada parte es válida
            if tokens[i].text.contains('-') {
                if self.is_valid_compound_word(&tokens[i].text, &spelling_corrector) {
                    continue;
                }
            }

            // Skip technical measurements: number + unit abbreviation (500W, 100km, etc.)
            // Pattern: starts with digit(s), ends with letter(s)
            if Self::is_technical_measurement(&tokens[i].text) {
                continue;
            }

            // Skip uppercase codes/acronyms: BB, BBB, UK, DD, HH, BBB-, BB+, etc.
            if Self::is_uppercase_code(&tokens[i].text) {
                continue;
            }

            // Skip tokens that are part of URLs: https://es.wikipedia.org/wiki/...
            if Self::is_part_of_url(&tokens, i) {
                continue;
            }

            // Skip unit-like words when preceded by a number: "100 kWh", "5000 mAh", "100 Mbps"
            if Self::is_unit_like(&tokens[i].text) && Self::is_preceded_by_number(&tokens, i) {
                continue;
            }

            if !spelling_corrector.is_correct(&tokens[i].text) {
                // En español, si el VerbRecognizer reconoce la forma verbal, no debe
                // entrar en el corrector ortográfico aunque la forma no exista en el diccionario
                // (ej: "cuecen" → no sugerir "crecen").
                if self
                    .verb_recognizer
                    .as_ref()
                    .map_or(false, |vr| vr.is_valid_verb_form(&tokens[i].text))
                {
                    continue;
                }

                // Fallback: si parece forma verbal y el contexto es verbal,
                // no marcar como error aunque el infinitivo no esté en diccionario
                if self
                    .language
                    .is_likely_verb_form_in_context(&tokens[i].text, &tokens, i)
                {
                    continue;
                }

                let suggestions = spelling_corrector.get_suggestions(&tokens[i].text);
                if !suggestions.is_empty() {
                    let suggestion_text: Vec<String> =
                        suggestions.iter().map(|s| s.word.clone()).collect();
                    tokens[i].corrected_spelling = Some(suggestion_text.join(","));
                } else {
                    tokens[i].corrected_spelling = Some("?".to_string());
                }
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

        self.reconstruct_with_markers(&tokens)
    }

    /// Reconstruye el texto con marcadores de corrección
    fn reconstruct_with_markers(&self, tokens: &[crate::grammar::Token]) -> String {
        use crate::grammar::tokenizer::TokenType;

        let mut result = String::new();
        let sep = &self.config.spelling_separator;
        let (gram_open, gram_close) = &self.config.grammar_separator;

        for (i, token) in tokens.iter().enumerate() {
            // Si este token es whitespace y el anterior tenía corrección o tachado, saltarlo
            // (el whitespace se añadirá después del marcador de corrección)
            if token.token_type == TokenType::Whitespace && i > 0 {
                let prev = &tokens[i - 1];
                if prev.corrected_spelling.is_some()
                    || prev.corrected_grammar.is_some()
                    || prev.strikethrough
                {
                    continue;
                }
            }

            // Si el token está tachado, mostrarlo entre ~~
            if token.strikethrough {
                result.push_str("~~");
                result.push_str(&token.text);
                result.push_str("~~");
            } else {
                result.push_str(&token.text);
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
                result.push(' ');
                result.push_str(gram_open);
                result.push_str(grammar);
                result.push_str(gram_close);
            }

            // Si hubo corrección y hay whitespace después, preservar el whitespace original
            // (en lugar de reemplazarlo con un espacio fijo)
            if has_correction {
                if let Some(ws) = next_whitespace {
                    result.push_str(ws);
                }
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
        if word.is_empty() {
            return false;
        }

        // Debe empezar con dígito
        let first_char = word.chars().next().unwrap();
        if !first_char.is_ascii_digit() {
            return false;
        }

        // Buscar la transición de dígitos/puntos/comas a letras/superíndices
        let mut found_digit = false;
        let mut found_unit_char = false;

        for ch in word.chars() {
            if ch.is_ascii_digit() || ch == '.' || ch == ',' {
                found_digit = true;
            } else if ch.is_alphabetic() || Self::is_unit_suffix_char(ch) {
                found_unit_char = true;
            } else {
                // Otro carácter no válido
                return false;
            }
        }

        // Debe tener tanto dígitos como caracteres de unidad
        found_digit && found_unit_char
    }

    /// Verifica si un carácter es válido en un sufijo de unidad
    /// Incluye superíndices (², ³, ⁻¹), ^ y - para exponentes ASCII (m^-1)
    fn is_unit_suffix_char(ch: char) -> bool {
        matches!(
            ch,
            '²' | '³' | '⁻' | '¹' | '⁰' | '⁴' | '⁵' | '⁶' | '⁷' | '⁸' | '⁹' | '^' | '-'
        )
    }

    /// Extrae el sufijo de unidad de una medición técnica (ej: "100km" → "km", "10m" → "m")
    /// Retorna None si no es una medición técnica válida
    fn extract_unit_suffix(word: &str) -> Option<String> {
        if !Self::is_technical_measurement(word) {
            return None;
        }

        // Extraer la parte alfabética final (el sufijo de unidad)
        let suffix: String = word
            .chars()
            .skip_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
            .collect();

        if suffix.is_empty() {
            None
        } else {
            Some(suffix)
        }
    }

    /// Detecta siglas y códigos en mayúsculas que no deben corregirse
    /// Ejemplos: BB, BBB, UK, DD, HH, BBB-, BB+, A+, etc.
    fn is_uppercase_code(word: &str) -> bool {
        if word.is_empty() || word.len() > 6 {
            return false;
        }

        // Extraer la parte alfabética (sin guiones ni signos finales)
        let alpha_part: String = word.chars().take_while(|c| c.is_alphabetic()).collect();

        // Debe tener al menos 1 letra
        if alpha_part.is_empty() {
            return false;
        }

        // Todas las letras deben ser mayúsculas
        if !alpha_part.chars().all(|c| c.is_uppercase()) {
            return false;
        }

        // El resto (si existe) debe ser signos como +, -, etc.
        let suffix: String = word.chars().skip(alpha_part.len()).collect();

        // Sufijo vacío o solo signos permitidos (+, -, números)
        suffix.is_empty()
            || suffix
                .chars()
                .all(|c| c == '+' || c == '-' || c.is_numeric())
    }

    /// Detecta si un token es parte de una URL
    /// Ejemplos: https://es.wikipedia.org/wiki/Articulo
    fn is_part_of_url(tokens: &[crate::grammar::Token], idx: usize) -> bool {
        use crate::grammar::tokenizer::TokenType;

        let word = &tokens[idx].text;
        let word_lower = word.to_lowercase();

        // Protocolos y prefijos de URL
        if matches!(
            word_lower.as_str(),
            "http" | "https" | "ftp" | "www" | "mailto"
        ) {
            return true;
        }

        // TLDs comunes (dominios de nivel superior)
        let common_tlds = [
            "com", "org", "net", "edu", "gov", "io", "co", "es", "mx", "ar", "cl", "pe", "ve",
            "ec", "bo", "py", "uy", "br", "uk", "de", "fr", "it", "pt", "ru", "cn", "jp", "kr",
            "au", "nz", "ca", "us", "info", "biz", "tv", "me", "app", "dev", "wiki", "html", "htm",
            "php", "asp", "jsp", "xml", "json", "css", "js",
        ];
        if common_tlds.contains(&word_lower.as_str()) {
            return true;
        }

        // Buscar contexto de URL mirando tokens cercanos
        // Si hay "://" o "www." cerca, es parte de URL
        let context_range = 10; // mirar 10 tokens atrás y adelante
        let start = idx.saturating_sub(context_range);
        let end = (idx + context_range).min(tokens.len());

        for i in start..end {
            let t = &tokens[i];
            if t.token_type == TokenType::Punctuation {
                // Detectar :// o patterns de URL
                if t.text == ":"
                    && i + 2 < tokens.len()
                    && tokens[i + 1].text == "/"
                    && tokens[i + 2].text == "/"
                {
                    return true;
                }
            }
            if t.token_type == TokenType::Word {
                let lower = t.text.to_lowercase();
                if lower == "http" || lower == "https" || lower == "www" {
                    return true;
                }
            }
        }

        false
    }

    /// Verifica si un token está en contexto de unidad numérica
    /// Detecta: número + unidad, número + unidad + / + unidad, número + ° + C/F
    fn is_preceded_by_number(tokens: &[crate::grammar::Token], idx: usize) -> bool {
        use crate::grammar::tokenizer::TokenType;

        // Buscar tokens anteriores (saltando whitespace)
        let mut prev_tokens: Vec<(usize, &crate::grammar::Token)> = Vec::new();
        for i in (0..idx).rev() {
            if tokens[i].token_type == TokenType::Whitespace {
                continue;
            }
            prev_tokens.push((i, &tokens[i]));
            if prev_tokens.len() >= 4 {
                break;
            }
        }

        if prev_tokens.is_empty() {
            return false;
        }

        // Caso 1: número directamente antes
        if prev_tokens[0].1.token_type == TokenType::Number {
            return true;
        }

        // Caso 2: ° antes (para °C, °F)
        if prev_tokens[0].1.text == "°" || prev_tokens[0].1.text == "º" {
            if prev_tokens.len() >= 2 && prev_tokens[1].1.token_type == TokenType::Number {
                return true;
            }
        }

        // Caso 3: / + unidad antes (para km/h, m/s, etc.)
        if prev_tokens[0].1.text == "/" {
            if prev_tokens.len() >= 2 {
                let prev_word = &prev_tokens[1].1.text;

                // Caso 3a: unidad directa (ej: "km / h" → "km" es unidad)
                if units::is_unit_like(prev_word) {
                    // Verificar que hay número antes de la primera unidad
                    if prev_tokens.len() >= 3 && prev_tokens[2].1.token_type == TokenType::Number {
                        return true;
                    }
                    // O whitespace + número
                    if prev_tokens.len() >= 3 {
                        for j in 2..prev_tokens.len() {
                            if prev_tokens[j].1.token_type == TokenType::Number {
                                return true;
                            }
                            if prev_tokens[j].1.token_type != TokenType::Whitespace {
                                break;
                            }
                        }
                    }
                }

                // Caso 3b: medición técnica (ej: "100km/h" → "100km" es medición)
                // Extraer el sufijo de unidad y validarlo
                if let Some(unit_suffix) = Self::extract_unit_suffix(prev_word) {
                    if units::is_unit_like(&unit_suffix) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Detecta unidades de medida (delega a módulo centralizado)
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
