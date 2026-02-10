//! Corrección de homófonos (ortonimia)
//!
//! Detecta y corrige confusiones entre palabras que suenan igual pero se escriben diferente:
//! - hay/ahí/ay
//! - haya/halla/aya
//! - vaya/valla/baya
//! - hecho/echo
//! - tuvo/tubo
//! - a ver/haber
//! - iba (no "iva")
//! - hierba/hierva
//! - bello/vello
//! - botar/votar

use crate::grammar::{has_sentence_boundary, Token, TokenType};

/// Correccion sugerida para homofonos
#[derive(Debug, Clone)]
pub struct HomophoneCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

/// Analizador de homofonos
pub struct HomophoneAnalyzer;

impl HomophoneAnalyzer {
    fn token_text_for_homophone(token: &Token) -> &str {
        // Para homófonos priorizamos correcciones gramaticales previas, pero
        // ignoramos sugerencias ortográficas (lista "a,b,c" o "?") para no
        // perder reglas contextuales como echo/hecho sobre palabras desconocidas.
        if let Some(ref correction) = token.corrected_grammar {
            if !correction.starts_with("falta")
                && !correction.starts_with("sobra")
                && correction != "desbalanceado"
            {
                return correction;
            }
        }
        token.text.as_str()
    }

    /// Analiza los tokens y detecta errores de homofonos
    pub fn analyze(tokens: &[Token]) -> Vec<HomophoneCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .collect();

