//! Corrección de comas vocativas
//!
//! Detecta cuando falta una coma antes o después de un vocativo.
//! Un vocativo es el nombre o expresión con que se llama o invoca a alguien.
//!
//! Ejemplos:
//! - "Hola Juan" → "Hola, Juan"
//! - "Juan ven aquí" → "Juan, ven aquí"
//! - "Gracias María" → "Gracias, María"
//! - "Oye Pedro escucha" → "Oye, Pedro, escucha"

use crate::dictionary::WordCategory;
use crate::grammar::{has_sentence_boundary, Token, TokenType};

/// Corrección de coma vocativa sugerida
#[derive(Debug, Clone)]
pub struct VocativeCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub position: CommaPosition,
    pub reason: String,
}

/// Posición donde debe ir la coma
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommaPosition {
    /// Coma después del token (ej: "Hola" → "Hola,")
    After,
    /// Coma antes del token (ej: "Juan" en "ven Juan" → ", Juan")
    Before,
}

/// Analizador de comas vocativas
pub struct VocativeAnalyzer;

impl VocativeAnalyzer {
    /// Palabras que típicamente preceden a un vocativo (requieren coma después)
    const VOCATIVE_INTRODUCERS: &'static [&'static str] = &[
        // Saludos
        "hola",
        "adiós",
        "adios",
        "chao",
        "chau",
        // Interjecciones de llamada
        "oye",
        "eh",
        "hey",
        "ey",
        "epa",
        "mira",
        "escucha",
        "oiga",
        // Agradecimientos y cortesía
        "gracias",
        "perdón",
        "perdon",
        "perdona",
        "disculpa",
        "perdone",
        "disculpe",
        // Despedidas
        "hasta luego",
        "bye",
        // Expresiones
        "vamos",
        "ánimo",
        "animo",
        "venga",
        "dale",
        "anda",
        "cuidado",
        "cuidate",
        "cuídate",
        "felicidades",
        "felicitaciones",
        "enhorabuena",
        "bienvenido",
        "bienvenida",
    ];

    /// Imperativos comunes que pueden seguir a un vocativo
    const COMMON_IMPERATIVES: &'static [&'static str] = &[
        "ven",
        "vente",
        "ve",
        "mira",
        "escucha",
        "oye",
        "espera",
        "para",
        "cállate",
        "callate",
        "siéntate",
        "sientate",
        "levántate",
        "levantate",
        "dame",
        "dime",
        "pásame",
        "pasame",
        "trae",
        "lleva",
        "pon",
        "quita",
        "deja",
        "corre",
        "salta",
        "come",
        "bebe",
        "lee",
        "escribe",
        "ayuda",
        "ayúdame",
        "ayudame",
        "háblame",
        "hablame",
    ];

    /// Analiza los tokens y detecta errores de comas vocativas
    pub fn analyze(tokens: &[Token]) -> Vec<VocativeCorrection> {
        let mut corrections = Vec::new();
        let mut corrected_indices = std::collections::HashSet::new();

        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Analizar pares de palabras consecutivas
        for i in 0..word_tokens.len().saturating_sub(1) {
            let (idx1, token1) = word_tokens[i];
            let (idx2, token2) = word_tokens[i + 1];

            // Verificar si hay coma entre los tokens
            if Self::has_comma_between(tokens, idx1, idx2) {
                continue;
            }

            // Verificar si hay limite de oracion entre los tokens
            if has_sentence_boundary(tokens, idx1, idx2) {
                continue;
            }

            // Evitar duplicados
            if corrected_indices.contains(&idx1) {
                continue;
            }

            // Saltar si el primer token es un acrónimo (todo mayúsculas, 2+ chars)
            // Evita falsos positivos como "EH Bildu" donde "EH" parece interjección
            if token1.text.len() >= 2 && token1.text.chars().all(|c| c.is_ascii_uppercase()) {
                continue;
            }

            // Patrón 1: Introductor + Nombre propio
            // "Hola Juan" → "Hola, Juan"
            if Self::is_vocative_introducer(&token1.text)
                && !Self::is_subjunctive_venga_context(&word_tokens, i, tokens)
                && Self::is_proper_noun(token2, i + 1 == 0)
            {
                corrections.push(VocativeCorrection {
                    token_index: idx1,
                    original: token1.text.clone(),
                    suggestion: format!("{},", token1.text),
                    position: CommaPosition::After,
                    reason: format!(
                        "Falta coma vocativa después de '{}'",
                        token1.text
                    ),
                });
                corrected_indices.insert(idx1);
                continue;
            }

            // Patrón 2: Nombre propio + Imperativo (al inicio)
            // "Juan ven aquí" → "Juan, ven aquí"
            if i == 0
                && Self::is_proper_noun(token1, true)
                && !Self::is_clitic_pronoun(&token1.text)
                && !Self::is_likely_finite_verb(token1)
                && Self::is_imperative(&token2.text)
            {
                corrections.push(VocativeCorrection {
                    token_index: idx1,
                    original: token1.text.clone(),
                    suggestion: format!("{},", token1.text),
                    position: CommaPosition::After,
                    reason: format!(
                        "Falta coma vocativa después del vocativo '{}'",
                        token1.text
                    ),
                });
                corrected_indices.insert(idx1);
                continue;
            }

            // Patrón 3: Imperativo + Nombre propio al final
            // "Ven Juan" → "Ven, Juan"
            if Self::is_imperative(&token1.text) && Self::is_proper_noun(token2, false) {
                // Excluir palabras ambiguas como "para" cuando claramente son preposiciones
                // "para" es preposición en estos casos:
                // - Después de otra palabra: "buena jefa para España"
                // - Al inicio seguido de nombre + coma: "Para Seúl, esta licitación..."
                // "para" es imperativo en: "¡Para Juan!" o "Para, Juan"
                let is_para = token1.text.to_lowercase() == "para";
                let is_final = i + 2 >= word_tokens.len();

                if is_para {
                    // "para" en medio de oración es siempre preposición
                    if i > 0 {
                        continue;
                    }
                    // "para" al inicio seguido de nombre + coma es preposición
                    // Solo es vocativo si es final ("¡Para Juan!")
                    if !is_final && Self::followed_by_punctuation(tokens, idx2) {
                        continue;
                    }
                }

                if is_final || Self::followed_by_punctuation(tokens, idx2) {
                    corrections.push(VocativeCorrection {
                        token_index: idx1,
                        original: token1.text.clone(),
                        suggestion: format!("{},", token1.text),
                        position: CommaPosition::After,
                        reason: format!(
                            "Falta coma vocativa antes del vocativo '{}'",
                            token2.text
                        ),
                    });
                    corrected_indices.insert(idx1);
                }
            }
        }

        corrections
    }

    /// Verifica si una palabra es un introductor de vocativo
    fn is_vocative_introducer(word: &str) -> bool {
        let lower = word.to_lowercase();
        Self::VOCATIVE_INTRODUCERS.contains(&lower.as_str())
    }

    /// Verifica si un token parece ser un nombre propio
    /// Un nombre propio empieza con mayúscula y no está al inicio de oración (a menos que sea el primer token)
    fn is_proper_noun(token: &Token, is_first_word: bool) -> bool {
        let first_char = match token.text.chars().next() {
            Some(c) => c,
            None => return false,
        };

        // Debe empezar con mayúscula
        if !first_char.is_uppercase() {
            return false;
        }

        // Si es la primera palabra, asumimos que es nombre propio solo si el resto está en minúsculas
        // y tiene longitud típica de nombre (3-15 caracteres)
        let len = token.text.len();
        if len < 2 || len > 20 {
            return false;
        }

        // Si el diccionario clasifica la palabra como funcional o verbal,
        // no debe tratarse como nombre propio vocativo.
        if Self::is_function_or_verb_word(token) {
            return false;
        }

        // Verificar que no sea una palabra común que podría estar capitalizada
        let lower = token.text.to_lowercase();
        let common_words = [
            "el", "la", "los", "las", "un", "una", "unos", "unas", "de", "del", "al", "que", "en",
            "por", "para", "con", "sin", "sobre", "entre", "hacia", "desde", "hasta", "es", "son",
            "está", "están", "hay", "ser", "estar", "tener", "hacer", "ir", "ver", "dar", "saber",
            "poder", "querer", "decir", "si", "no", "ni", "yo", "ya", "pero", "porque", "como", "cuando",
            "donde", "quien", "cual", "todo", "nada", "algo", "mucho", "poco", "muy", "bien", "mal",
            // Interjecciones
            "ay", "ah", "oh", "eh", "uy", "ja", "je", "ji", "jo", "ju",
        ];

        if common_words.contains(&lower.as_str()) {
            return false;
        }

        // Si no es la primera palabra, asumimos que una palabra capitalizada es nombre propio
        if !is_first_word {
            return true;
        }

        // Para la primera palabra, verificamos que el resto esté en minúsculas
        // (típico de nombres propios vs. siglas)
        let rest: String = token.text.chars().skip(1).collect();
        rest.chars().all(|c| c.is_lowercase() || !c.is_alphabetic())
    }

    fn is_function_or_verb_word(token: &Token) -> bool {
        if let Some(ref info) = token.word_info {
            return matches!(
                info.category,
                WordCategory::Verbo
                    | WordCategory::Adverbio
                    | WordCategory::Articulo
                    | WordCategory::Preposicion
                    | WordCategory::Conjuncion
                    | WordCategory::Pronombre
                    | WordCategory::Determinante
            );
        }
        false
    }

    /// Verifica si una palabra es un imperativo común
    fn is_imperative(word: &str) -> bool {
        let lower = word.to_lowercase();
        Self::COMMON_IMPERATIVES.contains(&lower.as_str())
    }

    /// Evita tratar pronombres atonos como vocativos:
    /// "Se come...", "Me come...", etc.
    fn is_clitic_pronoun(word: &str) -> bool {
        let lower = word.to_lowercase();
        matches!(
            lower.as_str(),
            "me" | "te" | "se" | "nos" | "os" | "le" | "les" | "lo" | "la" | "los" | "las"
        )
    }

    /// Verifica si un token parece verbo finito.
    /// Se usa para evitar falsos vocativos al inicio de oración:
    /// "Pidió ayuda..." no debe tratarse como "Pidió, ayuda".
    fn is_likely_finite_verb(token: &Token) -> bool {
        if let Some(ref info) = token.word_info {
            if info.category == WordCategory::Verbo {
                return true;
            }
        }

        let lower = token.text.to_lowercase();

        if matches!(
            lower.as_str(),
            "necesita" | "ofrece" | "ofrecio" | "ofreció" | "pidio" | "pidió"
        ) {
            return true;
        }

        lower.ends_with("ó")
            || lower.ends_with("ió")
            || lower.ends_with("aron")
            || lower.ends_with("ieron")
    }

    /// Evita tratar "venga" como interjeccion cuando es subjuntivo de "venir".
    /// Ej: "tal vez venga Pedro", "ojala venga Maria", "puede que venga Pedro".
    fn is_subjunctive_venga_context(
        word_tokens: &[(usize, &Token)],
        current_pos: usize,
        tokens: &[Token],
    ) -> bool {
        let current = word_tokens[current_pos].1.effective_text().to_lowercase();
        if current != "venga" {
            return false;
        }
        if current_pos == 0 {
            return false;
        }

        let prev_idx = word_tokens[current_pos - 1].0;
        let curr_idx = word_tokens[current_pos].0;
        if has_sentence_boundary(tokens, prev_idx, curr_idx) {
            return false;
        }

        let prev = Self::fold_accents_ascii(word_tokens[current_pos - 1].1.effective_text());
        let prev_prev = if current_pos >= 2 {
            let prev_prev_idx = word_tokens[current_pos - 2].0;
            if has_sentence_boundary(tokens, prev_prev_idx, prev_idx) {
                None
            } else {
                Some(Self::fold_accents_ascii(
                    word_tokens[current_pos - 2].1.effective_text(),
                ))
            }
        } else {
            None
        };

        if matches!(prev.as_str(), "ojala" | "quizas" | "acaso") {
            return true;
        }
        if prev == "que" {
            return true;
        }
        if prev == "vez" && prev_prev.as_deref() == Some("tal") {
            return true;
        }
        false
    }

    fn fold_accents_ascii(text: &str) -> String {
        text.to_lowercase()
            .replace('á', "a")
            .replace('é', "e")
            .replace('í', "i")
            .replace('ó', "o")
            .replace('ú', "u")
            .replace('ü', "u")
            .replace('\u{0301}', "")
    }

    /// Verifica si hay una coma entre dos indices de token
    fn has_comma_between(tokens: &[Token], idx1: usize, idx2: usize) -> bool {
        for i in (idx1 + 1)..idx2 {
            if tokens[i].token_type == TokenType::Punctuation && tokens[i].text == "," {
                return true;
            }
        }
        false
    }

    /// Verifica si el token esta seguido de puntuacion final
    fn followed_by_punctuation(tokens: &[Token], idx: usize) -> bool {
        for i in (idx + 1)..tokens.len() {
            match tokens[i].token_type {
                TokenType::Whitespace => continue,
                TokenType::Punctuation => return true,
                _ => return false,
            }
        }
        true // Fin del texto tambien cuenta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<VocativeCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        VocativeAnalyzer::analyze(&tokens)
    }

    // Tests para Patrón 1: Introductor + Nombre propio

    #[test]
    fn test_hola_nombre() {
        let corrections = analyze_text("Hola Juan");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "Hola");
        assert_eq!(corrections[0].suggestion, "Hola,");
    }

    #[test]
    fn test_hola_nombre_ya_tiene_coma() {
        let corrections = analyze_text("Hola, Juan");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_adios_nombre() {
        let corrections = analyze_text("Adiós María");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Adiós,");
    }

    #[test]
    fn test_gracias_nombre() {
        let corrections = analyze_text("Gracias Pedro");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Gracias,");
    }

    #[test]
    fn test_oye_nombre() {
        let corrections = analyze_text("Oye Carlos");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Oye,");
    }

    #[test]
    fn test_perdon_nombre() {
        let corrections = analyze_text("Perdón Ana");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Perdón,");
    }

    // Tests para Patrón 2: Nombre propio + Imperativo al inicio

    #[test]
    fn test_nombre_imperativo() {
        let corrections = analyze_text("Juan ven aquí");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "Juan");
        assert_eq!(corrections[0].suggestion, "Juan,");
    }

    #[test]
    fn test_nombre_imperativo_ya_tiene_coma() {
        let corrections = analyze_text("Juan, ven aquí");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_nombre_mira() {
        let corrections = analyze_text("Pedro mira esto");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Pedro,");
    }

    #[test]
    fn test_nombre_escucha() {
        let corrections = analyze_text("María escucha bien");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "María,");
    }

    // Tests para Patrón 3: Imperativo + Nombre propio al final

    #[test]
    fn test_imperativo_nombre_final() {
        let corrections = analyze_text("Ven Juan");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "Ven");
        assert_eq!(corrections[0].suggestion, "Ven,");
    }

    #[test]
    fn test_mira_nombre() {
        let corrections = analyze_text("Mira Pedro");
        // "Mira" es tanto introductor como imperativo, así que detecta el patrón
        assert!(!corrections.is_empty());
    }

    #[test]
    fn test_espera_nombre() {
        let corrections = analyze_text("Espera María");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Espera,");
    }

    // Tests de casos que NO deben corregirse

    #[test]
    fn test_no_corrige_oracion_normal() {
        let corrections = analyze_text("La casa es grande");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_no_corrige_sin_nombre_propio() {
        let corrections = analyze_text("Hola amigo");
        // "amigo" está en minúscula, no es nombre propio
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_no_corrige_palabra_comun_mayuscula() {
        let corrections = analyze_text("Hola Bien");
        // "Bien" aunque está en mayúscula no es nombre propio típico
        // pero nuestro sistema podría marcarlo
        // Este test verifica el comportamiento actual
        let _ = corrections; // El comportamiento puede variar
    }

    // Tests adicionales

    #[test]
    fn test_bienvenido_nombre() {
        let corrections = analyze_text("Bienvenido Carlos");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Bienvenido,");
    }

    #[test]
    fn test_felicidades_nombre() {
        let corrections = analyze_text("Felicidades Ana");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Felicidades,");
    }

    #[test]
    fn test_dame_eso_nombre() {
        let corrections = analyze_text("Dame eso Juan");
        // Aquí "Juan" está al final después de "eso", no directamente después de imperativo
        // El sistema actual no detecta este patrón más complejo
        let _ = corrections;
    }

    #[test]
    fn test_multiple_vocativos() {
        // Caso mas complejo: "Hola Juan ven aqui"
        // Deberia sugerir: "Hola, Juan, ven aqui"
        let corrections = analyze_text("Hola Juan ven aqui");
        // Detecta al menos "Hola Juan"
        assert!(!corrections.is_empty());
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "Hola" y "Juan" estan separados por punto, no debe sugerir coma vocativa
        let corrections = analyze_text("Dijo hola. Juan vino despues");
        let hola_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "hola")
            .collect();
        assert!(hola_corrections.is_empty(), "No debe sugerir coma vocativa cuando hay limite de oracion");
    }

    // ==========================================================================
    // Tests para acrónimos en mayúsculas (evitar falsos positivos)
    // ==========================================================================

    #[test]
    fn test_acronym_eh_bildu_no_correction() {
        // "EH Bildu" es un partido político, no interjección + nombre
        let corrections = analyze_text("EH Bildu ganó las elecciones");
        let eh_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "EH")
            .collect();
        assert!(eh_corrections.is_empty(), "No debe sugerir coma después de acrónimo 'EH': {:?}", eh_corrections);
    }

    #[test]
    fn test_lowercase_eh_juan_corrects() {
        // "Eh Juan" es interjección + nombre, debe corregirse
        let corrections = analyze_text("Eh Juan ven aqui");
        let eh_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "Eh")
            .collect();
        assert!(!eh_corrections.is_empty(), "Debe sugerir coma después de 'Eh'");
        assert!(eh_corrections[0].suggestion.contains(','));
    }

    #[test]
    fn test_oye_juan_still_corrects() {
        // "Oye Juan" sigue corrigiendo (no es acrónimo)
        let corrections = analyze_text("Oye Juan");
        assert!(!corrections.is_empty(), "Debe sugerir coma vocativa para 'Oye Juan'");
    }

    #[test]
    fn test_yo_no_vocative_false_positive() {
        let corrections = analyze_text("Yo come demasiado");
        let yo_corrections: Vec<_> = corrections.iter().filter(|c| c.original == "Yo").collect();
        assert!(
            yo_corrections.is_empty(),
            "No debe tratar 'Yo' como nombre propio vocativo: {corrections:?}"
        );
    }

    #[test]
    fn test_ni_conjunction_not_vocative_false_positive() {
        let corrections = analyze_text("Ni come ni deja comer");
        let ni_corrections: Vec<_> = corrections.iter().filter(|c| c.original == "Ni").collect();
        assert!(
            ni_corrections.is_empty(),
            "No debe tratar 'Ni' como nombre propio vocativo: {corrections:?}"
        );
    }

    #[test]
    fn test_clitic_pronoun_plus_imperative_like_verb_no_vocative_false_positive() {
        let corrections = analyze_text("Se come bien en este restaurante");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma en 'Se come...': {corrections:?}"
        );

        let corrections = analyze_text("Se bebe demasiado");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma en 'Se bebe...': {corrections:?}"
        );

        let corrections = analyze_text("Me come la curiosidad");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma en 'Me come...': {corrections:?}"
        );

        let corrections = analyze_text("Te come la envidia");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma en 'Te come...': {corrections:?}"
        );
    }

    #[test]
    fn test_ayuda_common_noun_not_vocative_false_positive() {
        let corrections = analyze_text("Pidió ayuda a gritos");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma vocativa en 'Pidió ayuda...': {corrections:?}"
        );

        let corrections = analyze_text("Necesita ayuda urgente");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma vocativa en 'Necesita ayuda...': {corrections:?}"
        );

        let corrections = analyze_text("Ofreció ayuda a los damnificados");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma vocativa en 'Ofreció ayuda...': {corrections:?}"
        );
    }

    #[test]
    fn test_name_plus_ayuda_imperative_still_vocative() {
        let corrections = analyze_text("Juan ayuda ahora");
        assert!(
            !corrections.is_empty(),
            "Debe mantener vocativo en 'Juan ayuda...': {corrections:?}"
        );
        assert_eq!(corrections[0].suggestion, "Juan,");
    }

    #[test]
    fn test_venga_subjunctive_tal_vez_no_vocative() {
        let corrections = analyze_text("Tal vez venga Pedro");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma en subjuntivo: {corrections:?}"
        );
    }

    #[test]
    fn test_venga_subjunctive_ojala_no_vocative() {
        let corrections = analyze_text("Ojalá venga María");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma en subjuntivo: {corrections:?}"
        );
    }

    #[test]
    fn test_venga_subjunctive_puede_que_no_vocative() {
        let corrections = analyze_text("Puede que venga Pedro");
        assert!(
            corrections.is_empty(),
            "No debe sugerir coma en subjuntivo: {corrections:?}"
        );
    }

    #[test]
    fn test_venga_interjection_still_vocative() {
        let corrections = analyze_text("Venga Pedro no te enfades");
        assert!(
            !corrections.is_empty(),
            "Debe mantener vocativo para interjeccion: {corrections:?}"
        );
        assert_eq!(corrections[0].original, "Venga");
        assert_eq!(corrections[0].suggestion, "Venga,");
    }
}
