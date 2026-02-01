//! Motor principal de corrección

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use crate::config::Config;
use crate::dictionary::{DictionaryLoader, ProperNames, Trie};
use crate::grammar::{GrammarAnalyzer, Tokenizer};
use crate::languages::spanish::{CapitalizationAnalyzer, CompoundVerbAnalyzer, DequeismoAnalyzer, DiacriticAnalyzer, HomophoneAnalyzer, PleonasmAnalyzer, PronounAnalyzer, PunctuationAnalyzer, RelativeAnalyzer, SubjectVerbAnalyzer, VerbRecognizer, VocativeAnalyzer};
use crate::languages::{get_language, Language};
use crate::spelling::SpellingCorrector;
use crate::units;

/// Motor principal del corrector
pub struct Corrector {
    dictionary: Trie,
    proper_names: ProperNames,
    verb_recognizer: VerbRecognizer,
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

        // Crear reconocedor de verbos (una sola vez, reutilizado en cada correct())
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        Ok(Self {
            dictionary,
            proper_names,
            verb_recognizer,
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
            .with_verb_recognizer(&self.verb_recognizer);

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

            // Skip unit-like words when preceded by a number: "100 kWh", "5000 mAh", "100 Mbps"
            if Self::is_unit_like(&tokens[i].text) && Self::is_preceded_by_number(&tokens, i) {
                continue;
            }

            if !spelling_corrector.is_correct(&tokens[i].text) {
                // Fallback: si parece forma verbal y el contexto es verbal,
                // no marcar como error aunque el infinitivo no esté en diccionario
                if Self::is_likely_verb_form_no_dict(&tokens[i].text)
                    && Self::is_verbal_context(&tokens, i)
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
            // (el whitespace se añadirá después del marcador de corrección)
            if token.token_type == TokenType::Whitespace && i > 0 {
                let prev = &tokens[i - 1];
                if prev.corrected_spelling.is_some() || prev.corrected_grammar.is_some() {
                    continue;
                }
            }

            result.push_str(&token.text);

            let has_correction =
                token.corrected_spelling.is_some() || token.corrected_grammar.is_some();

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
            .with_verb_recognizer(&self.verb_recognizer);
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
        let corrector = SpellingCorrector::new(&self.dictionary)
            .with_verb_recognizer(&self.verb_recognizer);
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

    /// Verifica si una palabra parece forma verbal por sus terminaciones
    /// (para fallback cuando el infinitivo no está en diccionario)
    fn is_likely_verb_form_no_dict(word: &str) -> bool {
        let word_lower = word.to_lowercase();
        let len = word_lower.len();

        // Mínimo 5 caracteres para evitar falsos positivos
        if len < 5 {
            return false;
        }

        // Terminaciones muy específicas de verbos (ordenadas por longitud descendente)
        // Estas terminaciones son casi exclusivamente verbales

        // 5+ caracteres
        if word_lower.ends_with("ieron") // comieron, vivieron
            || word_lower.ends_with("ieron")
            || word_lower.ends_with("arían") // hablarían
            || word_lower.ends_with("erían") // comerían
            || word_lower.ends_with("irían") // vivirían
            || word_lower.ends_with("ieran") // comieran
            || word_lower.ends_with("iesen") // comiesen
            || word_lower.ends_with("iendo") // comiendo (gerundio)
        {
            return true;
        }

        // 4 caracteres
        if word_lower.ends_with("aron") // hablaron
            || word_lower.ends_with("aban") // hablaban
            || word_lower.ends_with("ando") // hablando (gerundio)
            || word_lower.ends_with("aste") // hablaste
            || word_lower.ends_with("iste") // comiste
            || word_lower.ends_with("amos") // hablamos (cuidado: sustantivos como "ramos")
            || word_lower.ends_with("emos") // comemos
            || word_lower.ends_with("imos") // vivimos
            || word_lower.ends_with("arán") // hablarán
            || word_lower.ends_with("erán") // comerán
            || word_lower.ends_with("irán") // vivirán
            || word_lower.ends_with("aran") // hablaran
            || word_lower.ends_with("asen") // hablasen
            || word_lower.ends_with("aría") // hablaría
            || word_lower.ends_with("ería") // comería
            || word_lower.ends_with("iría") // viviría
            || word_lower.ends_with("iera") // comiera
            || word_lower.ends_with("iese") // comiese
        {
            // Excluir palabras conocidas que no son verbos
            let non_verbs = ["abecedario", "acuario", "calendario", "canario",
                           "diario", "escenario", "horario", "salario", "vocabulario",
                           "matadero", "panadero", "soltero"];
            if non_verbs.iter().any(|&nv| word_lower == nv) {
                return false;
            }
            return true;
        }

        // 3 caracteres - muy conservador
        if word_lower.ends_with("ían") && len >= 6 { // comían, vivían
            return true;
        }

        false
    }

    /// Verifica si el contexto indica que la siguiente palabra es probablemente un verbo
    fn is_verbal_context(tokens: &[crate::grammar::Token], current_idx: usize) -> bool {
        use crate::grammar::tokenizer::TokenType;

        // Buscar palabra anterior (saltando whitespace)
        let mut prev_word_idx = None;
        for i in (0..current_idx).rev() {
            if tokens[i].token_type == TokenType::Word {
                prev_word_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = prev_word_idx {
            let prev = tokens[idx].text.to_lowercase();

            // Pronombres sujeto
            let subject_pronouns = [
                "yo", "tú", "él", "ella", "usted",
                "nosotros", "nosotras", "vosotros", "vosotras",
                "ellos", "ellas", "ustedes"
            ];
            if subject_pronouns.contains(&prev.as_str()) {
                return true;
            }

            // Relativos e interrogativos que introducen cláusulas verbales
            let verbal_introducers = ["que", "quien", "quienes", "donde", "cuando", "como"];
            if verbal_introducers.contains(&prev.as_str()) {
                return true;
            }

            // Pronombres reflexivos/objeto que preceden verbos
            let object_pronouns = ["se", "me", "te", "nos", "os", "le", "les", "lo", "la", "los", "las"];
            if object_pronouns.contains(&prev.as_str()) {
                return true;
            }
        }

        false
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

    #[test]
    fn test_whitespace_preserved_after_correction() {
        // Los saltos de línea y tabs deben preservarse después de correcciones
        let corrector = create_test_corrector();

        // Test con salto de línea después de palabra corregida
        let result = corrector.correct("cassa grande\ncasa pequeña");
        assert!(result.contains('\n'), "Debería preservar el salto de línea: {:?}", result);

        // Verificar que hay exactamente 2 líneas
        let line_count = result.lines().count();
        assert_eq!(line_count, 2, "Debería tener 2 líneas, tiene {}: {:?}", line_count, result);
    }

    #[test]
    fn test_whitespace_preserved_tab_after_grammar_correction() {
        // Los tabs deben preservarse después de correcciones gramaticales
        let corrector = create_test_corrector();

        // "el casa" → "la casa" con tab después
        let result = corrector.correct("el casa\tgrande");
        assert!(result.contains("[la]"), "Debería corregir 'el' → 'la': {}", result);
        assert!(result.contains('\t'), "Debería preservar el tab: {:?}", result);
    }

    #[test]
    fn test_whitespace_preserved_multiple_newlines() {
        // Múltiples saltos de línea deben preservarse
        let corrector = create_test_corrector();

        let result = corrector.correct("el casa\n\ngrande");
        assert!(result.contains("\n\n"), "Debería preservar los dos saltos de línea: {:?}", result);
    }

    #[test]
    fn test_whitespace_preserved_crlf() {
        // CRLF (Windows) debe preservarse
        let corrector = create_test_corrector();

        let result = corrector.correct("el casa\r\ngrande");
        assert!(result.contains("\r\n"), "Debería preservar CRLF: {:?}", result);
    }

    // ==========================================================================
    // Tests de integración para número entre artículo y sustantivo
    // ==========================================================================

    #[test]
    fn test_number_between_unit_no_correction() {
        // "los 10 MB" no debe corregirse - MB es unidad invariable
        let corrector = create_test_corrector();
        let result = corrector.correct("los 10 MB de RAM");

        // No debe haber corrección de artículo (solo mayúscula inicial)
        assert!(!result.contains("[las]"), "No debería corregir 'los' a 'las' con unidad MB: {}", result);
    }

    #[test]
    fn test_number_between_currency_corrects() {
        // "la 10 euros" debe corregirse a "los 10 euros"
        let corrector = create_test_corrector();
        let result = corrector.correct("cuesta la 10 euros");

        assert!(result.contains("[los]"), "Debería corregir 'la' a 'los' con moneda: {}", result);
    }

    #[test]
    fn test_number_between_regular_noun_corrects() {
        // "los 3 casas" debe corregirse a "las 3 casas"
        let corrector = create_test_corrector();
        let result = corrector.correct("tengo los 3 casas");

        assert!(result.contains("[las]"), "Debería corregir 'los' a 'las' con sustantivo regular: {}", result);
    }

    // ==========================================================================
    // Tests de fallback para verbos sin infinitivo en diccionario
    // ==========================================================================

    #[test]
    fn test_verb_fallback_with_subject_pronoun() {
        // "Ellos cliquearon" no debe marcarse como error (aunque "cliquear" no está)
        let corrector = create_test_corrector();
        let result = corrector.correct("Ellos cliquearon el botón");

        assert!(!result.contains("|?|"), "No debería marcar 'cliquearon' como desconocida: {}", result);
        assert!(!result.contains("cliquearon |"), "No debería haber corrección para 'cliquearon': {}", result);
    }

    #[test]
    fn test_verb_fallback_with_que() {
        // "que instanciaron" no debe marcarse como error
        let corrector = create_test_corrector();
        let result = corrector.correct("Los objetos que instanciaron funcionan");

        assert!(!result.contains("|?|"), "No debería marcar 'instanciaron' como desconocida: {}", result);
        assert!(!result.contains("instanciaron |"), "No debería haber corrección para 'instanciaron': {}", result);
    }

    #[test]
    fn test_verb_fallback_with_object_pronoun() {
        // "los cliquearon" no debe marcarse (pronombre objeto precede verbo)
        let corrector = create_test_corrector();
        let result = corrector.correct("Los usuarios los cliquearon");

        assert!(!result.contains("|?|"), "No debería marcar 'cliquearon' como desconocida: {}", result);
    }

    #[test]
    fn test_verb_fallback_without_context_marks_error() {
        // "El zumbificaron" debe marcarse como error (artículo, no pronombre)
        // Usamos verbo inventado para que no esté en diccionario
        let corrector = create_test_corrector();
        let result = corrector.correct("El zumbificaron fue rápido");

        assert!(result.contains("|?|"), "Debería marcar 'zumbificaron' sin contexto verbal: {}", result);
    }

    #[test]
    fn test_verb_fallback_gerund_with_se() {
        // "se renderizando" no debe marcarse (se + gerundio)
        let corrector = create_test_corrector();
        let result = corrector.correct("La página se está renderizando");

        assert!(!result.contains("renderizando |"), "No debería marcar 'renderizando': {}", result);
    }

    #[test]
    fn test_verb_fallback_imperfect_with_pronoun() {
        // "Nosotros deployábamos" no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Nosotros deployábamos el código");

        assert!(!result.contains("|?|"), "No debería marcar 'deployábamos' como desconocida: {}", result);
    }

    // ==========================================================================
    // Tests de unidades mixtas (kWh, mAh, dB, Mbps, etc.)
    // ==========================================================================

    #[test]
    fn test_unit_mah_with_number() {
        // "5000 mAh" no debe marcarse como error
        let corrector = create_test_corrector();
        let result = corrector.correct("La batería de 5000 mAh dura mucho");

        assert!(!result.contains("mAh |"), "No debería marcar 'mAh' como error: {}", result);
    }

    #[test]
    fn test_unit_mbps_with_number() {
        // "100 Mbps" no debe marcarse como error
        let corrector = create_test_corrector();
        let result = corrector.correct("Conexión de 100 Mbps");

        assert!(!result.contains("Mbps |"), "No debería marcar 'Mbps' como error: {}", result);
    }

    #[test]
    fn test_unit_kwh_with_number() {
        // "100 kWh" no debe marcarse como error
        let corrector = create_test_corrector();
        let result = corrector.correct("El coche tiene 100 kWh de batería");

        assert!(!result.contains("kWh |"), "No debería marcar 'kWh' como error: {}", result);
    }

    #[test]
    fn test_unit_db_with_number() {
        // "85 dB" no debe marcarse como error
        let corrector = create_test_corrector();
        let result = corrector.correct("Potencia de 85 dB");

        assert!(!result.contains("dB |"), "No debería marcar 'dB' como error: {}", result);
    }

    #[test]
    fn test_unit_without_number_marks_error() {
        // "El mAh es" debe marcarse (no hay número precedente)
        let corrector = create_test_corrector();
        let result = corrector.correct("El mAh es una unidad");

        assert!(result.contains("mAh |"), "Debería marcar 'mAh' sin número: {}", result);
    }

    // ==========================================================================
    // Tests de unidades con barra (km/h, m/s, etc.)
    // ==========================================================================

    #[test]
    fn test_unit_km_per_h() {
        // "100 km/h" no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Velocidad de 100 km/h");

        assert!(!result.contains("|"), "No debería haber errores en '100 km/h': {}", result);
    }

    #[test]
    fn test_unit_m_per_s_squared() {
        // "10 m/s²" no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Aceleración de 10 m/s²");

        assert!(!result.contains("s² |"), "No debería marcar 's²': {}", result);
    }

    #[test]
    fn test_unit_m3_per_s() {
        // "5 m³/s" no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Flujo de 5 m³/s");

        assert!(!result.contains("|"), "No debería haber errores en '5 m³/s': {}", result);
    }

    // ==========================================================================
    // Tests de temperatura (°C, °F)
    // ==========================================================================

    #[test]
    fn test_unit_celsius() {
        // "20 °C" no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Temperatura de 20 °C");

        assert!(!result.contains("C |"), "No debería marcar 'C' tras °: {}", result);
    }

    #[test]
    fn test_unit_fahrenheit() {
        // "68 °F" no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Temperatura de 68 °F");

        assert!(!result.contains("F |"), "No debería marcar 'F' tras °: {}", result);
    }

    // ==========================================================================
    // Tests de mediciones técnicas sin espacio (100km/h, 10m/s²)
    // ==========================================================================

    #[test]
    fn test_unit_km_per_h_no_space() {
        // "100km/h" (sin espacio) no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Velocidad de 100km/h");

        assert!(!result.contains("|"), "No debería haber errores en '100km/h': {}", result);
    }

    #[test]
    fn test_unit_m_per_s_squared_no_space() {
        // "10m/s²" (sin espacio) no debe marcarse
        let corrector = create_test_corrector();
        let result = corrector.correct("Aceleración de 10m/s²");

        assert!(!result.contains("|"), "No debería haber errores en '10m/s²': {}", result);
    }

    #[test]
    fn test_unit_extract_suffix() {
        // Verifica que extract_unit_suffix funciona correctamente
        assert_eq!(Corrector::extract_unit_suffix("100km"), Some("km".to_string()));
        assert_eq!(Corrector::extract_unit_suffix("10m"), Some("m".to_string()));
        assert_eq!(Corrector::extract_unit_suffix("9.8m"), Some("m".to_string()));
        assert_eq!(Corrector::extract_unit_suffix("3,14rad"), Some("rad".to_string()));
        assert_eq!(Corrector::extract_unit_suffix("km"), None); // No empieza con dígito
        assert_eq!(Corrector::extract_unit_suffix("100"), None); // No tiene letras
    }
}