        for (pos, (idx, token)) in word_tokens.iter().enumerate() {
            // Saltar palabras que probablemente son siglas o nombres propios
            // (todas mayúsculas como "AI", "IBM", "NASA")
            let original_text = Self::token_text_for_homophone(token);
            if original_text.len() >= 2 && original_text.chars().all(|c| c.is_uppercase()) {
                continue;
            }

            // Usar effective_text() para ver correcciones de fases anteriores
            let word_lower = original_text.to_lowercase();

            // Obtener contexto (tambien con effective_text)
            // Solo considerar palabra anterior si no hay limite de oracion entre ellas
            let prev_word = if pos > 0 {
                let prev_idx = word_tokens[pos - 1].0;
                if has_sentence_boundary(tokens, prev_idx, *idx) {
                    None
                } else {
                    Some(Self::token_text_for_homophone(word_tokens[pos - 1].1).to_lowercase())
                }
            } else {
                None
            };
            let prev_token = if pos > 0 {
                let prev_idx = word_tokens[pos - 1].0;
                if has_sentence_boundary(tokens, prev_idx, *idx) {
                    None
                } else {
                    Some(word_tokens[pos - 1].1)
                }
            } else {
                None
            };
            let prev_prev_word = if pos > 1 {
                let prev_prev_idx = word_tokens[pos - 2].0;
                if has_sentence_boundary(tokens, prev_prev_idx, *idx) {
                    None
                } else {
                    Some(Self::token_text_for_homophone(word_tokens[pos - 2].1).to_lowercase())
                }
            } else {
                None
            };
            let prev_third_word = if pos > 2 {
                let prev_third_idx = word_tokens[pos - 3].0;
                if has_sentence_boundary(tokens, prev_third_idx, *idx) {
                    None
                } else {
                    Some(Self::token_text_for_homophone(word_tokens[pos - 3].1).to_lowercase())
                }
            } else {
                None
            };
            let prev_prev_token = if pos > 1 {
                let prev_prev_idx = word_tokens[pos - 2].0;
                if has_sentence_boundary(tokens, prev_prev_idx, *idx) {
                    None
                } else {
                    Some(word_tokens[pos - 2].1)
                }
            } else {
                None
            };

            // Solo considerar palabra siguiente si no hay limite de oracion entre ellas
            let next_word = if pos + 1 < word_tokens.len() {
                let next_idx = word_tokens[pos + 1].0;
                if has_sentence_boundary(tokens, *idx, next_idx) {
                    None
                } else {
                    Some(Self::token_text_for_homophone(word_tokens[pos + 1].1).to_lowercase())
                }
            } else {
                None
            };
            let next_token = if pos + 1 < word_tokens.len() {
                let next_idx = word_tokens[pos + 1].0;
                if has_sentence_boundary(tokens, *idx, next_idx) {
                    None
                } else {
                    Some(word_tokens[pos + 1].1)
                }
            } else {
                None
            };

            // Segunda palabra siguiente (para detectar locuciones como "hecho de menos")
            let next_next_word = if pos + 2 < word_tokens.len() {
                let next_next_idx = word_tokens[pos + 2].0;
                if has_sentence_boundary(tokens, *idx, next_next_idx) {
                    None
                } else {
                    Some(Self::token_text_for_homophone(word_tokens[pos + 2].1).to_lowercase())
                }
            } else {
                None
            };

            // Verificar cada grupo de homófonos
            if let Some(correction) = Self::check_hay_ahi_ay(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_haya_halla(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                prev_prev_word.as_deref(),
                prev_third_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_a_ver_haber(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                prev_prev_word.as_deref(),
                next_word.as_deref(),
                next_next_word.as_deref(),
                prev_token,
                prev_prev_token,
                next_token,
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_por_que_family(
                &word_lower,
                *idx,
                token,
                pos,
                &word_tokens,
                tokens,
                prev_word.as_deref(),
                prev_prev_word.as_deref(),
                next_word.as_deref(),
                prev_token,
                prev_prev_token,
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_sino_si_no(
                &word_lower,
                *idx,
                token,
                pos,
                &word_tokens,
                tokens,
                prev_word.as_deref(),
                next_word.as_deref(),
                next_token,
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_vaya_valla(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_voy_boy(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_hecho_echo(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                prev_token,
                next_word.as_deref(),
                next_next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_tuvo_tubo(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_iba(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_hierba_hierva(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            } else if let Some(correction) =
                Self::check_bello_vello(&word_lower, *idx, token, prev_word.as_deref())
            {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_botar_votar(
                &word_lower,
                *idx,
                token,
                prev_word.as_deref(),
                next_word.as_deref(),
            ) {
                corrections.push(correction);
            }
        }

        corrections
    }

    /// hay (verbo haber) / ahí (adverbio lugar) / ay (interjección)
    fn check_hay_ahi_ay(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "hay" => {
                // "hay" es correcto cuando es verbo impersonal: "hay mucha gente"
                // Error común: usar "hay" en lugar de "ahí" (lugar)
                // Contexto: después de preposición de lugar suele ser "ahí"
                if let Some(p) = prev {
                    if matches!(p, "por" | "de" | "desde" | "hasta" | "hacia") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "ahí"),
                            reason: "Adverbio de lugar (no verbo haber)".to_string(),
                        });
                    }
                }
                None
            }
            "ahi" => {
                // "ahi" sin tilde es incorrecto, siempre es "ahí"
                Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "ahí"),
                    reason: "Adverbio de lugar (requiere tilde)".to_string(),
                })
            }
            "ahí" => {
                // "ahí" es correcto como adverbio de lugar
                // Error: usar "ahí" en lugar de "hay" (verbo)
                // Contexto: si va seguido de sustantivo/artículo, puede ser "hay"
                if let Some(n) = next {
                    if matches!(
                        n,
                        "un" | "una"
                            | "unos"
                            | "unas"
                            | "mucho"
                            | "mucha"
                            | "muchos"
                            | "muchas"
                            | "poco"
                            | "poca"
                    ) {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hay"),
                            reason: "Verbo haber impersonal".to_string(),
                        });
                    }
                }
                None
            }
            "ai" => {
                // "ai" es incorrecto, probablemente quiso decir "ahí" o "ay"
                // Excepción: dominios de internet como ".ai" (Q.ai, X.ai)
                // Si prev es None pero el token anterior inmediato es un punto,
                // probablemente es un dominio, no una interjección
                if prev.is_none() {
                    // Si no hay palabra anterior, puede ser dominio (.ai) o inicio
                    // No corregir en estos casos ambiguos
                    return None;
                }
                if let Some(prev_word) = prev {
                    if prev_word.len() == 1 && prev_word.chars().all(|c| c.is_alphabetic()) {
                        return None;
                    }
                    // OpenAI, xAI, etc. (termina en mayúscula antes de AI)
                    if prev_word.ends_with(|c: char| c.is_uppercase()) {
                        return None;
                    }
                }
                // Si está solo o con signos de exclamación, es "ay"
                Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "ay"),
                    reason: "Interjección de dolor/sorpresa".to_string(),
                })
            }
            _ => None,
        }
    }

    /// haya (verbo haber/árbol) / halla (verbo hallar) / aya (niñera)
    fn check_haya_halla(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        prev_prev: Option<&str>,
        prev_third: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "halla" => {
                // "halla" es verbo hallar (encontrar)
                // Error: usar "halla" en lugar de "haya" (subjuntivo de haber)
                // Contexto: después de "que", "aunque", "ojalá" suele ser "haya"
                let has_subjunctive_trigger = prev.map_or(false, Self::is_haya_subjunctive_trigger)
                    || (prev.map_or(false, Self::is_haya_interposed_word)
                        && prev_prev.map_or(false, Self::is_haya_subjunctive_trigger))
                    || (prev.map_or(false, Self::is_haya_interposed_word)
                        && prev_prev.map_or(false, Self::is_haya_interposed_word)
                        && prev_third.map_or(false, Self::is_haya_subjunctive_trigger));
                if has_subjunctive_trigger {
                    // Verificar si va seguido de participio (entonces es "haya")
                    if let Some(n) = next {
                        if n.ends_with("ado")
                            || n.ends_with("ido")
                            || n.ends_with("to")
                            || n.ends_with("cho")
                        {
                            return Some(HomophoneCorrection {
                                token_index: idx,
                                original: token.text.clone(),
                                suggestion: Self::preserve_case(&token.text, "haya"),
                                reason: "Subjuntivo de haber + participio".to_string(),
                            });
                        }
                    }
                }
                None
            }
            "haya" => {
                // "haya" puede ser subjuntivo de haber o el árbol
                // Error: usar "haya" en lugar de "halla" (encontrar)
                // Contexto: si va seguido de complemento directo sin participio
                if let Some(p) = prev {
                    // "se haya" + no participio = probablemente "se halla"
                    if p == "se" {
                        if let Some(n) = next {
                            if !n.ends_with("ado")
                                && !n.ends_with("ido")
                                && !n.ends_with("to")
                                && !n.ends_with("cho")
                            {
                                // Probablemente es "se halla" (se encuentra)
                                return Some(HomophoneCorrection {
                                    token_index: idx,
                                    original: token.text.clone(),
                                    suggestion: Self::preserve_case(&token.text, "halla"),
                                    reason: "Verbo hallar (encontrar)".to_string(),
                                });
                            }
                        }
                    }
                }
                None
            }
            "aya" => {
                // "aya" es niñera (arcaico), muy raro
                // Probablemente quiso decir "haya"
                if let Some(p) = prev {
                    if matches!(p, "que" | "aunque" | "ojalá" | "quizá" | "quizás") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "haya"),
                            reason: "Subjuntivo de haber".to_string(),
                        });
                    }
                }
                None
            }
            "haiga" => {
                // "haiga" es incorrecto, siempre es "haya"
                Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "haya"),
                    reason: "Forma correcta del subjuntivo de haber".to_string(),
                })
            }
            _ => None,
        }
    }

    fn is_haya_subjunctive_trigger(word: &str) -> bool {
        matches!(
            word,
            "que" | "aunque" | "ojalá" | "quizá" | "quizás" | "cuando" | "si"
        )
    }

    fn is_clitic_pronoun(word: &str) -> bool {
        matches!(
            word,
            "me" | "te" | "se" | "nos" | "os" | "lo" | "la" | "los" | "las" | "le" | "les"
        )
    }

    fn is_negative_adverb(word: &str) -> bool {
        matches!(word, "no" | "nunca" | "jamás" | "jamas" | "tampoco")
    }

    fn is_haya_interposed_word(word: &str) -> bool {
        Self::is_clitic_pronoun(word) || Self::is_negative_adverb(word)
    }

    /// "a ver" (locucion) / "haber" (verbo o sustantivo)
    fn check_a_ver_haber(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        prev_prev: Option<&str>,
        next: Option<&str>,
        next_next: Option<&str>,
        prev_token: Option<&Token>,
        prev_prev_token: Option<&Token>,
        next_token: Option<&Token>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "haber" | "aver" | "aber" => {
                if Self::is_a_ver_intro_context(prev)
                    && next.map_or(false, Self::is_a_ver_locution_trigger)
                {
                    return Some(HomophoneCorrection {
                        token_index: idx,
                        original: token.text.clone(),
                        suggestion: Self::preserve_case(&token.text, "a ver"),
                        reason: "Locucion 'a ver'".to_string(),
                    });
                }
                None
            }
            "ha" => {
                // Error frecuente: "voy ha comprar" en lugar de "voy a comprar".
                // Regla conservadora: solo cuando "ha" va seguido de infinitivo.
                if let Some(n) = next {
                    if Self::is_likely_infinitive(n) || Self::looks_like_infinitive_with_enclitic(n)
                    {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "a"),
                            reason: "Preposición 'a' antes de infinitivo".to_string(),
                        });
                    }
                }
                None
            }
            "a" => {
                // Error frecuente: "se a ido" en lugar de "se ha ido".
                // Cobertura ampliada pero conservadora:
                // - clítico + a + participio ("se a ido")
                // - sujeto + a + participio ("yo a venido")
                // - inicio de oración + a + participio ("A echo su tarea")
                // Filtra falsos positivos nominales como "a lado" usando info de categoría.
                if next == Some("grosso") && next_next == Some("modo") {
                    // Locución fija: "a grosso modo" (preposición + latinismo)
                    return None;
                }
                if let Some(n) = next {
                    if Self::is_likely_participle_with_context(n, next_token) {
                        let prev_is_temporal = prev
                            .map_or(false, |p| Self::is_temporal_complement_head(p, prev_token));
                        let prev_is_clitic = prev.map_or(false, |p| {
                            matches!(
                                p,
                                "me" | "te"
                                    | "se"
                                    | "nos"
                                    | "os"
                                    | "lo"
                                    | "la"
                                    | "los"
                                    | "las"
                                    | "le"
                                    | "les"
                            )
                        });

                        let prev_is_subject = prev
                            .map_or(false, |p| Self::is_subject_pronoun_candidate(p, prev_token));
                        let prev_is_nominal_subject = Self::is_nominal_subject_candidate(
                            prev_token,
                            prev_prev,
                            prev_prev_token,
                        );
                        let prev_is_negation =
                            prev.map_or(false, Self::is_negative_adverb);
                        let prev_prev_is_subject = prev_is_negation
                            && prev_prev.map_or(false, |p| {
                                Self::is_subject_pronoun_candidate(p, prev_prev_token)
                            });
                        let prev_prev_is_nominal_subject =
                            prev_is_negation
                                && Self::is_nominal_subject_candidate(prev_prev_token, None, None);
                        let negated_without_explicit_subject = prev_is_negation
                            && !prev_prev_is_subject
                            && !prev_prev_is_nominal_subject;

                        let at_sentence_start = prev.is_none();

                        if prev_is_temporal
                            || prev_is_clitic
                            || prev_is_subject
                            || prev_is_nominal_subject
                            || prev_prev_is_subject
                            || prev_prev_is_nominal_subject
                            || negated_without_explicit_subject
                            || at_sentence_start
                        {
                            let haber_form = if prev_is_temporal {
                                "ha"
                            } else if prev_is_clitic || at_sentence_start {
                                "ha"
                            } else if prev_is_subject {
                                let p = prev.unwrap_or("el");
                                Self::get_haber_aux_for_subject(p).unwrap_or("ha")
                            } else if prev_is_nominal_subject {
                                Self::get_haber_aux_for_nominal_subject(
                                    prev_token,
                                    prev_prev,
                                    prev_prev_token,
                                )
                                .unwrap_or("ha")
                            } else if prev_prev_is_subject {
                                let p = prev_prev.unwrap_or("el");
                                Self::get_haber_aux_for_subject(p).unwrap_or("ha")
                            } else if prev_prev_is_nominal_subject {
                                Self::get_haber_aux_for_nominal_subject(prev_prev_token, None, None)
                                    .unwrap_or("ha")
                            } else if negated_without_explicit_subject {
                                "ha"
                            } else if let Some(p) = prev {
                                if Self::is_subject_pronoun_candidate(p, prev_token) {
                                    Self::get_haber_aux_for_subject(p).unwrap_or("ha")
                                } else {
                                    Self::get_haber_aux_for_nominal_subject(
                                        prev_token,
                                        prev_prev,
                                        prev_prev_token,
                                    )
                                    .unwrap_or("ha")
                                }
                            } else {
                                "ha"
                            };
                            return Some(HomophoneCorrection {
                                token_index: idx,
                                original: token.text.clone(),
                                suggestion: Self::preserve_case(&token.text, haber_form),
                                reason: "Auxiliar haber en tiempo compuesto".to_string(),
                            });
                        }
                    }
                }
                None
            }
            "haz" => {
                // Error frecuente: "haz visto/hecho" en lugar de "has visto/hecho".
                // Solo corregir ante participio para no tocar el imperativo válido:
                // "haz la tarea", "hazlo", etc.
                if let Some(n) = next {
                    if Self::is_likely_participle_with_context(n, next_token) {
                        let prev_is_temporal = prev
                            .map_or(false, |p| Self::is_temporal_complement_head(p, prev_token));
                        let prev_is_clitic = prev.map_or(false, |p| {
                            matches!(
                                p,
                                "me" | "te"
                                    | "se"
                                    | "nos"
                                    | "os"
                                    | "lo"
                                    | "la"
                                    | "los"
                                    | "las"
                                    | "le"
                                    | "les"
                            )
                        });
                        let prev_is_subject = prev
                            .map_or(false, |p| Self::is_subject_pronoun_candidate(p, prev_token));
                        let prev_is_nominal_subject = Self::is_nominal_subject_candidate(
                            prev_token,
                            prev_prev,
                            prev_prev_token,
                        );
                        let prev_is_negation = prev.map_or(false, Self::is_negative_adverb);
                        let prev_prev_is_subject = prev_is_negation
                            && prev_prev.map_or(false, |p| {
                                Self::is_subject_pronoun_candidate(p, prev_prev_token)
                            });
                        let prev_prev_is_nominal_subject =
                            prev_is_negation
                                && Self::is_nominal_subject_candidate(prev_prev_token, None, None);
                        let negated_without_explicit_subject = prev_is_negation
                            && !prev_prev_is_subject
                            && !prev_prev_is_nominal_subject;
                        let at_sentence_start = prev.is_none();

                        if prev_is_temporal
                            || prev_is_clitic
                            || prev_is_subject
                            || prev_is_nominal_subject
                            || prev_prev_is_subject
                            || prev_prev_is_nominal_subject
                            || negated_without_explicit_subject
                            || at_sentence_start
                        {
                            let haber_form = if prev_is_temporal {
                                "ha"
                            } else if prev_is_clitic || at_sentence_start {
                                "has"
                            } else if prev_is_subject {
                                let p = prev.unwrap_or("tu");
                                Self::get_haber_aux_for_subject(p).unwrap_or("has")
                            } else if prev_is_nominal_subject {
                                Self::get_haber_aux_for_nominal_subject(
                                    prev_token,
                                    prev_prev,
                                    prev_prev_token,
                                )
                                .unwrap_or("has")
                            } else if prev_prev_is_subject {
                                let p = prev_prev.unwrap_or("tu");
                                Self::get_haber_aux_for_subject(p).unwrap_or("has")
                            } else if prev_prev_is_nominal_subject {
                                Self::get_haber_aux_for_nominal_subject(prev_prev_token, None, None)
                                    .unwrap_or("has")
                            } else {
                                "has"
                            };
                            return Some(HomophoneCorrection {
                                token_index: idx,
                                original: token.text.clone(),
                                suggestion: Self::preserve_case(&token.text, haber_form),
                                reason: "Auxiliar haber en tiempo compuesto".to_string(),
                            });
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// porque / por que / porqué / por qué
    ///
    /// - Interrogativo (directo o indirecto): "por qué"
    /// - Sustantivo: "el porqué (de...)"
    /// - Conjunción causal: "porque"
    fn check_por_que_family(
        word: &str,
        idx: usize,
        token: &Token,
        pos: usize,
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
        prev: Option<&str>,
        prev_prev: Option<&str>,
        next: Option<&str>,
        prev_token: Option<&Token>,
        prev_prev_token: Option<&Token>,
    ) -> Option<HomophoneCorrection> {
        let normalized = Self::normalize_simple(word);

        // Caso 1: "porque"/"porqué" en una sola palabra
        if normalized == "porque" {
            let has_acute_e = word.chars().any(|c| c == '\u{00E9}' || c == '\u{00C9}');
            let is_nominal =
                Self::is_porque_nominal_context(prev, prev_prev, next, prev_token, prev_prev_token);
            let is_interrogative =
                Self::is_por_que_interrogative_context(all_tokens, idx, prev, prev_prev);

            if is_nominal {
                if has_acute_e {
                    return None;
                }
                return Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "porqu\u{00E9}"),
                    reason: "Sustantivo: 'el porqu\u{00E9}'".to_string(),
                });
            }

            if is_interrogative {
                return Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "por qu\u{00E9}"),
                    reason: "Interrogativo: 'por qu\u{00E9}'".to_string(),
                });
            }

            // "porqué" fuera de contexto nominal/interrogativo suele ser error por "porque".
            if has_acute_e {
                return Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "porque"),
                    reason: "Conjunci\u{00F3}n causal: 'porque'".to_string(),
                });
            }

            return None;
        }

        // Caso 2: secuencia "por que" (dos tokens): acentuar "que" en contexto interrogativo
        if normalized == "que"
            && prev.is_some_and(|p| Self::normalize_simple(p) == "por")
            && !word.chars().any(|c| c == '\u{00E9}' || c == '\u{00C9}')
        {
            let por_idx = if pos > 0 { word_tokens[pos - 1].0 } else { idx };
            let trigger_prev = prev_prev;
            let trigger_prev_prev = if pos >= 3 {
                Some(Self::token_text_for_homophone(word_tokens[pos - 3].1))
            } else {
                None
            };
            let is_interrogative = Self::is_por_que_interrogative_context(
                all_tokens,
                por_idx,
                trigger_prev,
                trigger_prev_prev,
            );

            if is_interrogative {
                return Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "qu\u{00E9}"),
                    reason: "Interrogativo: 'por qu\u{00E9}'".to_string(),
                });
            }
        }

        None
    }

    /// sino / si no
    ///
    /// - Adversativo/exclusivo: "no A, sino B" (una palabra)
    /// - Condicional negativo: "si no + verbo..." (dos palabras)
    fn check_sino_si_no(
        word: &str,
        idx: usize,
        token: &Token,
        pos: usize,
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
        prev: Option<&str>,
        next: Option<&str>,
        next_token: Option<&Token>,
    ) -> Option<HomophoneCorrection> {
        let normalized = Self::normalize_simple(word);

        // "sino vienes..." -> "si no vienes..."
        if normalized == "sino" && Self::should_split_sino_as_si_no(pos, word_tokens, all_tokens) {
            return Some(HomophoneCorrection {
                token_index: idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, "si no"),
                reason: "Condicional negativo: 'si no'".to_string(),
            });
        }

        // "si no ..." (adversativo) -> "sino ..."
        if normalized == "si"
            && next.is_some_and(|w| Self::normalize_simple(w) == "no")
            && Self::should_merge_si_no_as_sino(pos, word_tokens, all_tokens)
        {
            return Some(HomophoneCorrection {
                token_index: idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, "sino"),
                reason: "Conjunción adversativa: 'sino'".to_string(),
            });
        }

        // Marcar el "no" sobrante para la fusión "si no" -> "sino".
        if normalized == "no"
            && prev.is_some_and(|w| Self::normalize_simple(w) == "si")
            && pos > 0
            && Self::should_merge_si_no_as_sino(pos - 1, word_tokens, all_tokens)
        {
            return Some(HomophoneCorrection {
                token_index: idx,
                original: token.text.clone(),
                suggestion: "sobra".to_string(),
                reason: "Conjunción adversativa: 'sino'".to_string(),
            });
        }

        // Evitar warning por parámetro no usado cuando no aplica ninguna rama.
        let _ = next_token;
        None
    }

    fn should_merge_si_no_as_sino(
        si_pos: usize,
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
    ) -> bool {
        if si_pos + 1 >= word_tokens.len() {
            return false;
        }

        let (si_idx, si_token) = word_tokens[si_pos];
        let (no_idx, no_token) = word_tokens[si_pos + 1];
        let si_norm = Self::normalize_simple(Self::token_text_for_homophone(si_token));
        let no_norm = Self::normalize_simple(Self::token_text_for_homophone(no_token));
        if si_norm != "si" || no_norm != "no" || has_sentence_boundary(all_tokens, si_idx, no_idx) {
            return false;
        }

        let Some((follower_word, follower_token)) =
            Self::first_non_skippable_after_no(si_pos + 1, word_tokens, all_tokens)
        else {
            return false;
        };

        if !Self::has_prior_negation_before_si(si_pos, word_tokens, all_tokens) {
            return false;
        }

        !Self::is_likely_finite_verb_form(&follower_word, follower_token)
    }

    fn should_split_sino_as_si_no(
        sino_pos: usize,
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
    ) -> bool {
        if sino_pos >= word_tokens.len() {
            return false;
        }

        let (_, sino_token) = word_tokens[sino_pos];
        let sino_norm = Self::normalize_simple(Self::token_text_for_homophone(sino_token));
        if sino_norm != "sino" {
            return false;
        }

        let Some((follower_word, follower_token)) =
            Self::first_non_skippable_after_no(sino_pos, word_tokens, all_tokens)
        else {
            return false;
        };

        Self::is_likely_finite_verb_form(&follower_word, follower_token)
    }

    fn first_non_skippable_after_no<'a>(
        head_pos: usize,
        word_tokens: &[(usize, &'a Token)],
        all_tokens: &[Token],
    ) -> Option<(String, Option<&'a Token>)> {
        if head_pos + 1 >= word_tokens.len() {
            return None;
        }

        let head_idx = word_tokens[head_pos].0;
        for (_, (idx, token)) in word_tokens.iter().enumerate().skip(head_pos + 1) {
            if has_sentence_boundary(all_tokens, head_idx, *idx) {
                break;
            }

            let norm = Self::normalize_simple(Self::token_text_for_homophone(token));
            if Self::is_si_no_skip_word(&norm) {
                continue;
            }

            return Some((norm, Some(*token)));
        }

        None
    }

    fn is_si_no_skip_word(word: &str) -> bool {
        matches!(
            word,
            "me" | "te"
                | "se"
                | "nos"
                | "os"
                | "lo"
                | "la"
                | "los"
                | "las"
                | "le"
                | "les"
                | "ya"
                | "tambien"
                | "solo"
                | "aun"
                | "todavia"
        )
    }

    fn has_prior_negation_before_si(
        si_pos: usize,
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
    ) -> bool {
        if si_pos == 0 {
            return false;
        }

        let si_idx = word_tokens[si_pos].0;
        let mut scanned = 0usize;
        for p in (0..si_pos).rev() {
            let (idx, token) = word_tokens[p];
            if has_sentence_boundary(all_tokens, idx, si_idx) {
                break;
            }

            let norm = Self::normalize_simple(Self::token_text_for_homophone(token));
            if matches!(
                norm.as_str(),
                "no" | "nunca"
                    | "jamas"
                    | "tampoco"
                    | "nadie"
                    | "nada"
                    | "ningun"
                    | "ninguna"
                    | "ninguno"
                    | "ningunos"
                    | "ningunas"
            ) {
                return true;
            }

            scanned += 1;
            if scanned >= 10 {
                break;
            }
        }

        false
    }

    fn is_likely_finite_verb_form(word: &str, token: Option<&Token>) -> bool {
        if Self::is_likely_infinitive(word) || Self::looks_like_infinitive_with_enclitic(word) {
            return false;
        }

        if matches!(word, "ando" | "iendo" | "yendo")
            || word.ends_with("ando")
            || word.ends_with("iendo")
            || word.ends_with("yendo")
        {
            return false;
        }

        if let Some(tok) = token {
            if let Some(info) = tok.word_info.as_ref() {
                // Si el diccionario la reconoce como verbo, aceptar de inmediato.
                if info.category == crate::dictionary::WordCategory::Verbo {
                    return true;
                }

                // Mantener alta precision para categorias no verbales claras.
                // Excepcion: "como" es homografo frecuente (conjuncion/adverbio vs verbo "comer").
                if Self::normalize_simple(word) != "como" {
                    return false;
                }
            }
        }

        if matches!(
            word,
            "es"
                | "son"
                | "era"
                | "eran"
                | "fue"
                | "fueron"
                | "soy"
                | "eres"
                | "somos"
                | "estoy"
                | "estas"
                | "esta"
                | "estan"
                | "voy"
                | "vas"
                | "va"
                | "vamos"
                | "van"
                | "viene"
                | "vienen"
                | "vienes"
                | "vino"
                | "vinieron"
                | "hay"
                | "habia"
                | "habian"
                | "hara"
                | "haran"
                | "puede"
                | "pueden"
                | "quiere"
                | "quieren"
                | "tiene"
                | "tienen"
                | "tengo"
                | "hace"
                | "hacen"
                | "dice"
                | "dicen"
                | "dijo"
                | "dijeron"
        ) {
            return true;
        }

        let finite_suffixes = [
            "as", "es", "a", "an", "en", "amos", "emos", "imos", "ais", "eis", "aba", "aban",
            "ia", "ian", "ara", "aras", "aran", "ase", "ases", "asen", "aria", "arias",
            "arian", "aste", "iste", "aron", "ieron", "o", "io",
        ];

        finite_suffixes
            .iter()
            .any(|suffix| word.len() > suffix.len() + 1 && word.ends_with(suffix))
    }

    fn looks_like_infinitive_with_enclitic(word: &str) -> bool {
        const CLITICS: [&str; 11] = [
            "me", "te", "se", "nos", "os", "lo", "la", "los", "las", "le", "les",
        ];

        for first in CLITICS {
            if let Some(stem) = word.strip_suffix(first) {
                if Self::is_likely_infinitive(stem) {
                    return true;
                }
                for second in CLITICS {
                    if let Some(stem2) = stem.strip_suffix(second) {
                        if Self::is_likely_infinitive(stem2) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn is_porque_nominal_context(
        prev: Option<&str>,
        prev_prev: Option<&str>,
        next: Option<&str>,
        prev_token: Option<&Token>,
        prev_prev_token: Option<&Token>,
    ) -> bool {
        if next.is_some_and(|w| Self::normalize_simple(w) == "de") {
            return true;
        }

        if prev.is_some_and(|w| Self::is_nominal_determiner(w, prev_token)) {
            return true;
        }

        // Permitir "el principal porqué": adjetivo entre determinante y sustantivo.
        if let (Some(prev_word), Some(prev_prev_word), Some(prev_tok)) = (prev, prev_prev, prev_token)
        {
            if let Some(info) = prev_tok.word_info.as_ref() {
                if info.category == crate::dictionary::WordCategory::Adjetivo
                    && Self::is_nominal_determiner(prev_prev_word, prev_prev_token)
                {
                    return true;
                }
            } else {
                let prev_norm = Self::normalize_simple(prev_word);
                if (prev_norm.ends_with('o') || prev_norm.ends_with('a'))
                    && Self::is_nominal_determiner(prev_prev_word, prev_prev_token)
                {
                    return true;
                }
            }
        }

        false
    }

    fn is_por_que_interrogative_context(
        tokens: &[Token],
        phrase_idx: usize,
        trigger_prev: Option<&str>,
        trigger_prev_prev: Option<&str>,
    ) -> bool {
        Self::is_reason_question_trigger(trigger_prev, trigger_prev_prev)
            || (Self::is_in_direct_question_span(tokens, phrase_idx)
                && Self::is_por_que_question_front(tokens, phrase_idx))
    }

    fn is_reason_question_trigger(trigger_prev: Option<&str>, trigger_prev_prev: Option<&str>) -> bool {
        let Some(prev_word) = trigger_prev else {
            return false;
        };

        let prev_norm = Self::normalize_simple(prev_word);
        if Self::is_reason_question_verb(&prev_norm) {
            return true;
        }

        if prev_norm == "se" {
            return trigger_prev_prev.is_some_and(|w| {
                matches!(
                    Self::normalize_simple(w).as_str(),
                    "no" | "yo" | "ya" | "ni" | "nunca" | "tampoco" | "quizas" | "quiza"
                )
            });
        }

        // "no entiendo/comprendo/explico porque" → interrogative (with negation)
        if Self::is_negated_cognitive_verb(&prev_norm) {
            return trigger_prev_prev.is_some_and(|w| {
                Self::normalize_simple(w) == "no"
            });
        }

        false
    }

    fn is_reason_question_verb(word: &str) -> bool {
        matches!(
            word,
            "saber"
                | "se"
                | "sabe"
                | "sabes"
                | "sabemos"
                | "saben"
                | "sabia"
                | "sabias"
                | "sabiamos"
                | "sabian"
                | "supe"
                | "supiste"
                | "supo"
                | "supimos"
                | "supieron"
                | "sabre"
                | "sabras"
                | "sabremos"
                | "sabran"
                | "sabria"
                | "sabrias"
                | "sabriamos"
                | "sabrian"
        ) || word.starts_with("pregunt")
            || word.starts_with("ignor")
            || word.starts_with("desconoc")
            || word.starts_with("averigu")
            || matches!(
                word,
                "dime"
                    | "dinos"
                    | "digame"
                    | "diganos"
                    | "diga"
                    | "cuentame"
                    | "cuentanos"
                    | "cuente"
                    | "cuenteme"
                    | "explicame"
                    | "explicanos"
                    | "expliqueme"
                    | "expliquenos"
                    | "explique"
            )
    }

    /// Verbs that only introduce indirect questions when negated:
    /// "no entiendo porque" → interrogative, but "entiendo porque" → causal.
    fn is_negated_cognitive_verb(word: &str) -> bool {
        word.starts_with("entiend")
            || word.starts_with("entend")
            || word.starts_with("comprend")
            || word.starts_with("concib")
    }

    fn is_in_direct_question_span(tokens: &[Token], token_idx: usize) -> bool {
        let mut has_open = false;
        for i in (0..token_idx).rev() {
            let token = &tokens[i];
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.text == "\u{00BF}" {
                has_open = true;
                break;
            }
            if token.is_sentence_boundary() {
                break;
            }
        }

        let mut has_close = false;
        for token in tokens.iter().skip(token_idx + 1) {
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.text == "?" {
                has_close = true;
                break;
            }
            if token.is_sentence_boundary() {
                break;
            }
        }

        has_open || has_close
    }

    fn is_por_que_question_front(tokens: &[Token], token_idx: usize) -> bool {
        let prior_words = Self::collect_prior_words_in_clause(tokens, token_idx, 2);
        prior_words.is_empty()
            || (prior_words.len() == 1 && Self::is_question_intro_connector(&prior_words[0]))
    }

    fn collect_prior_words_in_clause(
        tokens: &[Token],
        token_idx: usize,
        max_words: usize,
    ) -> Vec<String> {
        let mut words = Vec::new();
        for i in (0..token_idx).rev() {
            let token = &tokens[i];
            if token.token_type == TokenType::Whitespace {
                continue;
            }

            if token.token_type == TokenType::Punctuation {
                if token.text == "\u{00BF}" || token.is_sentence_boundary() {
                    break;
                }
                continue;
            }

            if token.token_type != TokenType::Word {
                break;
            }

            words.push(Self::normalize_simple(Self::token_text_for_homophone(token)));
            if words.len() >= max_words {
                break;
            }
        }

        words
    }

    fn is_question_intro_connector(word: &str) -> bool {
        matches!(
            word,
            "y" | "e" | "o" | "u" | "pero" | "pues" | "entonces" | "bueno"
        )
    }

    fn is_likely_infinitive(word: &str) -> bool {
        let len = word.chars().count();
        if len < 3 {
            return false;
        }
        word.ends_with("ar") || word.ends_with("er") || word.ends_with("ir")
    }

    fn is_a_ver_locution_trigger(word: &str) -> bool {
        matches!(
            Self::normalize_simple(word).as_str(),
            "si" | "que" | "como" | "cuando" | "donde" | "quien" | "quienes" | "cual" | "cuales"
        )
    }

    fn is_a_ver_intro_context(prev: Option<&str>) -> bool {
        match prev {
            None => true,
            Some(word) => matches!(
                Self::normalize_simple(word).as_str(),
                "y" | "e" | "pues" | "bueno" | "entonces" | "vamos"
            ),
        }
    }

    fn is_likely_participle(word: &str) -> bool {
        matches!(
            word,
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
        ) || word.ends_with("ado")
            || word.ends_with("ada")
            || word.ends_with("ados")
            || word.ends_with("adas")
            || word.ends_with("ido")
            || word.ends_with("ida")
            || word.ends_with("idos")
            || word.ends_with("idas")
            || word.ends_with("ído")
            || word.ends_with("ída")
            || word.ends_with("ídos")
            || word.ends_with("ídas")
            || word.ends_with("to")
            || word.ends_with("ta")
            || word.ends_with("tos")
            || word.ends_with("tas")
            || word.ends_with("cho")
            || word.ends_with("cha")
            || word.ends_with("chos")
            || word.ends_with("chas")
            || word.ends_with("so")
            || word.ends_with("sa")
            || word.ends_with("sos")
            || word.ends_with("sas")
    }

    fn is_likely_participle_with_context(word: &str, token: Option<&Token>) -> bool {
        if !Self::is_likely_participle(word) {
            return false;
        }

        if let Some(tok) = token {
            if let Some(info) = tok.word_info.as_ref() {
                // Si el diccionario lo marca solo como sustantivo, suele ser falso positivo
                // de sufijo (-ado/-ido) como "lado", no participio verbal.
                if info.category == crate::dictionary::WordCategory::Sustantivo {
                    let w = word.to_lowercase();
                    if !matches!(
                        w.as_str(),
                        "hecho"
                            | "hecha"
                            | "hechos"
                            | "hechas"
                            | "dicho"
                            | "dicha"
                            | "dichos"
                            | "dichas"
                            | "visto"
                            | "vista"
                            | "vistos"
                            | "vistas"
                            | "puesto"
                            | "puesta"
                            | "puestos"
                            | "puestas"
                    ) {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn is_subject_pronoun_candidate(word: &str, token: Option<&Token>) -> bool {
        if matches!(
            word,
            "yo" | "tu"
                | "tú"
                | "el"
                | "él"
                | "ella"
                | "usted"
                | "nosotros"
                | "nosotras"
                | "vosotros"
                | "vosotras"
                | "ellos"
                | "ellas"
                | "ustedes"
                | "que"
                | "quien"
                | "quienes"
        ) {
            return true;
        }

        // Fallback por categoría para pronombres no listados explícitamente.
        token
            .and_then(|t| t.word_info.as_ref())
            .map(|info| info.category == crate::dictionary::WordCategory::Pronombre)
            .unwrap_or(false)
    }

    fn get_haber_aux_for_subject(word: &str) -> Option<&'static str> {
        let norm = Self::normalize_simple(word);
        match norm.as_str() {
            "yo" => Some("he"),
            "tu" => Some("has"),
            "el" | "ella" | "usted" => Some("ha"),
            "nosotros" | "nosotras" => Some("hemos"),
            "vosotros" | "vosotras" => Some("habéis"),
            "ellos" | "ellas" | "ustedes" => Some("han"),
            _ => None,
        }
    }

    fn is_nominal_subject_candidate(
        prev_token: Option<&Token>,
        prev_prev: Option<&str>,
        prev_prev_token: Option<&Token>,
    ) -> bool {
        let prev_word = prev_token.map(Self::token_text_for_homophone);
        if prev_word.map_or(false, Self::is_temporal_noun_like) {
            return false;
        }

        if let Some(token) = prev_token {
            if let Some(info) = token.word_info.as_ref() {
                return matches!(
                    info.category,
                    crate::dictionary::WordCategory::Sustantivo
                        | crate::dictionary::WordCategory::Adjetivo
                );
            }

            let starts_with_uppercase = token
                .text
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            let is_acronym = token.text.chars().all(|c| c.is_uppercase());
            if starts_with_uppercase && !is_acronym && token.text.chars().count() > 1 {
                return true;
            }
        }

        // Fallback conservador sin word_info: requerir determinante previo.
        prev_prev.map_or(false, |w| Self::is_nominal_determiner(w, prev_prev_token))
    }

    fn get_haber_aux_for_nominal_subject(
        prev_token: Option<&Token>,
        prev_prev: Option<&str>,
        prev_prev_token: Option<&Token>,
    ) -> Option<&'static str> {
        if let Some(token) = prev_token {
            if let Some(info) = token.word_info.as_ref() {
                return match info.number {
                    crate::dictionary::Number::Plural => Some("han"),
                    crate::dictionary::Number::Singular => Some("ha"),
                    crate::dictionary::Number::None => None,
                };
            }
        }

        if prev_prev.map_or(false, |w| {
            Self::is_plural_nominal_determiner(w, prev_prev_token)
        }) {
            return Some("han");
        }
        if prev_prev.map_or(false, |w| {
            Self::is_singular_nominal_determiner(w, prev_prev_token)
        }) {
            return Some("ha");
        }
        None
    }

    fn normalize_simple(word: &str) -> String {
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

    fn is_nominal_determiner(word: &str, token: Option<&Token>) -> bool {
        if let Some(tok) = token {
            if let Some(info) = tok.word_info.as_ref() {
                if matches!(
                    info.category,
                    crate::dictionary::WordCategory::Articulo
                        | crate::dictionary::WordCategory::Determinante
                ) {
                    return true;
                }
            }
        }
        Self::is_plural_nominal_determiner(word, token)
            || Self::is_singular_nominal_determiner(word, token)
    }

    fn is_plural_nominal_determiner(word: &str, token: Option<&Token>) -> bool {
        if let Some(tok) = token {
            if let Some(info) = tok.word_info.as_ref() {
                if matches!(
                    info.category,
                    crate::dictionary::WordCategory::Articulo
                        | crate::dictionary::WordCategory::Determinante
                ) {
                    return info.number == crate::dictionary::Number::Plural;
                }
            }
        }

        matches!(
            word,
            "los"
                | "las"
                | "unos"
                | "unas"
                | "estos"
                | "estas"
                | "esos"
                | "esas"
                | "aquellos"
                | "aquellas"
                | "mis"
                | "tus"
                | "sus"
                | "nuestros"
                | "nuestras"
                | "vuestros"
                | "vuestras"
        )
    }

    fn is_singular_nominal_determiner(word: &str, token: Option<&Token>) -> bool {
        if let Some(tok) = token {
            if let Some(info) = tok.word_info.as_ref() {
                if matches!(
                    info.category,
                    crate::dictionary::WordCategory::Articulo
                        | crate::dictionary::WordCategory::Determinante
                ) {
                    return info.number == crate::dictionary::Number::Singular;
                }
            }
        }

        matches!(
            word,
            "el" | "la"
                | "un"
                | "una"
                | "este"
                | "esta"
                | "ese"
                | "esa"
                | "aquel"
                | "aquella"
                | "mi"
                | "tu"
                | "su"
                | "nuestro"
                | "nuestra"
                | "vuestro"
                | "vuestra"
        )
    }

    fn is_temporal_complement_head(word: &str, token: Option<&Token>) -> bool {
        if !Self::is_temporal_noun_like(word) {
            return false;
        }

        if let Some(tok) = token {
            if let Some(info) = tok.word_info.as_ref() {
                return matches!(
                    info.category,
                    crate::dictionary::WordCategory::Sustantivo
                        | crate::dictionary::WordCategory::Adverbio
                );
            }
        }

        true
    }

    fn is_temporal_noun_like(word: &str) -> bool {
        let norm = Self::normalize_simple(word);
        matches!(
            norm.as_str(),
            "lunes"
                | "martes"
                | "miercoles"
                | "jueves"
                | "viernes"
                | "sabado"
                | "sabados"
                | "domingo"
                | "domingos"
                | "dia"
                | "dias"
                | "semana"
                | "semanas"
                | "mes"
                | "meses"
                | "ano"
                | "anos"
                | "manana"
                | "mananas"
                | "tarde"
                | "tardes"
                | "noche"
                | "noches"
                | "verano"
                | "veranos"
                | "invierno"
                | "inviernos"
                | "primavera"
                | "primaveras"
                | "otono"
                | "otonos"
                | "enero"
                | "febrero"
                | "marzo"
                | "abril"
                | "mayo"
                | "junio"
                | "julio"
                | "agosto"
                | "septiembre"
                | "setiembre"
                | "octubre"
                | "noviembre"
                | "diciembre"
        )
    }

    fn has_nominal_determiner_context(prev: &str, prev_token: Option<&Token>) -> bool {
        if let Some(token) = prev_token {
            if let Some(info) = token.word_info.as_ref() {
                if matches!(
                    info.category,
                    crate::dictionary::WordCategory::Articulo
                        | crate::dictionary::WordCategory::Determinante
                ) {
                    return true;
                }
            }
        }

        // Fallback mínimo para uso aislado sin word_info (tests/unitarios).
        matches!(
            prev,
            "un" | "una"
                | "unos"
                | "unas"
                | "el"
                | "la"
                | "los"
                | "las"
                | "este"
                | "esta"
                | "estos"
                | "estas"
        )
    }

    /// vaya (verbo ir) / valla (cerca) / baya (fruto)
    fn check_vaya_valla(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "valla" => {
                // "valla" es cerca/obstáculo
                // Error: usar "valla" en lugar de "vaya" (verbo ir)
                if let Some(p) = prev {
                    // "que valla" = "que vaya"
                    if matches!(p, "que" | "ojalá" | "quizá" | "aunque") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                // "valla a" = "vaya a"
                if let Some(n) = next {
                    if n == "a" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                None
            }
            "baya" => {
                // "baya" es fruto pequeño
                // Error: usar "baya" en lugar de "vaya"
                if let Some(p) = prev {
                    if matches!(p, "que" | "ojalá" | "quizá" | "aunque") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                if let Some(n) = next {
                    if n == "a" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// voy (1a persona de ir) / boy (anglicismo)
    fn check_voy_boy(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        if word != "boy" {
            return None;
        }

        // Casos muy claros de verbo "ir": "boy a ...", "boy al ...", "yo boy ..."
        if matches!(next, Some("a") | Some("al")) || prev == Some("yo") {
            return Some(HomophoneCorrection {
                token_index: idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, "voy"),
                reason: "Primera persona de 'ir' (voy)".to_string(),
            });
        }

        None
    }

    /// hecho (participio hacer) / echo (verbo echar)
    fn check_hecho_echo(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        prev_token: Option<&Token>,
        next: Option<&str>,
        next2: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "echo" => {
                // "echo" es verbo echar (yo echo, él echa)
                // Error: usar "echo" en lugar de "hecho" (participio)
                if let Some(p) = prev {
                    // "he echo" = "he hecho"
                    // También cubrir "a echo" cuando "a" es error por "ha".
                    if matches!(
                        p,
                        "he" | "has" | "ha" | "hemos" | "habéis" | "han" | "había" | "habías" | "a"
                    ) {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hecho"),
                            reason: "Participio de hacer".to_string(),
                        });
                    }
                    // "de echo" = "de hecho"
                    if p == "de" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hecho"),
                            reason: "Locución 'de hecho'".to_string(),
                        });
                    }

                    // "un/el ... echo" suele ser sustantivo: "un hecho", "el hecho de que"
                    if Self::has_nominal_determiner_context(p, prev_token) {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hecho"),
                            reason: "Sustantivo 'hecho'".to_string(),
                        });
                    }
                }
                None
            }
            "echos" => {
                if let Some(p) = prev {
                    // Tras auxiliar, el participio de "hacer" es invariable: "han hecho".
                    if matches!(
                        p,
                        "he" | "has" | "ha" | "hemos" | "habéis" | "han" | "había" | "habías" | "a"
                    ) {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hecho"),
                            reason: "Participio de hacer".to_string(),
                        });
                    }

                    // Uso nominal plural frecuente: "los hechos", "son hechos conocidos".
                    if Self::is_plural_masculine_determiner(p)
                        || (Self::is_copular_verb(p)
                            && next.map_or(false, Self::is_likely_plural_masculine_adjective))
                    {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hechos"),
                            reason: "Sustantivo 'hechos'".to_string(),
                        });
                    }
                }
                None
            }
            "hecho" => {
                // "hecho" es participio de hacer o sustantivo
                // Error: usar "hecho" en lugar de "echo" (echar)
                if next == Some("de") && next2 == Some("menos") {
                    return Some(HomophoneCorrection {
                        token_index: idx,
                        original: token.text.clone(),
                        suggestion: Self::preserve_case(&token.text, "echo"),
                        reason: "Locucion verbal 'echo de menos'".to_string(),
                    });
                }
                if let Some(p) = prev {
                    // "lo hecho" cuando debería ser "lo echo" (yo lo echo)
                    // Difícil de detectar sin más contexto
                    // "te hecho de menos" = "te echo de menos"
                    if matches!(
                        p,
                        "te" | "lo" | "la" | "le" | "los" | "las" | "les" | "me" | "nos"
                    ) {
                        // Podría ser "te echo" pero también "lo hecho está hecho"
                        // Solo corregir casos claros como "te hecho de menos"
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn is_plural_masculine_determiner(word: &str) -> bool {
        matches!(
            word,
            "los"
                | "unos"
                | "estos"
                | "esos"
                | "aquellos"
                | "muchos"
                | "pocos"
                | "varios"
                | "algunos"
                | "otros"
                | "todos"
        )
    }

    fn is_copular_verb(word: &str) -> bool {
        matches!(
            word,
            "es" | "son" | "era" | "eran" | "fue" | "fueron" | "sea" | "sean" | "sera" | "seran"
        )
    }

    fn is_likely_plural_masculine_adjective(word: &str) -> bool {
        let len = word.chars().count();
        len > 3 && word.ends_with("os")
    }

    /// tuvo (verbo tener) / tubo (sustantivo)
    fn check_tuvo_tubo(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "tubo" => {
                // "tubo" es sustantivo (cilindro)
                // Error: usar "tubo" en lugar de "tuvo" (verbo tener)
                if let Some(n) = next {
                    // "tubo que" = "tuvo que"
                    if n == "que" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "tuvo"),
                            reason: "Pretérito de tener".to_string(),
                        });
                    }
                }
                // Después de pronombre personal suele ser verbo
                if let Some(p) = prev {
                    if matches!(
                        p,
                        "él" | "ella" | "usted" | "quien" | "que" | "no" | "lo" | "la" | "le"
                    ) {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "tuvo"),
                            reason: "Pretérito de tener".to_string(),
                        });
                    }
                }
                None
            }
            "tuvo" => {
                // "tuvo" es verbo tener
                // Raro confundir en esta dirección, pero verificar contexto de sustantivo
                if let Some(p) = prev {
                    // "el tuvo" cuando es sustantivo = "el tubo"
                    if matches!(p, "el" | "un" | "este" | "ese" | "aquel") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "tubo"),
                            reason: "Sustantivo (cilindro)".to_string(),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// iba (verbo ir) - "iva" no existe
    fn check_iba(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        if word == "iva" {
            // "iva" no existe como palabra (excepto siglas IVA)
            // Si está en minúsculas, probablemente es "iba"
            if token.text == "iva" {
                return Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "iba"),
                    reason: "Imperfecto de ir (con b)".to_string(),
                });
            }
            // Inicio de oración en mayúscula: "Iva al colegio" -> "Iba al colegio".
            // Mantener conservador para no tocar nombres propios (Iva Morales).
            if token.text == "Iva"
                && prev.is_none()
                && next.is_some_and(Self::is_likely_iba_continuation)
            {
                return Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "iba"),
                    reason: "Imperfecto de ir (con b)".to_string(),
                });
            }
        }
        None
    }

    fn is_likely_iba_continuation(next: &str) -> bool {
        matches!(next, "a" | "al" | "hacia" | "para" | "por" | "en" | "de" | "del")
            || next.ends_with("ando")
            || next.ends_with("iendo")
            || next.ends_with("yendo")
    }

    /// hierba (planta) / hierva (verbo hervir)
    fn check_hierba_hierva(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        _next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "hierva" => {
                // "hierva" es subjuntivo de hervir
                // Error: usar "hierva" en lugar de "hierba" (planta)
                if let Some(p) = prev {
                    // "la hierva" = "la hierba"
                    if matches!(
                        p,
                        "la" | "una" | "esta" | "esa" | "aquella" | "mala" | "buena"
                    ) {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hierba"),
                            reason: "Sustantivo (planta)".to_string(),
                        });
                    }
                }
                None
            }
            "hierba" => {
                // "hierba" es sustantivo
                // Error: usar "hierba" en lugar de "hierva" (verbo)
                if let Some(p) = prev {
                    // "que hierba" = "que hierva"
                    if matches!(p, "que" | "ojalá" | "cuando") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hierva"),
                            reason: "Subjuntivo de hervir".to_string(),
                        });
                    }
                }
                None
            }
            "yerba" | "yerva" => {
                // Variantes, "yerba" es aceptado, "yerva" no
                if word == "yerva" {
                    return Some(HomophoneCorrection {
                        token_index: idx,
                        original: token.text.clone(),
                        suggestion: Self::preserve_case(&token.text, "yerba"),
                        reason: "Variante de hierba".to_string(),
                    });
                }
                None
            }
            _ => None,
        }
    }

    /// bello (hermoso) / vello (pelo)
    fn check_bello_vello(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "vello" => {
                // "vello" es pelo fino
                // Error: usar "vello" en lugar de "bello" (hermoso)
                if let Some(p) = prev {
                    // "muy vello" = "muy bello"
                    if matches!(p, "muy" | "tan" | "qué" | "más" | "menos" | "lo") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "bello"),
                            reason: "Adjetivo (hermoso)".to_string(),
                        });
                    }
                }
                None
            }
            "bello" => {
                // "bello" es adjetivo
                // Error: usar "bello" en lugar de "vello" (pelo)
                if let Some(p) = prev {
                    // "el bello corporal" = "el vello corporal"
                    if matches!(p, "el" | "del" | "con" | "sin") {
                        // Solo si parece contexto de pelo
                        // Difícil de detectar, mejor no corregir
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// botar (saltar/tirar) / votar (elecciones)
    fn check_botar_votar(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "botar" | "boto" | "bota" | "botas" | "botan" | "botó" | "botaron" => {
                // Verbos de botar (saltar/tirar basura)
                // Error: usar "botar" en lugar de "votar"
                if let Some(n) = next {
                    // "botar por" = "votar por"
                    if matches!(n, "por" | "a" | "en") {
                        if let Some(p) = prev {
                            if matches!(
                                p,
                                "voy"
                                    | "vamos"
                                    | "vas"
                                    | "van"
                                    | "ir"
                                    | "para"
                                    | "quiero"
                                    | "puedo"
                                    | "debo"
                            ) {
                                let suggestion = word.replacen('b', "v", 1);
                                return Some(HomophoneCorrection {
                                    token_index: idx,
                                    original: token.text.clone(),
                                    suggestion: Self::preserve_case(&token.text, &suggestion),
                                    reason: "Verbo votar (elecciones)".to_string(),
                                });
                            }
                        }
                    }
                }
                None
            }
            "votar" | "voto" | "vota" | "votas" | "votan" | "votó" | "votaron" => {
                // Verbos de votar
                // Error: usar "votar" en lugar de "botar" (tirar)
                if let Some(n) = next {
                    // "votar la basura" = "botar la basura"
                    if matches!(n, "la" | "el" | "eso" | "esto" | "aquello") {
                        // Verificar si el contexto sugiere "tirar"
                        // Difícil sin más contexto
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Preserva mayusculas del original
    fn preserve_case(original: &str, replacement: &str) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<HomophoneCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        HomophoneAnalyzer::analyze(&tokens)
    }

    // Tests para hay/ahí/ay
    #[test]
    fn test_hay_correct() {
        let corrections = analyze_text("hay mucha gente");
        assert!(
            corrections.is_empty(),
            "No debería corregir 'hay' como verbo"
        );
    }

    #[test]
    fn test_hay_should_be_ahi() {
        let corrections = analyze_text("por hay está");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ahí");
    }

    #[test]
    fn test_ahi_without_accent() {
        let corrections = analyze_text("esta ahi");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ahí");
    }

    #[test]
    fn test_ahi_should_be_hay() {
        let corrections = analyze_text("ahí mucha gente");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hay");
    }

    // Tests para haya/halla
    #[test]
    fn test_halla_should_be_haya() {
        let corrections = analyze_text("que halla llegado");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_halla_should_be_haya_with_interposed_clitic_lo() {
        let corrections = analyze_text("que lo halla hecho");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_halla_should_be_haya_with_interposed_clitic_se() {
        let corrections = analyze_text("que se halla ido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_halla_should_be_haya_with_interposed_clitic_me() {
        let corrections = analyze_text("que me halla visto");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_halla_should_be_haya_with_interposed_negation_no() {
        let corrections = analyze_text("que no halla venido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_halla_should_be_haya_with_interposed_negation_nunca() {
        let corrections = analyze_text("que nunca halla existido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_halla_should_be_haya_with_si_no_sequence() {
        let corrections = analyze_text("si no halla llegado");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_haya_should_be_halla() {
        let corrections = analyze_text("se haya aquí");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "halla");
    }

    #[test]
    fn test_haiga_incorrect() {
        let corrections = analyze_text("que haiga venido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    // Tests para vaya/valla
    #[test]
    fn test_valla_should_be_vaya() {
        let corrections = analyze_text("que valla bien");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vaya");
    }

    #[test]
    fn test_valla_a_should_be_vaya() {
        let corrections = analyze_text("valla a casa");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vaya");
    }

    #[test]
    fn test_baya_should_be_vaya() {
        let corrections = analyze_text("que baya rápido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vaya");
    }

    // Tests para hecho/echo
    #[test]
    fn test_echo_should_be_hecho() {
        let corrections = analyze_text("he echo la tarea");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hecho");
    }

    #[test]
    fn test_de_echo_should_be_de_hecho() {
        let corrections = analyze_text("de echo es así");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hecho");
    }

    #[test]
    fn test_un_echo_should_be_un_hecho() {
        let corrections = analyze_text("es un echo conocido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hecho");
    }

    #[test]
    fn test_el_echo_de_que_should_be_el_hecho_de_que() {
        let corrections = analyze_text("el echo de que no viniera");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hecho");
    }

    #[test]
    fn test_los_echos_should_be_hechos() {
        let corrections = analyze_text("los echos importan");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hechos");
    }

    #[test]
    fn test_son_echos_conocidos_should_be_hechos() {
        let corrections = analyze_text("son echos conocidos");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hechos");
    }

    #[test]
    fn test_hecho_de_menos_should_be_echo() {
        let corrections = analyze_text("hecho de menos a mi familia");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "echo");
    }

    #[test]
    fn test_yo_echo_sal_no_correction() {
        let corrections = analyze_text("yo echo sal");
        assert!(corrections.is_empty(), "No debe tocar 'echo' verbal");
    }

    #[test]
    fn test_haber_si_should_be_a_ver() {
        let corrections = analyze_text("haber si vienes");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "a ver");
    }

    #[test]
    fn test_haber_que_should_be_a_ver() {
        let corrections = analyze_text("haber que pasa");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "a ver");
    }

    #[test]
    fn test_haber_cuando_should_be_a_ver() {
        let corrections = analyze_text("haber cuando llegas");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "a ver");
    }

    #[test]
    fn test_pues_haber_si_should_be_a_ver() {
        let corrections = analyze_text("pues haber si vienes");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "a ver");
    }

    #[test]
    fn test_vamos_haber_que_should_be_a_ver() {
        let corrections = analyze_text("vamos haber que pasa");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "a ver");
    }

    #[test]
    fn test_puede_haber_que_no_a_ver_correction() {
        let corrections = analyze_text("puede haber que esperar");
        assert!(
            corrections.is_empty(),
            "No debe cambiar 'haber' verbal en 'puede haber que esperar'"
        );
    }

    #[test]
    fn test_porque_direct_question_should_be_por_que() {
        let corrections = analyze_text("\u{00BF}porque vienes?");
        assert!(
            corrections.iter().any(|c| c.suggestion == "por qu\u{00E9}"),
            "Debe corregir interrogativo directo 'porque' -> 'por qu\u{00E9}': {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_se_porque_should_be_por_que() {
        let corrections = analyze_text("no se porque vino");
        assert!(
            corrections.iter().any(|c| c.suggestion == "por qu\u{00E9}"),
            "Debe corregir interrogativo indirecto 'no se porque' -> 'no se por qu\u{00E9}': {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_se_por_que_should_accent_que() {
        let corrections = analyze_text("no se por que vino");
        assert!(
            corrections.iter().any(|c| c.suggestion == "qu\u{00E9}"),
            "Debe corregir 'por que' -> 'por qu\u{00E9}' en subordinada interrogativa: {:?}",
            corrections
        );
    }

    #[test]
    fn test_lucho_por_que_should_not_change() {
        let corrections = analyze_text("lucho por que vengas");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'por que' en uso final/relativo no interrogativo: {:?}",
            corrections
        );
    }

    #[test]
    fn test_el_porque_de_should_be_nominal_porque() {
        let corrections = analyze_text("el porque de todo");
        assert!(
            corrections.iter().any(|c| c.suggestion == "porqu\u{00E9}"),
            "Debe corregir sustantivo 'el porque' -> 'el porqu\u{00E9}': {:?}",
            corrections
        );
    }

    #[test]
    fn test_el_porque_de_already_correct() {
        let corrections = analyze_text("el porqu\u{00E9} de todo");
        assert!(
            corrections.is_empty(),
            "No debe tocar sustantivo correctamente acentuado: {:?}",
            corrections
        );
    }

    #[test]
    fn test_question_with_causal_porque_should_not_change() {
        let corrections = analyze_text("\u{00BF}te fuiste porque llovia?");
        assert!(
            corrections.is_empty(),
            "No debe forzar 'por qu\u{00E9}' cuando 'porque' es causal dentro de pregunta: {:?}",
            corrections
        );
    }

    #[test]
    fn test_porque_with_acute_in_causal_context_should_be_because() {
        let corrections = analyze_text("no vine porqu\u{00E9} llovia");
        assert!(
            corrections.iter().any(|c| c.suggestion == "porque"),
            "Debe corregir 'porqu\u{00E9}' causal -> 'porque': {:?}",
            corrections
        );
    }

    #[test]
    fn test_si_no_contrast_should_be_sino() {
        let corrections = analyze_text("no quiero ir, si no quedarme");
        assert!(
            corrections.iter().any(|c| c.suggestion == "sino"),
            "Debe corregir 'si no' -> 'sino' en contraste: {:?}",
            corrections
        );
        assert!(
            corrections.iter().any(|c| c.suggestion == "sobra"),
            "Debe marcar el 'no' sobrante al fusionar 'sino': {:?}",
            corrections
        );
    }

    #[test]
    fn test_si_no_conditional_should_not_change() {
        let corrections = analyze_text("si no vienes me voy");
        assert!(
            corrections.is_empty(),
            "No debe corregir condicional negativo 'si no + verbo': {:?}",
            corrections
        );
    }

    #[test]
    fn test_sino_conditional_should_be_si_no() {
        let corrections = analyze_text("sino vienes me voy");
        assert!(
            corrections.iter().any(|c| c.suggestion == "si no"),
            "Debe corregir 'sino + verbo' -> 'si no + verbo': {:?}",
            corrections
        );
    }

    #[test]
    fn test_sino_como_conditional_should_be_si_no() {
        let corrections = analyze_text("sino como me muero");
        assert!(
            corrections.iter().any(|c| c.suggestion == "si no"),
            "Debe corregir 'sino como ...' -> 'si no como ...': {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_explico_porque_causal_should_not_change() {
        let corrections = analyze_text("no explico porque estoy cansado");
        assert!(
            corrections.is_empty(),
            "No debe forzar interrogativo en causal con 'no explico porque ...': {:?}",
            corrections
        );
    }

    #[test]
    fn test_voy_ha_comprar_should_be_voy_a_comprar() {
        let corrections = analyze_text("voy ha comprar pan");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "a");
    }

    #[test]
    fn test_voy_ha_comprarlo_should_be_voy_a_comprarlo() {
        let corrections = analyze_text("voy ha comprarlo");
        assert!(
            corrections.iter().any(|c| c.suggestion == "a"),
            "Debe corregir 'ha' -> 'a' antes de infinitivo con enclítico: {:?}",
            corrections
        );
    }

    #[test]
    fn test_se_fue_ha_verlo_should_be_se_fue_a_verlo() {
        let corrections = analyze_text("se fue ha verlo");
        assert!(
            corrections.iter().any(|c| c.suggestion == "a"),
            "Debe corregir 'ha' -> 'a' antes de infinitivo con enclítico: {:?}",
            corrections
        );
    }

    #[test]
    fn test_fue_ha_ver_should_be_fue_a_ver() {
        let corrections = analyze_text("fue ha ver al medico");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "a");
    }

    #[test]
    fn test_ha_comido_no_correction() {
        let corrections = analyze_text("ha comido");
        assert!(corrections.is_empty(), "No debe tocar 'ha + participio'");
    }

    #[test]
    fn test_ha_de_venir_no_correction() {
        let corrections = analyze_text("ha de venir");
        assert!(
            corrections.is_empty(),
            "No debe tocar perífrasis 'ha de + infinitivo'"
        );
    }

    #[test]
    fn test_se_a_ido_should_be_se_ha_ido() {
        let corrections = analyze_text("se a ido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ha");
    }

    #[test]
    fn test_me_a_dicho_should_be_me_ha_dicho() {
        let corrections = analyze_text("me a dicho");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ha");
    }

    #[test]
    fn test_maria_no_a_llamado_should_be_ha() {
        let corrections = analyze_text("María no a llamado");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'María no a llamado' a 'ha': {:?}",
            corrections
        );
    }

    #[test]
    fn test_el_no_a_comido_should_be_ha() {
        let corrections = analyze_text("el no a comido");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'el no a comido' a 'ha': {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_a_hecho_nada_should_be_ha() {
        let corrections = analyze_text("No a hecho nada");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'No a hecho nada' a 'ha': {:?}",
            corrections
        );
    }

    #[test]
    fn test_haz_visto_should_be_has() {
        let corrections = analyze_text("haz visto eso");
        let haz_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("haz") && c.suggestion.eq_ignore_ascii_case("has")
        });
        assert!(
            haz_correction.is_some(),
            "Debe corregir 'haz visto' a 'has visto': {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_haz_hecho_should_be_has() {
        let corrections = analyze_text("no haz hecho nada");
        let haz_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("haz") && c.suggestion.eq_ignore_ascii_case("has")
        });
        assert!(
            haz_correction.is_some(),
            "Debe corregir 'no haz hecho' a 'no has hecho': {:?}",
            corrections
        );
    }

    #[test]
    fn test_haz_imperative_no_correction() {
        let corrections = analyze_text("haz la tarea");
        let haz_correction = corrections
            .iter()
            .any(|c| c.original.eq_ignore_ascii_case("haz"));
        assert!(
            !haz_correction,
            "No debe tocar el imperativo valido 'haz la tarea': {:?}",
            corrections
        );
    }

    #[test]
    fn test_nunca_a_dicho_eso_should_be_ha() {
        let corrections = analyze_text("Nunca a dicho eso");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'Nunca a dicho eso' a 'ha': {:?}",
            corrections
        );
    }

    #[test]
    fn test_aun_no_a_terminado_should_be_ha() {
        let corrections = analyze_text("Aún no a terminado");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'Aún no a terminado' a 'ha': {:?}",
            corrections
        );
    }

    #[test]
    fn test_sentence_start_a_echo_should_be_ha() {
        let corrections = analyze_text("A echo su tarea");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("A") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'A' inicial a 'Ha' ante participio: {:?}",
            corrections
        );
    }

    #[test]
    fn test_yo_a_venido_should_match_subject_auxiliary() {
        let corrections = analyze_text("yo a venido temprano");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("he")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'yo a venido' a auxiliar 'he': {:?}",
            corrections
        );
    }

    #[test]
    fn test_tu_a_venido_should_match_subject_auxiliary() {
        let corrections = analyze_text("tú a venido temprano");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("has")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'tú a venido' a auxiliar 'has': {:?}",
            corrections
        );
    }

    #[test]
    fn test_nosotros_a_venido_should_match_subject_auxiliary() {
        let corrections = analyze_text("nosotros a venido temprano");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("hemos")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'nosotros a venido' a auxiliar 'hemos': {:?}",
            corrections
        );
    }

    #[test]
    fn test_ellos_a_venido_should_match_subject_auxiliary() {
        let corrections = analyze_text("ellos a venido temprano");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("han")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'ellos a venido' a auxiliar 'han': {:?}",
            corrections
        );
    }

    #[test]
    fn test_nominal_singular_a_venido_should_match_subject_auxiliary() {
        let corrections = analyze_text("la gente a venido temprano");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'la gente a venido' a auxiliar 'ha': {:?}",
            corrections
        );
    }

    #[test]
    fn test_nominal_plural_a_venido_should_match_subject_auxiliary() {
        let corrections = analyze_text("los niños a venido temprano");
        let a_correction = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("han")
        });
        assert!(
            a_correction.is_some(),
            "Debe corregir 'los niños a venido' a auxiliar 'han': {:?}",
            corrections
        );
    }

    #[test]
    fn test_temporal_plural_a_venido_should_prefer_ha_not_han() {
        let corrections = analyze_text("estos días a venido mucha gente");
        let a_to_ha = corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        let a_to_han = corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("han")
        });
        assert!(
            a_to_ha,
            "Debe corregir 'a' por 'ha' en complemento temporal: {:?}",
            corrections
        );
        assert!(
            !a_to_han,
            "No debe forzar 'han' por temporal plural inicial: {:?}",
            corrections
        );
    }

    #[test]
    fn test_proper_name_subject_a_venido_should_match_auxiliary() {
        let corrections = analyze_text("Juan a venido tarde");
        let a_to_ha = corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            a_to_ha,
            "Debe corregir 'Juan a venido' con auxiliar 'ha': {:?}",
            corrections
        );
    }

    #[test]
    fn test_a_lado_no_false_ha() {
        let corrections = analyze_text("estoy a lado de casa");
        let false_ha = corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            !false_ha,
            "No debe cambiar preposición 'a' por 'ha' en contexto nominal: {:?}",
            corrections
        );
    }

    #[test]
    fn test_a_grosso_modo_no_false_ha() {
        let corrections = analyze_text("a grosso modo");
        let false_ha = corrections.iter().any(|c| {
            c.original.eq_ignore_ascii_case("a") && c.suggestion.eq_ignore_ascii_case("ha")
        });
        assert!(
            !false_ha,
            "No debe cambiar 'a' por 'ha' en locucion 'a grosso modo': {:?}",
            corrections
        );
    }

    #[test]
    fn test_voy_a_casa_no_a_ha_correction() {
        let corrections = analyze_text("voy a casa");
        assert!(
            corrections.is_empty(),
            "No debe cambiar preposición 'a' por 'ha'"
        );
    }

    #[test]
    fn test_boy_a_ir_should_be_voy() {
        let corrections = analyze_text("boy a ir");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "voy");
    }

    #[test]
    fn test_boy_al_cine_should_be_voy() {
        let corrections = analyze_text("boy al cine");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "voy");
    }

    #[test]
    fn test_boy_scout_no_correction() {
        let corrections = analyze_text("el boy scout llego");
        assert!(
            corrections.is_empty(),
            "No debe tocar anglicismo nominal 'boy'"
        );
    }

    // Tests para tuvo/tubo
    #[test]
    fn test_tubo_should_be_tuvo() {
        let corrections = analyze_text("tubo que salir");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tuvo");
    }

    #[test]
    fn test_tuvo_should_be_tubo() {
        let corrections = analyze_text("el tuvo de metal");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tubo");
    }

    // Tests para iba/iva
    #[test]
    fn test_iva_should_be_iba() {
        let corrections = analyze_text("iva caminando");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "iba");
    }

    #[test]
    fn test_iva_sentence_start_capitalized_should_be_iba() {
        let corrections = analyze_text("Iva al colegio");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Iba");
    }

    #[test]
    fn test_iva_proper_name_no_correction() {
        let corrections = analyze_text("Iva Morales vino");
        assert!(corrections.is_empty());
    }

    // Tests para hierba/hierva
    #[test]
    fn test_hierva_should_be_hierba() {
        let corrections = analyze_text("la hierva verde");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hierba");
    }

    #[test]
    fn test_hierba_should_be_hierva() {
        let corrections = analyze_text("que hierba el agua");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hierva");
    }

    #[test]
    fn test_yerva_incorrect() {
        let corrections = analyze_text("la yerva");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "yerba");
    }

    // Tests para bello/vello
    #[test]
    fn test_vello_should_be_bello() {
        let corrections = analyze_text("muy vello");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "bello");
    }

    // Test de preservacion de mayusculas
    #[test]
    fn test_preserve_case() {
        let corrections = analyze_text("Haiga venido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Haya");
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "por" y "hay" estan separados por punto, no debe sugerir "ahi"
        let corrections = analyze_text("Vino por. Hay mucha gente");
        let ahi_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.suggestion == "ahi" || c.suggestion == "ahí")
            .collect();
        assert!(
            ahi_corrections.is_empty(),
            "No debe corregir 'hay' cuando hay limite de oracion"
        );
    }
}
