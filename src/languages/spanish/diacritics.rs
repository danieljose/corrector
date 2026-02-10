//! Corrección de tildes diacríticas
//!
//! Detecta y corrige pares de palabras que se distinguen por la tilde:
//! - el/él, tu/tú, mi/mí, te/té, se/sé, de/dé, si/sí, mas/más, aun/aún

#[cfg(test)]
use super::conjugation::VerbRecognizer;
use crate::dictionary::ProperNames;
use crate::grammar::{has_sentence_boundary, Token, TokenType};
use crate::languages::VerbFormRecognizer;

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
    /// para evitar falsos positivos como "No se trata..." �?' "No sé trata..."
    ///
    /// El `proper_names` opcional permite verificar si una palabra es un nombre propio
    /// para evitar falsos positivos como "Artur Mas" �?' "Artur Más"
    pub fn analyze(
        tokens: &[Token],
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
        proper_names: Option<&ProperNames>,
    ) -> Vec<DiacriticCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .collect();

        for (pos, (idx, token)) in word_tokens.iter().enumerate() {
            let word_lower = token.text.to_lowercase();

            if let Some(correction) =
                Self::check_a_ver_interrogative(tokens, &word_tokens, pos, *idx, token)
            {
                corrections.push(correction);
            }

            if let Some(correction) =
                Self::check_saber_imperfect_without_accent(tokens, &word_tokens, pos, *idx, token)
            {
                corrections.push(correction);
            }

            // Buscar si es una palabra con posible tilde diacrítica
            for pair in DIACRITIC_PAIRS {
                if word_lower == pair.without_accent || word_lower == pair.with_accent {
                    if let Some(correction) = Self::check_diacritic(
                        pair,
                        tokens,
                        &word_tokens,
                        pos,
                        *idx,
                        token,
                        verb_recognizer,
                        proper_names,
                    ) {
                        corrections.push(correction);
                    }
                    break;
                }
            }
        }

        corrections
    }

    /// Detecta interrogativos sin tilde en la locución "a ver":
    /// "a ver que/cuando/donde..." -> "a ver qué/cuándo/dónde..."
    /// También cubre "haber ..." en contexto de inicio discursivo, porque
    /// el homófono se corrige después a "a ver".
    fn check_a_ver_interrogative(
        all_tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
        token_idx: usize,
        token: &Token,
    ) -> Option<DiacriticCorrection> {
        let word_lower = token.text.to_lowercase();
        let suggestion_base = Self::a_ver_interrogative_with_accent(&word_lower)?;
        if suggestion_base == word_lower {
            return None;
        }

        let prev = if pos > 0 {
            let prev_idx = word_tokens[pos - 1].0;
            if has_sentence_boundary(all_tokens, prev_idx, token_idx) {
                None
            } else {
                Some(word_tokens[pos - 1].1.text.to_lowercase())
            }
        } else {
            None
        };

        let prev_prev = if pos > 1 {
            let prev_prev_idx = word_tokens[pos - 2].0;
            if has_sentence_boundary(all_tokens, prev_prev_idx, token_idx) {
                None
            } else {
                Some(word_tokens[pos - 2].1.text.to_lowercase())
            }
        } else {
            None
        };

        let in_a_ver_context = matches!(
            (prev.as_deref(), prev_prev.as_deref()),
            (Some("ver"), Some("a"))
        ) || matches!(prev.as_deref(), Some("haber" | "aver" | "aber"))
            && Self::is_a_ver_intro_context(prev_prev.as_deref());

        if !in_a_ver_context {
            return None;
        }

        Some(DiacriticCorrection {
            token_index: token_idx,
            original: token.text.clone(),
            suggestion: Self::preserve_case(&token.text, suggestion_base),
            reason: "Interrogativo indirecto en locución 'a ver'".to_string(),
        })
    }

    fn a_ver_interrogative_with_accent(word: &str) -> Option<&'static str> {
        match Self::normalize_spanish(word).as_str() {
            "que" => Some("qué"),
            "como" => Some("cómo"),
            "cuando" => Some("cuándo"),
            "donde" => Some("dónde"),
            "adonde" => Some("adónde"),
            "quien" => Some("quién"),
            "quienes" => Some("quiénes"),
            "cual" => Some("cuál"),
            "cuales" => Some("cuáles"),
            "cuanto" => Some("cuánto"),
            "cuanta" => Some("cuánta"),
            "cuantos" => Some("cuántos"),
            "cuantas" => Some("cuántas"),
            _ => None,
        }
    }

    fn is_a_ver_intro_context(prev: Option<&str>) -> bool {
        match prev {
            None => true,
            Some(word) => matches!(
                Self::normalize_spanish(word).as_str(),
                "y" | "e" | "pues" | "bueno" | "entonces" | "vamos"
            ),
        }
    }

    fn saber_imperfect_with_accent(word: &str) -> Option<&'static str> {
        match Self::normalize_spanish(word).as_str() {
            "sabia" => Some("sabía"),
            "sabias" => Some("sabías"),
            "sabiamos" => Some("sabíamos"),
            "sabian" => Some("sabían"),
            _ => None,
        }
    }

    fn has_written_accent(word: &str) -> bool {
        word.chars().any(|c| {
            matches!(
                c,
                'á' | 'é'
                    | 'í'
                    | 'ó'
                    | 'ú'
                    | 'Á'
                    | 'É'
                    | 'Í'
                    | 'Ó'
                    | 'Ú'
            )
        })
    }

    fn is_subject_pronoun_or_form(word: &str) -> bool {
        matches!(
            Self::normalize_spanish(word).as_str(),
            "yo"
                | "tu"
                | "el"
                | "ella"
                | "nosotros"
                | "nosotras"
                | "vosotros"
                | "vosotras"
                | "ellos"
                | "ellas"
                | "usted"
                | "ustedes"
        )
    }

    fn check_saber_imperfect_without_accent(
        all_tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
        token_idx: usize,
        token: &Token,
    ) -> Option<DiacriticCorrection> {
        let suggestion = Self::saber_imperfect_with_accent(&token.text)?;
        if Self::has_written_accent(&token.text) {
            return None;
        }

        let prev = if pos > 0 {
            let prev_idx = word_tokens[pos - 1].0;
            if has_sentence_boundary(all_tokens, prev_idx, token_idx) {
                None
            } else {
                Some(word_tokens[pos - 1].1.text.to_lowercase())
            }
        } else {
            None
        };

        let prev_prev = if pos >= 2 {
            let prev_prev_idx = word_tokens[pos - 2].0;
            let prev_idx = word_tokens[pos - 1].0;
            if has_sentence_boundary(all_tokens, prev_prev_idx, prev_idx) {
                None
            } else {
                Some(word_tokens[pos - 2].1.text.to_lowercase())
            }
        } else {
            None
        };

        let next = if pos + 1 < word_tokens.len() {
            let next_idx = word_tokens[pos + 1].0;
            if has_sentence_boundary(all_tokens, token_idx, next_idx) {
                None
            } else {
                Some(word_tokens[pos + 1].1.text.to_lowercase())
            }
        } else {
            None
        };

        let next_next = if pos + 2 < word_tokens.len() {
            let next_idx = word_tokens[pos + 1].0;
            let next_next_idx = word_tokens[pos + 2].0;
            if has_sentence_boundary(all_tokens, next_idx, next_next_idx) {
                None
            } else {
                Some(word_tokens[pos + 2].1.text.to_lowercase())
            }
        } else {
            None
        };

        let prev_norm = prev.as_deref().map(Self::normalize_spanish);
        let prev_prev_norm = prev_prev.as_deref().map(Self::normalize_spanish);
        let next_norm = next.as_deref().map(Self::normalize_spanish);
        let next_next_norm = next_next.as_deref().map(Self::normalize_spanish);

        let has_subject_before = prev_norm
            .as_deref()
            .is_some_and(Self::is_subject_pronoun_or_form)
            || (prev_norm.as_deref() == Some("no")
                && prev_prev_norm
                    .as_deref()
                    .is_some_and(Self::is_subject_pronoun_or_form));

        let has_saber_complement = next_norm.as_deref().is_some_and(|w| {
            Self::is_no_ya_interrogative_saber(w, next_next_norm.as_deref(), None)
                || Self::is_saber_nonverbal_complement(w)
                || Self::is_negative_indefinite_following_saber(w)
        });

        let is_negated_interrogative_saber = prev_norm.as_deref() == Some("no")
            && next_norm.as_deref().is_some_and(|w| {
                Self::is_no_ya_interrogative_saber(w, next_next_norm.as_deref(), None)
            });

        if has_subject_before && has_saber_complement || is_negated_interrogative_saber {
            return Some(DiacriticCorrection {
                token_index: token_idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, suggestion),
                reason: "Forma verbal de 'saber' en imperfecto".to_string(),
            });
        }

        None
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

    /// Devuelve true si una palabra está en MAY�sSCULAS (solo letras).
    fn is_all_caps_word(word: &str) -> bool {
        let mut has_alpha = false;
        for ch in word.chars() {
            if ch.is_alphabetic() {
                has_alpha = true;
                if ch.is_lowercase() {
                    return false;
                }
            }
        }
        has_alpha
    }

    /// Devuelve true si la oración completa está en mayúsculas.
    fn is_all_caps_sentence(
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
        pos: usize,
    ) -> bool {
        if word_tokens.is_empty() {
            return false;
        }

        // Buscar inicio de oración
        let mut start = pos;
        while start > 0 {
            let prev_idx = word_tokens[start - 1].0;
            let curr_idx = word_tokens[start].0;
            if has_sentence_boundary(all_tokens, prev_idx, curr_idx) {
                break;
            }
            start -= 1;
        }

        // Buscar fin de oración
        let mut end = pos;
        while end + 1 < word_tokens.len() {
            let curr_idx = word_tokens[end].0;
            let next_idx = word_tokens[end + 1].0;
            if has_sentence_boundary(all_tokens, curr_idx, next_idx) {
                break;
            }
            end += 1;
        }

        // Si alguna palabra tiene minúsculas, no es ALL-CAPS
        for i in start..=end {
            if word_tokens[i].1.text.chars().any(|c| c.is_lowercase()) {
                return false;
            }
        }

        true
    }

    fn check_diacritic(
        pair: &DiacriticPair,
        all_tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
        token_idx: usize,
        token: &Token,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
        proper_names: Option<&ProperNames>,
    ) -> Option<DiacriticCorrection> {
        let word_lower = token.text.to_lowercase();
        let has_accent = word_lower == pair.with_accent;

        // Evitar corregir pronombres ALL-CAPS en texto mixto (probables siglas: TU, EL, MI, etc.)
        let is_pronoun_pair = matches!(
            pair.without_accent,
            "el" | "tu" | "mi" | "te" | "se" | "de" | "si"
        );
        if is_pronoun_pair
            && Self::is_all_caps_word(&token.text)
            && !Self::is_all_caps_sentence(word_tokens, all_tokens, pos)
        {
            return None;
        }

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
                if matches!(
                    prev_lower.as_str(),
                    "para"
                        | "en"
                        | "de"
                        | "por"
                        | "a"
                        | "ante"
                        | "sobre"
                        | "entre"
                        | "tras"
                        | "contra"
                        | "hacia"
                        | "desde"
                        | "sin"
                        | "con"
                ) {
                    return None;
                }
            }
            // "Entonces sí", "ahora sí", "eso sí", "claro que sí" - sí enfático
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(
                    prev_lower.as_str(),
                    "entonces" | "ahora" | "eso" | "esto" | "claro" | "seguro"
                ) {
                    return None;
                }
            }
            // "un sí", "el sí" - sí como sustantivo (la afirmación)
            if pos > 0 {
                let prev_lower = word_tokens[pos - 1].1.text.to_lowercase();
                if matches!(
                    prev_lower.as_str(),
                    "un" | "el" | "este" | "ese" | "aquel" | "su" | "mi" | "tu"
                ) {
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
                    Self::recognizer_is_valid_verb_form(&next_lower, recognizer)
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
                        Self::recognizer_is_valid_verb_form(&next_lower, recognizer)
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
                        Self::recognizer_is_valid_verb_form(&next_lower, recognizer)
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
                if matches!(
                    prev_lower.as_str(),
                    "como" | "igual" | "que" | "entre" | "excepto" | "salvo"
                ) {
                    return None; // Mantener tilde
                }
            }
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                let next_lower = next_token.text.to_lowercase();
                // Si va seguido de pronombre clítico (lo, la, le, me, te, se, nos, os) o verbo, mantener tilde
                if matches!(
                    next_lower.as_str(),
                    "lo" | "la" | "le" | "les" | "los" | "las" | "me" | "te" | "se" | "nos" | "os"
                ) {
                    return None;
                }
                // Si va seguido de verbo conjugado, mantener tilde
                let is_verb = if let Some(recognizer) = verb_recognizer {
                    Self::recognizer_is_valid_verb_form(&next_lower, recognizer)
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

        // Tercera palabra hacia adelante (p. ej. "tu + adverbio + no + verbo")
        let next_third_word = if pos + 3 < word_tokens.len() {
            Some(word_tokens[pos + 3].1.text.to_lowercase())
        } else {
            None
        };

        // Cuarta palabra hacia adelante (para patrones como "se + mucho + en + la + piscina")
        let next_fourth_word = if pos + 4 < word_tokens.len() {
            Some(word_tokens[pos + 4].1.text.to_lowercase())
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
        // "el" es siempre artículo (ej: "el 52,7% se declara" �?' "el" es artículo)
        if pair.without_accent == "el" && pair.with_accent == "él" && !has_accent {
            if pos + 1 < word_tokens.len() {
                let next_word_idx = word_tokens[pos + 1].0;
                if Self::has_number_between(all_tokens, token_idx, next_word_idx) {
                    return None; // "el" seguido de número = artículo, no corregir
                }
            }
        }

        // Caso especial el/él: si la siguiente palabra es sustantivo/adjetivo del diccionario,
        // "el" es artículo (no necesita tilde).
        // "para el partido" -> "el" es artículo (no corregir)
        // "según el informe" -> "el" es artículo (no corregir)
        // "para el es difícil" -> "el" es pronombre (corregir -> "él")
        if pair.without_accent == "el" && pair.with_accent == "él" && !has_accent {
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                let next_lower = next_token.text.to_lowercase();
                // "mismo/misma" tiene lógica especial en needs_accent (verifica si hay
                // sustantivo después), así que no bloquear aquí.
                if next_lower != "mismo" && next_lower != "misma" {
                    if let Some(ref info) = next_token.word_info {
                        use crate::dictionary::WordCategory;
                        if matches!(
                            info.category,
                            WordCategory::Sustantivo
                                | WordCategory::Adjetivo
                                | WordCategory::Determinante
                        ) {
                            return None; // "el" seguido de sustantivo/adjetivo/determinante = artículo
                        }
                    }
                    // Fallback léxico/morfológico cuando el diccionario no clasifica bien
                    // ciertos núcleos nominales (p. ej., "resultado").
                    if Self::is_nominal_after_mismo(next_lower.as_str(), verb_recognizer) {
                        return None;
                    }
                }
            }
        }

        // Caso especial mi/mí: verificar si la siguiente palabra es sustantivo/adjetivo del diccionario
        // "de mi carrera" �?' "mi" es posesivo (no necesita tilde)
        // "para mi" �?' "mi" es pronombre (necesita tilde �?' "mí")
        // "mí casa" �?' incorrecto, debe ser "mi casa" (se maneja en needs_accent, no aquí)
        if pair.without_accent == "mi" && pair.with_accent == "mí" && !has_accent {
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                if let Some(ref info) = next_token.word_info {
                    use crate::dictionary::WordCategory;
                    if matches!(
                        info.category,
                        WordCategory::Sustantivo | WordCategory::Adjetivo
                    ) {
                        return None; // "mi" seguido de sustantivo/adjetivo = posesivo, no necesita tilde
                    }
                }
            }
        }

        // Caso especial tu/tú: verificar si la siguiente palabra es sustantivo/adjetivo del diccionario
        // "tu enfado" �?' "tu" es posesivo (no necesita tilde)
        // "tú cantas" �?' "tú" es pronombre (necesita tilde)
        // PERO: algunas palabras como "mando" son tanto sustantivo como forma verbal.
        // Si VerbRecognizer dice que es verbo, no descartar como posesivo.
        if pair.without_accent == "tu" && pair.with_accent == "tú" && !has_accent {
            if pos + 1 < word_tokens.len() {
                let next_token = word_tokens[pos + 1].1;
                let next_word_text = next_token.text.to_lowercase();

                // Primero verificar si es verbo (tiene prioridad)
                let is_verb = if let Some(recognizer) = verb_recognizer {
                    Self::recognizer_is_valid_verb_form(&next_word_text, recognizer)
                } else {
                    false
                };

                // Patrón posesivo frecuente: preposición + "tu" + forma ambigua + adverbio + verbo.
                // Ej: "Sobre tu pregunta ya respondo", "Sobre tu pregunta mañana respondo".
                // Aquí "tu" es posesivo, no pronombre tónico.
                if is_verb {
                    if let Some(prev_lower) = prev_word.as_deref() {
                        if Self::is_preposition(prev_lower) && pos + 3 < word_tokens.len() {
                            let bridge_token = word_tokens[pos + 2].1;
                            let bridge_lower = bridge_token.text.to_lowercase();
                            let bridge_is_adverb = if let Some(ref info) = bridge_token.word_info {
                                use crate::dictionary::WordCategory;
                                info.category == WordCategory::Adverbio
                            } else {
                                Self::is_likely_adverb(&bridge_lower)
                                    || Self::is_common_adverb(&bridge_lower)
                            };

                            if bridge_is_adverb {
                                let tail_lower = word_tokens[pos + 3].1.text.to_lowercase();
                                let tail_is_verb = if let Some(recognizer) = verb_recognizer {
                                    Self::recognizer_is_valid_verb_form(&tail_lower, recognizer)
                                        || Self::is_common_verb(&tail_lower)
                                        || Self::is_verb_form(&tail_lower)
                                        || Self::is_possible_first_person_verb(&tail_lower)
                                        || Self::is_likely_conjugated_verb(&tail_lower)
                                } else {
                                    Self::is_common_verb(&tail_lower)
                                        || Self::is_verb_form(&tail_lower)
                                        || Self::is_possible_first_person_verb(&tail_lower)
                                        || Self::is_likely_conjugated_verb(&tail_lower)
                                };
                                if tail_is_verb {
                                    return None;
                                }
                                // Variante frecuente: adverbio + no + verbo
                                // "Sobre tu pregunta claramente no respondo"
                                if tail_lower == "no" && pos + 4 < word_tokens.len() {
                                    let after_no_lower = word_tokens[pos + 4].1.text.to_lowercase();
                                    let after_no_is_verb = if let Some(recognizer) = verb_recognizer
                                    {
                                        Self::recognizer_is_valid_verb_form(
                                            &after_no_lower,
                                            recognizer,
                                        ) || Self::is_common_verb(&after_no_lower)
                                            || Self::is_verb_form(&after_no_lower)
                                            || Self::is_possible_first_person_verb(&after_no_lower)
                                            || Self::is_likely_conjugated_verb(&after_no_lower)
                                    } else {
                                        Self::is_common_verb(&after_no_lower)
                                            || Self::is_verb_form(&after_no_lower)
                                            || Self::is_possible_first_person_verb(&after_no_lower)
                                            || Self::is_likely_conjugated_verb(&after_no_lower)
                                    };
                                    if after_no_is_verb {
                                        return None;
                                    }
                                }
                            }
                        }
                    }
                }

                // Identificar si el siguiente token es nominal (sustantivo/adjetivo)
                let mut is_nominal = false;
                if let Some(ref info) = next_token.word_info {
                    use crate::dictionary::WordCategory;
                    if matches!(
                        info.category,
                        WordCategory::Sustantivo | WordCategory::Adjetivo
                    ) {
                        is_nominal = true;
                    }
                }

                // Si NO es verbo y es nominal �?' posesivo
                if !is_verb && is_nominal {
                    // Excepción contextual: "tu mejor/peor + verbo" suele ser pronombre tónico.
                    if matches!(next_word_text.as_str(), "mejor" | "peor")
                        && pos + 2 < word_tokens.len()
                    {
                        let next_next_lower = word_tokens[pos + 2].1.text.to_lowercase();
                        let next_next_is_verb = if let Some(recognizer) = verb_recognizer {
                            Self::recognizer_is_valid_verb_form(&next_next_lower, recognizer)
                        } else {
                            Self::is_second_person_verb(&next_next_lower)
                                || Self::is_common_verb(&next_next_lower)
                                || Self::is_likely_conjugated_verb(&next_next_lower)
                        };
                        if !next_next_is_verb {
                            return None;
                        }
                    } else {
                        return None; // "tu" seguido de sustantivo/adjetivo = posesivo
                    }
                }

                // Si es verbo y además nominal (ambigüedad), exigir una pista verbal adicional
                if is_verb && is_nominal {
                    // En sintagmas preposicionales ("sobre tu pregunta ...", "con tu mando ..."),
                    // "tu + nominal" suele ser posesivo aunque el nominal también pueda ser forma verbal.
                    // Evita falsos positivos como "Sobre tu pregunta ya respondo".
                    if let Some(prev_lower) = prev_word.as_deref() {
                        if Self::is_preposition(prev_lower) {
                            return None;
                        }
                    }

                    let has_verbal_cue = if pos + 2 < word_tokens.len() {
                        let next_next = word_tokens[pos + 2].1;
                        if let Some(ref info) = next_next.word_info {
                            use crate::dictionary::WordCategory;
                            matches!(
                                info.category,
                                WordCategory::Adverbio
                                    | WordCategory::Pronombre
                                    | WordCategory::Preposicion
                                    | WordCategory::Verbo
                            )
                        } else {
                            let next_next_lower = next_next.text.to_lowercase();
                            let next_next_is_verb = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(&next_next_lower, recognizer)
                            } else {
                                Self::is_second_person_verb(&next_next_lower)
                                    || Self::is_common_verb(&next_next_lower)
                                    || Self::is_likely_conjugated_verb(&next_next_lower)
                            };
                            next_next_is_verb
                                || matches!(
                                    next_next_lower.as_str(),
                                    // clíticos
                                    "me" | "te" | "se" | "nos" | "os" | "lo" | "la" | "los" | "las" | "le" | "les" |
                                // preposiciones frecuentes
                                "a" | "de" | "en" | "con" | "por" | "para" | "sin" | "sobre" | "tras" | "desde" | "hacia" |
                                // adverbios comunes
                                "ya" | "hoy" | "ayer" | "ahora" | "luego" | "todavía" | "siempre" | "nunca" |
                                "aquí" | "ahí" | "allí"
                                )
                        }
                    } else {
                        false
                    };

                    if !has_verbal_cue {
                        return None; // Ambiguo: preferir posesivo si no hay pista verbal
                    }
                }
            }
        }

        // Caso especial mas/más: "Mas" con mayúscula puede ser apellido (Artur Mas)
        // No corregir si:
        // 1. La palabra está capitalizada Y está en el diccionario de nombres propios
        // 2. La palabra anterior también está capitalizada (patrón "Nombre Apellido")
        if pair.without_accent == "mas" && pair.with_accent == "más" && !has_accent {
            let is_capitalized = token
                .text
                .chars()
                .next()
                .map_or(false, |c| c.is_uppercase());

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
                    let prev_is_capitalized = prev_token_text
                        .chars()
                        .next()
                        .map_or(false, |c| c.is_uppercase());
                    if prev_is_capitalized {
                        // Patrón "Nombre Mas" �?' apellido, no corregir
                        return None;
                    }
                }
            }
        }

        // Caso especial: "Si, ..." al inicio suele ser afirmación.
        if pair.without_accent == "si"
            && pair.with_accent == "sí"
            && !has_accent
            && (Self::is_affirmative_si_with_comma(all_tokens, token_idx)
                || Self::is_demonstrative_affirmative_si_with_comma(all_tokens, token_idx)
                || Self::is_discourse_marker_affirmative_si_with_comma(all_tokens, token_idx))
        {
            return Some(DiacriticCorrection {
                token_index: token_idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, pair.with_accent),
                reason: Self::get_reason(pair, true),
            });
        }

        // Determinar si necesita tilde basándose en el contexto
        let needs_accent = Self::needs_accent(
            pair,
            prev_word.as_deref(),
            next_word.as_deref(),
            next_next_word.as_deref(),
            next_third_word.as_deref(),
            next_fourth_word.as_deref(),
            prev_prev_word.as_deref(),
            verb_recognizer,
        );

        // "té" sustantivo es muy frecuente y ambiguo fuera de contextos claros.
        // Si ya viene con tilde, solo quitarla cuando el contexto sea claramente pronominal.
        if pair.without_accent == "te" && has_accent && !needs_accent {
            if !Self::is_clear_te_pronoun_context(
                prev_word.as_deref(),
                next_word.as_deref(),
                prev_prev_word.as_deref(),
                verb_recognizer,
            ) {
                return None;
            }
        }

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
    fn needs_accent(
        pair: &DiacriticPair,
        prev: Option<&str>,
        next: Option<&str>,
        next_next: Option<&str>,
        next_third: Option<&str>,
        next_fourth: Option<&str>,
        prev_prev: Option<&str>,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
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
                    // "el no + verbo": patrón muy probable de pronombre sujeto
                    // ("El no sabia que hacer" -> "Él no sabía que hacer")
                    if next_word == "no" {
                        if let Some(word_after_no) = next_next {
                            let normalized = Self::normalize_spanish(word_after_no);
                            let is_verb_after_no = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(word_after_no, recognizer)
                                    || Self::recognizer_is_valid_verb_form(
                                        normalized.as_str(),
                                        recognizer,
                                    )
                                    || Self::is_common_verb(word_after_no)
                                    || Self::is_likely_conjugated_verb(word_after_no)
                                    || Self::is_common_verb(normalized.as_str())
                                    || Self::is_likely_conjugated_verb(normalized.as_str())
                                    || matches!(normalized.as_str(), "sabia" | "sabian")
                            } else {
                                Self::is_common_verb(word_after_no)
                                    || Self::is_likely_conjugated_verb(word_after_no)
                                    || Self::is_common_verb(normalized.as_str())
                                    || Self::is_likely_conjugated_verb(normalized.as_str())
                                    || matches!(normalized.as_str(), "sabia" | "sabian")
                            };
                            if is_verb_after_no {
                                return true;
                            }
                        }
                    }
                    // "el si + verbo" en contexto de cláusula suele ser "él sí + verbo".
                    // Ej.: "pero el si sabe", "creo que el si puede".
                    if next_word == "si" {
                        if let Some(word_after_si) = next_next {
                            let normalized = Self::normalize_spanish(word_after_si);
                            let is_verb_after_si = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(word_after_si, recognizer)
                                    || Self::recognizer_is_valid_verb_form(
                                        normalized.as_str(),
                                        recognizer,
                                    )
                                    || Self::is_common_verb(word_after_si)
                                    || Self::is_likely_conjugated_verb(word_after_si)
                                    || Self::is_common_verb(normalized.as_str())
                                    || Self::is_likely_conjugated_verb(normalized.as_str())
                            } else {
                                Self::is_common_verb(word_after_si)
                                    || Self::is_likely_conjugated_verb(word_after_si)
                                    || Self::is_common_verb(normalized.as_str())
                                    || Self::is_likely_conjugated_verb(normalized.as_str())
                            };
                            let prev_is_clause_intro = prev.is_none()
                                || prev.map(Self::normalize_spanish).is_some_and(|p| {
                                    p == "que" || Self::is_discourse_connector(p.as_str())
                                });
                            if is_verb_after_si && prev_is_clause_intro {
                                return true;
                            }
                        }
                    }
                    // "el mismo" vs "él mismo":
                    // - "él mismo" (pronombre + énfasis): "él mismo lo hizo"
                    // - "el mismo [sustantivo]" (artículo + adjetivo): "el mismo cuello"
                    // Si "mismo/a" va seguido de sustantivo, es artículo (no corregir)
                    if next_word == "mismo" || next_word == "misma" {
                        // Verificar si hay sintagma nominal después de "mismo/a"
                        if let Some(word_after_mismo) = next_next {
                            // Si hay núcleo nominal después, "el" es artículo
                            if Self::is_nominal_after_mismo(word_after_mismo, verb_recognizer) {
                                return false; // "el mismo cuello" - artículo
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
                // "preposición + el + verbo" suele ser pronombre tónico ("para él es...")
                if let Some(prev_word) = prev {
                    if Self::is_preposition(prev_word) {
                        if let Some(next_word) = next {
                            let normalized = Self::normalize_spanish(next_word);
                            let looks_like_verb = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(next_word, recognizer)
                                    || Self::recognizer_is_valid_verb_form(
                                        normalized.as_str(),
                                        recognizer,
                                    )
                                    || Self::is_common_verb(next_word)
                                    || Self::is_likely_conjugated_verb(next_word)
                                    || Self::is_common_verb(normalized.as_str())
                                    || Self::is_likely_conjugated_verb(normalized.as_str())
                            } else {
                                Self::is_common_verb(next_word)
                                    || Self::is_likely_conjugated_verb(next_word)
                                    || Self::is_common_verb(normalized.as_str())
                                    || Self::is_likely_conjugated_verb(normalized.as_str())
                            };
                            if looks_like_verb {
                                return true;
                            }
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
                    // Patrón específico de error verbal:
                    // "tu a venido" -> "tú has venido"
                    // Si "tu" va seguido de "a + participio", es pronombre sujeto.
                    if next_word == "a" {
                        if let Some(word_after_a) = next_next {
                            if Self::is_likely_participle_after_aux(word_after_a, verb_recognizer) {
                                return true;
                            }
                        }
                    }

                    // "mejor/peor" pueden funcionar como adverbio:
                    // "tú mejor sabes", "tú peor entiendes".
                    if matches!(next_word, "mejor" | "peor") {
                        if let Some(word_after) = next_next {
                            let is_verb_after = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(word_after, recognizer)
                            } else {
                                Self::is_second_person_verb(word_after)
                                    || Self::is_common_verb(word_after)
                                    || Self::is_likely_conjugated_verb(word_after)
                            };
                            if is_verb_after {
                                return true;
                            }
                        }
                    }
                    if Self::is_likely_noun_or_adj(next_word) {
                        return false; // Es posesivo: "tu casa", "tu hermana"
                    }
                }

                // Si está precedido por verbo de segunda persona, es pronombre enfático
                // "opinas tú", "crees tú", "piensas tú"
                if let Some(prev_word) = prev {
                    if Self::is_second_person_verb(prev_word) {
                        return true; // Pronombre enfático: "opinas tú"
                    }
                }

                // Si está precedido por conjunción Y no va seguido de sustantivo,
                // es pronombre en sujeto compuesto (él y tú sois)
                if let Some(prev_word) = prev {
                    if prev_word == "y" || prev_word == "e" || prev_word == "o" || prev_word == "ni"
                    {
                        return true; // Sujeto compuesto: "él y tú sois"
                    }
                }

                // Verificar contexto de la siguiente palabra
                if let Some(next_word) = next {
                    // Si va seguido de conjunción, es pronombre (tú y yo)
                    if next_word == "y" || next_word == "e" || next_word == "o" || next_word == "ni"
                    {
                        return true;
                    }
                    // Si va seguido de "mismo/a", es pronombre (tú mismo)
                    if next_word == "mismo" || next_word == "misma" {
                        return true;
                    }
                    // Si va seguido de verbo conjugado, es pronombre (tú cantas)
                    // Usar VerbRecognizer si está disponible (más preciso)
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        Self::recognizer_is_valid_verb_form(next_word, recognizer)
                    } else {
                        Self::is_common_verb(next_word) || Self::is_verb_form(next_word)
                    };
                    if is_verb {
                        return true;
                    }
                    // Si va seguido de posible verbo en 1ª persona, probablemente es pronombre
                    // con error de concordancia: "tu canto" �?' "tú cantas"
                    if Self::is_possible_first_person_verb(next_word) {
                        return true;
                    }
                    // Caso especial: "tu no + verbo" -> pronombre sujeto (tú no puedes...)
                    // Evita falsos positivos en usos nominales como "tu no rotundo".
                    if next_word == "no" {
                        if let Some(word_after_no) = next_next {
                            let is_verb_after_no = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(word_after_no, recognizer)
                            } else {
                                Self::is_second_person_verb(word_after_no)
                                    || Self::is_common_verb(word_after_no)
                                    || Self::is_likely_conjugated_verb(word_after_no)
                            };
                            if is_verb_after_no {
                                return true;
                            }
                        }
                    }
                    // Si va seguido de adverbio común, es pronombre sujeto (tú también, tú siempre)
                    if Self::is_common_adverb(next_word) {
                        return true;
                    }
                    // Patrón general: "tú + adverbio + verbo"
                    // Ej.: "tu claramente sabes", "tu ahora entiendes".
                    if Self::is_likely_adverb(next_word) {
                        if let Some(word_after_adv) = next_next {
                            let is_verb_after_adv = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(word_after_adv, recognizer)
                            } else {
                                Self::is_second_person_verb(word_after_adv)
                                    || Self::is_common_verb(word_after_adv)
                                    || Self::is_likely_conjugated_verb(word_after_adv)
                            };
                            if is_verb_after_adv {
                                return true;
                            }
                            // Patrón robusto: "tú + adverbio + no + verbo"
                            // Ej.: "Tu claramente no sabes", "Tu ahora no quieres".
                            if word_after_adv == "no" {
                                if let Some(word_after_no) = next_third {
                                    let is_verb_after_no = if let Some(recognizer) = verb_recognizer
                                    {
                                        Self::recognizer_is_valid_verb_form(
                                            word_after_no,
                                            recognizer,
                                        )
                                    } else {
                                        Self::is_second_person_verb(word_after_no)
                                            || Self::is_common_verb(word_after_no)
                                            || Self::is_likely_conjugated_verb(word_after_no)
                                    };
                                    if is_verb_after_no {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    // Locución adverbial muy común: "tal vez"
                    if next_word == "tal" && matches!(next_next, Some("vez")) {
                        return true;
                    }
                    // Si va seguido de interrogativo, es pronombre sujeto (¿tú qué harías?, ¿tú cuándo vienes?)
                    if Self::is_interrogative(next_word) {
                        return true;
                    }
                    // Si va seguido de sustantivo o adjetivo común, es posesivo (tu casa, tu ayuda)
                    if Self::is_likely_noun_or_adj(next_word) {
                        return false; // Es posesivo, no necesita tilde
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
                            // Si siguiente es sustantivo/adjetivo �?' posesivo
                            if Self::is_likely_noun_or_adj(next_word)
                                || Self::is_common_noun_after_mi(next_word)
                            {
                                return false; // Posesivo: "en mi lugar", "por mi parte"
                            }
                            // Si siguiente es pronombre clítico �?' pronombre tónico
                            if Self::is_clitic_pronoun(next_word) {
                                return true; // Pronombre: "a mí me gusta", "para mí te digo"
                            }
                            // Si siguiente es verbo conjugado �?' pronombre tónico
                            if let Some(vr) = verb_recognizer {
                                if Self::recognizer_is_valid_verb_form(next_word, vr) {
                                    return true; // Pronombre: "para mí es", "a mí parece"
                                }
                            }
                        }
                        return true; // Pronombre (preposición + mi sin sustantivo)
                    }
                }
                false
            }

            // te/té
            ("te", "té") => {
                // "té" es sustantivo (la bebida) en múltiples contextos nominales:
                // determinantes, cuantificadores, preposición o tras verbo transitivo.
                Self::is_tea_noun_context(prev, next, prev_prev, verb_recognizer)
            }

            // se/sé
            ("se", "sé") => {
                // "sé" es verbo saber (yo sé, no sé) o imperativo de ser (sé bueno)
                // "se" es pronombre reflexivo/pasivo (se fue, se implementó, no se puede)

                // Patrón clítico: "se lo/la/los/las + verbo"
                // Ej: "yo se lo dije", "yo se lo pedí" -> "se" es pronombre, no "sé".
                // OJO: no bloquear "yo se lo que..." (aquí "sé" es verbo saber).
                if let Some(next_word) = next {
                    if matches!(next_word, "lo" | "la" | "los" | "las") {
                        if let Some(next_next_word) = next_next {
                            let is_verb = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(next_next_word, recognizer)
                            } else {
                                Self::is_conjugated_verb_for_se(next_next_word)
                                    || Self::is_likely_conjugated_verb(next_next_word)
                                    || Self::is_second_person_verb(next_next_word)
                                    || Self::is_possible_first_person_verb(next_next_word)
                                    || Self::is_first_person_preterite_form(next_next_word)
                            };
                            if is_verb {
                                return false;
                            }
                        }
                    }
                }

                // Primero verificar si "se" va seguido de verbo conjugado
                // En ese caso es pronombre reflexivo/pasivo, NO el verbo "saber"
                // Ejemplos: "se implementó", "no se puede", "ya se terminó", "no se trata"
                if let Some(next_word) = next {
                    let next_word_norm = Self::normalize_spanish(next_word);
                    let prev_norm = prev.map(Self::normalize_spanish);
                    let prev_prev_norm = prev_prev.map(Self::normalize_spanish);
                    let next_next_norm = next_next.map(Self::normalize_spanish);
                    let next_third_norm = next_third.map(Self::normalize_spanish);
                    let follows_discourse_connector =
                        next_next.map(Self::normalize_spanish).is_some_and(|w| {
                            matches!(w.as_str(), "pero" | "y" | "e" | "ni" | "sino" | "aunque")
                        });
                    let has_likely_saber_de_tail = next_next_norm.as_deref() == Some("de")
                        && next_third_norm
                            .as_deref()
                            .is_some_and(Self::is_likely_saber_de_complement);
                    let has_saber_topic_tail = next_next_norm.as_deref() == Some("sobre")
                        || (next_next_norm.as_deref() == Some("acerca")
                            && next_third_norm.as_deref() == Some("de"))
                        || (next_next_norm.as_deref() == Some("respecto")
                            && next_third_norm.as_deref() == Some("a"));
                    let is_no_ya_interrogative_saber =
                        matches!(prev_norm.as_deref(), Some("no" | "ya"))
                            && Self::is_no_ya_interrogative_saber(
                                next_word_norm.as_str(),
                                next_next_norm.as_deref(),
                                next_third_norm.as_deref(),
                            );
                    let is_no_ya_indefinite_saber =
                        matches!(prev_norm.as_deref(), Some("no" | "ya"))
                            && Self::is_negative_indefinite_following_saber(
                                next_word_norm.as_str(),
                            )
                            && (prev_prev_norm.as_deref() == Some("yo")
                                || next_next.is_none()
                                || follows_discourse_connector
                                || has_likely_saber_de_tail
                                || has_saber_topic_tail);
                    if is_no_ya_indefinite_saber || is_no_ya_interrogative_saber {
                        // No bloquear aquí: en "yo no se nada/nadie/..." suele ser "sé".
                        // También cubrir cierres de cláusula sin sujeto explícito:
                        // "No se nada", "No se nada, pero...", "No se nada de eso".
                    } else {
                        // Usar VerbRecognizer si está disponible (más preciso)
                        let is_verb = if let Some(recognizer) = verb_recognizer {
                            Self::recognizer_is_valid_verb_form(next_word, recognizer)
                        } else {
                            // Fallback a lista hardcodeada
                            Self::is_conjugated_verb_for_se(next_word)
                        };
                        if is_verb {
                            return false; // Es "se" reflexivo/pasivo
                        }
                    }
                }

                if let Some(prev_word) = prev {
                    let prev_word_norm = Self::normalize_spanish(prev_word);
                    let prev_prev_norm = prev_prev.map(Self::normalize_spanish);
                    // "yo sé" es claramente verbo saber
                    if prev_word_norm == "yo" {
                        return true;
                    }
                    // Tras conectores ("pero/y/ni/aunque"), aceptar patrones equivalentes
                    // al inicio de cláusula: "pero se que...", "y se lo que...".
                    if Self::is_discourse_connector(prev_word_norm.as_str()) {
                        if let Some(next_word) = next {
                            let next_word_norm = Self::normalize_spanish(next_word);
                            let next_next_norm = next_next.map(Self::normalize_spanish);
                            if next_word_norm == "que"
                                || (next_word_norm == "lo"
                                    && next_next_norm.as_deref() == Some("que"))
                                || Self::is_adjective_indicator(next_word_norm.as_str())
                            {
                                return true;
                            }
                        }
                    }
                    // "no sé" o "ya sé" solo si NO va seguido de verbo conjugado
                    // (ya verificamos arriba que no hay verbo después)
                    if prev_word_norm == "no" || prev_word_norm == "ya" {
                        // Si no hay siguiente palabra, es "no sé" / "ya sé"
                        if next.is_none() {
                            return true;
                        }
                        // Si va seguido de "que", "cuánto", "dónde", etc., es verbo saber
                        if let Some(next_word) = next {
                            let next_word_norm = Self::normalize_spanish(next_word);
                            let next_next_norm = next_next.map(Self::normalize_spanish);
                            let next_third_norm = next_third.map(Self::normalize_spanish);
                            let next_fourth_norm = next_fourth.map(Self::normalize_spanish);
                            if Self::is_no_ya_interrogative_saber(
                                next_word_norm.as_str(),
                                next_next_norm.as_deref(),
                                next_third_norm.as_deref(),
                            ) {
                                return true;
                            }
                            if Self::is_saber_nonverbal_complement(next_word_norm.as_str()) {
                                let has_impersonal_locative_tail = prev_prev_norm.as_deref()
                                    != Some("yo")
                                    && Self::is_impersonal_locative_quantity_tail(
                                        next_next_norm.as_deref(),
                                        next_third_norm.as_deref(),
                                        next_fourth_norm.as_deref(),
                                    );
                                if !has_impersonal_locative_tail {
                                    return true;
                                }
                            }
                            if Self::is_saber_modifier_before_indefinite(next_word_norm.as_str()) {
                                if let Some(neg_norm) = next_next_norm.as_deref() {
                                    if Self::is_negative_indefinite_following_saber(neg_norm) {
                                        let after_neg_norm = next_third_norm.as_deref();
                                        let after_after_neg_norm = next_fourth_norm.as_deref();
                                        let locative_impersonal = neg_norm == "nada"
                                            && prev_prev_norm.as_deref().is_some_and(
                                                Self::is_locative_adverb_for_nadar_ambiguity,
                                            )
                                            && after_neg_norm == Some("en");
                                        if prev_prev_norm.as_deref() == Some("yo") {
                                            return true;
                                        }
                                        if locative_impersonal {
                                            return false;
                                        }
                                        if after_neg_norm.is_none() {
                                            return true;
                                        }
                                        if matches!(
                                            after_neg_norm,
                                            Some("pero" | "y" | "e" | "ni" | "sino" | "aunque")
                                        ) {
                                            return true;
                                        }
                                        if after_neg_norm == Some("de")
                                            && after_after_neg_norm
                                                .is_some_and(Self::is_likely_saber_de_complement)
                                        {
                                            return true;
                                        }
                                        if after_neg_norm == Some("sobre")
                                            || (after_neg_norm == Some("acerca")
                                                && after_after_neg_norm == Some("de"))
                                            || (after_neg_norm == Some("respecto")
                                                && after_after_neg_norm == Some("a"))
                                        {
                                            return true;
                                        }
                                    }
                                }
                            }
                            if Self::is_negative_indefinite_following_saber(next_word_norm.as_str())
                            {
                                let locative_nadar_pattern = next_word_norm == "nada"
                                    && prev_prev_norm
                                        .as_deref()
                                        .is_some_and(Self::is_locative_adverb_for_nadar_ambiguity)
                                    && !next_next
                                        .map(Self::normalize_spanish)
                                        .as_deref()
                                        .is_some_and(|w| w == "de");
                                if prev_prev_norm.as_deref() == Some("yo") {
                                    return true;
                                }
                                if locative_nadar_pattern {
                                    return false;
                                }
                                if next_next.map(Self::normalize_spanish).as_deref() == Some("de")
                                    && next_third
                                        .map(Self::normalize_spanish)
                                        .as_deref()
                                        .is_some_and(Self::is_likely_saber_de_complement)
                                {
                                    return true;
                                }
                                let next_next_norm = next_next.map(Self::normalize_spanish);
                                let next_third_norm = next_third.map(Self::normalize_spanish);
                                if next_next_norm.as_deref() == Some("sobre")
                                    || (next_next_norm.as_deref() == Some("acerca")
                                        && next_third_norm.as_deref() == Some("de"))
                                    || (next_next_norm.as_deref() == Some("respecto")
                                        && next_third_norm.as_deref() == Some("a"))
                                {
                                    return true;
                                }
                                // Sin sujeto explícito, aceptar "sé" solo cuando el patrón
                                // parece cierre discursivo ("no se nada", "no se nada, pero..."),
                                // evitando el impersonal "no se nada en ...".
                                if next_next.is_none() {
                                    return true;
                                }
                                if let Some(next_next_word) = next_next {
                                    let next_next_norm = Self::normalize_spanish(next_next_word);
                                    if matches!(
                                        next_next_norm.as_str(),
                                        "pero" | "y" | "e" | "ni" | "sino" | "aunque"
                                    ) {
                                        return true;
                                    }
                                }
                            }
                        }
                        // En otros casos con "no/ya" + se + algo, asumir reflexivo
                        return false;
                    }
                } else if let Some(next_word) = next {
                    // Al inicio: "sé que...", "sé lo que..." o "sé bueno" (imperativo de ser)
                    let next_word_norm = Self::normalize_spanish(next_word);
                    let next_next_norm = next_next.map(Self::normalize_spanish);
                    if next_word_norm == "que"
                        || (next_word_norm == "lo" && next_next_norm.as_deref() == Some("que"))
                        || Self::is_adjective_indicator(next_word_norm.as_str())
                    {
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
                            return false; // "de verdad", "de nuevo", "de hecho", etc.
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
                            if matches!(
                                prev_prev,
                                "más" | "menos" | "antes" | "después" | "mejor" | "peor"
                            ) {
                                return false;
                            }
                        }
                        // Verificar si "de" introduce una cláusula relativa: "de lo que", "de la que"
                        // En "que de lo que no se puede hablar", "de" es preposición
                        if let Some(next_word) = next {
                            if matches!(
                                next_word,
                                "lo" | "la" | "los" | "las" | "el" | "un" | "una" | "unos" | "unas"
                            ) {
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
                let next_is_mismo = next.map_or(false, |n| {
                    matches!(n, "mismo" | "misma" | "mismos" | "mismas")
                });

                if let Some(prev_word) = prev {
                    // "como si" es construcción condicional, NO sí enfático
                    // "como si participaran", "como si fuera", "como si nada"
                    if prev_word == "como" {
                        return false;
                    }
                    // "eso/esto si": distinguir condicional de enfático.
                    // - "eso sí que..." -> enfático (con tilde)
                    // - "haría eso si tuviera...", "eso si llueve..." -> condicional (sin tilde)
                    if prev_word == "eso" || prev_word == "esto" {
                        if let Some(next_word) = next {
                            if next_word == "que" {
                                return true;
                            }
                            // "eso si no te importa" → conditional (no accent)
                            if next_word == "no" {
                                return false;
                            }
                            let next_is_verb = if let Some(recognizer) = verb_recognizer {
                                Self::recognizer_is_valid_verb_form(next_word, recognizer)
                            } else {
                                Self::is_likely_conjugated_verb(next_word)
                                    || Self::is_common_verb(next_word)
                                    || Self::is_verb_form(next_word)
                            };
                            if next_is_verb {
                                return false;
                            }
                        }
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
                    // "no se si", "ya se si", "yo se si" -> "si" condicional (sin tilde)
                    // Evita falsos positivos de "sí" al final de frase en este patrón.
                    if (prev_word == "se" || prev_word == "sé")
                        && prev_prev.map_or(false, |pp| matches!(pp, "no" | "ya" | "yo"))
                    {
                        return false;
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
                        Self::recognizer_is_valid_verb_form(next_word, recognizer)
                    } else {
                        Self::is_likely_conjugated_verb(next_word)
                    };
                    if is_verb {
                        // "si + subjuntivo imperfecto" suele ser prótasis condicional,
                        // no "sí" enfático: "si pudiera", "si tuviera", "si fuese".
                        if Self::is_likely_imperfect_subjunctive_form(next_word) {
                            return false;
                        }
                        // Si está al inicio de oración (prev == None), es conjunción condicional
                        if prev.is_none() {
                            return false; // "Si es...", "Si vienes..." - conjunción, no enfático
                        }
                        // Solo aceptar "sí" enfático después de pronombres personales/demostrativos
                        // "él sí vino", "eso sí funciona", "esto sí me gusta"
                        let prev_is_subject_pronoun = prev.map_or(false, |p| {
                            matches!(
                                p,
                                "él" | "ella"
                                    | "el"
                                    | "ellos"
                                    | "ellas"
                                    | "eso"
                                    | "esto"
                                    | "ello"
                                    | "usted"
                                    | "ustedes"
                                    | "yo"
                                    | "tú"
                                    | "tu"
                                    | "nosotros"
                                    | "nosotras"
                                    | "vosotros"
                                    | "vosotras"
                            )
                        });
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
                    let next_norm = Self::normalize_spanish(next_word);

                    // Casos claros de "aún" = todavía
                    if next_norm == "no" || next_norm == "mas" || next_norm == "menos" {
                        return true;
                    }

                    // Casos claros de "aun" = incluso (sin tilde)
                    // Importante: evaluarlos ANTES de "aún + verbo", porque en construcciones
                    // concesivas puede venir un gerundio verbal ("aun siendo difícil").
                    if next_norm == "asi"
                        || next_norm == "cuando"
                        || next_norm == "con"
                        || next_norm == "sin"
                    {
                        return false;
                    }
                    if next_norm.ends_with("ando")
                        || next_norm.ends_with("iendo")
                        || next_norm.ends_with("yendo")
                    {
                        return false;
                    }

                    // "aún + verbo" = todavía (aún es, aún hay, aún está, aún queda)
                    // Usar VerbRecognizer si está disponible
                    let is_verb = if let Some(recognizer) = verb_recognizer {
                        Self::recognizer_is_valid_verb_form(next_word, recognizer)
                    } else {
                        // Fallback a lista hardcodeada
                        matches!(
                            next_word,
                            "es" | "son"
                                | "era"
                                | "eran"
                                | "fue"
                                | "fueron"
                                | "está"
                                | "están"
                                | "estaba"
                                | "estaban"
                                | "hay"
                                | "había"
                                | "hubo"
                                | "queda"
                                | "quedan"
                                | "quedaba"
                                | "quedaban"
                                | "falta"
                                | "faltan"
                                | "faltaba"
                                | "faltaban"
                                | "tiene"
                                | "tienen"
                                | "tenía"
                                | "tenían"
                                | "puede"
                                | "pueden"
                                | "podía"
                                | "podían"
                                | "sigue"
                                | "siguen"
                                | "seguía"
                                | "seguían"
                                | "existe"
                                | "existen"
                                | "existía"
                                | "existían"
                        )
                    };
                    if is_verb {
                        return true;
                    }
                    // "aún + participio" = todavía (aún encabezado, aún dormido, aún vivo)
                    if next_word.ends_with("ado")
                        || next_word.ends_with("ido")
                        || next_word.ends_with("ada")
                        || next_word.ends_with("ida")
                        || next_word.ends_with("ados")
                        || next_word.ends_with("idos")
                        || next_word.ends_with("adas")
                        || next_word.ends_with("idas")
                    {
                        return true;
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

    fn is_tea_noun_context(
        prev: Option<&str>,
        next: Option<&str>,
        _prev_prev: Option<&str>,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let has_strong_nominal_left = prev.is_some_and(|prev_word| {
            Self::is_article(prev_word)
                || Self::is_tea_determiner(prev_word)
                || Self::is_tea_quantifier(prev_word)
        });
        let has_weak_nominal_left = prev.is_some_and(|prev_word| {
            Self::is_preposition(prev_word) || Self::is_adjective_indicator(prev_word)
        });
        let has_likely_verb_left =
            prev.is_some_and(|prev_word| Self::is_likely_verb_word(prev_word, verb_recognizer));

        if let Some(next_word) = next {
            // "té caliente", "té verde": adjetivo explícito a la derecha.
            // Debe evaluarse antes de la ruta verbal porque formas como
            // "caliente" pueden ser ambiguas para el recognizer.
            if Self::is_adjective_indicator(next_word) {
                return true;
            }

            // "te + clítico/verbo" suele ser pronombre átono:
            // "te lo dije", "como te decía", "se te cayó", "¿cómo te va?".
            // Solo mantener "té" si hay un contexto nominal muy fuerte a la izquierda
            // ("el/este/más té está...").
            if Self::is_clitic_pronoun(next_word)
                || Self::is_likely_verb_word(next_word, verb_recognizer)
            {
                return has_strong_nominal_left;
            }

            // Sustantivo/adjetivo no verbal: "té sabor", "té natural".
            if Self::is_likely_noun_or_adj(next_word) {
                return true;
            }
        }

        // Sin evidencia pronominal a la derecha:
        // - fuerte/nominal izquierda: "el té", "más té"
        // - verbo izquierda: "quiero té"
        // - nominal débil izquierda: "de té" (conservador)
        has_strong_nominal_left || has_likely_verb_left || has_weak_nominal_left
    }

    fn is_clear_te_pronoun_context(
        prev: Option<&str>,
        next: Option<&str>,
        _prev_prev: Option<&str>,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if let Some(prev_word) = prev {
            if Self::is_tea_nominal_left_word(prev_word) {
                return false;
            }
        }

        if let Some(next_word) = next {
            // "te quiero", "te lo dije", "te vi"
            if Self::is_clitic_pronoun(next_word)
                || Self::is_likely_verb_word(next_word, verb_recognizer)
            {
                return true;
            }
        }

        false
    }

    fn is_tea_nominal_left_word(word: &str) -> bool {
        Self::is_article(word)
            || Self::is_preposition(word)
            || Self::is_adjective_indicator(word)
            || Self::is_tea_determiner(word)
            || Self::is_tea_quantifier(word)
    }

    fn is_tea_determiner(word: &str) -> bool {
        matches!(
            word,
            "este"
                | "esta"
                | "estos"
                | "estas"
                | "ese"
                | "esa"
                | "esos"
                | "esas"
                | "aquel"
                | "aquella"
                | "aquellos"
                | "aquellas"
                | "mi"
                | "mis"
                | "tu"
                | "tus"
                | "su"
                | "sus"
                | "nuestro"
                | "nuestra"
                | "nuestros"
                | "nuestras"
                | "vuestro"
                | "vuestra"
                | "vuestros"
                | "vuestras"
        )
    }

    fn is_tea_quantifier(word: &str) -> bool {
        matches!(
            word,
            "mas"
                | "más"
                | "menos"
                | "mucho"
                | "mucha"
                | "muchos"
                | "muchas"
                | "poco"
                | "poca"
                | "pocos"
                | "pocas"
                | "bastante"
                | "bastantes"
                | "demasiado"
                | "demasiada"
                | "demasiados"
                | "demasiadas"
                | "todo"
                | "toda"
                | "todos"
                | "todas"
        )
    }

    fn is_likely_verb_word(word: &str, verb_recognizer: Option<&dyn VerbFormRecognizer>) -> bool {
        if let Some(recognizer) = verb_recognizer {
            return Self::recognizer_is_valid_verb_form(word, recognizer);
        }

        Self::is_common_verb(word)
            || Self::is_likely_conjugated_verb(word)
            || Self::is_second_person_verb(word)
            || Self::is_possible_first_person_verb(word)
            || Self::is_first_person_preterite_form(word)
    }

    fn is_likely_participle_after_aux(
        word: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let lower = word.to_lowercase();
        let has_participle_shape = matches!(
            lower.as_str(),
            // Irregulares frecuentes
            "hecho"
                | "dicho"
                | "visto"
                | "puesto"
                | "muerto"
                | "abierto"
                | "escrito"
                | "roto"
                | "vuelto"
                | "cubierto"
                | "resuelto"
                | "devuelto"
                | "frito"
                | "impreso"
                | "satisfecho"
                | "deshecho"
        ) || lower.ends_with("ado")
            || lower.ends_with("ada")
            || lower.ends_with("ados")
            || lower.ends_with("adas")
            || lower.ends_with("ido")
            || lower.ends_with("ida")
            || lower.ends_with("idos")
            || lower.ends_with("idas")
            || lower.ends_with("ído")
            || lower.ends_with("ída")
            || lower.ends_with("ídos")
            || lower.ends_with("ídas")
            || lower.ends_with("to")
            || lower.ends_with("ta")
            || lower.ends_with("tos")
            || lower.ends_with("tas")
            || lower.ends_with("cho")
            || lower.ends_with("cha")
            || lower.ends_with("chos")
            || lower.ends_with("chas")
            || lower.ends_with("so")
            || lower.ends_with("sa")
            || lower.ends_with("sos")
            || lower.ends_with("sas");

        if !has_participle_shape {
            return false;
        }

        // Filtro mínimo para evitar nominales muy frecuentes en "-ado/-ido"
        // que no deberían activar el patrón verbal "tu a + ...".
        if matches!(
            lower.as_str(),
            "lado"
                | "grado"
                | "estado"
                | "mercado"
                | "resultado"
                | "cuidado"
                | "soldado"
                | "abogado"
                | "delegado"
                | "pecado"
                | "partido"
                | "apellido"
                | "sentido"
                | "sonido"
                | "ruido"
                | "vestido"
                | "marido"
                | "contenido"
                | "significado"
        ) {
            return false;
        }

        // Si el recognizer confirma forma verbal, reforzar la confianza.
        if let Some(recognizer) = verb_recognizer {
            return Self::recognizer_is_valid_verb_form(word, recognizer);
        }

        true
    }

    /// Verifica forma verbal con recognizer y reintenta con forma normalizada
    /// sin tildes para cubrir casos como "continúa" -> "continua".
    fn recognizer_is_valid_verb_form(word: &str, recognizer: &dyn VerbFormRecognizer) -> bool {
        if recognizer.is_valid_verb_form(word) {
            return true;
        }

        let normalized = Self::normalize_spanish(word);
        normalized != word && recognizer.is_valid_verb_form(&normalized)
    }

    fn normalize_spanish(word: &str) -> String {
        word.to_lowercase()
            .chars()
            .map(|c| match c {
                'á' | 'à' | 'ä' | 'â' => 'a',
                'é' | 'è' | 'ë' | 'ê' => 'e',
                'í' | 'ì' | 'ï' | 'î' => 'i',
                'ó' | 'ò' | 'ö' | 'ô' => 'o',
                'ú' | 'ù' | 'ü' | 'û' => 'u',
                'ñ' => 'n',
                _ => c,
            })
            .collect()
    }

    /// Verifica si es un verbo común conjugado (restrictivo, para evitar falsos positivos)
    fn is_common_verb(word: &str) -> bool {
        // Solo verbos muy comunes en tercera persona que claramente indican "él + verbo"
        matches!(
            word,
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
            "cáncer",
            "cancer",
            "líder",
            "lider",
            "taller",
            "alfiler",
            "carácter",
            "caracter",
            "cadáver",
            "cadaver",
            "esfínter",
            "esfinter",
            "máster",
            "master",
            "póster",
            "poster",
            "súper",
            "super",
            "hámster",
            "hamster",
            "bunker",
            "búnker",
            "láser",
            "laser",
            "cráter",
            "crater",
            "éter",
            "eter",
            "mártir",
            // Sustantivos en -ar
            "hogar",
            "lugar",
            "azúcar",
            "azucar",
            "altar",
            "avatar",
            "bar",
            "bazar",
            "collar",
            "dólar",
            "dolar",
            "ejemplar",
            "hangar",
            "militar",
            "pilar",
            "radar",
            "solar",
            "titular",
            "angular",
            "celular",
            "familiar",
            "nuclear",
            "particular",
            "popular",
            "regular",
            "secular",
            "similar",
            "singular",
            "vulgar",
            // Sustantivos en -ir
            "elixir",
            "nadir",
            "faquir",
            "tapir",
            "yogur",
            // Preposiciones (terminan en -e pero no son verbos)
            "sobre",
            "ante",
            "entre",
            "desde",
            "durante",
            "mediante",
            // Otras palabras comunes que no son verbos
            "posible",
            "probable",
            "grande",
            "siempre",
            "entonces",
            "mientras",
            "donde",
            "adonde",
            "aunque",
            "porque",
            "parte",
            // Sustantivos que terminan en -ido/-ado (parecen participios pero son sustantivos)
            "sentido",
            "sonido",
            "ruido",
            "vestido",
            "marido",
            "partido",
            "apellido",
            "contenido",
            "significado",
            "mercado",
            "estado",
            "lado",
            "grado",
            "pasado",
            "cuidado",
            "resultado",
            "soldado",
            "abogado",
            "delegado",
            "pecado",
        ];
        if non_verb_nouns.contains(&word) {
            return false;
        }

        let len = word.len();
        // Terminaciones verbales comunes y poco ambiguas
        // Infinitivos (solo si tienen longitud mínima y no tienen tilde en raíz)
        let has_accent_in_root = word
            .chars()
            .take(word.len().saturating_sub(2))
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
        matches!(
            word,
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
    /// Usado para detectar "tu canto" �?' "tú cantas" donde "canto" es verbo mal conjugado
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
            "soy",
            "voy",
            "doy",
            "estoy",
            "hago",
            "tengo",
            "vengo",
            "pongo",
            "salgo",
            "traigo",
            "digo",
            "oigo",
            "caigo",
            "conozco",
            "parezco",
            "nazco",
            "crezco",
            "agradezco",
            "ofrezco",
            "produzco",
            "conduzco",
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
        if lower.ends_with("ario")
            || lower.ends_with("orio")
            || lower.ends_with("erio")
            || lower.ends_with("uario")
        {
            return false;
        }
        // Excluir sustantivos muy comunes que terminan en -o
        let common_nouns_in_o = [
            "libro",
            "tiempo",
            "trabajo",
            "cuerpo",
            "mundo",
            "pueblo",
            "grupo",
            "medio",
            "centro",
            "punto",
            "caso",
            "modo",
            "tipo",
            "lado",
            "fondo",
            "hecho",
            "derecho",
            "gobierno",
            "desarrollo",
            "proceso",
            "servicio",
            "precio",
            "espacio",
            "campo",
            "proyecto",
            "número",
            "periodo",
            "periodo",
            "cuento",
            "viento",
            "cielo",
            "suelo",
            "pelo",
            "dedo",
            "brazo",
            "cuello",
            "pecho",
            "ojo",
            "labio",
            "hueso",
            "nervio",
            "músculo",
            "órgano",
            "banco",
            "barco",
            "carro",
            "auto",
            "vuelo",
            "juego",
            "fuego",
            "riesgo",
            "cargo",
            "pago",
            "gasto",
            "cambio",
            "inicio",
            "término",
            "acuerdo",
            "resto",
            "texto",
            "éxito",
            "motivo",
            "objetivo",
            "efecto",
            "aspecto",
            "elemento",
            "momento",
            "movimiento",
            "sentimiento",
            "pensamiento",
            "alimento",
            "aumento",
            "instrumento",
            "documento",
            "argumento",
            "tratamiento",
            "procedimiento",
            "conocimiento",
            "acontecimiento",
            "crecimiento",
            "nacimiento",
            "sufrimiento",
            "comportamiento",
            // Adjetivos/determinantes que terminan en -o
            "otro",
            "mismo",
            "todo",
            "poco",
            "mucho",
            "tanto",
            "cuanto",
            "primero",
            "segundo",
            "tercero",
            "cuarto",
            "quinto",
            "último",
            "cierto",
            "propio",
            "solo",
            "nuevo",
            "antiguo",
            "largo",
            "corto",
            "alto",
            "bajo",
            "ancho",
            "negro",
            "blanco",
            "rojo",
            "claro",
            "oscuro",
            // Otros
            "euro",
            "metro",
            "litro",
            "kilo",
            "grado",
            "minuto",
            "segundo",
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
        matches!(
            word,
            "también"
                | "tampoco"
                | "siempre"
                | "nunca"
                | "jamás"
                | "ya"
                | "todavía"
                | "aún"
                | "apenas"
                | "solo"
                | "sólo"
                | "bien"
                | "mal"
                | "mejor"
                | "peor"
                | "mucho"
                | "poco"
                | "muy"
                | "bastante"
                | "demasiado"
                | "casi"
                | "realmente"
                | "probablemente"
                | "seguramente"
                | "ciertamente"
                | "obviamente"
                | "quizá"
                | "quiza"
                | "quizás"
                | "quizas"
                | "acaso"
        )
    }

    /// Heurística amplia de adverbio para el patrón "tu + adverbio + verbo".
    fn is_likely_adverb(word: &str) -> bool {
        let w = Self::normalize_spanish(word);
        if w.ends_with("mente") {
            return true;
        }
        matches!(
            w.as_str(),
            "ademas"
                | "anoche"
                | "enseguida"
                | "deprisa"
                | "cerca"
                | "lejos"
                | "delante"
                | "detras"
                | "siquiera"
                | "recien"
                | "inclusive"
                | "manana"
                | "hoy"
                | "ayer"
                | "ahora"
                | "antes"
                | "despues"
                | "entonces"
                | "aqui"
                | "ahi"
                | "alli"
                | "alla"
                | "aca"
                | "afuera"
                | "adentro"
                | "encima"
                | "debajo"
        )
    }

    /// Verifica si es palabra interrogativa/exclamativa
    fn is_interrogative(word: &str) -> bool {
        matches!(
            word,
            "qué"
                | "que"
                | "quién"
                | "quien"
                | "quiénes"
                | "quienes"
                | "cuál"
                | "cual"
                | "cuáles"
                | "cuales"
                | "cómo"
                | "como"
                | "cu\u{00E1}ndo"
                | "cuando"
                | "cu\u{00E1}nto"
                | "cuanto"
                | "cu\u{00E1}nta"
                | "cuanta"
                | "cu\u{00E1}ntos"
                | "cuantos"
                | "cu\u{00E1}ntas"
                | "cuantas"
                | "dónde"
                | "donde"
                | "adónde"
                | "adonde"
        )
    }

    fn is_negative_indefinite_following_saber(word: &str) -> bool {
        matches!(word, "nada" | "nadie" | "nunca" | "jamas" | "tampoco")
    }

    fn is_no_ya_interrogative_saber(
        next_word: &str,
        next_next_word: Option<&str>,
        next_third_word: Option<&str>,
    ) -> bool {
        if next_word == "que" || next_word == "si" || Self::is_interrogative(next_word) {
            return true;
        }

        // "no se lo que..." -> "no sé lo que..."
        if next_word == "lo" && next_next_word == Some("que") {
            return true;
        }

        // "no se de quien...", "no se con cual...", "no se por donde..."
        if Self::is_preposition(next_word) && next_next_word.is_some_and(Self::is_interrogative) {
            return true;
        }

        // "no se acerca de que..." / "no se respecto a cual..."
        if matches!(next_word, "acerca" | "respecto") {
            let valid_bridge = (next_word == "acerca" && next_next_word == Some("de"))
                || (next_word == "respecto" && next_next_word == Some("a"));
            if valid_bridge && next_third_word.is_some_and(Self::is_interrogative) {
                return true;
            }
        }

        false
    }

    fn is_discourse_connector(word: &str) -> bool {
        matches!(
            word,
            "pero" | "y" | "e" | "ni" | "sino" | "aunque" | "pues" | "porque"
        )
    }

    fn is_saber_nonverbal_complement(word: &str) -> bool {
        matches!(
            word,
            "mucho"
                | "poco"
                | "bastante"
                | "demasiado"
                | "algo"
                | "bien"
                | "mal"
                | "mejor"
                | "peor"
        )
    }

    fn is_saber_modifier_before_indefinite(word: &str) -> bool {
        matches!(
            word,
            "casi" | "absolutamente" | "practicamente" | "realmente"
        )
    }

    fn is_locative_adverb_for_nadar_ambiguity(word: &str) -> bool {
        matches!(
            word,
            "aqui" | "ahi" | "alli" | "alla" | "aca" | "adentro" | "afuera"
        )
    }

    fn is_impersonal_locative_quantity_tail(
        next_next: Option<&str>,
        next_third: Option<&str>,
        next_fourth: Option<&str>,
    ) -> bool {
        if next_next != Some("en") {
            return false;
        }
        let Some(w3) = next_third else {
            return false;
        };
        if Self::is_article(w3) {
            return next_fourth.is_some_and(Self::is_likely_swimming_location_noun);
        }
        Self::is_likely_swimming_location_noun(w3)
    }

    fn is_likely_swimming_location_noun(word: &str) -> bool {
        matches!(
            word,
            "piscina" | "alberca" | "pileta" | "mar" | "playa" | "rio" | "lago" | "agua" | "oceano"
        )
    }

    fn is_likely_saber_de_complement(word: &str) -> bool {
        // "no se nada de X" suele ser "no sé nada de X" (desconocimiento),
        // excepto en expresiones típicas de nado: "nadar de espaldas/de braza/...".
        !matches!(
            word,
            "espalda" | "espaldas" | "braza" | "crol" | "crawl" | "mariposa" | "pecho" | "perrito"
        )
    }

    /// Detecta "si," afirmativo al inicio de oración.
    fn is_affirmative_si_with_comma(tokens: &[Token], token_idx: usize) -> bool {
        if token_idx >= tokens.len() {
            return false;
        }
        if !Self::is_sentence_start_position(tokens, token_idx) {
            return false;
        }

        let mut next_idx = token_idx + 1;
        while next_idx < tokens.len() && tokens[next_idx].token_type == TokenType::Whitespace {
            next_idx += 1;
        }

        next_idx < tokens.len()
            && tokens[next_idx].token_type == TokenType::Punctuation
            && tokens[next_idx].text == ","
    }

    /// Detecta "eso/esto/aquello si," con valor enfático afirmativo.
    /// Ej.: "Eso si, es verdad" -> "Eso sí, ..."
    fn is_demonstrative_affirmative_si_with_comma(tokens: &[Token], token_idx: usize) -> bool {
        if token_idx >= tokens.len() {
            return false;
        }

        let mut next_idx = token_idx + 1;
        while next_idx < tokens.len() && tokens[next_idx].token_type == TokenType::Whitespace {
            next_idx += 1;
        }
        if !(next_idx < tokens.len()
            && tokens[next_idx].token_type == TokenType::Punctuation
            && tokens[next_idx].text == ",")
        {
            return false;
        }

        if token_idx == 0 {
            return false;
        }

        for idx in (0..token_idx).rev() {
            let token = &tokens[idx];
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.token_type != TokenType::Word {
                return false;
            }
            let prev = token.text.to_lowercase();
            return matches!(prev.as_str(), "eso" | "esto" | "aquello");
        }

        false
    }

    /// Detecta "pues/bueno/claro si," como marcador discursivo afirmativo.
    /// Ej.: "Pues si, claro..." -> "Pues sí, claro..."
    fn is_discourse_marker_affirmative_si_with_comma(tokens: &[Token], token_idx: usize) -> bool {
        if token_idx >= tokens.len() {
            return false;
        }

        let mut next_idx = token_idx + 1;
        while next_idx < tokens.len() && tokens[next_idx].token_type == TokenType::Whitespace {
            next_idx += 1;
        }
        if !(next_idx < tokens.len()
            && tokens[next_idx].token_type == TokenType::Punctuation
            && tokens[next_idx].text == ",")
        {
            return false;
        }

        // Buscar marcador anterior a "si", permitiendo coma parentética:
        // "Bueno, si, ..." / "Pues si, ..."
        let mut prev_word_idx: Option<usize> = None;
        let mut saw_preceding_comma = false;
        for idx in (0..token_idx).rev() {
            let token = &tokens[idx];
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.token_type == TokenType::Punctuation
                && token.text == ","
                && !saw_preceding_comma
            {
                saw_preceding_comma = true;
                continue;
            }
            if token.token_type != TokenType::Word {
                return false;
            }
            prev_word_idx = Some(idx);
            break;
        }

        let Some(prev_idx) = prev_word_idx else {
            return false;
        };
        let prev_word = tokens[prev_idx].text.to_lowercase();
        if !matches!(prev_word.as_str(), "pues" | "bueno" | "claro") {
            return false;
        }

        // Debe estar al inicio de oración o tras puntuación/fuente de corte.
        for idx in (0..prev_idx).rev() {
            let token = &tokens[idx];
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.token_type == TokenType::Punctuation {
                return token.is_sentence_boundary() || token.text == ",";
            }
            return false;
        }

        true
    }

    /// Verifica si el token está en inicio de oración,
    /// permitiendo signos de apertura antes de la palabra.
    fn is_sentence_start_position(tokens: &[Token], token_idx: usize) -> bool {
        if token_idx == 0 {
            return true;
        }

        for idx in (0..token_idx).rev() {
            let token = &tokens[idx];
            if token.token_type == TokenType::Whitespace {
                continue;
            }

            if token.token_type == TokenType::Punctuation {
                if token.is_sentence_boundary() {
                    return true;
                }
                if matches!(
                    token.text.as_str(),
                    "¿" | "¡" | "\"" | "'" | "«" | "(" | "[" | "{"
                ) {
                    continue;
                }
            }

            return false;
        }

        true
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

    /// Verifica formas frecuentes de 1a persona del preterito simple.
    /// Ayuda a detectar patrones cliticos "se lo + verbo" sin VerbRecognizer.
    fn is_first_person_preterite_form(word: &str) -> bool {
        let lower = word.to_lowercase();
        if lower.len() > 2 && (lower.ends_with("é") || lower.ends_with("í")) {
            return true;
        }
        matches!(
            lower.as_str(),
            "dije"
                | "hice"
                | "puse"
                | "quise"
                | "supe"
                | "traje"
                | "conduje"
                | "produje"
                | "anduve"
                | "estuve"
                | "tuve"
                | "fui"
                | "vi"
                | "di"
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
            return true; // implementó, terminó, hizo, dijo
        }
        if word.ends_with("aron") || word.ends_with("ieron") || word.ends_with("yeron") {
            return true; // implementaron, hicieron, dijeron
        }

        // Presente indicativo (3ª persona singular): verbos comunes
        if matches!(
            word,
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
        if (word.ends_with("ará") || word.ends_with("erá") || word.ends_with("irá")) && len >= 5
        {
            return true; // implementará, podrá, hará
        }
        if (word.ends_with("arán") || word.ends_with("erán") || word.ends_with("irán")) && len >= 6
        {
            return true; // implementarán, podrán, harán
        }

        // Condicional (3ª persona): -ía, -ían (pero cuidado con imperfecto -ía)
        // No usar terminación genérica -ía porque es ambigua

        // Subjuntivo presente (3ª persona): -e, -en (para -ar), -a, -an (para -er/-ir)
        // Solo verbos específicos porque -e/-a son muy ambiguas
        if matches!(
            word,
            "pueda"
                | "puedan"
                | "deba"
                | "deban"
                | "haga"
                | "hagan"
                | "diga"
                | "digan"
                | "sepa"
                | "sepan"
                | "vea"
                | "vean"
                | "tenga"
                | "tengan"
                | "quiera"
                | "quieran"
        ) {
            return true;
        }

        false
    }

    /// Verifica si la palabra siguiente forma una locución adverbial con "de"
    /// Usado para evitar corregir "de" a "dé" en frases como "de verdad", "de nuevo"
    fn is_adverbial_phrase_with_de(next_word: &str) -> bool {
        matches!(
            next_word,
            // Locuciones adverbiales muy comunes
            "verdad"
                | "veras"
                | "nuevo"
                | "pronto"
                | "repente"
                | "hecho"
                | "forma"
                | "manera"
                | "modo"
                | "golpe"
                | "momento"
                | "inmediato"
                | "improviso"
                | "súbito"
                | "sobra"
                | "sobras"
                | "acuerdo"
                | "antemano"
                | "memoria"
                | "corazón"
                | "cabeza"
                | "frente"
                | "espaldas"
                | "lado"
                | "cerca"
                | "lejos"
                | "más"
                | "menos"
                | "vez"
                | "veces"
                | "día"
                | "noche"
                | "madrugada"
                | "mañana"
                | "tarde"
                | "paso"
                | "camino"
                | "vuelta"
                | "regreso"
                | "ida"
                | "pie"
                | "rodillas"
                | "puntillas"
                | "bruces"
        )
    }

    /// Verifica si una palabra parece ser verbo conjugado (para detectar sí enfático)
    /// Usado en "la imagen sí pasa" donde "sí" es enfático antes de verbo
    fn is_likely_conjugated_verb(word: &str) -> bool {
        // Verbos comunes en tercera persona
        if matches!(
            word,
            "es" | "son"
                | "era"
                | "eran"
                | "fue"
                | "fueron"
                | "está"
                | "están"
                | "estaba"
                | "estaban"
                | "tiene"
                | "tienen"
                | "tenía"
                | "tenían"
                | "hace"
                | "hacen"
                | "hacía"
                | "hacían"
                | "hizo"
                | "hicieron"
                | "va"
                | "van"
                | "iba"
                | "iban"
                | "puede"
                | "pueden"
                | "podía"
                | "podían"
                | "pudo"
                | "pudieron"
                | "quiere"
                | "quieren"
                | "quería"
                | "querían"
                | "viene"
                | "vienen"
                | "venía"
                | "venían"
                | "vino"
                | "vinieron"
                | "sale"
                | "salen"
                | "salía"
                | "salían"
                | "salió"
                | "salieron"
                | "pasa"
                | "pasan"
                | "pasaba"
                | "pasaban"
                | "pasó"
                | "pasaron"
                | "llega"
                | "llegan"
                | "llegaba"
                | "llegaban"
                | "llegó"
                | "llegaron"
                | "funciona"
                | "funcionan"
                | "funcionaba"
                | "funcionaban"
                | "sirve"
                | "sirven"
                | "servía"
                | "servían"
                | "sirvió"
                | "sirvieron"
                | "sigue"
                | "siguen"
                | "seguía"
                | "seguían"
                | "siguió"
                | "siguieron"
                | "parece"
                | "parecen"
                | "parecía"
                | "parecían"
                | "pareció"
                | "parecieron"
                | "cree"
                | "creen"
                | "creía"
                | "creían"
                | "creyó"
                | "creyeron"
                | "piensa"
                | "piensan"
                | "pensaba"
                | "pensaban"
                | "pensó"
                | "pensaron"
                | "siente"
                | "sienten"
                | "sentía"
                | "sentían"
                | "sintió"
                | "sintieron"
                | "queda"
                | "quedan"
                | "quedaba"
                | "quedaban"
                | "quedó"
                | "quedaron"
                | "falta"
                | "faltan"
                | "faltaba"
                | "faltaban"
                | "faltó"
                | "faltaron"
                | "importa"
                | "importan"
                | "importaba"
                | "importaban"
                | "necesita"
                | "necesitan"
                | "necesitaba"
                | "necesitaban"
                | "conviene"
                | "convenía"
                | "convino"
                | "basta"
                | "bastan"
                | "bastaba"
                | "bastaban"
                | "bastó"
                | "existe"
                | "existen"
                | "existía"
                | "existían"
                | "sabe"
                | "saben"
                | "sabía"
                | "sabían"
                | "supo"
                | "supieron"
                | "ve"
                | "ven"
                | "veía"
                | "veían"
                | "vio"
                | "vieron"
                | "da"
                | "dan"
                | "daba"
                | "daban"
                | "dio"
                | "dieron"
                | "dice"
                | "dicen"
                | "decía"
                | "decían"
                | "dijo"
                | "dijeron"
                | "hay"
                | "había"
                | "hubo"
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

    /// Heurística conservadora para detectar subjuntivo imperfecto tras "si":
    /// evita false positives de "sí" enfático en condicionales ("si pudiera...").
    fn is_likely_imperfect_subjunctive_form(word: &str) -> bool {
        let norm = Self::normalize_spanish(word);
        let len = norm.len();
        if len < 4 {
            return false;
        }

        // Formas frecuentes irregulares
        if matches!(
            norm.as_str(),
            "fuera"
                | "fueras"
                | "fuéramos"
                | "fueramos"
                | "fuerais"
                | "fueran"
                | "fuese"
                | "fueses"
                | "fuésemos"
                | "fuesemos"
                | "fueseis"
                | "fuesen"
                | "hubiera"
                | "hubieras"
                | "hubiéramos"
                | "hubieramos"
                | "hubierais"
                | "hubieran"
                | "hubiese"
                | "hubieses"
                | "hubiésemos"
                | "hubiesemos"
                | "hubieseis"
                | "hubiesen"
        ) {
            return true;
        }

        // Terminaciones regulares de subjuntivo imperfecto
        norm.ends_with("ra")
            || norm.ends_with("ras")
            || norm.ends_with("ramos")
            || norm.ends_with("rais")
            || norm.ends_with("ran")
            || norm.ends_with("se")
            || norm.ends_with("ses")
            || norm.ends_with("semos")
            || norm.ends_with("seis")
            || norm.ends_with("sen")
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
               word.ends_with("encia") || word.ends_with("ancia")
            // ciencia, instancia
            {
                return true;
            }
        }
        // Sustantivos muy comunes que pueden seguir a "el/la mismo/a"
        matches!(
            word,
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

    /// Heurística para distinguir "el mismo + SN" de "él mismo".
    /// Si la palabra tras "mismo/a" parece nominal, tratamos "el" como artículo.
    fn is_nominal_after_mismo(
        word: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if !word.chars().any(|c| c.is_alphabetic()) {
            return false;
        }

        if Self::is_common_noun_for_mismo(word) || Self::is_likely_noun_or_adj(word) {
            return true;
        }

        let lower = word.to_lowercase();

        if Self::is_clitic_pronoun(word)
            || Self::is_article(word)
            || Self::is_preposition(word)
            || Self::is_interrogative(word)
            || matches!(
                word,
                "que"
                    | "quien"
                    | "quienes"
                    | "y"
                    | "e"
                    | "o"
                    | "u"
                    | "ni"
                    | "pero"
                    | "aunque"
                    | "porque"
                    | "como"
                    | "cuando"
                    | "donde"
                    | "adonde"
                    | "si"
                    | "sí"
            )
        {
            return false;
        }

        // Tras "mismo/a", las formas en -o/-a suelen ser núcleo nominal ("el mismo trato").
        // Evitar solo verbos muy claros ("él mismo vino/hizo...").
        if lower.len() >= 4
            && (lower.ends_with('o')
                || lower.ends_with('a')
                || lower.ends_with("os")
                || lower.ends_with("as"))
            && !Self::is_common_verb(word)
        {
            return true;
        }

        if Self::is_likely_verb_word(word, verb_recognizer) {
            return false;
        }
        if lower.ends_with("ción")
            || lower.ends_with("sión")
            || lower.ends_with("dad")
            || lower.ends_with("tad")
            || lower.ends_with("ez")
            || lower.ends_with("eza")
            || lower.ends_with("ura")
            || lower.ends_with("aje")
            || lower.ends_with("miento")
            || lower.ends_with("ncia")
            || lower.ends_with("ismo")
            || lower.ends_with("ista")
            || lower.ends_with("or")
            || lower.ends_with("ora")
            || lower.ends_with("ero")
            || lower.ends_with("era")
        {
            return true;
        }

        false
    }

    /// Verifica si es sustantivo común que puede seguir a "mi" posesivo
    fn is_common_noun_after_mi(word: &str) -> bool {
        matches!(
            word,
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
        let has_alpha = original.chars().any(|c| c.is_alphabetic());
        let is_all_caps = has_alpha
            && original
                .chars()
                .all(|c| !c.is_alphabetic() || c.is_uppercase());

        if is_all_caps {
            return replacement.to_uppercase();
        }

        if original
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
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
    fn test_para_el_es_dificil_needs_accent() {
        let corrections = analyze_text("para el es dificil");
        assert!(corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("el") && c.suggestion.to_lowercase() == "él"
        }));
    }

    #[test]
    fn test_el_no_sabia_que_hacer_detects_el_and_sabia() {
        let corrections = analyze_text("el no sabia que hacer");
        assert!(corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("el") && c.suggestion.to_lowercase() == "él"
        }));
        assert!(corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("sabia") && c.suggestion.to_lowercase() == "sabía"
        }));
    }

    #[test]
    fn test_el_mismo_plus_noun_no_false_positive() {
        let samples = [
            "Los estudiantes obtienen el mismo título",
            "Le dieron el mismo trato",
            "Reciben el mismo sueldo",
        ];

        for text in samples {
            let corrections = analyze_text(text);
            let el_corrections: Vec<_> = corrections
                .iter()
                .filter(|c| {
                    c.original.eq_ignore_ascii_case("el") && c.suggestion.to_lowercase() == "él"
                })
                .collect();
            assert!(
                el_corrections.is_empty(),
                "No debe corregir 'el mismo + sustantivo' en '{}': {:?}",
                text,
                corrections
            );
        }
    }

    #[test]
    fn test_el_before_nominal_head_after_preposition_no_false_positive() {
        let samples = ["para el partido", "segun el informe", "con el resultado"];
        for text in samples {
            let corrections = analyze_text(text);
            let el_corrections: Vec<_> = corrections
                .iter()
                .filter(|c| {
                    c.original.eq_ignore_ascii_case("el") && c.suggestion.to_lowercase() == "él"
                })
                .collect();
            assert!(
                el_corrections.is_empty(),
                "No debe corregir 'el' como pronombre antes de núcleo nominal: '{}': {:?}",
                text,
                corrections
            );
        }
    }

    #[test]
    fn test_el_mismo_pronoun_still_corrects() {
        let corrections = analyze_text("el mismo lo hizo");
        let el_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| {
                c.original.eq_ignore_ascii_case("el") && c.suggestion.to_lowercase() == "él"
            })
            .collect();

        assert_eq!(
            el_corrections.len(),
            1,
            "Debe corregir 'el mismo lo hizo' como pronombre enfático: {:?}",
            corrections
        );
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
    fn test_all_caps_pronoun_not_corrected_in_mixed_case() {
        // "TU" en texto mixto suele ser sigla, no pronombre
        let corrections = analyze_text("TU renunció a su cargo");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'TU' en texto mixto"
        );
    }

    #[test]
    fn test_all_caps_sentence_still_corrects_pronoun() {
        // En texto completamente en mayúsculas, sí corregimos diacríticas
        let corrections = analyze_text("TU CANTAS MUY BIEN");
        let tu_corrections: Vec<_> = corrections.iter().filter(|c| c.original == "TU").collect();
        assert_eq!(tu_corrections.len(), 1);
        assert_eq!(tu_corrections[0].suggestion, "T\u{00DA}");
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
        // "a mi me gusta" �?' "a mí me gusta" (mi seguido de clítico = pronombre tónico)
        let corrections = analyze_text("a mi me gusta");
        let mi_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "mi")
            .collect();
        assert_eq!(
            mi_corrections.len(),
            1,
            "Debería corregir 'mi' antes de clítico"
        );
        assert_eq!(mi_corrections[0].suggestion, "mí");
    }

    #[test]
    fn test_mi_before_verb_needs_accent() {
        // "para mi es importante" �?' "para mí es importante" (mi seguido de verbo = pronombre tónico)
        let corrections = analyze_text("para mi es importante");
        let mi_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "mi")
            .collect();
        assert_eq!(
            mi_corrections.len(),
            1,
            "Debería corregir 'mi' antes de verbo"
        );
        assert_eq!(mi_corrections[0].suggestion, "mí");
    }

    #[test]
    fn test_mi_with_accent_before_noun_corrected() {
        // "mí casa" �?' "mi casa" (mí con tilde seguido de sustantivo = incorrecto)
        let corrections = analyze_text("mí casa");
        let mi_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "mí")
            .collect();
        assert_eq!(
            mi_corrections.len(),
            1,
            "Debería corregir 'mí' antes de sustantivo"
        );
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
    fn test_tea_noun_with_accent_not_removed_in_common_contexts() {
        let cases = [
            "Quiero té",
            "Bebe té todos los días",
            "Prefiero té caliente",
            "Este té es delicioso",
            "Añade más té a la tetera",
        ];

        for text in cases {
            let corrections = analyze_text(text);
            let tea_corrections: Vec<_> = corrections
                .iter()
                .filter(|c| c.original.to_lowercase() == "té")
                .collect();
            assert!(
                tea_corrections.is_empty(),
                "No debe quitar tilde de 'té' en contexto nominal: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_te_with_accent_in_clear_pronoun_context_still_corrects() {
        let corrections = analyze_text("té quiero");
        let tea_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "té")
            .collect();
        assert_eq!(
            tea_corrections.len(),
            1,
            "Debe corregir 'té' a 'te' en contexto pronominal claro: {corrections:?}"
        );
        assert_eq!(tea_corrections[0].suggestion, "te");
    }

    #[test]
    fn test_te_plus_ambiguous_verb_with_recognizer_no_false_tea() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("Te apoyo en esta decisión");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let te_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "te")
            .collect();
        assert!(
            te_corrections.is_empty(),
            "No debe corregir 'Te apoyo...' a 'Té': {:?}",
            te_corrections
        );

        let tokens = tokenizer.tokenize("Te cuento un secreto importante");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let te_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "te")
            .collect();
        assert!(
            te_corrections.is_empty(),
            "No debe corregir 'Te cuento...' a 'Té': {:?}",
            te_corrections
        );
    }

    #[test]
    fn test_te_clitic_contexts_not_corrected_to_tea() {
        let cases = [
            "¿Cómo te va?",
            "Como te decía ayer",
            "Se te cayó el vaso",
            "Así como te dije",
            "No se te ocurra",
        ];

        for text in cases {
            let corrections = analyze_text(text);
            let te_corrections: Vec<_> = corrections
                .iter()
                .filter(|c| c.original.to_lowercase() == "te" && c.suggestion == "té")
                .collect();
            assert!(
                te_corrections.is_empty(),
                "No debe corregir 'te' a 'té' en contexto clítico: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_se_verb_needs_accent() {
        // "se" después de "yo" o "no" debería ser "sé"
        let corrections = analyze_text("yo se la respuesta");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_yo_se_lo_clitic_no_correction() {
        let corrections = analyze_text("yo se lo dije claramente");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir 'se' en patron clitico 'se lo + verbo'"
        );

        let corrections = analyze_text("yo se lo pedí ayer");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir 'se' en patron clitico 'se lo + verbo'"
        );
    }

    #[test]
    fn test_yo_se_que_and_nadar_still_need_accent() {
        let corrections = analyze_text("yo se que es verdad");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");

        let corrections = analyze_text("yo se nadar");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_yo_se_lo_que_still_needs_accent() {
        let corrections = analyze_text("yo se lo que pasó");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_si_affirmative_needs_accent() {
        // "si" después de "que" (respuesta) debería ser "sí"
        let corrections = analyze_text("dijo que si");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sí");
    }

    #[test]
    fn test_eso_esto_si_conditional_no_accent() {
        let corrections = analyze_text("Har\u{00ED}a eso si tuviera tiempo");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert!(
            si_corrections.is_empty(),
            "No debe corregir 'si' condicional en 'haría eso si tuviera...': {:?}",
            si_corrections
        );

        let corrections = analyze_text("Comprar\u{00ED}a esto si pudiera");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert!(
            si_corrections.is_empty(),
            "No debe corregir 'si' condicional en 'compraría esto si pudiera': {:?}",
            si_corrections
        );
    }

    #[test]
    fn test_eso_si_que_affirmative_needs_accent() {
        let corrections = analyze_text("Eso si que es verdad");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert_eq!(
            si_corrections.len(),
            1,
            "Debe corregir 'Eso si que...' a 'Eso sí que...': {:?}",
            si_corrections
        );
        assert_eq!(si_corrections[0].suggestion, "s\u{00ED}");
    }

    #[test]
    fn test_eso_si_with_comma_affirmative_needs_accent() {
        let corrections = analyze_text("Eso si, es verdad");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert_eq!(
            si_corrections.len(),
            1,
            "Debe corregir 'Eso si, ...' a 'Eso sí, ...': {:?}",
            si_corrections
        );
        assert_eq!(si_corrections[0].suggestion, "s\u{00ED}");

        let corrections = analyze_text("Esto si, funciona");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert_eq!(
            si_corrections.len(),
            1,
            "Debe corregir 'Esto si, ...' a 'Esto sí, ...': {:?}",
            si_corrections
        );
        assert_eq!(si_corrections[0].suggestion, "s\u{00ED}");
    }

    #[test]
    fn test_discourse_marker_si_with_comma_affirmative_needs_accent() {
        let corrections = analyze_text("Pues si, claro que puedo");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert_eq!(
            si_corrections.len(),
            1,
            "Debe corregir 'Pues si, ...' a 'Pues sí, ...': {:?}",
            si_corrections
        );
        assert_eq!(si_corrections[0].suggestion, "s\u{00ED}");

        let corrections = analyze_text("Bueno, si, está bien");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert_eq!(
            si_corrections.len(),
            1,
            "Debe corregir 'Bueno, si, ...' a 'Bueno, sí, ...': {:?}",
            si_corrections
        );
        assert_eq!(si_corrections[0].suggestion, "s\u{00ED}");
    }

    #[test]
    fn test_mid_sentence_el_si_combined_needs_dual_accent() {
        let cases = ["pero el si sabe", "creo que el si puede"];

        for text in cases {
            let corrections = analyze_text(text);
            let el_correction = corrections
                .iter()
                .find(|c| c.original.to_lowercase() == "el" && c.suggestion == "él");
            let si_correction = corrections
                .iter()
                .find(|c| c.original.to_lowercase() == "si" && c.suggestion == "sí");
            assert!(
                el_correction.is_some(),
                "Debe corregir 'el' -> 'él' en: {text} -> {corrections:?}"
            );
            assert!(
                si_correction.is_some(),
                "Debe corregir 'si' -> 'sí' en: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_el_si_pudiera_conditional_si_not_accented() {
        let cases = ["el si pudiera", "pero el si pudiera", "creo que el si pudiera"];

        for text in cases {
            let corrections = analyze_text(text);
            let si_correction = corrections
                .iter()
                .find(|c| c.original.to_lowercase() == "si" && c.suggestion == "sí");
            assert!(
                si_correction.is_none(),
                "No debe corregir 'si' a 'sí' en condicional: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_y_si_with_comma_conditional_not_accented() {
        let corrections = analyze_text("Y si, tienes razón");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert!(
            si_corrections.is_empty(),
            "No debe corregir 'Y si, ...' por defecto: {:?}",
            si_corrections
        );
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
        let mas_corrections: Vec<_> = corrections.iter().filter(|c| c.original == "Mas").collect();
        assert!(
            mas_corrections.is_empty(),
            "No debe corregir 'Mas' a 'Más' cuando es apellido (patrón Nombre Apellido)"
        );
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
    fn test_aun_asi_and_aun_siendo_no_accent() {
        let corrections = analyze_text("Aun así, estoy bien");
        let aun_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert!(
            aun_corrections.is_empty(),
            "No debe corregir 'aun' en 'aun así': {:?}",
            aun_corrections
        );

        let corrections = analyze_text("Aun siendo difícil, lo logró");
        let aun_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert!(
            aun_corrections.is_empty(),
            "No debe corregir 'aun' en construcción concesiva con gerundio: {:?}",
            aun_corrections
        );
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
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir 'se' en pasiva refleja"
        );
    }

    #[test]
    fn test_se_reflexive_with_puede() {
        // "no se puede" es pasiva refleja
        let corrections = analyze_text("no se puede hacer");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir 'se' en 'no se puede'"
        );
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
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_connector_se_que_verb_saber() {
        let cases = [
            "pero se que tienes razon",
            "y se que tienes razon",
            "ni se que hacer",
            "aunque se que mientes",
            "pues se que no es facil",
            "porque se que dijo la verdad",
        ];
        for text in cases {
            let corrections = analyze_text(text);
            let se_corrections: Vec<_> = corrections
                .iter()
                .filter(|c| c.original.to_lowercase() == "se")
                .collect();
            assert_eq!(
                se_corrections.len(),
                1,
                "Debe corregir 'se' a 'sé' tras conector en: {text}"
            );
            assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
        }
    }

    #[test]
    fn test_connector_se_lo_que_verb_saber_and_clitic_guard() {
        let cases = [
            "pero se lo que pasa",
            "y se lo que hiciste",
            "ni se lo que buscas",
            "aunque se lo que diras",
            "pues se lo que quieres",
            "porque se lo que dijo",
        ];
        for text in cases {
            let corrections = analyze_text(text);
            let se_corrections: Vec<_> = corrections
                .iter()
                .filter(|c| c.original.to_lowercase() == "se")
                .collect();
            assert_eq!(
                se_corrections.len(),
                1,
                "Debe corregir 'se' a 'sé' en patron 'se lo que' tras conector: {text}"
            );
            assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
        }

        let corrections = analyze_text("pero se lo dije");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir clitico real tras conector en 'pero se lo dije'"
        );

        let corrections = analyze_text("porque se lo dije");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir clitico real tras conector en 'porque se lo dije'"
        );
    }

    #[test]
    fn test_no_se_lo_que_verb_saber() {
        let corrections = analyze_text("no se lo que pasa");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_se_lo_que_sentence_start_verb_saber() {
        let corrections = analyze_text("se lo que hiciste");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_la_sabia_mujer_not_corrected_as_saber() {
        let corrections = analyze_text("la sabia mujer hablo");
        let sabia_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "sabia")
            .collect();
        assert!(
            sabia_corrections.is_empty(),
            "No debe forzar 'sabia' adjetivo a 'sabía': {corrections:?}"
        );
    }

    #[test]
    fn test_no_se_como_hacer_verb_saber() {
        let corrections = analyze_text("no se como hacerlo");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_no_se_cuanto_cuesta_verb_saber() {
        let corrections = analyze_text("no se cuanto cuesta");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_no_se_por_que_vino_verb_saber() {
        let corrections = analyze_text("no se por que vino");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_no_se_por_donde_empezar_verb_saber() {
        let corrections = analyze_text("no se por donde empezar");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_no_se_de_quien_hablas_verb_saber() {
        let corrections = analyze_text("no se de quien hablas");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_no_se_con_quien_vino_verb_saber() {
        let corrections = analyze_text("no se con quien vino");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "s\u{00E9}");
    }

    #[test]
    fn test_yo_no_se_nada_verb_saber() {
        let corrections = analyze_text("yo no se nada");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_impersonal_not_forced_to_saber() {
        let corrections = analyze_text("no se nada en la piscina");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty());
    }

    #[test]
    fn test_no_se_nada_sentence_end_verb_saber() {
        let corrections = analyze_text("no se nada");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_with_discourse_connector_verb_saber() {
        let corrections = analyze_text("no se nada pero sigo");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_de_eso_verb_saber() {
        let corrections = analyze_text("no se nada de eso");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_de_espaldas_impersonal() {
        let corrections = analyze_text("no se nada de espaldas");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty());
    }

    #[test]
    fn test_no_se_mucho_verb_saber() {
        let corrections = analyze_text("no se mucho");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_yo_no_se_bien_verb_saber() {
        let corrections = analyze_text("yo no se bien");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_mucho_en_la_piscina_impersonal_not_forced() {
        let corrections = analyze_text("no se mucho en la piscina");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty());
    }

    #[test]
    fn test_no_se_poco_en_el_mar_impersonal_not_forced() {
        let corrections = analyze_text("no se poco en el mar");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty());
    }

    #[test]
    fn test_yo_no_se_mucho_en_la_piscina_still_saber() {
        let corrections = analyze_text("yo no se mucho en la piscina");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_casi_nada_still_saber() {
        let corrections = analyze_text("no se casi nada");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_absolutamente_nada_sobre_quimica_still_saber() {
        let corrections = analyze_text("no se absolutamente nada sobre química");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_aqui_no_se_casi_nada_en_la_piscina_impersonal() {
        let corrections = analyze_text("aquí no se casi nada en la piscina");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty());
    }

    #[test]
    fn test_aqui_no_se_nada_impersonal_not_forced_to_saber() {
        let corrections = analyze_text("aquí no se nada");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty());
    }

    #[test]
    fn test_aqui_no_se_nada_de_eso_still_saber() {
        let corrections = analyze_text("aquí no se nada de eso");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_de_quimica_still_saber() {
        let corrections = analyze_text("no se nada de química");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_de_braza_impersonal() {
        let corrections = analyze_text("no se nada de braza");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(se_corrections.is_empty());
    }

    #[test]
    fn test_no_se_nada_sobre_quimica_still_saber() {
        let corrections = analyze_text("no se nada sobre química");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_acerca_de_quimica_still_saber() {
        let corrections = analyze_text("no se nada acerca de química");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_no_se_nada_respecto_a_quimica_still_saber() {
        let corrections = analyze_text("no se nada respecto a química");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
    }

    #[test]
    fn test_a_ver_que_interrogative_needs_accent() {
        let corrections = analyze_text("a ver que pasa");
        let que_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "que")
            .collect();
        assert_eq!(que_corrections.len(), 1);
        assert_eq!(que_corrections[0].suggestion, "qué");
    }

    #[test]
    fn test_a_ver_cuando_interrogative_needs_accent() {
        let corrections = analyze_text("a ver cuando vienes");
        let cuando_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "cuando")
            .collect();
        assert_eq!(cuando_corrections.len(), 1);
        assert_eq!(cuando_corrections[0].suggestion, "cuándo");
    }

    #[test]
    fn test_haber_que_intro_interrogative_needs_accent() {
        let corrections = analyze_text("haber que pasa");
        let que_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "que")
            .collect();
        assert_eq!(que_corrections.len(), 1);
        assert_eq!(que_corrections[0].suggestion, "qué");
    }

    #[test]
    fn test_puede_haber_que_no_interrogative_accent() {
        let corrections = analyze_text("puede haber que esperar");
        let que_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "que")
            .collect();
        assert!(
            que_corrections.is_empty(),
            "No debe acentuar 'que' en uso verbal de 'haber': {:?}",
            que_corrections
        );
    }

    #[test]
    fn test_no_se_si_verb_saber() {
        // "no se si" -> "no sé si": "se" debe llevar tilde y "si" mantenerse condicional.
        let corrections = analyze_text("no se si");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();

        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
        assert!(
            si_corrections.is_empty(),
            "No debe corregir 'si' a 'sí' en 'no se si'"
        );
    }

    #[test]
    fn test_ya_se_si_verb_saber() {
        let corrections = analyze_text("ya se si");
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();

        assert_eq!(se_corrections.len(), 1);
        assert_eq!(se_corrections[0].suggestion, "sé");
        assert!(
            si_corrections.is_empty(),
            "No debe corregir 'si' a 'sí' en 'ya se si'"
        );
    }

    // ==========================================================================
    // Tests para "Sí" tras dos puntos
    // ==========================================================================

    #[test]
    fn test_si_after_colon_with_accent_no_correction() {
        // "Explicó: Sí, podemos" - Sí afirmativo tras : no debe corregirse
        let corrections = analyze_text("Explicó: Sí, podemos");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "sí")
            .collect();
        assert!(
            si_corrections.is_empty(),
            "No debe sugerir quitar tilde a 'Sí' tras ':': {:?}",
            si_corrections
        );
    }

    #[test]
    fn test_si_after_colon_without_accent_no_forced_correction() {
        // "Explicó: si quieres, ven" - si condicional tras : no debe forzarse a sí
        let corrections = analyze_text("Explicó: si quieres, ven");
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert!(
            si_corrections.is_empty(),
            "No debe forzar tilde en 'si' condicional tras ':': {:?}",
            si_corrections
        );
    }

    // ==========================================================================
    // Tests con VerbRecognizer (integración)
    // ==========================================================================

    #[test]
    fn test_se_trata_with_verb_recognizer() {
        // "No se trata..." - "trata" es forma verbal reconocida por VerbRecognizer
        // No debe corregir "se" a "sé"
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

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
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir 'se' en 'No se trata' con VerbRecognizer: {:?}",
            se_corrections
        );
    }

    #[test]
    fn test_se_dice_with_verb_recognizer() {
        // "No se dice así" - "dice" es forma verbal
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

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
        let se_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "se")
            .collect();
        assert!(
            se_corrections.is_empty(),
            "No debe corregir 'se' en 'No se dice' con VerbRecognizer: {:?}",
            se_corrections
        );
    }

    #[test]
    fn test_tu_cantas_with_verb_recognizer() {
        // "tu cantas" - "cantas" es verbo, debe sugerir "tú cantas"
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

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
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'tu' a 'tú' cuando va seguido de verbo: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_tu_mando_with_verb_recognizer() {
        // "tu mando aquí" - "mando" es verbo 1ª persona (no gerundio), debe sugerir "tú mando"
        // Con pista verbal ("aquí") para evitar ambigüedad posesiva
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("tu mando aquí");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(tu_corrections.len(), 1,
            "Debe corregir 'tu' a 'tú' cuando va seguido de verbo (mando = 1ª persona de mandar): {:?}", tu_corrections);
        assert_eq!(tu_corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_tu_pregunta_with_adverb_after_prep_no_false_positive() {
        // "Sobre tu pregunta ya/mañana respondo" -> "tu" es posesivo, no "tú".
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("Sobre tu pregunta ya respondo");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert!(
            tu_corrections.is_empty(),
            "No debe corregir 'tu' en 'Sobre tu pregunta ya respondo': {:?}",
            tu_corrections
        );

        let tokens = tokenizer.tokenize("Sobre tu pregunta mañana respondo");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert!(
            tu_corrections.is_empty(),
            "No debe corregir 'tu' en 'Sobre tu pregunta mañana respondo': {:?}",
            tu_corrections
        );
    }

    #[test]
    fn test_tu_no_verb_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("Tu no puedes hacer eso");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu no puedes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");

        let tokens = tokenizer.tokenize("Tu no sabes nada");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu no sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_tu_no_nominal_no_false_positive() {
        let corrections = analyze_text("tu no rotundo me sorprendió");
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert!(
            tu_corrections.is_empty(),
            "No debe corregir 'tu' posesivo en uso nominal de 'no': {:?}",
            tu_corrections
        );
    }

    #[test]
    fn test_tu_trabajas_with_verb_recognizer() {
        // "tu trabajas" - "trabajas" es verbo reconocido dinámicamente
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

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
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'tu' a 'tú' cuando va seguido de verbo: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_tu_quiza_sabes_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Tu quizá sabes la respuesta");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu quizá sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_tu_tal_vez_sabes_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Tu tal vez sabes la respuesta");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu tal vez sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_tu_adverb_mente_plus_verb_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Tu claramente sabes la respuesta");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu claramente sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_tu_temporal_adverb_plus_verb_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Tu ahora sabes la verdad");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu ahora sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_tu_adverb_no_plus_verb_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("Tu claramente no sabes la respuesta");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu claramente no sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");

        let tokens = tokenizer.tokenize("Tu ahora no quieres hablar");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu ahora no quieres...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_tu_nominal_with_adverb_no_plus_verb_after_prep_no_false_positive() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("Sobre tu pregunta claramente no respondo");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert!(
            tu_corrections.is_empty(),
            "No debe corregir 'tu' posesivo en 'Sobre tu pregunta claramente no respondo': {:?}",
            tu_corrections
        );
    }

    #[test]
    fn test_tu_a_participle_detected_as_pronoun() {
        let corrections = analyze_text("tu a venido tarde");
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'tu' a 'tú' en patrón 'tu a + participio': {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "tú");
    }

    #[test]
    fn test_tu_a_nominal_not_pronoun() {
        let corrections = analyze_text("tu a lado derecho");
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert!(
            tu_corrections.is_empty(),
            "No debe corregir 'tu' en patrón nominal 'tu a lado': {:?}",
            tu_corrections
        );
    }

    #[test]
    fn test_tu_ademas_sabes_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Tu además sabes la respuesta");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu además sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_tu_mejor_peor_plus_verb_with_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("Tu mejor sabes esto");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu mejor sabes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");

        let tokens = tokenizer.tokenize("Tu peor entiendes esto");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tu")
            .collect();
        assert_eq!(
            tu_corrections.len(),
            1,
            "Debe corregir 'Tu peor entiendes...' a pronombre tónico: {:?}",
            tu_corrections
        );
        assert_eq!(tu_corrections[0].suggestion, "Tú");
    }

    #[test]
    fn test_aun_continua_with_verb_recognizer() {
        // "aun continúa" - "continúa" es verbo, debe ser "aún continúa" (todavía)
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

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
        let aun_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert_eq!(
            aun_corrections.len(),
            1,
            "Debe corregir 'aun' a 'aún' cuando va seguido de verbo (todavía): {:?}",
            aun_corrections
        );
        assert_eq!(aun_corrections[0].suggestion.to_lowercase(), "aún");
    }

    #[test]
    fn test_aun_continua_accented_with_verb_recognizer() {
        // Cobertura adicional: forma acentuada "continúa".
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("aun continúa el problema");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let aun_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert_eq!(
            aun_corrections.len(),
            1,
            "Debe corregir 'aun' a 'aún' con verbo acentuado: {:?}",
            aun_corrections
        );
        assert_eq!(aun_corrections[0].suggestion.to_lowercase(), "aún");
    }

    #[test]
    fn test_aun_asi_and_aun_siendo_with_verb_recognizer_no_correction() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let tokenizer = Tokenizer::new();

        let tokens = tokenizer.tokenize("Aun así, estoy bien");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let aun_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert!(
            aun_corrections.is_empty(),
            "No debe corregir 'aun' en 'aun así' con VerbRecognizer: {:?}",
            aun_corrections
        );

        let tokens = tokenizer.tokenize("Aun siendo difícil, lo logró");
        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let aun_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert!(
            aun_corrections.is_empty(),
            "No debe corregir 'aun' en 'aun siendo' con VerbRecognizer: {:?}",
            aun_corrections
        );
    }

    #[test]
    fn test_eso_si_llueve_conditional_with_verb_recognizer_no_accent() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Eso si llueve nos quedamos");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert!(
            si_corrections.is_empty(),
            "No debe corregir 'si' condicional en 'Eso si llueve...': {:?}",
            si_corrections
        );
    }

    #[test]
    fn test_eso_si_continua_conditional_with_accented_verb_no_accent() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Eso si continúa nos vamos");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert!(
            si_corrections.is_empty(),
            "No debe corregir 'si' condicional con verbo acentuado en 'Eso si continúa...': {:?}",
            si_corrections
        );
    }

    #[test]
    fn test_aun_actua_with_accented_verb_recognizer() {
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Aun actúa con cautela");

        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let corrections = DiacriticAnalyzer::analyze(&tokens, Some(&verb_recognizer), None);
        let aun_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "aun")
            .collect();
        assert_eq!(
            aun_corrections.len(),
            1,
            "Debe corregir 'aun' a 'aún' con verbo acentuado 'actúa': {:?}",
            aun_corrections
        );
        assert_eq!(aun_corrections[0].suggestion.to_lowercase(), "aún");
    }

    #[test]
    fn test_si_enfatico_with_verb_recognizer() {
        // "él sí trabaja" - "sí" enfático antes de verbo
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

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
        let si_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "si")
            .collect();
        assert_eq!(
            si_corrections.len(),
            1,
            "Debe corregir 'si' a 'sí' (enfático) cuando pronombre + si + verbo: {:?}",
            si_corrections
        );
        assert_eq!(si_corrections[0].suggestion, "sí");
    }

    #[test]
    fn test_tu_with_accent_before_verb_protected() {
        // "tú trabajas" - ya tiene tilde, no debe sugerir quitarla
        use super::VerbRecognizer;
        use crate::dictionary::{DictionaryLoader, Trie};

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
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tú")
            .collect();
        assert!(
            tu_corrections.is_empty(),
            "No debe sugerir quitar tilde de 'tú' cuando va seguido de verbo: {:?}",
            tu_corrections
        );
    }

    #[test]
    fn test_tu_with_accent_before_a_participle_protected() {
        let corrections = analyze_text("tú a venido tarde");
        let tu_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "tú")
            .collect();
        assert!(
            tu_corrections.is_empty(),
            "No debe sugerir quitar tilde de 'tú' en patrón 'tú a + participio': {:?}",
            tu_corrections
        );
    }
}
