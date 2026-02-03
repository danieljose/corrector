//! Corrección de tildes diacríticas
//!
//! Detecta y corrige pares de palabras que se distinguen por la tilde:
//! - el/él, tu/tú, mi/mí, te/té, se/sé, de/dé, si/sí, mas/más, aun/aún

use crate::dictionary::ProperNames;
use crate::grammar::{has_sentence_boundary, Token};
use super::conjugation::VerbRecognizer;

/// Par de palabras con tilde diacrítica
#[derive(Debug, Clone)]
pub struct DiacriticPair {
    /// Palabra sin tilde
    pub without_accent: &'static str,
    /// Palabra con tilde
    pub with_accent: &'static str,
    /// Categoría gramatical sin tilde
    pub category_without: DiacriticCategory,
    /// Categoría gramatical con tilde
    pub category_with: DiacriticCategory,
}

/// Categorías gramaticales para tildes diacríticas
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DiacriticCategory {
    /// Artículo definido (el)
    Article,
    /// Pronombre personal (él, tú, mí)
    PersonalPronoun,
    /// Posesivo (tu, mi)
    Possessive,
    /// Pronombre reflexivo/objeto (te, se)
    ReflexivePronoun,
    /// Sustantivo (té)
    Noun,
    /// Verbo (sé, dé)
    Verb,
    /// Preposición (de)
    Preposition,
    /// Conjunción (si, mas)
    Conjunction,
    /// Adverbio afirmativo (sí)
    AffirmativeAdverb,
    /// Adverbio de cantidad (más)
    QuantityAdverb,
    /// Adverbio temporal (aún = todavía)
    TemporalAdverb,
    /// Adverbio inclusivo (aun = incluso)
    InclusiveAdverb,
}

/// Todos los pares de tildes diacríticas
pub const DIACRITIC_PAIRS: &[DiacriticPair] = &[
    DiacriticPair {
        without_accent: "el",
        with_accent: "él",
        category_without: DiacriticCategory::Article,
        category_with: DiacriticCategory::PersonalPronoun,
    },
    DiacriticPair {
        without_accent: "tu",
        with_accent: "tú",
        category_without: DiacriticCategory::Possessive,
        category_with: DiacriticCategory::PersonalPronoun,
    },
    DiacriticPair {
        without_accent: "mi",
        with_accent: "mí",
        category_without: DiacriticCategory::Possessive,
        category_with: DiacriticCategory::PersonalPronoun,
    },
    DiacriticPair {
        without_accent: "te",
        with_accent: "té",
        category_without: DiacriticCategory::ReflexivePronoun,
        category_with: DiacriticCategory::Noun,
    },
    DiacriticPair {
        without_accent: "se",
        with_accent: "sé",
        category_without: DiacriticCategory::ReflexivePronoun,
        category_with: DiacriticCategory::Verb,
    },
    DiacriticPair {
        without_accent: "de",
        with_accent: "dé",
        category_without: DiacriticCategory::Preposition,
        category_with: DiacriticCategory::Verb,
    },
    DiacriticPair {
        without_accent: "si",
        with_accent: "sí",
        category_without: DiacriticCategory::Conjunction,
        category_with: DiacriticCategory::AffirmativeAdverb,
    },
    DiacriticPair {
        without_accent: "mas",
        with_accent: "más",
        category_without: DiacriticCategory::Conjunction,
        category_with: DiacriticCategory::QuantityAdverb,
    },
    DiacriticPair {
        without_accent: "aun",
        with_accent: "aún",
        category_without: DiacriticCategory::InclusiveAdverb,
        category_with: DiacriticCategory::TemporalAdverb,
    },
];

/// Corrección sugerida para tilde diacrítica
#[derive(Debug, Clone)]
pub struct DiacriticCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

/// Analizador de tildes diacríticas
pub struct DiacriticAnalyzer;

impl DiacriticAnalyzer {
    /// Analiza los tokens y detecta errores de tildes diacríticas
    ///
    /// El `verb_recognizer` opcional permite detectar formas verbales conjugadas
    /// para evitar falsos positivos como "No se trata..." → "No sé trata..."
    ///
    /// El `proper_names` opcional permite verificar si una palabra es un nombre propio
    /// para evitar falsos positivos como "Artur Mas" → "Artur Más"
    pub fn analyze(tokens: &[Token], verb_recognizer: Option<&VerbRecognizer>, proper_names: Option<&ProperNames>) -> Vec<DiacriticCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .collect();

        for (pos, (idx, token)) in word_tokens.iter().enumerate() {
            let word_lower = token.text.to_lowercase();

            // Buscar si es una palabra con posible tilde diacrítica
            for pair in DIACRITIC_PAIRS {
                if word_lower == pair.without_accent || word_lower == pair.with_accent {
                    if let Some(correction) =
                        Self::check_diacritic(pair, tokens, &word_tokens, pos, *idx, token, verb_recognizer, proper_names)
                    {
                        corrections.push(correction);
                    }
                    break;
                }
            }
        }

