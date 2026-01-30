//! Corrección de tildes diacríticas
//!
//! Detecta y corrige pares de palabras que se distinguen por la tilde:
//! - el/él, tu/tú, mi/mí, te/té, se/sé, de/dé, si/sí, mas/más, aun/aún

use crate::grammar::Token;

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
    pub fn analyze(tokens: &[Token]) -> Vec<DiacriticCorrection> {
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
                        Self::check_diacritic(pair, tokens, &word_tokens, pos, *idx, token)
                    {
                        corrections.push(correction);
                    }
                    break;
                }
            }
        }

        corrections
    }

    /// Verifica si hay un límite de oración (. ! ?) entre dos índices de tokens
    fn has_sentence_boundary(tokens: &[Token], from_idx: usize, to_idx: usize) -> bool {
        for i in from_idx..to_idx {
            if let Some(token) = tokens.get(i) {
                if token.token_type == crate::grammar::TokenType::Punctuation {
                    let punct = &token.text;
                    if punct == "." || punct == "!" || punct == "?" || punct == "..." {
                        return true;
                    }
                }
            }
        }
        false
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

        // Obtener contexto: palabra anterior y siguiente
        // Si hay límite de oración entre la palabra anterior y la actual, tratarla como inicio de oración
        let prev_word = if pos > 0 {
            let prev_idx = word_tokens[pos - 1].0;
            // Verificar si hay un límite de oración entre la palabra anterior y la actual
            if Self::has_sentence_boundary(all_tokens, prev_idx, token_idx) {
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
        if pair.without_accent == "mi" && pair.with_accent == "mí" {
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

        // Determinar si necesita tilde basándose en el contexto
        let needs_accent = Self::needs_accent(pair, prev_word.as_deref(), next_word.as_deref(), next_next_word.as_deref());

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
    fn needs_accent(pair: &DiacriticPair, prev: Option<&str>, next: Option<&str>, next_next: Option<&str>) -> bool {
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
                    if Self::is_common_verb(next_word) || Self::is_verb_form(next_word) {
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
                        // Preposición + mi + sustantivo = posesivo (en mi lugar, por mi parte)
                        // Preposición + mi (final) = pronombre (para mí, a mí)
                        if let Some(next_word) = next {
                            // Si hay palabra siguiente, verificar si es sustantivo/adjetivo
                            // En ese caso es posesivo: "en mi lugar", "por mi parte"
                            if Self::is_likely_noun_or_adj(next_word) || Self::is_common_noun_after_mi(next_word) {
                                return false;  // Posesivo
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
                // Ejemplos: "se implementó", "no se puede", "ya se terminó"
                if let Some(next_word) = next {
                    if Self::is_conjugated_verb_for_se(next_word) {
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
                    // "que dé", "para que dé", "ojalá dé"
                    prev_word == "que" || prev_word == "ojalá" || prev_word == "quizá"
                } else {
                    false
                }
            }

            // si/sí
            ("si", "sí") => {
                // "sí" es afirmación, pronombre reflexivo (por sí mismo), o enfático (él sí vino)
                // "si" es conjunción condicional (si vienes..., como si fuera...)
                if let Some(prev_word) = prev {
                    // "como si" es construcción condicional, NO sí enfático
                    // "como si participaran", "como si fuera", "como si nada"
                    if prev_word == "como" {
                        return false;
                    }
                    // "dijo que sí", "por sí", "de por sí", "a sí mismo", "en sí", "eso sí"
                    if prev_word == "que" || prev_word == "por" || prev_word == "en" || prev_word == "a" || prev_word == "eso" {
                        return true;
                    }
                }

                if next.is_none() {
                    // "sí" solo al final (¿Vienes? Sí.)
                    return true;
                }

                if let Some(next_word) = next {
                    // "sí mismo/a" (reflexive)
                    if next_word == "mismo" || next_word == "misma" || next_word == "mismos" || next_word == "mismas" {
                        return true;
                    }
                    // Sí enfático seguido de verbo: "él sí vino", "la imagen sí pasa"
                    // Detectar si la siguiente palabra es un verbo conjugado
                    if Self::is_likely_conjugated_verb(next_word) {
                        return true;
                    }
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
                    // "aún + verbo común" = todavía (aún es, aún hay, aún está, aún queda)
                    if matches!(next_word, "es" | "son" | "era" | "eran" | "fue" | "fueron" |
                        "está" | "están" | "estaba" | "estaban" |
                        "hay" | "había" | "hubo" |
                        "queda" | "quedan" | "quedaba" | "quedaban" |
                        "falta" | "faltan" | "faltaba" | "faltaban" |
                        "tiene" | "tienen" | "tenía" | "tenían" |
                        "puede" | "pueden" | "podía" | "podían" |
                        "sigue" | "siguen" | "seguía" | "seguían" |
                        "existe" | "existen" | "existía" | "existían") {
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
            "autoriza" | "autorizan" | "autorizaba" | "autorizaban" | "autorizó" | "autorizaron"
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
            "trabajo" | "proyecto" | "plan" | "idea" | "concepto" | "principio"
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
        DiacriticAnalyzer::analyze(&tokens)
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
}
