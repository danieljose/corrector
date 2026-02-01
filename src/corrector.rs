//! Motor principal de corrección

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::config::Config;
use crate::dictionary::{DictionaryLoader, ProperNames, Trie};
use crate::grammar::{GrammarAnalyzer, Tokenizer};
use crate::languages::spanish::{CapitalizationAnalyzer, CompoundVerbAnalyzer, DequeismoAnalyzer, DiacriticAnalyzer, HomophoneAnalyzer, PleonasmAnalyzer, PronounAnalyzer, PunctuationAnalyzer, RelativeAnalyzer, SubjectVerbAnalyzer, VocativeAnalyzer};
use crate::languages::{get_language, Language};
use crate::spelling::SpellingCorrector;

/// Motor principal del corrector
pub struct Corrector {
    dictionary: Trie,
    proper_names: ProperNames,
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

        // Cargar diccionario principal
        let dict_path = config.data_dir.join(&config.language).join("words.txt");
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
        let custom_dict_path = config.data_dir.join(&config.language).join("custom.txt");
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

        Ok(Self {
            dictionary,
            proper_names,
            tokenizer: Tokenizer::new(),
            grammar_analyzer,
            language,
            config: config.clone(),
            custom_dict_path,
        })
    }

    /// Corrige el texto proporcionado
    pub fn correct(&self, text: &str) -> String {
        let mut tokens = self.tokenizer.tokenize(text);
        let spelling_corrector = SpellingCorrector::new(&self.dictionary)
            .with_verb_recognition();

        // Fase 1: Corrección ortográfica
        for token in &mut tokens {
            if !token.is_word() {
                continue;
            }

            // Verificar si la palabra es una excepción conocida
            if self.language.is_exception(&token.text) {
                continue;
            }

            // Verificar si es un nombre propio (empieza con mayúscula y está en la lista)
            if self.proper_names.is_proper_name(&token.text) {
                continue;
            }

            // Verificar si es una palabra compuesta con guión donde cada parte es válida
            if token.text.contains('-') {
                if self.is_valid_compound_word(&token.text, &spelling_corrector) {
                    continue;
                }
            }

            // Skip technical measurements: number + unit abbreviation (500W, 100km, etc.)
            // Pattern: starts with digit(s), ends with letter(s)
            if Self::is_technical_measurement(&token.text) {
                continue;
            }

            // Skip uppercase codes/acronyms: BB, BBB, UK, DD, HH, BBB-, BB+, etc.
            if Self::is_uppercase_code(&token.text) {
                continue;
            }

            if !spelling_corrector.is_correct(&token.text) {
                let suggestions = spelling_corrector.get_suggestions(&token.text);
                if !suggestions.is_empty() {
                    let suggestion_text: Vec<String> =
                        suggestions.iter().map(|s| s.word.clone()).collect();
                    token.corrected_spelling = Some(suggestion_text.join(","));
                } else {
                    token.corrected_spelling = Some("?".to_string());
                }
            }
        }

        // Fase 2: Corrección gramatical
        // Trabajamos con las palabras corregidas ortográficamente
        let corrections = self
            .grammar_analyzer
            .analyze(&mut tokens, &self.dictionary, self.language.as_ref());

        // Aplicar correcciones gramaticales a los tokens
        for correction in corrections {
            if correction.token_index < tokens.len() {
                tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
            }
        }

        // Fase 3: Corrección de tildes diacríticas (solo para español)
        if self.config.language == "es" {
            let diacritic_corrections = DiacriticAnalyzer::analyze(&tokens);
            for correction in diacritic_corrections {
                if correction.token_index < tokens.len() {
                    // Solo aplicar si no hay ya una corrección gramatical
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion);
                    }
                }
            }
        }

        // Fase 4: Corrección de homófonos (solo para español)
        if self.config.language == "es" {
            let homophone_corrections = HomophoneAnalyzer::analyze(&tokens);
            for correction in homophone_corrections {
                if correction.token_index < tokens.len() {
                    // Solo aplicar si no hay ya una corrección
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion);
                    }
                }
            }
        }

        // Fase 5: Corrección de dequeísmo/queísmo (solo para español)
        if self.config.language == "es" {
            let deq_corrections = DequeismoAnalyzer::analyze(&tokens);
            for correction in deq_corrections {
                if correction.token_index < tokens.len() {
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        let suggestion = match correction.error_type {
                            crate::languages::spanish::dequeismo::DequeismoErrorType::Dequeismo => {
                                "sobra".to_string() // "de" sobra
                            }
                            crate::languages::spanish::dequeismo::DequeismoErrorType::Queismo => {
                                correction.suggestion.clone() // "de que" en lugar de "que"
                            }
                        };
                        tokens[correction.token_index].corrected_grammar = Some(suggestion);
                    }
                }
            }
        }

        // Fase 6: Corrección de laísmo/leísmo/loísmo (solo para español)
        if self.config.language == "es" {
            let pronoun_corrections = PronounAnalyzer::analyze(&tokens);
            for correction in pronoun_corrections {
                if correction.token_index < tokens.len() {
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion.clone());
                    }
                }
            }
        }

        // Fase 7: Corrección de tiempos compuestos (solo para español)
        if self.config.language == "es" {
            let compound_analyzer = CompoundVerbAnalyzer::new();
            let compound_corrections = compound_analyzer.analyze(&tokens);
            for correction in compound_corrections {
                if correction.token_index < tokens.len() {
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion.clone());
                    }
                }
            }
        }

        // Fase 8: Corrección de concordancia sujeto-verbo (solo para español)
        if self.config.language == "es" {
            let subject_verb_corrections = SubjectVerbAnalyzer::analyze(&tokens);
            for correction in subject_verb_corrections {
                if correction.token_index < tokens.len() {
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion.clone());
                    }
                }
            }
        }

        // Fase 9: Corrección de concordancia de relativos (solo para español)
        if self.config.language == "es" {
            let relative_corrections = RelativeAnalyzer::analyze(&tokens);
            for correction in relative_corrections {
                if correction.token_index < tokens.len() {
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion.clone());
                    }
                }
            }
        }

        // Fase 10: Detección de pleonasmos (solo para español)
        if self.config.language == "es" {
            let pleonasm_corrections = PleonasmAnalyzer::analyze(&tokens);
            for correction in pleonasm_corrections {
                if correction.token_index < tokens.len() {
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion.clone());
                    }
                }
            }
        }

        // Fase 11: Corrección de mayúsculas (solo para español)
        if self.config.language == "es" {
            let cap_corrections = CapitalizationAnalyzer::analyze(&tokens);
            for correction in cap_corrections {
                if correction.token_index < tokens.len() {
                    // Solo aplicar si no hay ya una corrección
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion);
                    }
                }
            }
        }

        // Fase 12: Validación de puntuación (solo para español)
        if self.config.language == "es" {
            let punct_errors = PunctuationAnalyzer::analyze(&tokens);
            for error in punct_errors {
                if error.token_index < tokens.len() {
                    // Marcar el signo con su error
                    if tokens[error.token_index].corrected_grammar.is_none() {
                        let suggestion = match error.error_type {
                            crate::languages::spanish::punctuation::PunctuationErrorType::MissingOpening => {
                                format!("falta {}", Self::get_opening_sign(&error.original))
                            }
                            crate::languages::spanish::punctuation::PunctuationErrorType::MissingClosing => {
                                format!("falta {}", Self::get_closing_sign(&error.original))
                            }
                            crate::languages::spanish::punctuation::PunctuationErrorType::Unbalanced => {
                                "desbalanceado".to_string()
                            }
                        };
                        tokens[error.token_index].corrected_grammar = Some(suggestion);
                    }
                }
            }
        }

        // Fase 13: Corrección de comas vocativas (solo para español)
        if self.config.language == "es" {
            let vocative_corrections = VocativeAnalyzer::analyze(&tokens);
            for correction in vocative_corrections {
                if correction.token_index < tokens.len() {
                    // Solo aplicar si no hay ya una corrección
                    if tokens[correction.token_index].corrected_grammar.is_none() {
                        tokens[correction.token_index].corrected_grammar =
                            Some(correction.suggestion);
                    }
                }
            }
        }

        // Fase 14: Reconstruir texto con marcadores
        self.reconstruct_with_markers(&tokens)
    }

    /// Reconstruye el texto con marcadores de corrección
    fn reconstruct_with_markers(&self, tokens: &[crate::grammar::Token]) -> String {
        use crate::grammar::tokenizer::TokenType;

        let mut result = String::new();
        let sep = &self.config.spelling_separator;
        let (gram_open, gram_close) = &self.config.grammar_separator;

        for (i, token) in tokens.iter().enumerate() {
            // Si este token es whitespace y el anterior tenía corrección, saltarlo
            // (el espacio ya fue añadido después del marcador)
            if token.token_type == TokenType::Whitespace && i > 0 {
                let prev = &tokens[i - 1];
                if prev.corrected_spelling.is_some() || prev.corrected_grammar.is_some() {
                    continue;
                }
            }

            result.push_str(&token.text);

            let has_correction =
                token.corrected_spelling.is_some() || token.corrected_grammar.is_some();
            let has_next_whitespace = tokens
                .get(i + 1)
                .map(|t| t.token_type == TokenType::Whitespace)
                .unwrap_or(false);

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

            // Si hubo corrección y hay whitespace después, añadir espacio
            // (reemplaza el whitespace que saltaremos en la siguiente iteración)
            if has_correction && has_next_whitespace {
                result.push(' ');
            }
        }

        result
    }

    /// Añade una palabra al diccionario personalizado
    pub fn add_custom_word(&mut self, word: &str) -> Result<(), String> {
        // Crear directorio si no existe
        if let Some(parent) = self.custom_dict_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Error creando directorio: {}", e))?;
        }

        // Añadir al archivo
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.custom_dict_path)
            .map_err(|e| format!("Error abriendo archivo: {}", e))?;

        writeln!(file, "{}", word)
            .map_err(|e| format!("Error escribiendo: {}", e))?;

        // Añadir al diccionario en memoria
        self.dictionary.insert_word(word);

        Ok(())
    }

    /// Verifica si una palabra está en el diccionario o es una forma verbal válida
    pub fn is_word_known(&self, word: &str) -> bool {
        let corrector = SpellingCorrector::new(&self.dictionary)
            .with_verb_recognition();
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
    /// Ejemplos: 500W, 100km, 13.6kWh, 17kWh
    fn is_technical_measurement(word: &str) -> bool {
        if word.is_empty() {
            return false;
        }

        // Debe empezar con dígito
        let first_char = word.chars().next().unwrap();
        if !first_char.is_ascii_digit() {
            return false;
        }

        // Buscar la transición de dígitos/puntos/comas a letras
        let mut found_digit = false;
        let mut found_letter = false;

        for ch in word.chars() {
            if ch.is_ascii_digit() || ch == '.' || ch == ',' {
                if found_letter {
                    // Dígito después de letra no es medida típica (ej: "km2" es ok, pero raro)
                    // Permitirlo de todos modos para casos como "CO2"
                }
                found_digit = true;
            } else if ch.is_alphabetic() {
                found_letter = true;
            } else {
                // Otro carácter, no es medida típica
                return false;
            }
        }

        // Debe tener tanto dígitos como letras
        found_digit && found_letter
    }

    /// Detecta siglas y códigos en mayúsculas que no deben corregirse
    /// Ejemplos: BB, BBB, UK, DD, HH, BBB-, BB+, A+, etc.
    fn is_uppercase_code(word: &str) -> bool {
        if word.is_empty() || word.len() > 6 {
            return false;
        }

        // Extraer la parte alfabética (sin guiones ni signos finales)
        let alpha_part: String = word.chars()
            .take_while(|c| c.is_alphabetic())
            .collect();

        // Debe tener al menos 1 letra
        if alpha_part.is_empty() {
            return false;
        }

        // Todas las letras deben ser mayúsculas
        if !alpha_part.chars().all(|c| c.is_uppercase()) {
            return false;
        }

        // El resto (si existe) debe ser signos como +, -, etc.
        let suffix: String = word.chars()
            .skip(alpha_part.len())
            .collect();

        // Sufijo vacío o solo signos permitidos (+, -, números)
        suffix.is_empty() || suffix.chars().all(|c| c == '+' || c == '-' || c.is_numeric())
    }

    /// Obtiene sugerencias para una palabra
    pub fn get_suggestions(&self, word: &str) -> Vec<String> {
        let corrector = SpellingCorrector::new(&self.dictionary)
            .with_verb_recognition();
        corrector
            .get_suggestions(word)
            .into_iter()
            .map(|s| s.word)
            .collect()
    }

    /// Obtiene el signo de apertura correspondiente a un signo de cierre
    fn get_opening_sign(closing: &str) -> &'static str {
        match closing {
            "?" => "¿",
            "!" => "¡",
            _ => "¿",
        }
    }

    /// Obtiene el signo de cierre correspondiente a un signo de apertura
    fn get_closing_sign(opening: &str) -> &'static str {
        match opening {
            "¿" => "?",
            "¡" => "!",
            _ => "?",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_corrector() -> Corrector {
        let config = Config {
            language: "es".to_string(),
            data_dir: PathBuf::from("data"),
            custom_dict: None,
            input_file: None,
            output_file: None,
            add_word: None,
            text: None,
            show_help: false,
            spelling_separator: "|".to_string(),
            grammar_separator: ("[".to_string(), "]".to_string()),
        };
        Corrector::new(&config).expect("Failed to create corrector")
    }

    // ==========================================================================
    // Tests de integración: flujo entre fases (effective_text)
    // ==========================================================================
    // Estos tests verifican que las correcciones de una fase son visibles
    // para las fases posteriores a través de Token::effective_text()

    #[test]
    fn test_integration_diacritics_then_subject_verb() {
        // Flujo: "tu canto" → (diacríticas) "tú" → (sujeto-verbo) "cantas"
        // Este test verifica que effective_text() propaga correcciones entre fases
        let corrector = create_test_corrector();
        let result = corrector.correct("tu canto muy bien");

        // Debe corregir AMBOS: "tu" → "tú" Y "canto" → "cantas"
        assert!(result.contains("[tú]"), "Debería corregir 'tu' → 'tú': {}", result);
        assert!(result.contains("[cantas]"), "Debería corregir 'canto' → 'cantas': {}", result);
    }

    #[test]
    fn test_integration_diacritics_then_subject_verb_hablo() {
        // Otro caso: "tu hablo" → "tú hablas"
        let corrector = create_test_corrector();
        let result = corrector.correct("tu hablo español");

        assert!(result.contains("[tú]"), "Debería corregir 'tu' → 'tú': {}", result);
        assert!(result.contains("[hablas]"), "Debería corregir 'hablo' → 'hablas': {}", result);
    }

    #[test]
    fn test_integration_possessive_tu_not_corrected() {
        // "tu casa" NO debe cambiar "tu" a "tú" (es posesivo válido)
        let corrector = create_test_corrector();
        let result = corrector.correct("tu casa es bonita");

        // No debe sugerir "tú" cuando es posesivo seguido de sustantivo
        assert!(!result.contains("[tú]"), "No debería corregir 'tu' posesivo: {}", result);
    }

    #[test]
    fn test_integration_correct_tu_cantas_unchanged() {
        // "tú cantas" ya es correcto, no debe haber corrección de sujeto-verbo
        let corrector = create_test_corrector();
        let result = corrector.correct("tú cantas muy bien");

        // No debe sugerir cambio en "cantas" (ya concuerda con "tú")
        assert!(!result.contains("[cantas]"), "No debería cambiar 'cantas' correcto: {}", result);
        assert!(!result.contains("[canto]"), "No debería cambiar a 'canto': {}", result);
    }

    #[test]
    fn test_integration_spelling_then_grammar() {
        // Verifica que correcciones ortográficas son visibles para gramática
        // (si hay un error ortográfico que al corregirse afecta el análisis)
        let corrector = create_test_corrector();

        // "el" + sustantivo femenino corregido ortográficamente
        // Este test verifica el flujo ortografía → gramática
        let result = corrector.correct("la casa blanco");
        assert!(result.contains("[blanca]"), "Debería corregir concordancia: {}", result);
    }

    #[test]
    fn test_integration_spelling_propagates_to_article_noun() {
        // Verifica que la corrección ortográfica propaga word_info a la gramática
        // "este cassa" → spelling: "cassa"→"casa" → grammar debe ver "casa" (fem)
        // y corregir "este"→"esta"
        let corrector = create_test_corrector();
        let result = corrector.correct("este cassa blanca");

        // Debe corregir ortografía: "cassa" (primera sugerencia es "casa")
        assert!(result.contains("|casa,") || result.contains("|casa|"),
            "Debería sugerir 'casa' para 'cassa': {}", result);
        // Debe corregir gramática: "este" → "esta" (porque "casa" es femenino)
        assert!(result.contains("[esta]"), "Debería corregir 'este' → 'esta': {}", result);
    }

    #[test]
    fn test_integration_spelling_propagates_to_adjective() {
        // Similar pero con adjetivo: "la cassa blanco"
        // spelling: "cassa"→"casa" → grammar debe ver "casa" (fem) y corregir "blanco"→"blanca"
        let corrector = create_test_corrector();
        let result = corrector.correct("la cassa blanco");

        // Debe corregir ortografía: "cassa" (primera sugerencia es "casa")
        assert!(result.contains("|casa,") || result.contains("|casa|"),
            "Debería sugerir 'casa' para 'cassa': {}", result);
        // Debe corregir gramática: "blanco" → "blanca"
        assert!(result.contains("[blanca]"), "Debería corregir 'blanco' → 'blanca': {}", result);
    }

    // ==========================================================================
    // Tests de palabras compuestas con guión
    // ==========================================================================

    #[test]
    fn test_compound_word_proper_names() {
        // "Madrid-Sevilla" debe ser aceptado (dos nombres propios)
        let corrector = create_test_corrector();
        let result = corrector.correct("la línea Madrid-Sevilla");
        assert!(!result.contains("|?|"), "No debería marcar Madrid-Sevilla como desconocida: {}", result);
        assert!(!result.contains("Madrid-Sevilla |"), "No debería haber corrección para Madrid-Sevilla: {}", result);
    }

    #[test]
    fn test_compound_word_mixed() {
        // "norte-sur" debe ser aceptado (dos palabras del diccionario)
        let corrector = create_test_corrector();
        let result = corrector.correct("dirección norte-sur");
        assert!(!result.contains("|?|"), "No debería marcar norte-sur como desconocida: {}", result);
    }

    #[test]
    fn test_compound_word_invalid() {
        // "asdfg-qwerty" no debe ser aceptado (palabras inexistentes)
        let corrector = create_test_corrector();
        let result = corrector.correct("esto es asdfg-qwerty");
        assert!(result.contains("|?|") || result.contains("|"), "Debería marcar como desconocida: {}", result);
    }

    #[test]
    fn test_proper_name_ai() {
        // "AI" debe ser reconocido como nombre propio (siglas)
        let corrector = create_test_corrector();
        let result = corrector.correct("Figure AI apunta a la industria");
        // No debe sugerir corrección para "AI"
        assert!(!result.contains("AI |"), "No debería corregir AI como error ortográfico: {}", result);
        assert!(!result.contains("[Ay]"), "No debería sugerir 'Ay' para AI: {}", result);
    }

    #[test]
    fn test_verb_car_orthographic_change() {
        // "indique" es subjuntivo de "indicar" (c→qu antes de e)
        let corrector = create_test_corrector();

        // Test is_word_known directly
        assert!(corrector.is_word_known("indique"), "'indique' debería ser reconocido como forma verbal de 'indicar'");
        assert!(corrector.is_word_known("aplique"), "'aplique' debería ser reconocido");
        assert!(corrector.is_word_known("busqué"), "'busqué' debería ser reconocido");

        // Test in full correction context
        let result = corrector.correct("a menos que el fabricante indique lo contrario");
        assert!(!result.contains("indique |"), "No debería marcar 'indique' como error: {}", result);
    }
}