        corrections
    }

    /// Verifica si hay un número entre dos índices de tokens
    fn has_number_between(tokens: &[Token], from_idx: usize, to_idx: usize) -> bool {
        for i in (from_idx + 1)..to_idx {
            if let Some(token) = tokens.get(i) {
                if token.token_type == crate::grammar::TokenType::Number {
                    return true;
                }
            }
        }
        false
    }

    fn check_diacritic(
        pair: &DiacriticPair,
        all_tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
        token_idx: usize,
        token: &Token,
        verb_recognizer: Option<&VerbRecognizer>,
        proper_names: Option<&ProperNames>,
    ) -> Option<DiacriticCorrection> {
        let word_lower = token.text.to_lowercase();
        let has_accent = word_lower == pair.with_accent;

        // Caso especial: "él" con tilde
        // Nunca sugerir quitar la tilde de "él" porque genera muchos falsos positivos.
        // Es muy difícil distinguir "él organice" (pronombre + subjuntivo) de "el orden" (artículo + sustantivo)
        // sin un análisis sintáctico completo. Si el usuario escribió "él", probablemente sabe que es pronombre.
        // Los errores típicos van en la otra dirección: olvidar la tilde en "el" cuando debería ser "él".
        if pair.without_accent == "el" && has_accent {
            return None;
        }

        // Caso especial: "sí" con tilde al inicio de oración o tras puntuación
        // "Sí, podemos..." es afirmación válida. No sugerir quitar la tilde.
        // Los errores típicos van en la otra dirección: olvidar la tilde en "si" afirmativo.
        if pair.without_accent == "si" && has_accent {
            // Si está al inicio de oración (pos == 0) o después de límite de oración, no quitar tilde
            let at_sentence_start = pos == 0 || {
                let prev_idx = word_tokens[pos - 1].0;
                has_sentence_boundary(all_tokens, prev_idx, token_idx)
            };
            if at_sentence_start {
                return None;
            }
            // "Explicó: Sí, podemos" - sí afirmativo tras dos puntos
            // El : introduce una respuesta o cita directa, así que "Sí" con tilde es correcto
            if pos > 0 {
                let prev_idx = word_tokens[pos - 1].0;
                for i in (prev_idx + 1)..token_idx {
                    if i < all_tokens.len() && all_tokens[i].text == ":" {
                        return None;
                    }
                }
            }
            // "porque sí", "pues sí", "claro que sí" - sí enfático tras conjunción causal
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(prev_lower.as_str(), "porque" | "pues" | "bueno") {
                    return None;
                }
            }
            // "veces sí y otras no" - sí contrastivo seguido de conjunción
            if pos + 1 < word_tokens.len() {
                let next_lower = word_tokens[pos + 1].1.text.to_lowercase();
                if matches!(next_lower.as_str(), "y" | "o" | "u" | "e") {
                    return None;
                }
            }
            // "sí o sí", "sí o no" - sí contrastivo precedido de conjunción
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(prev_lower.as_str(), "y" | "o" | "u" | "e") {
                    return None;
                }
            }
            // "para sí", "en sí", "de sí", "por sí", "a sí", "ante sí", "sobre sí", "entre sí", "consigo"
            // Pronombre reflexivo tras preposición - siempre lleva tilde
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(prev_lower.as_str(),
                    "para" | "en" | "de" | "por" | "a" | "ante" | "sobre" | "entre" |
                    "tras" | "contra" | "hacia" | "desde" | "sin" | "con"
                ) {
                    return None;
                }
            }
            // "Entonces sí", "ahora sí", "eso sí", "claro que sí" - sí enfático
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(prev_lower.as_str(), "entonces" | "ahora" | "eso" | "esto" | "claro" | "seguro") {
                    return None;
                }
            }
            // "un sí", "el sí" - sí como sustantivo (la afirmación)
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(prev_lower.as_str(), "un" | "el" | "este" | "ese" | "aquel" | "su" | "mi" | "tu") {
                    return None;
                }
            }
            // "sí, señor", "sí, vale" - sí afirmativo seguido de coma
            if pos + 1 < word_tokens.len() {
                let next_idx = word_tokens[pos + 1].0;
                // Buscar si hay coma entre sí y la siguiente palabra
                for i in (token_idx + 1)..next_idx {
                    if all_tokens[i].text == "," {
                        return None;
                    }
                }
            }
            // "X sí que Y" - sí enfático seguido de "que"
            // "Aquí sí que hay lógica", "Eso sí que es bueno", "Esta vez sí que lo logró"
            if pos + 1 < word_tokens.len() {
                let next_lower = word_tokens[pos + 1].1.text.to_lowercase();
                if next_lower == "que" {
                    return None;
                }
            }
            // "sí" seguido de verbo conjugado es enfático: "él sí viene", "esto sí funciona"
            if pos + 1 < word_tokens.len() {
                let next_lower = word_tokens[pos + 1].1.text.to_lowercase();
                let is_verb = if let Some(recognizer) = verb_recognizer {
                    recognizer.is_valid_verb_form(&next_lower)
                } else {
                    Self::is_likely_conjugated_verb(&next_lower)
                };
                if is_verb {
                    return None;
                }
            }
            // "sí" al final de frase (antes de punto) es afirmativo: "Hoy sí."
            // Verificar si hay punto después
            let next_word_idx = if pos + 1 < word_tokens.len() {
                word_tokens[pos + 1].0
            } else {
                all_tokens.len()
            };
            for i in (token_idx + 1)..next_word_idx {
                if i < all_tokens.len() && all_tokens[i].text == "." {
                    return None;
                }
            }
            // "sí, + verbo" es enfático: "sí, sirve", "sí, viene"
            // Buscar patrón: sí + coma + verbo
            if pos + 1 < word_tokens.len() {
                let next_idx = word_tokens[pos + 1].0;
                // Verificar si hay coma entre sí y la siguiente palabra
                let mut has_comma = false;
                for i in (token_idx + 1)..next_idx {
                    if i < all_tokens.len() && all_tokens[i].text == "," {
                        has_comma = true;
                        break;
                    }
                }
                if has_comma {
                    let next_lower = word_tokens[pos + 1].1.text.to_lowercase();
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        recognizer.is_valid_verb_form(&next_lower)
                    } else {
                        Self::is_likely_conjugated_verb(&next_lower)
                    };
                    if is_verb {
                        return None;
                    }
                }
            }
            // ", sí + verbo" es enfático: "sirve, sí sirve"
            // Patrón: coma antes de sí + verbo después
            if pos > 0 && pos + 1 < word_tokens.len() {
                let prev_idx = word_tokens[pos - 1].0;
                // Verificar si hay coma entre la palabra anterior y sí
                let mut has_comma_before = false;
                for i in (prev_idx + 1)..token_idx {
                    if i < all_tokens.len() && all_tokens[i].text == "," {
                        has_comma_before = true;
                        break;
                    }
                }
                if has_comma_before {
                    let next_lower = word_tokens[pos + 1].1.text.to_lowercase();
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        recognizer.is_valid_verb_form(&next_lower)
                    } else {
                        Self::is_likely_conjugated_verb(&next_lower)
                    };
                    if is_verb {
                        return None;
                    }
                }
            }
        }

        // Caso especial: "tú" con tilde seguido de verbo
        // "Tú lo has dicho" es correcto. No sugerir quitar la tilde cuando hay verbo después.
        if pair.without_accent == "tu" && has_accent {
            // "como tú", "igual que tú" - comparaciones donde "tú" es pronombre
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(prev_lower.as_str(), "como" | "igual" | "que" | "entre" | "excepto" | "salvo") {
                    return None;  // Mantener tilde
                }
            }
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                let next_lower = next_token.text.to_lowercase();
                // Si va seguido de pronombre clítico (lo, la, le, me, te, se, nos, os) o verbo, mantener tilde
                if matches!(next_lower.as_str(), "lo" | "la" | "le" | "les" | "los" | "las" | "me" | "te" | "se" | "nos" | "os") {
                    return None;
                }
                // Si va seguido de verbo conjugado, mantener tilde
                let is_verb = if let Some(recognizer) = verb_recognizer {
                    recognizer.is_valid_verb_form(&next_lower)
                } else {
                    Self::is_likely_conjugated_verb(&next_lower)
                };
                if is_verb {
                    return None;
                }
            }
        }

        // Caso especial: "sé" con tilde
        // Es difícil distinguir "yo sé" (verbo) de "se fue" (reflexivo) sin análisis completo.
        // Si el usuario escribió "sé", probablemente es intencional (verbo saber o imperativo de ser).
        if pair.without_accent == "se" && has_accent {
            // Ser conservador: si ya tiene tilde, no sugerir quitarla
            return None;
        }

        // Caso especial: "aún" con tilde
        // Ser conservador EXCEPTO en casos claros donde NO debe llevar tilde.
        // "aun así", "aun cuando" son casos claros de "incluso" (sin tilde).
        if pair.without_accent == "aun" && has_accent {
            // Verificar si está seguido de palabra que indica claramente "incluso"
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                let next_lower = next_token.text.to_lowercase();
                // "aun así", "aun cuando" - casos claros de "incluso" (sin tilde)
                if matches!(next_lower.as_str(), "así" | "cuando") {
                    // Permitir corregir estos casos claros
                    // (no retornar None, dejar que la lógica normal lo maneje)
                } else {
                    // Otros casos son ambiguos, ser conservador
                    return None;
                }
            } else {
                // Sin palabra siguiente, ser conservador
                return None;
            }
        }

        // Obtener contexto: palabra anterior y siguiente
        // Si hay límite de oración entre la palabra anterior y la actual, tratarla como inicio de oración
        let prev_word = if pos > 0 {
            let prev_idx = word_tokens[pos - 1].0;
            // Verificar si hay un límite de oración entre la palabra anterior y la actual
            if has_sentence_boundary(all_tokens, prev_idx, token_idx) {
                None // Tratar como inicio de oración
            } else {
                Some(word_tokens[pos - 1].1.text.to_lowercase())
            }
        } else {
            None
        };

        let next_word = if pos + 1 < word_tokens.len() {
            Some(word_tokens[pos + 1].1.text.to_lowercase())
        } else {
            None
        };

        // Palabra después de la siguiente (para contexto extendido)
        let next_next_word = if pos + 2 < word_tokens.len() {
            Some(word_tokens[pos + 2].1.text.to_lowercase())
        } else {
            None
        };

        // Palabra antes de la anterior (para contexto extendido)
        let prev_prev_word = if pos >= 2 {
            let prev_prev_idx = word_tokens[pos - 2].0;
            let prev_idx = word_tokens[pos - 1].0;
            // Verificar si hay un límite de oración
            if has_sentence_boundary(all_tokens, prev_prev_idx, prev_idx) {
                None
            } else {
                Some(word_tokens[pos - 2].1.text.to_lowercase())
            }
        } else {
            None
        };

        // Caso especial el/él: si hay un número entre "el" y la siguiente palabra,
        // "el" es siempre artículo (ej: "el 52,7% se declara" → "el" es artículo)
        if pair.without_accent == "el" && pair.with_accent == "él" && !has_accent {
            if pos + 1 < word_tokens.len() {
                let next_word_idx = word_tokens[pos + 1].0;
                if Self::has_number_between(all_tokens, token_idx, next_word_idx) {
                    return None; // "el" seguido de número = artículo, no corregir
                }
            }
        }

        // Caso especial mi/mí: verificar si la siguiente palabra es sustantivo/adjetivo del diccionario
        // "de mi carrera" → "mi" es posesivo (no necesita tilde)
        // "para mi" → "mi" es pronombre (necesita tilde → "mí")
        // "mí casa" → incorrecto, debe ser "mi casa" (se maneja en needs_accent, no aquí)
        if pair.without_accent == "mi" && pair.with_accent == "mí" && !has_accent {
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                if let Some(ref info) = next_token.word_info {
                    use crate::dictionary::WordCategory;
                    if matches!(info.category, WordCategory::Sustantivo | WordCategory::Adjetivo) {
                        return None; // "mi" seguido de sustantivo/adjetivo = posesivo, no necesita tilde
                    }
                }
            }
        }

        // Caso especial tu/tú: verificar si la siguiente palabra es sustantivo/adjetivo del diccionario
        // "tu enfado" → "tu" es posesivo (no necesita tilde)
        // "tú cantas" → "tú" es pronombre (necesita tilde)
        // PERO: algunas palabras como "mando" son tanto sustantivo como forma verbal.
        // Si VerbRecognizer dice que es verbo, no descartar como posesivo.
        if pair.without_accent == "tu" && pair.with_accent == "tú" && !has_accent {
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                let next_word_text = next_token.text.to_lowercase();

                // Primero verificar si es verbo (tiene prioridad)
                let is_verb = if let Some(recognizer) = verb_recognizer {
                    recognizer.is_valid_verb_form(&next_word_text)
                } else {
                    false
                };

                // Solo tratar como posesivo si NO es verbo
                if !is_verb {
                    if let Some(ref info) = next_token.word_info {
                        use crate::dictionary::WordCategory;
                        if matches!(info.category, WordCategory::Sustantivo | WordCategory::Adjetivo) {
                            return None; // "tu" seguido de sustantivo/adjetivo (no verbo) = posesivo
                        }
                    }
                }
            }
        }

        // Caso especial mas/más: "Mas" con mayúscula puede ser apellido (Artur Mas)
        // No corregir si:
        // 1. La palabra está capitalizada Y está en el diccionario de nombres propios
        // 2. La palabra anterior también está capitalizada (patrón "Nombre Apellido")
        if pair.without_accent == "mas" && pair.with_accent == "más" && !has_accent {
            let is_capitalized = token.text.chars().next().map_or(false, |c| c.is_uppercase());

            if is_capitalized {
                // Verificar si está en el diccionario de nombres propios
                if let Some(names) = proper_names {
                    if names.contains(&token.text) {
                        return None;
                    }
                }

                // Verificar si la palabra anterior también está capitalizada (patrón "Artur Mas")
                if pos > 0 && prev_word.is_some() {
                    let prev_token_text = &word_tokens[pos - 1].1.text;
                    let prev_is_capitalized = prev_token_text.chars().next().map_or(false, |c| c.is_uppercase());
                    if prev_is_capitalized {
                        // Patrón "Nombre Mas" → apellido, no corregir
                        return None;
                    }
                }
            }
        }

        // Determinar si necesita tilde basándose en el contexto
        let needs_accent = Self::needs_accent(pair, prev_word.as_deref(), next_word.as_deref(), next_next_word.as_deref(), prev_prev_word.as_deref(), verb_recognizer);

        if needs_accent && !has_accent {
            // Debería tener tilde pero no la tiene
            Some(DiacriticCorrection {
                token_index: token_idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, pair.with_accent),
                reason: Self::get_reason(pair, true),
            })
        } else if !needs_accent && has_accent {
            // Tiene tilde pero no debería
            Some(DiacriticCorrection {
                token_index: token_idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, pair.without_accent),
                reason: Self::get_reason(pair, false),
            })
        } else {
            None
        }
    }

    /// Determina si la palabra necesita tilde según el contexto
    fn needs_accent(pair: &DiacriticPair, prev: Option<&str>, next: Option<&str>, next_next: Option<&str>, prev_prev: Option<&str>, verb_recognizer: Option<&VerbRecognizer>) -> bool {
        match (pair.without_accent, pair.with_accent) {
            // el/él
            ("el", "él") => {
                // "él" es pronombre, "el" es artículo
                // NOTA: Esta distinción es MUY difícil sin análisis sintáctico completo.
                // Ser EXTREMADAMENTE conservador para evitar falsos positivos.
                // Solo detectar casos de altísima confianza.

                if let Some(next_word) = next {
                    // Caso claro: "él se/me/nos/os/le/les" (pronombre + clítico, no "te" ni "lo/la")
                    if matches!(next_word, "se" | "me" | "nos" | "os" | "le" | "les") {
                        return true;
                    }
                    // "el mismo" vs "él mismo":
                    // - "él mismo" (pronombre + énfasis): "él mismo lo hizo"
                    // - "el mismo [sustantivo]" (artículo + adjetivo): "el mismo cuello"
                    // Si "mismo/a" va seguido de sustantivo, es artículo (no corregir)
                    if next_word == "mismo" || next_word == "misma" {
                        // Verificar si hay sustantivo después de "mismo/a"
                        if let Some(word_after_mismo) = next_next {
                            // Si hay un sustantivo común después, "el" es artículo
                            if Self::is_common_noun_for_mismo(word_after_mismo) {
                                return false;  // "el mismo cuello" - artículo
                            }
                        }
                        // Sin sustantivo después, es pronombre: "él mismo"
                        return true;
                    }
                    // NO detectar "él + verbo" porque causa demasiados falsos positivos
                    // (ej: "el orden" se detectaba como "él orden")
                } else {
                    // Al final de oración después de preposición: "para él."
                    if let Some(prev_word) = prev {
                        if Self::is_preposition(prev_word) {
                            return true;
                        }
                    }
                }
                // En todos los demás casos, asumir artículo (no corregir)
                false
            }

            // tu/tú
            ("tu", "tú") => {
                // "tú" es pronombre (sujeto), "tu" es posesivo (va con sustantivo)
                // "entre tú y yo", "tú cantas", "tú mismo", "él y tú"
                // "opinas tú", "crees tú" = énfasis (pronombre después del verbo)
                // PERO: "por tu ayuda", "en tu casa" = posesivo (no necesita tilde)
                // PERO: "tu hermano y tu hermana" = posesivo (no necesita tilde)

                // Primero verificar si va seguido de sustantivo (entonces es posesivo)
                if let Some(next_word) = next {
                    if Self::is_likely_noun_or_adj(next_word) {
                        return false;  // Es posesivo: "tu casa", "tu hermana"
                    }
                }

                // Si está precedido por verbo de segunda persona, es pronombre enfático
                // "opinas tú", "crees tú", "piensas tú"
                if let Some(prev_word) = prev {
                    if Self::is_second_person_verb(prev_word) {
                        return true;  // Pronombre enfático: "opinas tú"
                    }
                }

                // Si está precedido por conjunción Y no va seguido de sustantivo,
                // es pronombre en sujeto compuesto (él y tú sois)
                if let Some(prev_word) = prev {
                    if prev_word == "y" || prev_word == "e" || prev_word == "o" || prev_word == "ni" {
                        return true;  // Sujeto compuesto: "él y tú sois"
                    }
                }

                // Verificar contexto de la siguiente palabra
                if let Some(next_word) = next {
                    // Si va seguido de conjunción, es pronombre (tú y yo)
                    if next_word == "y" || next_word == "e" || next_word == "o" || next_word == "ni" {
                        return true;
                    }
                    // Si va seguido de "mismo/a", es pronombre (tú mismo)
                    if next_word == "mismo" || next_word == "misma" {
                        return true;
                    }
                    // Si va seguido de verbo conjugado, es pronombre (tú cantas)
                    // Usar VerbRecognizer si está disponible (más preciso)
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        recognizer.is_valid_verb_form(next_word)
                    } else {
                        Self::is_common_verb(next_word) || Self::is_verb_form(next_word)
                    };
                    if is_verb {
                        return true;
                    }
                    // Si va seguido de posible verbo en 1ª persona, probablemente es pronombre
                    // con error de concordancia: "tu canto" → "tú cantas"
                    if Self::is_possible_first_person_verb(next_word) {
                        return true;
                    }
                    // Si va seguido de adverbio común, es pronombre sujeto (tú también, tú siempre)
                    if Self::is_common_adverb(next_word) {
                        return true;
                    }
                    // Si va seguido de interrogativo, es pronombre sujeto (¿tú qué harías?, ¿tú cuándo vienes?)
                    if Self::is_interrogative(next_word) {
                        return true;
                    }
                    // Si va seguido de sustantivo o adjetivo común, es posesivo (tu casa, tu ayuda)
                    if Self::is_likely_noun_or_adj(next_word) {
                        return false;  // Es posesivo, no necesita tilde
                    }
                    // Si no reconocemos la siguiente palabra, ser conservador (no cambiar)
                    false
                } else {
                    // Al final después de preposición: "entre tú"
                    if let Some(prev_word) = prev {
                        if Self::is_preposition(prev_word) {
                            return true;
                        }
                    }
                    false
                }
            }

            // mi/mí
            ("mi", "mí") => {
                // "mí" va después de preposición SIN sustantivo después (para mí, a mí)
                // "mi" es posesivo seguido de sustantivo (en mi casa, por mi culpa)
                if let Some(prev_word) = prev {
                    if Self::is_preposition(prev_word) {
                        // Preposición + mi + sustantivo/adjetivo = posesivo (en mi lugar, por mi parte)
                        // Preposición + mi (final) = pronombre (para mí, a mí)
                        // Preposición + mi + verbo/clítico = pronombre (a mí me gusta, para mí es)
                        if let Some(next_word) = next {
                            // Si siguiente es sustantivo/adjetivo → posesivo
                            if Self::is_likely_noun_or_adj(next_word) || Self::is_common_noun_after_mi(next_word) {
                                return false;  // Posesivo: "en mi lugar", "por mi parte"
                            }
                            // Si siguiente es pronombre clítico → pronombre tónico
                            if Self::is_clitic_pronoun(next_word) {
                                return true;  // Pronombre: "a mí me gusta", "para mí te digo"
                            }
                            // Si siguiente es verbo conjugado → pronombre tónico
                            if let Some(vr) = verb_recognizer {
                                if vr.is_valid_verb_form(next_word) {
                                    return true;  // Pronombre: "para mí es", "a mí parece"
                                }
                            }
                        }
                        return true;  // Pronombre (preposición + mi sin sustantivo)
                    }
                }
                false
            }

            // te/té
            ("te", "té") => {
                // "té" es sustantivo (la bebida)
                // "te" es pronombre (te quiero)
                if let Some(prev_word) = prev {
                    // Después de artículo, adjetivo o preposición "de/con/sin", es sustantivo
                    // "el té", "té caliente", "de té", "con té", "sin té"
                    Self::is_article(prev_word)
                        || Self::is_adjective_indicator(prev_word)
                        || matches!(prev_word, "de" | "con" | "sin")
                } else if let Some(next_word) = next {
                    // Si va seguido de adjetivo (té caliente), es sustantivo
                    Self::is_adjective_indicator(next_word)
                } else {
                    false
                }
            }

            // se/sé
            ("se", "sé") => {
                // "sé" es verbo saber (yo sé, no sé) o imperativo de ser (sé bueno)
                // "se" es pronombre reflexivo/pasivo (se fue, se implementó, no se puede)

                // Primero verificar si "se" va seguido de verbo conjugado
                // En ese caso es pronombre reflexivo/pasivo, NO el verbo "saber"
                // Ejemplos: "se implementó", "no se puede", "ya se terminó", "no se trata"
                if let Some(next_word) = next {
                    // Usar VerbRecognizer si está disponible (más preciso)
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        recognizer.is_valid_verb_form(next_word)
                    } else {
                        // Fallback a lista hardcodeada
                        Self::is_conjugated_verb_for_se(next_word)
                    };
                    if is_verb {
                        return false;  // Es "se" reflexivo/pasivo
                    }
                }

                if let Some(prev_word) = prev {
                    // "yo sé" es claramente verbo saber
                    if prev_word == "yo" {
                        return true;
                    }
                    // "no sé" o "ya sé" solo si NO va seguido de verbo conjugado
                    // (ya verificamos arriba que no hay verbo después)
                    if prev_word == "no" || prev_word == "ya" {
                        // Si no hay siguiente palabra, es "no sé" / "ya sé"
                        if next.is_none() {
                            return true;
                        }
                        // Si va seguido de "que", "cuánto", "dónde", etc., es verbo saber
                        if let Some(next_word) = next {
                            if next_word == "que" || Self::is_interrogative(next_word) {
                                return true;
                            }
                        }
                        // En otros casos con "no/ya" + se + algo, asumir reflexivo
                        return false;
                    }
                } else if let Some(next_word) = next {
                    // Al inicio: "sé que..." o "sé bueno" (imperativo de ser)
                    if next_word == "que" || Self::is_adjective_indicator(next_word) {
                        return true;
                    }
                } else {
                    // Solo "sé" al final probablemente es verbo (respuesta "sí, lo sé")
                    return true;
                }
                false
            }

            // de/dé
            ("de", "dé") => {
                // "dé" es subjuntivo de dar
                // "de" es preposición
                if let Some(prev_word) = prev {
                    // Verificar primero si "de" forma parte de una locución adverbial
                    // En ese caso SIEMPRE es preposición, no subjuntivo de "dar"
                    if let Some(next_word) = next {
                        if Self::is_adverbial_phrase_with_de(next_word) {
                            return false;  // "de verdad", "de nuevo", "de hecho", etc.
                        }
                    }
                    // "que se dé", "que me dé", "que te dé", "que le dé", "que nos dé"
                    // Patrón: "que" + pronombre reflexivo/objeto + "dé"
                    if matches!(prev_word, "se" | "me" | "te" | "le" | "les" | "nos" | "os") {
                        if let Some(prev_prev) = prev_prev {
                            if prev_prev == "que" {
                                return true; // "que se dé", "que me dé", etc.
                            }
                        }
                    }
                    // "que dé", "para que dé", "ojalá dé"
                    // PERO NO "más que de X" - aquí "de" es preposición
                    if prev_word == "que" {
                        // Verificar si "que" está precedido por "más", "menos", "antes", "después"
                        // En "más que de física", "de" es preposición comparativa
                        if let Some(prev_prev) = prev_prev {
                            if matches!(prev_prev, "más" | "menos" | "antes" | "después" | "mejor" | "peor") {
                                return false;
                            }
                        }
                        // Verificar si "de" introduce una cláusula relativa: "de lo que", "de la que"
                        // En "que de lo que no se puede hablar", "de" es preposición
                        if let Some(next_word) = next {
                            if matches!(next_word, "lo" | "la" | "los" | "las" | "el" | "un" | "una" | "unos" | "unas") {
                                return false; // "de lo/la/los/las/el..." es preposición + artículo
                            }
                        }
                        return true; // "que dé" sin comparativo anterior
                    }
                    prev_word == "ojalá" || prev_word == "quizá"
                } else {
                    false
                }
            }

            // si/sí
            ("si", "sí") => {
                // "sí" es afirmación, pronombre reflexivo (por sí mismo), o enfático (él sí vino)
                // "si" es conjunción condicional (si vienes..., como si fuera...)

                // Primero verificar la palabra siguiente
                let next_is_mismo = next.map_or(false, |n|
                    matches!(n, "mismo" | "misma" | "mismos" | "mismas"));

                if let Some(prev_word) = prev {
                    // "como si" es construcción condicional, NO sí enfático
                    // "como si participaran", "como si fuera", "como si nada"
                    if prev_word == "como" {
                        return false;
                    }
                    // "eso sí" es siempre enfático
                    if prev_word == "eso" {
                        return true;
                    }
                    // "dijo que sí" (al final) vs "que si venías" (condicional)
                    // Solo corregir "que sí" si está al final o si sigue "mismo/a"
                    if prev_word == "que" {
                        return next.is_none() || next_is_mismo;
                    }
                    // "por sí mismo", "a sí mismo", "en sí mismo" - requieren "mismo" después
                    // "por si acaso", "por si querías", "en si cabe" - NO llevan tilde (condicional)
                    if prev_word == "por" || prev_word == "en" || prev_word == "a" {
                        return next_is_mismo;
                    }
                }

                if let Some(next_word) = next {
                    // "si bien" es conjunción concesiva (= aunque), NO lleva tilde
                    if next_word == "bien" {
                        return false;
                    }
                    // "sí mismo/a" (reflexive) - ya verificado arriba con next_is_mismo
                    if next_is_mismo {
                        return true;
                    }
                    // Sí enfático seguido de verbo: "él sí vino", "la imagen sí pasa"
                    // PERO: ser muy conservador porque "si" + verbo es casi siempre condicional
                    // Solo detectar "sí" enfático cuando el prev es un pronombre claro
                    // "él sí vino", "ella sí puede", "eso sí funciona"
                    // Usar VerbRecognizer si está disponible
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        recognizer.is_valid_verb_form(next_word)
                    } else {
                        Self::is_likely_conjugated_verb(next_word)
                    };
                    if is_verb {
                        // Si está al inicio de oración (prev == None), es conjunción condicional
                        if prev.is_none() {
                            return false;  // "Si es...", "Si vienes..." - conjunción, no enfático
                        }
                        // Solo aceptar "sí" enfático después de pronombres personales/demostrativos
                        // "él sí vino", "eso sí funciona", "esto sí me gusta"
                        let prev_is_subject_pronoun = prev.map_or(false, |p|
                            matches!(p, "él" | "ella" | "ellos" | "ellas" | "eso" | "esto" |
                                       "ello" | "usted" | "ustedes" | "yo" | "tú" | "nosotros" |
                                       "nosotras" | "vosotros" | "vosotras"));
                        if prev_is_subject_pronoun {
                            return true;
                        }
                        // En otros casos, es muy probable que sea condicional
                        // "injusticias si producen", "casos si ocurren" - condicional
                        return false;
                    }
                }

                if next.is_none() {
                    // "sí" solo al final (¿Vienes? Sí.)
                    return true;
                }

                false
            }

            // mas/más
            ("mas", "más") => {
                // "más" es adverbio de cantidad (casi siempre)
                // "mas" es conjunción adversativa (arcaico, raro)
                // Por defecto, casi siempre es "más"
                if let Some(prev_word) = prev {
                    // Después de coma + "mas" es conjunción (", mas no lo hizo")
                    // Pero esto es muy raro en español moderno
                    prev_word != ","
                } else {
                    true
                }
            }

            // aun/aún
            ("aun", "aún") => {
                // "aún" = todavía (aún no llega, aún más, aún es pronto)
                // "aun" = incluso (aun así, aun cuando, y aun la administración)
                if let Some(next_word) = next {
                    // Casos claros de "aún" = todavía
                    if next_word == "no" || next_word == "más" || next_word == "menos" {
                        return true;
                    }
                    // "aún + verbo" = todavía (aún es, aún hay, aún está, aún queda)
                    // Usar VerbRecognizer si está disponible
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        recognizer.is_valid_verb_form(next_word)
                    } else {
                        // Fallback a lista hardcodeada
                        matches!(next_word, "es" | "son" | "era" | "eran" | "fue" | "fueron" |
                            "está" | "están" | "estaba" | "estaban" |
                            "hay" | "había" | "hubo" |
                            "queda" | "quedan" | "quedaba" | "quedaban" |
                            "falta" | "faltan" | "faltaba" | "faltaban" |
                            "tiene" | "tienen" | "tenía" | "tenían" |
                            "puede" | "pueden" | "podía" | "podían" |
                            "sigue" | "siguen" | "seguía" | "seguían" |
                            "existe" | "existen" | "existía" | "existían")
                    };
                    if is_verb {
                        return true;
                    }
                    // "aún + participio" = todavía (aún encabezado, aún dormido, aún vivo)
                    if next_word.ends_with("ado") || next_word.ends_with("ido") ||
                       next_word.ends_with("ada") || next_word.ends_with("ida") ||
                       next_word.ends_with("ados") || next_word.ends_with("idos") ||
                       next_word.ends_with("adas") || next_word.ends_with("idas") {
                        return true;
                    }
                    // Casos claros de "aun" = incluso (sin tilde)
                    // "aun así", "aun cuando", "aun con", "aun sin"
                    if next_word == "así" || next_word == "cuando" || next_word == "con" || next_word == "sin" {
                        return false;
                    }
                    // Si va seguido de artículo: "aun el/la/los/las..." = incluso
                    // NOTA: Verificar artículos ANTES de pronombres porque "la/lo/las/los"
                    // pueden ser ambos, y "aun la casa" es más común que "aun la vi"
                    if Self::is_article(next_word) {
                        return false;
                    }
                    // Si va seguido de pronombre reflexivo (no artículo): "aún me falta" = todavía
                    // Solo "me", "te", "se", "nos", "os", "le", "les" (excluyendo lo/la/los/las)
                    if matches!(next_word, "me" | "te" | "se" | "nos" | "os" | "le" | "les") {
                        return true;
                    }
                    // Por defecto conservador - no cambiar
                    false
                } else {
                    // Al final de oración: "lo hice aún" = "todavía lo hice"
                    true
                }
            }

            _ => false,
        }
    }

    /// Verifica si es un verbo común conjugado (restrictivo, para evitar falsos positivos)
    fn is_common_verb(word: &str) -> bool {
        // Solo verbos muy comunes en tercera persona que claramente indican "él + verbo"
        matches!(word,
            // Verbos copulativos y auxiliares
            "es" | "fue" | "era" | "será" | "sería" |
            "está" | "estaba" | "estuvo" | "estará" |
            // Verbos comunes - presente e imperfecto
            "tiene" | "tenía" | "tuvo" | "tendrá" |
            "hace" | "hacía" | "hizo" | "hará" |
            "dice" | "decía" | "dijo" | "dirá" |
            "puede" | "podía" | "pudo" | "podrá" |
            "quiere" | "quería" | "quiso" | "querrá" |
            "sabe" | "sabía" | "supo" | "sabrá" |
            "viene" | "venía" | "vino" | "vendrá" |
            "va" | "iba" | "irá" |
            // Verbos de acción comunes
            "canta" | "cantaba" | "corre" | "corría" | "come" | "comía" |
            "vive" | "vivía" | "trabaja" | "trabajaba" | "habla" | "hablaba" |
            "duerme" | "dormía" | "piensa" | "pensaba" | "siente" | "sentía" |
            "escucha" | "escuchaba" | "mira" | "miraba" | "lee" | "leía" |
            "escribe" | "escribía" | "sale" | "salía" | "entra" | "entraba" |
            "llega" | "llegaba" | "parte" | "partía"
        )
    }

    /// Verifica si una palabra parece ser forma verbal
    #[allow(dead_code)]
    fn is_verb_form(word: &str) -> bool {
        // Excluir sustantivos comunes que terminan en -ar, -er, -ir
        // pero que NO son verbos
        let non_verb_nouns = [
            // Sustantivos en -er
            "cáncer", "cancer", "líder", "lider", "taller", "alfiler", "carácter", "caracter",
            "cadáver", "cadaver", "esfínter", "esfinter", "máster", "master", "póster", "poster",
            "súper", "super", "hámster", "hamster", "bunker", "búnker", "láser", "laser",
            "cráter", "crater", "éter", "eter", "mártir",
            // Sustantivos en -ar
            "hogar", "lugar", "azúcar", "azucar", "altar", "avatar", "bar", "bazar",
            "collar", "dólar", "dolar", "ejemplar", "hangar", "militar", "pilar", "radar",
            "solar", "titular", "angular", "celular", "familiar", "nuclear", "particular",
            "popular", "regular", "secular", "similar", "singular", "vulgar",
            // Sustantivos en -ir
            "elixir", "nadir", "faquir", "tapir", "yogur",
            // Preposiciones (terminan en -e pero no son verbos)
            "sobre", "ante", "entre", "desde", "durante", "mediante",
            // Otras palabras comunes que no son verbos
            "posible", "probable", "grande", "siempre", "entonces", "mientras",
            "donde", "adonde", "aunque", "porque", "parte",
            // Sustantivos que terminan en -ido/-ado (parecen participios pero son sustantivos)
            "sentido", "sonido", "ruido", "vestido", "marido", "partido", "apellido",
            "contenido", "significado", "mercado", "estado", "lado", "grado", "pasado",
            "cuidado", "resultado", "soldado", "abogado", "delegado", "pecado",
        ];
        if non_verb_nouns.contains(&word) {
            return false;
        }

        let len = word.len();
        // Terminaciones verbales comunes y poco ambiguas
        // Infinitivos (solo si tienen longitud mínima y no tienen tilde en raíz)
        let has_accent_in_root = word.chars().take(word.len().saturating_sub(2))
            .any(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ú'));

        // Si tiene tilde en la raíz, probablemente no es infinitivo
        (!has_accent_in_root && word.ends_with("ar") && len > 3)
            || (!has_accent_in_root && word.ends_with("er") && len > 3)
            || (!has_accent_in_root && word.ends_with("ir") && len > 3)
            // Gerundios (muy específicos)
            || word.ends_with("ando")
            || word.ends_with("iendo")
            // Participios (específicos)
            || word.ends_with("ado")
            || word.ends_with("ido")
            // Plural - menos ambiguos
            || word.ends_with("amos")
            || word.ends_with("emos")
            || word.ends_with("imos")
            || word.ends_with("áis")
            || word.ends_with("éis")
            || word.ends_with("ís")
            // Tercera plural
            || (word.ends_with("an") && len > 3)  // cantan, pero no "pan"
            || (word.ends_with("en") && len > 3)  // comen, pero no "den"
            // Imperfecto indicativo
            || word.ends_with("aba")
            || word.ends_with("ía")
            // Pretérito con acento
            || word.ends_with("ó")
            || word.ends_with("ió")
            || word.ends_with("aron")
            || word.ends_with("ieron")
            // Subjuntivo imperfecto -ra/-se
            || word.ends_with("ara")
            || word.ends_with("iera")
            || word.ends_with("ase")
            || word.ends_with("iese")
            || word.ends_with("aras")
            || word.ends_with("ieras")
            || word.ends_with("ases")
            || word.ends_with("ieses")
            || word.ends_with("áramos")
            || word.ends_with("iéramos")
            || word.ends_with("ásemos")
            || word.ends_with("iésemos")
            // Condicional
            || word.ends_with("aría")
            || word.ends_with("ería")
            || word.ends_with("iría")
            // Segunda persona singular
            || (word.ends_with("as") && len > 3)  // cantas, pero no "las"
            || (word.ends_with("es") && len > 3)  // comes, pero no "les"
            // Verbos comunes específicos
            || matches!(word, "canta" | "come" | "vive" | "dice" | "hace" | "viene" |
                       "tiene" | "quiere" | "puede" | "sabe" | "va" | "está" | "es" |
                       "son" | "era" | "fue" | "dio" | "vio" | "hay")
    }

    /// Verifica si una palabra parece ser sustantivo o adjetivo (no verbo)
    /// Usado para distinguir "tu ayuda" (posesivo) de "tú cantas" (pronombre)
    fn is_likely_noun_or_adj(word: &str) -> bool {
        // Sustantivos y adjetivos comunes que pueden seguir a posesivos (tu, mi, su)
        matches!(word,
            // Sustantivos comunes
            "casa" | "coche" | "libro" | "trabajo" | "vida" | "familia" | "amigo" | "amiga" |
            "hijo" | "hija" | "padre" | "madre" | "hermano" | "hermana" | "nombre" | "edad" |
            "mano" | "cabeza" | "cuerpo" | "cara" | "ojo" | "ojos" | "pelo" | "voz" |
            "tiempo" | "día" | "noche" | "año" | "mes" | "semana" | "hora" | "momento" |
            "lugar" | "ciudad" | "país" | "mundo" | "tierra" | "agua" | "aire" | "fuego" |
            "idea" | "opinión" | "decisión" | "problema" | "situación" | "razón" | "culpa" |
            "dinero" | "comida" | "ropa" | "teléfono" | "ordenador" | "computadora" |
            "palabra" | "historia" | "cuenta" | "cuento" | "carta" | "mensaje" | "respuesta" |
            "ayuda" | "apoyo" | "error" | "éxito" | "esfuerzo" | "turno" | "parte" |
            // Adjetivos comunes
            "mejor" | "peor" | "nuevo" | "nueva" | "viejo" | "vieja" | "grande" | "pequeño" | "pequeña" |
            "bueno" | "buena" | "malo" | "mala" | "propio" | "propia" | "único" | "única" |
            "querido" | "querida" | "estimado" | "estimada" | "antiguo" | "antigua"
        )
    }

    /// Verifica si una palabra podría ser verbo REGULAR en primera persona singular (-o)
    /// Usado para detectar "tu canto" → "tú cantas" donde "canto" es verbo mal conjugado
    /// Solo detecta verbos regulares donde podemos inferir la forma correcta
    fn is_possible_first_person_verb(word: &str) -> bool {
        let lower = word.to_lowercase();
        // Debe terminar en -o y tener al menos 4 caracteres (para tener raíz mínima)
        if !lower.ends_with('o') || lower.len() < 4 {
            return false;
        }
        // Excluir verbos irregulares ya reconocidos (tienen conjugación especial)
        // No queremos que "tu soy" se interprete como "tú eres" por esta vía
        let irregular_first_person = [
            "soy", "voy", "doy", "estoy", "hago", "tengo", "vengo", "pongo",
            "salgo", "traigo", "digo", "oigo", "caigo", "conozco", "parezco",
            "nazco", "crezco", "agradezco", "ofrezco", "produzco", "conduzco",
        ];
        if irregular_first_person.contains(&lower.as_str()) {
            return false;
        }
        // Excluir palabras que son claramente adjetivos (participios o -ivo)
        if lower.ends_with("ado") || lower.ends_with("ido") || lower.ends_with("ivo") {
            return false;
        }
        // Excluir sustantivos con sufijos típicos (-ario, -orio, -erio, -uario)
        // secretario, comentario, horario, laboratorio, ministerio, acuario, etc.
        if lower.ends_with("ario") || lower.ends_with("orio") || lower.ends_with("erio") || lower.ends_with("uario") {
            return false;
        }
        // Excluir sustantivos muy comunes que terminan en -o
        let common_nouns_in_o = [
            "libro", "tiempo", "trabajo", "cuerpo", "mundo", "pueblo", "grupo",
            "medio", "centro", "punto", "caso", "modo", "tipo", "lado", "fondo",
            "hecho", "derecho", "gobierno", "desarrollo", "proceso", "servicio",
            "precio", "espacio", "campo", "proyecto", "número", "periodo", "periodo",
            "cuento", "viento", "cielo", "suelo", "pelo", "dedo", "brazo", "cuello",
            "pecho", "ojo", "labio", "hueso", "nervio", "músculo", "órgano",
            "banco", "barco", "carro", "auto", "vuelo", "juego", "fuego", "riesgo",
            "cargo", "pago", "gasto", "cambio", "inicio", "término", "acuerdo",
            "resto", "texto", "éxito", "motivo", "objetivo", "efecto", "aspecto",
            "elemento", "momento", "movimiento", "sentimiento", "pensamiento",
            "alimento", "aumento", "instrumento", "documento", "argumento",
            "tratamiento", "procedimiento", "conocimiento", "acontecimiento",
            "crecimiento", "nacimiento", "sufrimiento", "comportamiento",
            // Adjetivos/determinantes que terminan en -o
            "otro", "mismo", "todo", "poco", "mucho", "tanto", "cuanto",
            "primero", "segundo", "tercero", "cuarto", "quinto", "último",
            "cierto", "propio", "solo", "nuevo", "antiguo", "largo", "corto",
            "alto", "bajo", "ancho", "negro", "blanco", "rojo", "claro", "oscuro",
            // Otros
            "euro", "metro", "litro", "kilo", "grado", "minuto", "segundo",
        ];
        if common_nouns_in_o.contains(&lower.as_str()) {
            return false;
        }
        // Si no es sustantivo común, podría ser verbo en 1ª persona
        true
    }

    /// Verifica si es preposición
    fn is_preposition(word: &str) -> bool {
        matches!(
            word,
            "a" | "ante"
                | "bajo"
                | "con"
                | "contra"
                | "de"
                | "desde"
                | "en"
                | "entre"
                | "hacia"
                | "hasta"
                | "para"
                | "por"
                | "según"
                | "sin"
                | "sobre"
                | "tras"
        )
    }

    /// Verifica si es pronombre clítico (átono)
    fn is_clitic_pronoun(word: &str) -> bool {
        matches!(
            word,
            "me" | "te" | "se" | "nos" | "os" | "le" | "les" | "lo" | "la" | "los" | "las"
        )
    }

    /// Verifica si es artículo
    fn is_article(word: &str) -> bool {
        matches!(
            word,
            "el" | "la" | "los" | "las" | "un" | "una" | "unos" | "unas"
        )
    }

    /// Verifica si parece adjetivo (heurística simple)
    fn is_adjective_indicator(word: &str) -> bool {
        // Terminaciones comunes de adjetivos
        word.ends_with("nte")  // caliente, interesante
            || word.ends_with("oso")
            || word.ends_with("osa")
            || word.ends_with("ble")  // amable
            || word.ends_with("ico")
            || word.ends_with("ica")
            || matches!(word, "buen" | "bueno" | "buena" | "mal" | "malo" | "mala"
                | "gran" | "grande" | "nuevo" | "nueva" | "viejo" | "vieja"
                | "negro" | "negra" | "verde" | "caliente" | "frío" | "fría")
    }

    /// Verifica si es adverbio común que puede seguir a pronombre sujeto
    fn is_common_adverb(word: &str) -> bool {
        matches!(word,
            "también" | "tampoco" | "siempre" | "nunca" | "jamás" |
            "ya" | "todavía" | "aún" | "apenas" | "solo" | "sólo" |
            "bien" | "mal" | "mejor" | "peor" | "mucho" | "poco" |
            "muy" | "bastante" | "demasiado" | "casi" | "realmente" |
            "probablemente" | "seguramente" | "ciertamente" | "obviamente"
        )
    }

    /// Verifica si es palabra interrogativa/exclamativa
    fn is_interrogative(word: &str) -> bool {
        matches!(word,
            "qué" | "que" | "quién" | "quien" | "quiénes" | "quienes" |
            "cuál" | "cual" | "cuáles" | "cuales" | "cómo" | "como" |
            "cuándo" | "cuando" | "cuánto" | "cuanta" | "cuántos" | "cuántas" |
            "dónde" | "donde" | "adónde" | "adonde"
        )
    }

    /// Verifica si es verbo conjugado en segunda persona singular
    fn is_second_person_verb(word: &str) -> bool {
        // Verbos en -as (presente de -ar): cantas, opinas, piensas
        // Verbos en -es (presente de -er/-ir): comes, vives, crees
        // Verbos irregulares comunes: eres, tienes, vienes, dices, haces
        let len = word.len();
        (word.ends_with("as") && len > 3) ||  // cantas, opinas (not "las")
        (word.ends_with("es") && len > 3) ||  // comes, crees (not "les")
        matches!(word,
            "eres" | "tienes" | "vienes" | "dices" | "haces" | "puedes" |
            "quieres" | "sabes" | "vas" | "estás" | "estas" | "das"
        )
    }

    /// Verifica si es verbo conjugado que puede seguir a "se" reflexivo/pasivo
    /// Usado para distinguir "se implementó" (pasiva refleja) de "sé" (verbo saber)
    fn is_conjugated_verb_for_se(word: &str) -> bool {
        // Verbos en tercera persona (singular/plural) que son comunes en pasiva refleja
        // "se implementó", "se puede", "se hizo", "se dice", "se sabe", etc.

        // Terminaciones de tercera persona muy específicas
        let len = word.len();

        // Pretérito perfecto simple (3ª persona): -ó, -aron, -ieron, -yó, -yeron
        if word.ends_with("ó") && len >= 3 {
            return true;  // implementó, terminó, hizo, dijo
        }
        if word.ends_with("aron") || word.ends_with("ieron") || word.ends_with("yeron") {
            return true;  // implementaron, hicieron, dijeron
        }

        // Presente indicativo (3ª persona singular): verbos comunes
        if matches!(word,
            "puede" | "pueden" | "podía" | "podían" | "pudo" | "pudieron" |
            "debe" | "deben" | "debía" | "debían" |
            "hace" | "hacen" | "hacía" | "hacían" | "hizo" | "hicieron" |
            "dice" | "dicen" | "decía" | "decían" | "dijo" | "dijeron" |
            "sabe" | "saben" | "sabía" | "sabían" | "supo" | "supieron" |
            "ve" | "ven" | "veía" | "veían" | "vio" | "vieron" |
            "tiene" | "tienen" | "tenía" | "tenían" | "tuvo" | "tuvieron" |
            "quiere" | "quieren" | "quería" | "querían" | "quiso" | "quisieron" |
            "usa" | "usan" | "usaba" | "usaban" | "usó" | "usaron" |
            "trata" | "tratan" | "trataba" | "trataban" | "trató" | "trataron" |
            "encuentra" | "encuentran" | "encontraba" | "encontraban" | "encontró" | "encontraron" |
            "espera" | "esperan" | "esperaba" | "esperaban" | "esperó" | "esperaron" |
            "aplica" | "aplican" | "aplicaba" | "aplicaban" | "aplicó" | "aplicaron" |
            "considera" | "consideran" | "consideraba" | "consideraban" |
            "produce" | "producen" | "producía" | "producían" | "produjo" | "produjeron" |
            "conoce" | "conocen" | "conocía" | "conocían" |
            "mantiene" | "mantienen" | "mantenía" | "mantenían" | "mantuvo" | "mantuvieron" |
            "logra" | "logran" | "lograba" | "lograban" | "logró" | "lograron" |
            "necesita" | "necesitan" | "necesitaba" | "necesitaban" |
            "requiere" | "requieren" | "requería" | "requerían" |
            "establece" | "establecen" | "establecía" | "establecían" |
            "supone" | "suponen" | "suponía" | "suponían" | "supuso" | "supusieron" |
            "cumple" | "cumplen" | "cumplía" | "cumplían" | "cumplió" | "cumplieron" |
            "cree" | "creen" | "creía" | "creían" | "creyó" | "creyeron" |
            "llama" | "llaman" | "llamaba" | "llamaban" | "llamó" | "llamaron" |
            "nota" | "notan" | "notaba" | "notaban" | "notó" | "notaron" |
            "incluye" | "incluyen" | "incluía" | "incluían" | "incluyó" | "incluyeron" |
            "permite" | "permiten" | "permitía" | "permitían" | "permitió" | "permitieron" |
            "implementa" | "implementan" | "implementaba" | "implementaban" | "implementó" | "implementaron" |
            // Verbos adicionales comunes en pasiva refleja
            "critica" | "critican" | "criticaba" | "criticaban" | "criticó" | "criticaron" |
            "utiliza" | "utilizan" | "utilizaba" | "utilizaban" | "utilizó" | "utilizaron" |
            "realiza" | "realizan" | "realizaba" | "realizaban" | "realizó" | "realizaron" |
            "analiza" | "analizan" | "analizaba" | "analizaban" | "analizó" | "analizaron" |
            "organiza" | "organizan" | "organizaba" | "organizaban" | "organizó" | "organizaron" |
            "caracteriza" | "caracterizan" | "caracterizaba" | "caracterizaban" |
            "observa" | "observan" | "observaba" | "observaban" | "observó" | "observaron" |
            "presenta" | "presentan" | "presentaba" | "presentaban" | "presentó" | "presentaron" |
            "muestra" | "muestran" | "mostraba" | "mostraban" | "mostró" | "mostraron" |
            "indica" | "indican" | "indicaba" | "indicaban" | "indicó" | "indicaron" |
            "señala" | "señalan" | "señalaba" | "señalaban" | "señaló" | "señalaron" |
            "destaca" | "destacan" | "destacaba" | "destacaban" | "destacó" | "destacaron" |
            "calcula" | "calculan" | "calculaba" | "calculaban" | "calculó" | "calcularon" |
            "estima" | "estiman" | "estimaba" | "estimaban" | "estimó" | "estimaron" |
            "registra" | "registran" | "registraba" | "registraban" | "registró" | "registraron" |
            "publica" | "publican" | "publicaba" | "publicaban" | "publicó" | "publicaron" |
            "informa" | "informan" | "informaba" | "informaban" | "informó" | "informaron" |
            "confirma" | "confirman" | "confirmaba" | "confirmaban" | "confirmó" | "confirmaron" |
            "denuncia" | "denuncian" | "denunciaba" | "denunciaban" | "denunció" | "denunciaron" |
            "anuncia" | "anuncian" | "anunciaba" | "anunciaban" | "anunció" | "anunciaron" |
            "recomienda" | "recomiendan" | "recomendaba" | "recomendaban" | "recomendó" | "recomendaron" |
            "propone" | "proponen" | "proponía" | "proponían" | "propuso" | "propusieron" |
            "pretende" | "pretenden" | "pretendía" | "pretendían" | "pretendió" | "pretendieron" |
            "prevé" | "prevén" | "preveía" | "preveían" | "previó" | "previeron" |
            "recoge" | "recogen" | "recogía" | "recogían" | "recogió" | "recogieron" |
            "exige" | "exigen" | "exigía" | "exigían" | "exigió" | "exigieron" |
            "ofrece" | "ofrecen" | "ofrecía" | "ofrecían" | "ofreció" | "ofrecieron" |
            "añade" | "añaden" | "añadía" | "añadían" | "añadió" | "añadieron" |
            "busca" | "buscan" | "buscaba" | "buscaban" | "buscó" | "buscaron" |
            "alcanza" | "alcanzan" | "alcanzaba" | "alcanzaban" | "alcanzó" | "alcanzaron" |
            "plantea" | "plantean" | "planteaba" | "planteaban" | "planteó" | "plantearon" |
            "determina" | "determinan" | "determinaba" | "determinaban" | "determinó" | "determinaron" |
            "describe" | "describen" | "describía" | "describían" | "describió" | "describieron" |
            "define" | "definen" | "definía" | "definían" | "definió" | "definieron" |
            "detecta" | "detectan" | "detectaba" | "detectaban" | "detectó" | "detectaron" |
            "desarrolla" | "desarrollan" | "desarrollaba" | "desarrollaban" | "desarrolló" | "desarrollaron" |
            "demuestra" | "demuestran" | "demostraba" | "demostraban" | "demostró" | "demostraron" |
            "comprueba" | "comprueban" | "comprobaba" | "comprobaban" | "comprobó" | "comprobaron" |
            "garantiza" | "garantizan" | "garantizaba" | "garantizaban" | "garantizó" | "garantizaron" |
            "autoriza" | "autorizan" | "autorizaba" | "autorizaban" | "autorizó" | "autorizaron" |
            // Verbos adicionales para textos legales/BOE
            "modifica" | "modifican" | "modificaba" | "modificaban" | "modificó" | "modificaron" |
            "elimina" | "eliminan" | "eliminaba" | "eliminaban" | "eliminó" | "eliminaron" |
            "suprime" | "suprimen" | "suprimía" | "suprimían" | "suprimió" | "suprimieron" |
            "regula" | "regulan" | "regulaba" | "regulaban" | "reguló" | "regularon" |
            "aprueba" | "aprueban" | "aprobaba" | "aprobaban" | "aprobó" | "aprobaron" |
            "deroga" | "derogan" | "derogaba" | "derogaban" | "derogó" | "derogaron" |
            "incorpora" | "incorporan" | "incorporaba" | "incorporaban" | "incorporó" | "incorporaron" |
            "dispone" | "disponen" | "disponía" | "disponían" | "dispuso" | "dispusieron"
        ) {
            return true;
        }

        // Futuro (3ª persona): -á, -án
        if (word.ends_with("ará") || word.ends_with("erá") || word.ends_with("irá")) && len >= 5 {
            return true;  // implementará, podrá, hará
        }
        if (word.ends_with("arán") || word.ends_with("erán") || word.ends_with("irán")) && len >= 6 {
            return true;  // implementarán, podrán, harán
        }

        // Condicional (3ª persona): -ía, -ían (pero cuidado con imperfecto -ía)
        // No usar terminación genérica -ía porque es ambigua

        // Subjuntivo presente (3ª persona): -e, -en (para -ar), -a, -an (para -er/-ir)
        // Solo verbos específicos porque -e/-a son muy ambiguas
        if matches!(word,
            "pueda" | "puedan" | "deba" | "deban" | "haga" | "hagan" |
            "diga" | "digan" | "sepa" | "sepan" | "vea" | "vean" |
            "tenga" | "tengan" | "quiera" | "quieran"
        ) {
            return true;
        }

        false
    }

    /// Verifica si la palabra siguiente forma una locución adverbial con "de"
    /// Usado para evitar corregir "de" a "dé" en frases como "de verdad", "de nuevo"
    fn is_adverbial_phrase_with_de(next_word: &str) -> bool {
        matches!(next_word,
            // Locuciones adverbiales muy comunes
            "verdad" | "veras" | "nuevo" | "pronto" | "repente" |
            "hecho" | "forma" | "manera" | "modo" | "golpe" | "momento" |
            "inmediato" | "improviso" | "súbito" | "sobra" | "sobras" |
            "acuerdo" | "antemano" | "memoria" | "corazón" | "cabeza" |
            "frente" | "espaldas" | "lado" | "cerca" | "lejos" |
            "más" | "menos" | "vez" | "veces" | "día" | "noche" |
            "madrugada" | "mañana" | "tarde" | "paso" | "camino" |
            "vuelta" | "regreso" | "ida" | "pie" | "rodillas" |
            "puntillas" | "bruces"
        )
    }

    /// Verifica si una palabra parece ser verbo conjugado (para detectar sí enfático)
    /// Usado en "la imagen sí pasa" donde "sí" es enfático antes de verbo
    fn is_likely_conjugated_verb(word: &str) -> bool {
        // Verbos comunes en tercera persona
        if matches!(word,
            "es" | "son" | "era" | "eran" | "fue" | "fueron" |
            "está" | "están" | "estaba" | "estaban" |
            "tiene" | "tienen" | "tenía" | "tenían" |
            "hace" | "hacen" | "hacía" | "hacían" | "hizo" | "hicieron" |
            "va" | "van" | "iba" | "iban" |
            "puede" | "pueden" | "podía" | "podían" | "pudo" | "pudieron" |
            "quiere" | "quieren" | "quería" | "querían" |
            "viene" | "vienen" | "venía" | "venían" | "vino" | "vinieron" |
            "sale" | "salen" | "salía" | "salían" | "salió" | "salieron" |
            "pasa" | "pasan" | "pasaba" | "pasaban" | "pasó" | "pasaron" |
            "llega" | "llegan" | "llegaba" | "llegaban" | "llegó" | "llegaron" |
            "funciona" | "funcionan" | "funcionaba" | "funcionaban" |
            "sirve" | "sirven" | "servía" | "servían" | "sirvió" | "sirvieron" |
            "sigue" | "siguen" | "seguía" | "seguían" | "siguió" | "siguieron" |
            "parece" | "parecen" | "parecía" | "parecían" | "pareció" | "parecieron" |
            "cree" | "creen" | "creía" | "creían" | "creyó" | "creyeron" |
            "piensa" | "piensan" | "pensaba" | "pensaban" | "pensó" | "pensaron" |
            "siente" | "sienten" | "sentía" | "sentían" | "sintió" | "sintieron" |
            "queda" | "quedan" | "quedaba" | "quedaban" | "quedó" | "quedaron" |
            "falta" | "faltan" | "faltaba" | "faltaban" | "faltó" | "faltaron" |
            "importa" | "importan" | "importaba" | "importaban" |
            "necesita" | "necesitan" | "necesitaba" | "necesitaban" |
            "conviene" | "convenía" | "convino" |
            "basta" | "bastan" | "bastaba" | "bastaban" | "bastó" |
            "existe" | "existen" | "existía" | "existían" |
            "sabe" | "saben" | "sabía" | "sabían" | "supo" | "supieron" |
            "ve" | "ven" | "veía" | "veían" | "vio" | "vieron" |
            "da" | "dan" | "daba" | "daban" | "dio" | "dieron" |
            "dice" | "dicen" | "decía" | "decían" | "dijo" | "dijeron" |
            "hay" | "había" | "hubo"
        ) {
            return true;
        }
        // Terminaciones verbales comunes (3ª persona)
        let len = word.len();
        if len >= 3 {
            // Pretérito perfecto simple: -ó, -aron, -ieron
            if word.ends_with("ó") || word.ends_with("aron") || word.ends_with("ieron") {
                return true;
            }
            // Presente: -a (canta), -e (come), -an, -en
            // Pero evitar palabras muy cortas o ambiguas
            if len >= 4 && (word.ends_with("an") || word.ends_with("en")) {
                return true;
            }
        }
        false
    }

    /// Verifica si es sustantivo común que puede seguir a "mismo/a" (para "el mismo X")
    /// Usado para distinguir "él mismo" (pronombre) de "el mismo cuello" (artículo + adj + sust)
    fn is_common_noun_for_mismo(word: &str) -> bool {
        // Si termina en patrón típico de sustantivo, probablemente lo es
        let len = word.len();
        if len >= 4 {
            // Terminaciones típicas de sustantivos
            if word.ends_with("ción") || word.ends_with("sión") ||  // situación, decisión
               word.ends_with("dad") || word.ends_with("tad") ||   // ciudad, libertad
               word.ends_with("miento") ||                          // momento, movimiento
               word.ends_with("aje") ||                             // viaje, mensaje
               word.ends_with("ismo") ||                            // mecanismo
               word.ends_with("ista") ||                            // artista
               word.ends_with("ura") ||                             // estructura, cultura
               word.ends_with("eza") ||                             // naturaleza
               word.ends_with("encia") || word.ends_with("ancia")   // ciencia, instancia
            {
                return true;
            }
        }
        // Sustantivos muy comunes que pueden seguir a "el/la mismo/a"
        matches!(word,
            // Partes y ubicaciones
            "lugar" | "sitio" | "punto" | "lado" | "centro" | "fondo" | "borde" |
            "cuello" | "pie" | "techo" | "suelo" | "piso" | "nivel" | "grado" |
            // Tiempo
            "día" | "año" | "mes" | "semana" | "momento" | "instante" | "tiempo" |
            "hora" | "minuto" | "segundo" | "fecha" | "época" | "periodo" | "período" |
            // Objetos y conceptos
            "problema" | "tema" | "asunto" | "caso" | "hecho" | "modo" | "tipo" |
            "sistema" | "proceso" | "método" | "camino" | "rumbo" | "sentido" |
            "nombre" | "número" | "valor" | "precio" | "color" | "tamaño" | "peso" |
            "orden" | "grupo" | "equipo" | "partido" | "gobierno" | "estado" |
            // Personas y roles
            "hombre" | "mujer" | "persona" | "gente" | "individuo" | "autor" |
            "actor" | "presidente" | "director" | "jefe" | "líder" | "dueño" |
            // Abstractos
            "error" | "éxito" | "resultado" | "efecto" | "objetivo" | "motivo" |
            "razón" | "causa" | "fin" | "propósito" | "derecho" | "deber" |
            // Cosas físicas
            "coche" | "carro" | "auto" | "barco" | "avión" | "tren" | "edificio" |
            "casa" | "cuarto" | "habitación" | "oficina" | "empresa" | "negocio" |
            // Otros
            "botella" | "vaso" | "plato" | "libro" | "papel" | "documento" |
            "trabajo" | "proyecto" | "plan" | "idea" | "concepto" | "principio" |
            "informe" | "texto" | "artículo" | "estudio" | "análisis" | "dato" |
            // Tecnología
            "dispositivo" | "aparato" | "teléfono" | "móvil" | "ordenador" | "servidor"
        )
    }

    /// Verifica si es sustantivo común que puede seguir a "mi" posesivo
    fn is_common_noun_after_mi(word: &str) -> bool {
        matches!(word,
            // Sustantivos muy comunes con "mi"
            "lugar" | "parte" | "lado" | "caso" | "vez" | "manera" | "modo" |
            "opinión" | "parecer" | "gusto" | "cuenta" | "cargo" | "favor" |
            "juicio" | "entender" | "ver" | "pesar" | "alrededor" | "punto" |
            "perspectiva" | "posición" | "existencia" | "análisis" | "comprensión" |
            "proceso" | "desarrollo" | "funcionamiento" | "trabajo" | "objetivo" |
            "naturaleza" | "esencia" | "ser" | "mente" | "interior" |
            // Relaciones familiares
            "esposo" | "esposa" | "novio" | "novia" | "marido" | "mujer" |
            "suegro" | "suegra" | "cuñado" | "cuñada" | "yerno" | "nuera" |
            "tío" | "tía" | "primo" | "prima" | "sobrino" | "sobrina" |
            "abuelo" | "abuela" | "nieto" | "nieta" | "padrino" | "madrina" |
            // Sustantivos abstractos comunes
            "vida" | "muerte" | "amor" | "odio" | "salud" | "suerte" |
            "felicidad" | "tristeza" | "alegría" | "pena" | "dolor" |
            "capacidad" | "habilidad" | "posibilidad" | "responsabilidad" |
            // Partes del cuerpo y posesiones
            "corazón" | "alma" | "espíritu" | "conciencia"
        )
    }

    /// Preserva mayúsculas del original
    fn preserve_case(original: &str, replacement: &str) -> String {
        if original.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            let mut chars = replacement.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => replacement.to_string(),
            }
        } else {
            replacement.to_string()
        }
    }

    /// Genera mensaje explicativo
    fn get_reason(pair: &DiacriticPair, needs_accent: bool) -> String {
        if needs_accent {
            match pair.with_accent {
                "él" => "Pronombre personal (sujeto)".to_string(),
                "tú" => "Pronombre personal (sujeto)".to_string(),
                "mí" => "Pronombre personal (tras preposición)".to_string(),
                "té" => "Sustantivo (bebida)".to_string(),
                "sé" => "Verbo saber/ser".to_string(),
                "dé" => "Verbo dar (subjuntivo)".to_string(),
                "sí" => "Adverbio afirmativo o pronombre".to_string(),
                "más" => "Adverbio de cantidad".to_string(),
                "aún" => "Adverbio (todavía)".to_string(),
                _ => "Requiere tilde diacrítica".to_string(),
            }
        } else {
            match pair.without_accent {
                "el" => "Artículo definido".to_string(),
                "tu" => "Posesivo".to_string(),
                "mi" => "Posesivo".to_string(),
                "te" => "Pronombre reflexivo/objeto".to_string(),
                "se" => "Pronombre reflexivo".to_string(),
                "de" => "Preposición".to_string(),
                "si" => "Conjunción condicional".to_string(),
                "mas" => "Conjunción adversativa".to_string(),
                "aun" => "Adverbio (incluso)".to_string(),
                _ => "No requiere tilde".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<DiacriticCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        // Tests use None for verb_recognizer and proper_names (falls back to hardcoded lists)
        DiacriticAnalyzer::analyze(&tokens, None, None)
    }

    #[test]
    fn test_el_article_no_correction() {
        // "el" como artículo está correcto
        let corrections = analyze_text("el perro corre");
        assert!(
            corrections.is_empty(),
            "No debería corregir 'el' como artículo"
        );
    }

    #[test]
    fn test_el_pronoun_needs_accent() {
        // "el" al final de oración debería ser "él"
        let corrections = analyze_text("para el");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "él");
    }

    #[test]
    fn test_tu_possessive_no_correction() {
        // "tu" como posesivo está correcto
        let corrections = analyze_text("tu casa es bonita");
        assert!(
            corrections.is_empty(),
            "No debería corregir 'tu' como posesivo"
        );
    }

    #[test]
    fn test_tu_pronoun_needs_accent() {
        // "tu" seguido de verbo debería ser "tú"
        let corrections = analyze_text("tu cantas bien");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_mi_after_preposition_needs_accent() {
        // "mi" después de preposición debería ser "mí"
        let corrections = analyze_text("para mi");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "mí");
    }

    #[test]
    fn test_mi_possessive_no_correction() {
        // "mi" como posesivo está correcto
        let corrections = analyze_text("mi casa");
        assert!(
            corrections.is_empty(),
            "No debería corregir 'mi' como posesivo"
        );
    }

    #[test]
    fn test_mi_before_clitic_needs_accent() {
        // "a mi me gusta" → "a mí me gusta" (mi seguido de clítico = pronombre tónico)
        let corrections = analyze_text("a mi me gusta");
        let mi_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "mi")
            .collect();
        assert_eq!(mi_corrections.len(), 1, "Debería corregir 'mi' antes de clítico");
        assert_eq!(mi_corrections[0].suggestion, "mí");
    }

    #[test]
    fn test_mi_before_verb_needs_accent() {
        // "para mi es importante" → "para mí es importante" (mi seguido de verbo = pronombre tónico)
        let corrections = analyze_text("para mi es importante");
        let mi_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "mi")
            .collect();
        assert_eq!(mi_corrections.len(), 1, "Debería corregir 'mi' antes de verbo");
        assert_eq!(mi_corrections[0].suggestion, "mí");
    }

    #[test]
    fn test_mi_with_accent_before_noun_corrected() {
        // "mí casa" → "mi casa" (mí con tilde seguido de sustantivo = incorrecto)
        let corrections = analyze_text("mí casa");
        let mi_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "mí")
            .collect();
        assert_eq!(mi_corrections.len(), 1, "Debería corregir 'mí' antes de sustantivo");
        assert_eq!(mi_corrections[0].suggestion, "mi");
    }

    #[test]
    fn test_te_noun_needs_accent() {
        // "te" después de artículo debería ser "té"
        let corrections = analyze_text("el te está caliente");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "té");
    }

    #[test]
    fn test_se_verb_needs_accent() {
        // "se" después de "yo" o "no" debería ser "sé"
        let corrections = analyze_text("yo se la respuesta");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_si_affirmative_needs_accent() {
        // "si" después de "que" (respuesta) debería ser "sí"
        let corrections = analyze_text("dijo que si");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sí");
    }

    #[test]
    fn test_mas_quantity_needs_accent() {
        // "mas" generalmente debería ser "más"
        let corrections = analyze_text("quiero mas");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "más");
    }

    #[test]
    fn test_mas_surname_not_corrected() {
        // "Artur Mas" - Mas es apellido, no debe corregirse a "Más"
        // El patrón "Nombre Apellido" (ambos capitalizados) indica nombre propio
        let corrections = analyze_text("Artur Mas es político");
        let mas_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "Mas")
            .collect();
        assert!(mas_corrections.is_empty(),
            "No debe corregir 'Mas' a 'Más' cuando es apellido (patrón Nombre Apellido)");
    }

    #[test]
    fn test_aun_inclusive_no_accent() {
        // "aún así" debería ser "aun así" (sin tilde = incluso)
        let corrections = analyze_text("aún así lo hizo");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "aun");
    }

    #[test]
    fn test_aun_temporal_needs_accent() {
        // "aun no" debería ser "aún no" (con tilde = todavía)
        let corrections = analyze_text("aun no llega");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "aún");
    }

    #[test]
    fn test_preserve_uppercase() {
        let corrections = analyze_text("Tu cantas");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_se_reflexive_before_verb_no_correction() {
        // "se implementó" es pasiva refleja, no verbo "saber"
        let corrections = analyze_text("no se implementó la reducción");
        let se_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty(), "No debe corregir 'se' en pasiva refleja");
    }

    #[test]
    fn test_se_reflexive_with_puede() {
        // "no se puede" es pasiva refleja
        let corrections = analyze_text("no se puede hacer");
        let se_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty(), "No debe corregir 'se' en 'no se puede'");
    }

    #[test]
    fn test_no_se_verb_saber() {
        // "no sé" sin nada después es verbo saber
        let corrections = analyze_text("no se");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_que_verb_saber() {
        // "no sé que hacer" es verbo saber
        let corrections = analyze_text("no se que hacer");
        let se_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    // ==========================================================================
    // Tests para "Sí" tras dos puntos
    // ==========================================================================

    #[test]
    fn test_si_after_colon_with_accent_no_correction() {
        // "Explicó: Sí, podemos" - Sí afirmativo tras : no debe corregirse
        let corrections = analyze_text("Explicó: Sí, podemos");
        let si_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "sí")
            .collect();
        assert!(si_corrections.is_empty(), "No debe sugerir quitar tilde a 'Sí' tras ':': {:?}", si_corrections);
    }

    #[test]
    fn test_si_after_colon_without_accent_no_forced_correction() {
        // "Explicó: si quieres, ven" - si condicional tras : no debe forzarse a sí
        let corrections = analyze_text("Explicó: si quieres, ven");
        let si_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert!(si_corrections.is_empty(), "No debe forzar tilde en 'si' condicional tras ':': {:?}", si_corrections);
    }

    // ==========================================================================
    // Tests con VerbRecognizer (integración)
    // ==========================================================================

    #[test]
    fn test_se_trata_with_verb_recognizer() {
        // "No se trata..." - "trata" es forma verbal reconocida por VerbRecognizer
        // No debe corregir "se" a "sé"
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("No se trata de eso");

        // Cargar VerbRecognizer con diccionario real
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let se_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty(),
            "No debe corregir 'se' en 'No se trata' con VerbRecognizer: {:?}", se_corrections);
    }

    #[test]
    fn test_se_dice_with_verb_recognizer() {
        // "No se dice así" - "dice" es forma verbal
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("No se dice así");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let se_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty(),
            "No debe corregir 'se' en 'No se dice' con VerbRecognizer: {:?}", se_corrections);
    }

    #[test]
    fn test_tu_cantas_with_verb_recognizer() {
        // "tu cantas" - "cantas" es verbo, debe sugerir "tú cantas"
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("tu cantas muy bien");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(tu_corrections.len(), 1,
            "Debe corregir 'tu' a 'tú' cuando va seguido de verbo: {:?}", tu_corrections);
        assert_eq!(tu_corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_tu_mando_with_verb_recognizer() {
        // "tu mando" - "mando" es verbo 1ª persona (no gerundio), debe sugerir "tú mando"
        // Esto verifica que "mando" (termina en -ando pero NO es gerundio) se reconoce como verbo
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("tu mando");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(tu_corrections.len(), 1,
            "Debe corregir 'tu' a 'tú' cuando va seguido de verbo (mando = 1ª persona de mandar): {:?}", tu_corrections);
        assert_eq!(tu_corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_tu_trabajas_with_verb_recognizer() {
        // "tu trabajas" - "trabajas" es verbo reconocido dinámicamente
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("tu trabajas mucho");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(tu_corrections.len(), 1,
            "Debe corregir 'tu' a 'tú' cuando va seguido de verbo: {:?}", tu_corrections);
        assert_eq!(tu_corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_aun_continua_with_verb_recognizer() {
        // "aun continúa" - "continúa" es verbo, debe ser "aún continúa" (todavía)
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("aun continua el problema");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let aun_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert_eq!(aun_corrections.len(), 1,
            "Debe corregir 'aun' a 'aún' cuando va seguido de verbo (todavía): {:?}", aun_corrections);
        assert_eq!(aun_corrections[0].suggestion, "aún");
    }

    #[test]
    fn test_si_enfatico_with_verb_recognizer() {
        // "él sí trabaja" - "sí" enfático antes de verbo
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("él si trabaja mucho");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let si_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert_eq!(si_corrections.len(), 1,
            "Debe corregir 'si' a 'sí' (enfático) cuando pronombre + si + verbo: {:?}", si_corrections);
        assert_eq!(si_corrections[0].suggestion, "sí");
    }

    #[test]
    fn test_tu_with_accent_before_verb_protected() {
        // "tú trabajas" - ya tiene tilde, no debe sugerir quitarla
        use crate::dictionary::{DictionaryLoader, Trie};
        use super::VerbRecognizer;

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("tú trabajas mucho");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original.to_lowercase() == "tú")
            .collect();
        assert!(tu_corrections.is_empty(),
            "No debe sugerir quitar tilde de 'tú' cuando va seguido de verbo: {:?}", tu_corrections);
    }
}
