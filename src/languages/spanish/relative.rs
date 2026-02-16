//! Analisis de concordancia de pronombres relativos
//!
//! Detecta errores de concordancia entre el antecedente y el verbo en oraciones de relativo.
//! Ejemplo: "la persona que vinieron" -> "la persona que vino"
//!          "los ninos que llego" -> "los ninos que llegaron"

use crate::dictionary::{Number, WordCategory};
use crate::grammar::has_sentence_boundary as has_sentence_boundary_slow;
use crate::grammar::tokenizer::TokenType;
use crate::grammar::{SentenceBoundaryIndex, Token};
use crate::languages::spanish::conjugation::stem_changing::{
    fix_stem_changed_infinitive as fix_stem_changed_infinitive_shared, get_stem_changing_verbs,
    StemChangeType,
};
use crate::languages::spanish::exceptions;
use crate::languages::VerbFormRecognizer;
use std::cell::RefCell;

/// Corrección de concordancia de relativos
#[derive(Debug, Clone)]
pub struct RelativeCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub message: String,
}

/// Tiempo verbal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Tense {
    Present,
    Preterite,
    Imperfect,
    Future,
}

/// Analizador de concordancia de relativos
pub struct RelativeAnalyzer;

struct BoundaryCacheEntry {
    ptr: *const Token,
    len: usize,
    index: SentenceBoundaryIndex,
}

thread_local! {
    static BOUNDARY_CACHE: RefCell<Option<BoundaryCacheEntry>> = const { RefCell::new(None) };
}

struct BoundaryCacheGuard;

impl BoundaryCacheGuard {
    fn new(tokens: &[Token]) -> Self {
        BOUNDARY_CACHE.with(|cache| {
            *cache.borrow_mut() = Some(BoundaryCacheEntry {
                ptr: tokens.as_ptr(),
                len: tokens.len(),
                index: SentenceBoundaryIndex::new(tokens),
            });
        });
        Self
    }
}

impl Drop for BoundaryCacheGuard {
    fn drop(&mut self) {
        BOUNDARY_CACHE.with(|cache| {
            *cache.borrow_mut() = None;
        });
    }
}

#[inline]
fn has_sentence_boundary(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
    BOUNDARY_CACHE.with(|cache| {
        if let Some(entry) = cache.borrow().as_ref() {
            if entry.ptr == tokens.as_ptr() && entry.len == tokens.len() {
                return entry.index.has_between(start_idx, end_idx);
            }
        }
        has_sentence_boundary_slow(tokens, start_idx, end_idx)
    })
}

impl RelativeAnalyzer {
    /// Analiza tokens buscando errores de concordancia en oraciones de relativo
    pub fn analyze(tokens: &[Token]) -> Vec<RelativeCorrection> {
        Self::analyze_with_recognizer(tokens, None)
    }

    /// Analiza tokens con VerbRecognizer opcional para desambiguar formas verbales
    pub fn analyze_with_recognizer(
        tokens: &[Token],
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<RelativeCorrection> {
        let _boundary_cache_guard = BoundaryCacheGuard::new(tokens);
        let mut corrections = Vec::new();

        // Obtener solo tokens de palabras con sus índices originales
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Buscar patrón: sustantivo + [adjetivo]* + "que" + verbo
        // También maneja: sustantivo1 + "de" + sustantivo2 + "que" + verbo
        // y casos con adverbio comparativo intercalado: "que mejor/más/menos + verbo".
        for i in 0..word_tokens.len().saturating_sub(2) {
            let (_, relative) = word_tokens[i + 1];

            // Verificar si es "que" + verbo
            if !Self::is_relative_pronoun(&relative.text) {
                continue;
            }
            // "puesto que", "dado que", "ya que" suelen introducir causales/completivas,
            // no relativas con antecedente nominal inmediato.
            if Self::is_causal_que_conjunction_context(&word_tokens, i + 1, tokens) {
                continue;
            }
            let Some(verb_pos) =
                Self::find_relative_verb_position(&word_tokens, i + 1, verb_recognizer)
            else {
                continue;
            };
            let (verb_idx, verb) = word_tokens[verb_pos];

            // Buscar el sustantivo antecedente, saltando adjetivos
            // Ejemplo: "enfoques integrales que incluyan" -> antecedente = "enfoques"
            //
            // Para cláusulas explicativas (coma antes de "que"), usar ventana extendida
            // PERO solo si el token antes de "que" es nominal (sustantivo/adjetivo).
            // Si es verbo ("dijo, que"), NO usar ventana extendida (es "que" completivo).
            let has_comma = Self::has_comma_before_que(&word_tokens, i + 1, tokens);

            // Primero buscar con ventana corta
            let short_window_result = Self::find_noun_before_position(&word_tokens, i);
            let has_nominal_context =
                Self::is_noun(short_window_result) || Self::is_adjective(short_window_result);

            let mut potential_antecedent = if has_comma && has_nominal_context {
                // Cláusula explicativa con contexto nominal: buscar con ventana extendida
                match Self::find_noun_extended_window(&word_tokens, i, tokens) {
                    Some(noun) => noun,
                    None => short_window_result, // Fallback a ventana corta
                }
            } else {
                short_window_result
            };
            let antecedent_anchor_pos = if has_comma {
                if let Some(anchor_pos) =
                    Self::find_antecedent_anchor_before_como_example(&word_tokens, i, tokens)
                {
                    potential_antecedent = word_tokens[anchor_pos].1;
                    anchor_pos
                } else {
                    i
                }
            } else {
                i
            };

            // Caso eliptico: "uno/una de los/las que + verbo".
            // Forzamos antecedente plural ("los/las").
            let forced_plural_antecedent =
                Self::find_uno_de_article_relative_antecedent(&word_tokens, i + 1);

            // Evitar que un verbo finito mal etiquetado como sustantivo
            // (p. ej. "viéramos") se use como antecedente.
            if forced_plural_antecedent.is_none()
                && Self::is_spurious_nominal_antecedent_before_que(
                    potential_antecedent,
                    verb_recognizer,
                )
            {
                continue;
            }

            // Verificar si encontramos un sustantivo
            if !Self::is_noun(potential_antecedent) && forced_plural_antecedent.is_none() {
                continue;
            }
            // "decir/explicar/... a X que Y": aquí "que" es completiva, no relativo.
            // Evita falsos positivos tipo:
            // "se les dice a cualquier otra persona que son inteligentes".
            if Self::is_completive_que_after_indirect_object(
                &word_tokens,
                i,
                tokens,
                verb_recognizer,
            ) {
                continue;
            }

            // Filtrar subjuntivo exhortativo: "¡Que vengan todos!", "Que lo hagan"
            // Si "que" está al inicio de oración y el verbo parece subjuntivo,
            // probablemente no es un relativo sino una expresión desiderativa/exhortativa
            let verb_lower = verb.effective_text().to_lowercase();
            if Self::is_likely_exhortative_que(&word_tokens, i + 1, tokens, &verb_lower) {
                continue;
            }

            // Excluir tiempos compuestos: "que han permitido", "que ha hecho"
            // El auxiliar "haber" no debe analizarse para concordancia de relativos
            // porque la concordancia ya está determinada por el sujeto, no el antecedente
            let verb_lower = verb.effective_text().to_lowercase();
            if Self::is_haber_auxiliary(&verb_lower) {
                continue;
            }
            // Verificar que no haya puntuación de fin de oración entre antecedente y relativo
            let (ant_idx, _) = word_tokens[i];
            let (rel_idx_check, _) = word_tokens[i + 1];
            if has_sentence_boundary(tokens, ant_idx, rel_idx_check) {
                continue;
            }

            // Verificar si hay un inciso parentético con conector + verbo entre antecedente y "que"
            // Ejemplo: "Las medidas, según explicó el ministro, que aprobó..."
            // En estos casos, el verbo del relativo tiene su propio sujeto implícito
            if Self::has_parenthetical_clause(&word_tokens, i, i + 1) {
                continue;
            }

            // Verificar si después del verbo hay un sujeto propio (det/poss + noun)
            // Ejemplo: "las necesidades que tiene nuestra población"
            // En este caso, "población" es el sujeto de "tiene", no "necesidades"
            if Self::has_own_subject_after_verb(&word_tokens, verb_pos, tokens) {
                continue;
            }
            // Sujeto explícito antepuesto al verbo dentro de la subordinada:
            // "los coches que ella conduce", "los libros que María compró".
            // En estos casos, el antecedente funciona como objeto y no debe forzarse concordancia.
            if Self::has_own_subject_before_verb(&word_tokens, i + 1, verb_pos) {
                continue;
            }

            // Verbos copulativos (ser/estar) con predicativo plural:
            // Ejemplo: "la mortalidad, que son muertes causadas..."
            // La concordancia puede ser con el predicativo, no el antecedente
            if Self::is_copulative_with_plural_predicate(&word_tokens, verb_pos) {
                continue;
            }

            // Verbo + participio que concuerda con el antecedente:
            // Ejemplo: "las misiones que tiene previstas" - "misiones" es objeto directo
            // El sujeto de "tiene" es implícito y diferente del antecedente
            if Self::is_verb_with_agreeing_participle(&word_tokens, verb_pos, potential_antecedent)
            {
                continue;
            }

            // Buscar si hay un patrón "noun1 de [adj/num]* noun2 que verb"
            // En ese caso, el verdadero antecedente es noun1, no noun2
            // PERO: si noun2 ya concuerda con el verbo, mantener noun2 como antecedente
            // (esto evita falsos positivos como "trabajo de equipos que aportan")
            // Ejemplos donde noun1 es antecedente:
            //   "marcos de referencia que sirven" → referencia (s) vs sirven (p) → ant = marcos
            // Ejemplos donde noun2 es antecedente:
            //   "trabajo de equipos que aportan" → equipos (p) vs aportan (p) → ant = equipos
            let verb_info = Self::get_verb_info_with_tense(&verb_lower, verb_recognizer);
            let relative_norm = Self::normalize_spanish(&relative.effective_text().to_lowercase());
            let require_human_relative = matches!(relative_norm.as_str(), "quien" | "quienes");
            let mut antecedent = {
                if let Some(forced) = forced_plural_antecedent {
                    forced
                } else {
                    let noun2_number = Self::get_antecedent_number(potential_antecedent);

                    // Si noun2 concuerda con el verbo, usarlo directamente
                    if let (Some(n2_num), Some((v_num, _, _))) =
                        (noun2_number, verb_info.as_ref())
                    {
                        if n2_num == *v_num && n2_num != Number::None {
                            potential_antecedent
                        } else {
                            Self::find_true_antecedent(
                                &word_tokens,
                                antecedent_anchor_pos,
                                potential_antecedent,
                                tokens,
                            )
                        }
                    } else {
                        Self::find_true_antecedent(
                            &word_tokens,
                            antecedent_anchor_pos,
                            potential_antecedent,
                            tokens,
                        )
                    }
                }
            };
            let target_number = verb_info.as_ref().map(|(n, _, _)| *n);
            let antecedent_number = Self::get_antecedent_number(antecedent);
            let number_mismatch = target_number.is_some_and(|target| {
                antecedent_number.is_some_and(|num| num != Number::None && num != target)
            });
            let human_mismatch = require_human_relative && !Self::is_human_like_token(antecedent);
            if (has_comma || require_human_relative) && (number_mismatch || human_mismatch) {
                if let Some(better) = Self::find_better_antecedent_before_relative(
                    &word_tokens,
                    i,
                    tokens,
                    target_number,
                    require_human_relative,
                ) {
                    antecedent = better;
                }
            }

            if let Some(correction) = Self::check_verb_agreement(
                verb_idx,
                antecedent,
                verb,
                verb_recognizer,
                has_comma,
                &word_tokens,
                verb_pos,
                tokens,
            ) {
                corrections.push(correction);
            }
        }

        // Buscar patrón: sustantivo + "quien"/"quienes" (concordancia del relativo)
        for i in 0..word_tokens.len().saturating_sub(1) {
            let (_, raw_antecedent) = word_tokens[i];
            let (rel_idx, relative) = word_tokens[i + 1];
            let antecedent_idx = word_tokens[i].0;

            // No propagar concordancia quien/quienes entre oraciones:
            // "Pidieron disculpas. Quien..." no debe mirar "disculpas".
            if has_sentence_boundary(tokens, antecedent_idx, rel_idx) {
                continue;
            }

            if Self::is_noun(raw_antecedent) {
                let mut antecedent =
                    Self::find_true_antecedent(&word_tokens, i, raw_antecedent, tokens);
                let rel_norm = Self::normalize_spanish(&relative.effective_text().to_lowercase());
                let target_number = match rel_norm.as_str() {
                    "quien" => Some(Number::Singular),
                    "quienes" => Some(Number::Plural),
                    _ => None,
                };
                if let Some(better) = Self::find_better_antecedent_before_relative(
                    &word_tokens,
                    i,
                    tokens,
                    target_number,
                    true,
                ) {
                    antecedent = better;
                }
                // Excluir locuciones prepositivas como "al final quienes", "por fin quienes"
                // En estos casos, "quienes" es un relativo libre, no refiere al sustantivo anterior
                if i > 0 {
                    let (_, prev_word) = word_tokens[i - 1];
                    let prev_lower = prev_word.effective_text().to_lowercase();
                    // Si está precedido por "al", "por", "en", etc., probablemente es locución
                    if matches!(
                        prev_lower.as_str(),
                        "al" | "del" | "por" | "en" | "con" | "sin"
                    ) {
                        continue;
                    }
                }

                if let Some(correction) = Self::check_quien_agreement(rel_idx, antecedent, relative)
                {
                    corrections.push(correction);
                }
            }
        }

        corrections
    }

    fn is_subject_pronoun_word(word: &str) -> bool {
        matches!(
            word,
            "yo" | "tu"
                | "tú"
                | "el"
                | "él"
                | "ella"
                | "ello"
                | "usted"
                | "ustedes"
                | "vos"
                | "vosotros"
                | "vosotras"
                | "nosotros"
                | "nosotras"
                | "ellos"
                | "ellas"
        )
    }

    fn is_spurious_nominal_verb_form(token: &Token, lower: &str) -> bool {
        let looks_like_finite = lower.ends_with("ía")
            || lower.ends_with("ian")
            || lower.ends_with("ían")
            || lower.ends_with("ría")
            || lower.ends_with("rian")
            || lower.ends_with("rían")
            || lower.ends_with("aba")
            || lower.ends_with("aban")
            || lower.ends_with("aron")
            || lower.ends_with("ieron")
            || lower.ends_with("ió")
            || lower.ends_with("io")
            || lower.ends_with("amos")
            || lower.ends_with("emos")
            || lower.ends_with("imos");

        token.word_info.as_ref().is_some_and(|info| {
            info.category == WordCategory::Sustantivo && info.extra.is_empty() && looks_like_finite
        })
    }

    fn is_spurious_nominal_antecedent_before_que(
        token: &Token,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let lower = token.effective_text().to_lowercase();
        if !Self::is_spurious_nominal_verb_form(token, &lower) {
            return false;
        }

        verb_recognizer
            .map(|vr| vr.is_valid_verb_form(&lower))
            .unwrap_or_else(|| Self::detect_verb_info(&lower, None).is_some())
    }

    fn has_own_subject_before_verb(
        word_tokens: &[(usize, &Token)],
        relative_pos: usize,
        verb_pos: usize,
    ) -> bool {
        if verb_pos <= relative_pos + 1 {
            return false;
        }

        let mut pos = relative_pos + 1;
        while pos < verb_pos {
            let (_, token) = word_tokens[pos];
            let lower = token.effective_text().to_lowercase();

            if Self::is_subject_pronoun_word(&lower) {
                return true;
            }

            let is_capitalized = token
                .effective_text()
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            if is_capitalized {
                return true;
            }

            if token
                .word_info
                .as_ref()
                .is_some_and(|info| info.category == WordCategory::Sustantivo)
            {
                return true;
            }

            if token.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Articulo | WordCategory::Determinante
                )
            }) && pos + 1 < verb_pos
                && word_tokens[pos + 1]
                    .1
                    .word_info
                    .as_ref()
                    .is_some_and(|next_info| next_info.category == WordCategory::Sustantivo)
            {
                return true;
            }

            pos += 1;
        }

        false
    }

    /// Detecta el patron eliptico "uno/una de los/las que ...".
    /// Retorna "los/las" para forzar concordancia plural del relativo.
    fn find_uno_de_article_relative_antecedent<'a>(
        word_tokens: &[(usize, &'a Token)],
        relative_pos: usize,
    ) -> Option<&'a Token> {
        if relative_pos < 3 {
            return None;
        }

        let mut article_pos = relative_pos - 1;
        while article_pos > 0 {
            let (_, candidate) = word_tokens[article_pos];
            let candidate_lower = candidate.effective_text().to_lowercase();
            let is_modifier_between_article_and_que = Self::is_adjective(candidate)
                || Self::is_participle_like_modifier(candidate)
                || Self::is_mente_adverb(candidate)
                || Self::is_interposed_comparative_adverb(&candidate_lower);
            if !is_modifier_between_article_and_que {
                break;
            }
            article_pos -= 1;
        }

        if article_pos < 2 {
            return None;
        }

        let (_, article) = word_tokens[article_pos];
        let article_lower = article.effective_text().to_lowercase();
        if !matches!(article_lower.as_str(), "los" | "las") {
            return None;
        }

        let (_, de_token) = word_tokens[article_pos - 1];
        let de_lower = de_token.effective_text().to_lowercase();
        if de_lower != "de" {
            return None;
        }

        let (_, uno_token) = word_tokens[article_pos - 2];
        let uno_lower = uno_token.effective_text().to_lowercase();
        if !matches!(uno_lower.as_str(), "uno" | "una") {
            return None;
        }

        Some(article)
    }

    /// Obtiene la posicion del verbo principal tras un relativo.
    /// Acepta adverbio comparativo intercalado: "que mejor/más/menos/peor juega".
    fn find_relative_verb_position(
        word_tokens: &[(usize, &Token)],
        relative_pos: usize,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Option<usize> {
        if relative_pos + 1 >= word_tokens.len() {
            return None;
        }

        let mut start = relative_pos + 1;
        let (_, after_relative) = word_tokens[start];
        let after_lower = after_relative.effective_text().to_lowercase();
        if Self::is_interposed_comparative_adverb(&after_lower) {
            start += 1;
        }
        if start >= word_tokens.len() {
            return None;
        }

        let max_lookahead = 6usize;
        for probe in start..word_tokens.len().min(start + max_lookahead) {
            let (_, candidate) = word_tokens[probe];
            let candidate_lower = candidate.effective_text().to_lowercase();
            let is_capitalized = candidate
                .effective_text()
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);

            if candidate_lower.ends_with("mente") {
                continue;
            }
            if Self::is_subject_pronoun_word(&candidate_lower) {
                continue;
            }
            if is_capitalized {
                continue;
            }

            if candidate
                .word_info
                .as_ref()
                .is_some_and(|info| info.category == WordCategory::Verbo)
            {
                return Some(probe);
            }

            if verb_recognizer.is_some_and(|vr| vr.is_valid_verb_form(&candidate_lower)) {
                return Some(probe);
            }

            if candidate.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Sustantivo
                        | WordCategory::Pronombre
                        | WordCategory::Articulo
                        | WordCategory::Determinante
                )
            }) && !Self::is_spurious_nominal_verb_form(candidate, &candidate_lower)
            {
                continue;
            }

            if Self::detect_verb_info(&candidate_lower, verb_recognizer).is_some() {
                return Some(probe);
            }

            // Saltar sujetos explícitos frecuentes entre "que" y verbo:
            // "que María compró", "que ella conduce".
            if candidate.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Sustantivo
                        | WordCategory::Pronombre
                        | WordCategory::Articulo
                        | WordCategory::Determinante
                )
            }) || candidate_lower.ends_with("mente")
            {
                continue;
            }
        }

        None
    }

    fn is_interposed_comparative_adverb(word: &str) -> bool {
        matches!(word, "mejor" | "peor" | "más" | "mas" | "menos")
    }

    /// Verifica si el token es un sustantivo
    fn is_noun(token: &Token) -> bool {
        if let Some(ref info) = token.word_info {
            info.category == WordCategory::Sustantivo
        } else {
            false
        }
    }

    /// Verifica si el token es un adjetivo
    fn is_adjective(token: &Token) -> bool {
        if let Some(ref info) = token.word_info {
            info.category == WordCategory::Adjetivo
        } else {
            false
        }
    }

    fn is_mente_adverb(token: &Token) -> bool {
        token.effective_text().to_lowercase().ends_with("mente")
    }

    fn is_participle_like_modifier(token: &Token) -> bool {
        let lower = token.effective_text().to_lowercase();
        matches!(
            lower.as_str(),
            _ if lower.ends_with("ado")
                || lower.ends_with("ada")
                || lower.ends_with("ados")
                || lower.ends_with("adas")
                || lower.ends_with("ido")
                || lower.ends_with("ida")
                || lower.ends_with("idos")
                || lower.ends_with("idas")
                || lower.ends_with("to")
                || lower.ends_with("ta")
                || lower.ends_with("tos")
                || lower.ends_with("tas")
                || lower.ends_with("so")
                || lower.ends_with("sa")
                || lower.ends_with("sos")
                || lower.ends_with("sas")
                || lower.ends_with("cho")
                || lower.ends_with("cha")
                || lower.ends_with("chos")
                || lower.ends_with("chas")
        )
    }

    /// Busca el sustantivo antecedente antes de una posición, saltando adjetivos
    /// Ejemplo: "enfoques integrales que" -> pos apunta a "integrales", retorna "enfoques"
    fn find_noun_before_position<'a>(word_tokens: &[(usize, &'a Token)], pos: usize) -> &'a Token {
        // Empezar desde la posición actual
        let (_, current) = word_tokens[pos];

        // Si la posición actual ya es un sustantivo, retornarlo.
        // Excepción: adverbios en -mente mal etiquetados como sustantivo.
        if Self::is_noun(current) && !Self::is_mente_adverb(current) {
            return current;
        }

        // Si la posición actual es adjetivo o adverbio en -mente, buscar hacia atrás el sustantivo.
        // Ejemplos:
        // - "ratones modificados genéticamente que ..." -> antecedente = ratones
        // - "problemas graves internacionales que ..." -> antecedente = problemas
        if (Self::is_adjective(current)
            || Self::is_mente_adverb(current)
            || Self::is_participle_like_modifier(current))
            && pos > 0
        {
            // Buscar hacia atrás saltando modificadores adjetivales y adverbios en -mente.
            // Máximo 4 posiciones (noun + participio + adverbio -mente + adjetivo).
            let max_lookback = 4.min(pos);
            for offset in 1..=max_lookback {
                let check_pos = pos - offset;
                let (_, candidate) = word_tokens[check_pos];

                if Self::is_noun(candidate) && !Self::is_mente_adverb(candidate) {
                    // En secuencias "NOUN + MOD + esos/estas + que ...", el demostrativo
                    // suele apuntar al antecedente principal (a menudo plural), no al
                    // modificador inmediato:
                    // "premios Nobel esos que reconocen ..."
                    if let Some(demo_number) =
                        Self::demonstrative_surface_number(current.effective_text())
                    {
                        if let Some(info) = candidate.word_info.as_ref() {
                            if info.number != Number::None && info.number != demo_number {
                                continue;
                            }
                        }
                    }
                    return candidate;
                }

                // Si encontramos algo que no es modificador nominal, parar
                if !Self::is_adjective(candidate)
                    && !Self::is_mente_adverb(candidate)
                    && !Self::is_participle_like_modifier(candidate)
                {
                    break;
                }
            }
        }

        // Si no encontramos sustantivo, retornar el token original
        current
    }

    fn demonstrative_surface_number(word: &str) -> Option<Number> {
        match word.to_lowercase().as_str() {
            "este" | "esta" | "ese" | "esa" | "aquel" | "aquella" => Some(Number::Singular),
            "estos" | "estas" | "esos" | "esas" | "aquellos" | "aquellas" => {
                Some(Number::Plural)
            }
            _ => None,
        }
    }

    fn is_completive_que_after_indirect_object(
        word_tokens: &[(usize, &Token)],
        antecedent_pos: usize,
        all_tokens: &[Token],
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if antecedent_pos + 1 >= word_tokens.len() {
            return false;
        }
        let (_, que_token) = word_tokens[antecedent_pos + 1];
        if Self::normalize_spanish(&que_token.effective_text().to_lowercase()) != "que" {
            return false;
        }

        // Retroceder sobre modificadores nominales inmediatos: "cualquier otra persona".
        let mut np_start = antecedent_pos;
        while np_start > 0 {
            let (prev_idx, prev_token) = word_tokens[np_start - 1];
            let (curr_idx, _) = word_tokens[np_start];
            if has_sentence_boundary(all_tokens, prev_idx, curr_idx) {
                break;
            }

            let prev_is_modifier = prev_token.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Articulo | WordCategory::Determinante | WordCategory::Adjetivo
                )
            });
            if !prev_is_modifier {
                break;
            }
            np_start -= 1;
        }

        if np_start == 0 {
            return false;
        }

        let (prep_idx, prep_token) = word_tokens[np_start - 1];
        let prep_lower = Self::normalize_spanish(&prep_token.effective_text().to_lowercase());
        if !matches!(prep_lower.as_str(), "a" | "al") {
            return false;
        }

        // Buscar verbo rector antes de "a + SN", saltando clíticos.
        let mut scan = np_start as isize - 2;
        let mut scanned = 0usize;
        while scan >= 0 && scanned < 14 {
            let (idx, token) = word_tokens[scan as usize];
            if has_sentence_boundary(all_tokens, idx, prep_idx) {
                break;
            }

            let lower = Self::normalize_spanish(&token.effective_text().to_lowercase());
            if matches!(
                lower.as_str(),
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
                    | "también"
                    | "o"
                    | "u"
                    | "y"
                    | "e"
                    | "ni"
                    | "a"
                    | "al"
                    | "de"
                    | "del"
            ) {
                scan -= 1;
                scanned += 1;
                continue;
            }

            if Self::is_reporting_verb_form(token, &lower, verb_recognizer) {
                return true;
            }

            let looks_like_non_reporting_verb = token
                .word_info
                .as_ref()
                .is_some_and(|info| info.category == WordCategory::Verbo)
                || verb_recognizer
                    .map(|vr| vr.is_valid_verb_form(&lower))
                    .unwrap_or(false);
            if looks_like_non_reporting_verb {
                return false;
            }

            let is_nominal_or_function = token.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Sustantivo
                        | WordCategory::Determinante
                        | WordCategory::Articulo
                        | WordCategory::Adjetivo
                        | WordCategory::Pronombre
                        | WordCategory::Preposicion
                        | WordCategory::Conjuncion
                        | WordCategory::Adverbio
                )
            });
            if is_nominal_or_function {
                scan -= 1;
                scanned += 1;
                continue;
            }

            return false;
        }

        false
    }

    fn is_reporting_verb_form(
        token: &Token,
        lower: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if !token
            .word_info
            .as_ref()
            .is_some_and(|info| info.category == WordCategory::Verbo)
            && !verb_recognizer
                .map(|vr| vr.is_valid_verb_form(lower))
                .unwrap_or(false)
        {
            return false;
        }

        if let Some(vr) = verb_recognizer {
            if let Some(inf) = vr.get_infinitive(lower) {
                let inf_lower = Self::normalize_spanish(&inf.to_lowercase());
                return matches!(
                    inf_lower.as_str(),
                    "decir"
                        | "explicar"
                        | "contar"
                        | "comentar"
                        | "senalar"
                        | "señalar"
                        | "mencionar"
                        | "afirmar"
                        | "asegurar"
                        | "advertir"
                        | "recordar"
                        | "repetir"
                        | "comunicar"
                        | "indicar"
                        | "informar"
                        | "notificar"
                        | "preguntar"
                );
            }
        }

        matches!(
            lower,
            "dice"
                | "dicen"
                | "dije"
                | "dijo"
                | "dijeron"
                | "decia"
                | "decía"
                | "decian"
                | "decían"
                | "explica"
                | "explican"
                | "conta"
                | "cuenta"
                | "cuentan"
                | "comenta"
                | "comentan"
        )
    }

    /// Detecta si hay coma justo antes del token "que" (indica cláusula explicativa)
    fn has_comma_before_que(
        word_tokens: &[(usize, &Token)],
        que_word_pos: usize,
        all_tokens: &[Token],
    ) -> bool {
        if que_word_pos == 0 {
            return false;
        }
        let (que_idx, _) = word_tokens[que_word_pos];
        let (prev_word_idx, _) = word_tokens[que_word_pos - 1];

        // Buscar coma entre la palabra anterior y "que"
        for idx in (prev_word_idx + 1)..que_idx {
            if let Some(tok) = all_tokens.get(idx) {
                if tok.token_type == TokenType::Punctuation && tok.text == "," {
                    return true;
                }
            }
        }
        false
    }

    /// Busca el antecedente con ventana extendida para cláusulas explicativas
    /// Corta la búsqueda si encuentra signos fuertes (. ! ? ;) o una segunda coma
    /// (la primera coma es la de ", que" que activó esta búsqueda)
    fn find_noun_extended_window<'a>(
        word_tokens: &[(usize, &'a Token)],
        pos: usize,
        all_tokens: &[Token],
    ) -> Option<&'a Token> {
        // Ventana extendida: hasta 8 posiciones hacia atrás
        let max_lookback = 8.min(pos);
        // Empezamos con 1 porque la coma de ", que" ya cuenta como primera coma
        let mut comma_count = 1;

        for offset in 0..=max_lookback {
            let check_pos = pos - offset;
            let (check_idx, candidate) = word_tokens[check_pos];

            // Verificar si hay signos fuertes o comas entre esta posición y la anterior
            if offset > 0 {
                let (prev_idx, _) = word_tokens[check_pos + 1];
                for idx in (check_idx + 1)..prev_idx {
                    if let Some(tok) = all_tokens.get(idx) {
                        if tok.token_type == TokenType::Punctuation {
                            // Signos fuertes: cortar búsqueda
                            if matches!(tok.text.as_str(), "." | "!" | "?" | ";") {
                                return None;
                            }
                            // Segunda coma (después de la de ", que"): cortar búsqueda
                            if tok.text == "," {
                                comma_count += 1;
                                if comma_count >= 2 {
                                    return None;
                                }
                            }
                        }
                    }
                }
            }

            // Si encontramos sustantivo, retornarlo
            if Self::is_noun(candidate) {
                return Some(candidate);
            }
        }

        None
    }

    /// Verifica si el token es un sustantivo o un adjetivo nominalizado (precedido de artículo)
    /// Ejemplos: "El estampado de lunares" - "estampado" es adjetivo nominalizado (noun)
    fn is_noun_or_nominalized(word_tokens: &[(usize, &Token)], pos: usize) -> bool {
        let (_, token) = word_tokens[pos];

        // Si es sustantivo, siempre es válido
        if Self::is_noun(token) {
            return true;
        }

        // Si es adjetivo y está precedido de artículo, es un adjetivo nominalizado
        if let Some(ref info) = token.word_info {
            if info.category == WordCategory::Adjetivo {
                if pos > 0 {
                    let (_, prev_token) = word_tokens[pos - 1];
                    let prev_lower = prev_token.effective_text().to_lowercase();
                    // Artículos que nominalizan adjetivos
                    if matches!(
                        prev_lower.as_str(),
                        "el" | "la" | "los" | "las" | "un" | "una" | "unos" | "unas"
                    ) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn has_comma_between_tokens(tokens: &[Token], left_idx: usize, right_idx: usize) -> bool {
        let start = left_idx.min(right_idx);
        let end = left_idx.max(right_idx);
        for idx in (start + 1)..end {
            if let Some(tok) = tokens.get(idx) {
                if tok.token_type == TokenType::Punctuation && tok.text == "," {
                    return true;
                }
            }
        }
        false
    }

    /// En incisos explicativos con ejemplos ("..., como X, que ..."),
    /// usa como ancla el núcleo nominal previo a "como" para evitar
    /// tomar el ejemplo singular como antecedente principal.
    fn find_antecedent_anchor_before_como_example(
        word_tokens: &[(usize, &Token)],
        noun_pos: usize,
        all_tokens: &[Token],
    ) -> Option<usize> {
        if noun_pos < 2 {
            return None;
        }

        let mut probe = noun_pos;
        let mut inspected = 0usize;
        const MAX_LOOKBACK: usize = 6;

        while probe > 0 && inspected < MAX_LOOKBACK {
            let (curr_idx, _) = word_tokens[probe];
            let (prev_idx, prev_token) = word_tokens[probe - 1];
            if has_sentence_boundary(all_tokens, prev_idx, curr_idx)
                || Self::has_comma_between_tokens(all_tokens, prev_idx, curr_idx)
            {
                return None;
            }

            let prev_norm = Self::normalize_spanish(&prev_token.effective_text().to_lowercase());
            if prev_norm == "como" {
                if probe < 2 {
                    return None;
                }
                let anchor_pos = probe - 2;
                let (anchor_idx, anchor_token) = word_tokens[anchor_pos];
                if has_sentence_boundary(all_tokens, anchor_idx, prev_idx)
                    || Self::has_comma_between_tokens(all_tokens, anchor_idx, prev_idx)
                {
                    return None;
                }
                if Self::is_noun(anchor_token) || Self::is_adjective(anchor_token) {
                    return Some(anchor_pos);
                }
                return None;
            }

            let can_cross = matches!(prev_norm.as_str(), "y" | "e" | "o" | "u")
                || matches!(
                    prev_norm.as_str(),
                    "el" | "la" | "los" | "las" | "un" | "una" | "unos" | "unas"
                )
                || Self::is_noun(prev_token)
                || Self::is_adjective(prev_token)
                || prev_token
                    .word_info
                    .as_ref()
                    .is_some_and(|info| info.category == WordCategory::Otro);
            if !can_cross {
                return None;
            }

            probe -= 1;
            inspected += 1;
        }

        None
    }

    fn is_causal_que_conjunction_context(
        word_tokens: &[(usize, &Token)],
        que_pos: usize,
        all_tokens: &[Token],
    ) -> bool {
        if que_pos == 0 || que_pos >= word_tokens.len() {
            return false;
        }

        let (marker_idx, marker_token) = word_tokens[que_pos - 1];
        let marker_norm = Self::normalize_spanish(&marker_token.effective_text().to_lowercase());
        if !matches!(marker_norm.as_str(), "puesto" | "dado" | "ya") {
            return false;
        }
        if marker_norm == "ya" {
            return true;
        }

        // Inicio de oración: "Puesto que...", "Dado que...".
        if que_pos == 1 {
            return true;
        }

        let (left_idx, left_token) = word_tokens[que_pos - 2];
        if has_sentence_boundary(all_tokens, left_idx, marker_idx)
            || Self::has_comma_between_tokens(all_tokens, left_idx, marker_idx)
        {
            return true;
        }

        // No bloquear relativos nominales tipo "el puesto que..."
        // donde "puesto" funciona como sustantivo antecedente.
        let left_is_nominal_determiner = left_token.word_info.as_ref().is_some_and(|info| {
            matches!(
                info.category,
                WordCategory::Articulo | WordCategory::Determinante | WordCategory::Pronombre
            )
        });
        if left_is_nominal_determiner {
            return false;
        }

        let left_norm = Self::normalize_spanish(&left_token.effective_text().to_lowercase());
        matches!(
            left_norm.as_str(),
            "y"
                | "e"
                | "pero"
                | "aunque"
                | "si"
                | "pues"
                | "porque"
                | "como"
                | "cuando"
                | "mientras"
                | "entonces"
                | "ademas"
        )
    }

    /// Busca el verdadero antecedente en patrones "noun1 de [adj/num]* noun2 que verb"
    /// Retorna noun1 si se encuentra el patrón, o el potential_antecedent original
    ///
    /// IMPORTANTE: Si noun2 tiene un artículo definido (el/la/los/las), probablemente
    /// es el sujeto real del relativo, no noun1. Ejemplo:
    /// - "marcos de referencia que sirven" → antecedente = marcos (sin artículo)
    /// - "actualización de los umbrales que determinan" → antecedente = umbrales (con artículo)
    ///
    /// TAMBIÉN: Si hay una coma antes de noun2, es un apositivo que reinicia la referencia.
    /// - "millones de euros, cifra que llega" → antecedente = cifra (la coma indica apositivo)
    fn find_true_antecedent<'a>(
        word_tokens: &[(usize, &'a Token)],
        noun2_pos: usize,
        potential_antecedent: &'a Token,
        all_tokens: &[Token],
    ) -> &'a Token {
        // Verificar si hay una coma justo antes de noun2 (indica apositivo)
        // En ese caso, noun2 es el verdadero antecedente
        if noun2_pos > 0 {
            let (noun2_idx, _) = word_tokens[noun2_pos];
            // Buscar coma entre la palabra anterior y noun2
            if noun2_idx > 0 {
                for idx in (word_tokens[noun2_pos - 1].0 + 1)..noun2_idx {
                    if let Some(token) = all_tokens.get(idx) {
                        if token.token_type == TokenType::Punctuation && token.text == "," {
                            return potential_antecedent; // Coma indica apositivo, mantener noun2
                        }
                    }
                }
            }
        }

        // Si hay un artículo (definido o indefinido) justo antes de noun2, probablemente noun2 es el sujeto real
        // "de los umbrales que determinan" → umbrales es el sujeto
        // "un escenario que contrarresta" → escenario es el sujeto (no buscar más atrás)
        //
        // Excepción: en cabezas colectivas/partitivas ("la mayoría de los estudiantes que ...",
        // "el conjunto de los microorganismos que ..."), la concordancia puede ir
        // con el núcleo (singular) o con el complemento (plural).
        // No forzar noun2 en esos casos.
        if noun2_pos > 0 {
            let (_, prev_token) = word_tokens[noun2_pos - 1];
            let prev_lower = prev_token.effective_text().to_lowercase();
            if matches!(
                prev_lower.as_str(),
                "el" | "la" | "los" | "las" | "un" | "una" | "unos" | "unas"
            ) {
                if !Self::has_variable_collective_head_before_article_noun(word_tokens, noun2_pos) {
                    return potential_antecedent; // Mantener noun2 como antecedente
                }
            }
        }

        // Buscar hacia atrás desde noun2_pos para encontrar "de"
        // El patrón puede ser: noun1 de noun2 que verb
        //                   o: noun1 de adj noun2 que verb
        //                   o: noun1 de adj adj noun2 que verb
        // Máximo 4 posiciones hacia atrás (noun1 de adj adj)
        let max_lookback = 4.min(noun2_pos);

        // Verificar si hay puntuación de separación (: ; --) entre noun2 y posiciones anteriores
        // En "mortalidad: extrínseca, que incluye", no buscar más allá de ":"
        let (noun2_idx, _) = word_tokens[noun2_pos];
        let boundary_limit = if noun2_pos > 0 {
            // Buscar si hay : o ; entre las palabras anteriores
            let mut limit = 0usize;
            for check_offset in 1..=max_lookback.min(noun2_pos) {
                let check_pos = noun2_pos - check_offset;
                let (check_idx, _) = word_tokens[check_pos];
                // Verificar puntuación entre check_idx y la siguiente palabra
                for idx in check_idx..noun2_idx {
                    if let Some(tok) = all_tokens.get(idx) {
                        if tok.token_type == TokenType::Punctuation
                            && (tok.text == ":" || tok.text == ";")
                        {
                            limit = check_offset;
                            break;
                        }
                    }
                }
                if limit > 0 {
                    break;
                }
            }
            if limit > 0 {
                limit
            } else {
                max_lookback
            }
        } else {
            max_lookback
        };

        // Si hay una conjunción "y/e" inmediatamente antes de noun2, es una frase nominal compuesta
        // "capó y techo que generan" - buscar más atrás
        let mut coord_offset = 0;
        if noun2_pos > 0 {
            let (_, prev_token) = word_tokens[noun2_pos - 1];
            if matches!(
                prev_token.effective_text().to_lowercase().as_str(),
                "y" | "e" | "o" | "u"
            ) {
                coord_offset = 2; // Saltar conjunción y el sustantivo coordinado
            }
        }

        for offset in 1.max(coord_offset)..=boundary_limit {
            let check_pos = noun2_pos.saturating_sub(offset);
            if check_pos >= noun2_pos {
                continue;
            }
            let (_, token) = word_tokens[check_pos];
            let text_lower = token.effective_text().to_lowercase();

            if text_lower == "de" || text_lower == "del" {
                // Encontramos "de", ahora verificar si hay un sustantivo (o adjetivo nominalizado) antes
                if check_pos > 0 {
                    let (_, maybe_noun1) = word_tokens[check_pos - 1];
                    // Usar is_noun_or_nominalized para detectar "El estampado de lunares"
                    if Self::is_noun_or_nominalized(word_tokens, check_pos - 1) {
                        // Partitivos: concordancia variable.
                        // "la mayoría de estudiantes que vinieron" (plural) es válida
                        // y "la mayoría de estudiantes que vino" (singular) también.
                        // El llamador ya conserva noun2 cuando noun2 concuerda con el verbo;
                        // si estamos aquí, noun2 no concuerda, así que preferimos el núcleo noun1.
                        let noun1_lower = maybe_noun1.effective_text().to_lowercase();
                        if Self::is_variable_collective_head_noun(&noun1_lower) {
                            return maybe_noun1; // Mantener el núcleo partitivo (noun1)
                        }
                        // Verificar si hay más "de" antes - caso "procesos [adj]* de creación de neuronas"
                        // Buscar hacia atrás desde maybe_noun1, saltando adjetivos
                        let mut search_back = check_pos as isize - 2; // Empezar antes del sustantivo encontrado
                        while search_back >= 0 {
                            let (_, back_token) = word_tokens[search_back as usize];
                            let back_lower = back_token.effective_text().to_lowercase();

                            // Si encontramos otro "de", buscar sustantivo antes (saltando adjetivos)
                            if back_lower == "de" || back_lower == "del" {
                                // Buscar sustantivo antes de este "de", saltando adjetivos
                                let mut noun_search = search_back - 1;
                                while noun_search >= 0 {
                                    let (_, candidate) = word_tokens[noun_search as usize];
                                    if Self::is_noun_or_nominalized(
                                        word_tokens,
                                        noun_search as usize,
                                    ) {
                                        let outer_lower = candidate.effective_text().to_lowercase();
                                        if !Self::is_variable_collective_head_noun(&outer_lower) {
                                            return candidate; // "procesos" en "procesos [adj]* de creación de X"
                                        }
                                        break;
                                    }
                                    // Si es adjetivo, seguir buscando
                                    if let Some(ref info) = candidate.word_info {
                                        use crate::dictionary::WordCategory;
                                        if info.category == WordCategory::Adjetivo {
                                            noun_search -= 1;
                                            continue;
                                        }
                                    }
                                    break; // No es sustantivo ni adjetivo, parar
                                }
                                break;
                            }

                            // Si es un adjetivo, seguir buscando hacia atrás
                            if let Some(ref info) = back_token.word_info {
                                use crate::dictionary::WordCategory;
                                if info.category == WordCategory::Adjetivo {
                                    search_back -= 1;
                                    continue;
                                }
                            }
                            // No es "de" ni adjetivo, parar
                            break;
                        }
                        return maybe_noun1;
                    }
                }
            }

            // Manejar preposiciones locativas: "paneles en el capó y techo que generan"
            // El antecedente real es "paneles", no "techo"
            // También: "organismos unicelulares con concha que funcionan" - antecedente es "organismos"
            if matches!(
                text_lower.as_str(),
                "en" | "sobre"
                    | "bajo"
                    | "tras"
                    | "ante"
                    | "con"
                    | "sin"
                    | "entre"
                    | "hacia"
                    | "desde"
                    | "hasta"
                    | "para"
                    | "por"
            ) {
                // Buscar hacia atrás saltando adjetivos hasta encontrar un sustantivo
                let mut search_pos = check_pos;
                while search_pos > 0 {
                    search_pos -= 1;
                    let (_, maybe_noun1) = word_tokens[search_pos];
                    if Self::is_noun(maybe_noun1) {
                        return maybe_noun1;
                    }
                    // Si encontramos algo que no es adjetivo ni sustantivo, parar
                    if let Some(ref info) = maybe_noun1.word_info {
                        use crate::dictionary::WordCategory;
                        if !matches!(info.category, WordCategory::Adjetivo) {
                            break;
                        }
                    } else {
                        // Sin info del diccionario, asumir que no es adjetivo si no termina típicamente
                        let word = maybe_noun1.effective_text().to_lowercase();
                        if !word.ends_with("es") && !word.ends_with("os") && !word.ends_with("as") {
                            break;
                        }
                    }
                }
            }

            // Si encontramos un sustantivo antes de encontrar "de", detenerse
            // (evita cruzar límites de sintagma)
            // PERO: si estamos en una frase coordinada (coord_offset > 0), no parar
            // en el sustantivo coordinado (que está justo en offset == coord_offset)
            if Self::is_noun(token) && offset > 1 && offset != coord_offset {
                break;
            }
        }

        potential_antecedent
    }

    /// Detecta si noun2 forma parte de un complemento "de + artículo + noun2"
    /// cuyo núcleo previo es colectivo/partitivo ("mayoría de los estudiantes",
    /// "conjunto de los microorganismos").
    fn has_variable_collective_head_before_article_noun(
        word_tokens: &[(usize, &Token)],
        noun2_pos: usize,
    ) -> bool {
        if noun2_pos < 3 {
            return false;
        }

        let (_, de_token) = word_tokens[noun2_pos - 2];
        let de_lower = de_token.effective_text().to_lowercase();
        if de_lower != "de" {
            return false;
        }

        let (_, noun1) = word_tokens[noun2_pos - 3];
        if !Self::is_noun_or_nominalized(word_tokens, noun2_pos - 3) {
            return false;
        }

        let noun1_lower = noun1.effective_text().to_lowercase();
        Self::is_variable_collective_head_noun(&noun1_lower)
    }

    /// Verifica si la palabra es una cabeza colectiva/partitiva de concordancia variable.
    /// En estructuras "X de Y que ...", estos núcleos admiten concordancia con X o con Y.
    fn is_variable_collective_head_noun(word: &str) -> bool {
        exceptions::is_variable_collective_noun(word)
    }

    /// Verifica si la palabra es una forma del auxiliar "haber"
    /// Usado para excluir tiempos compuestos del análisis de relativos
    fn is_haber_auxiliary(word: &str) -> bool {
        matches!(
            word,
            // Presente indicativo
            "he" | "has" | "ha" | "hemos" | "habéis" | "han" |
            // Imperfecto
            "había" | "habías" | "habíamos" | "habíais" | "habían" |
            // Pretérito indefinido
            "hube" | "hubiste" | "hubo" | "hubimos" | "hubisteis" | "hubieron" |
            // Futuro
            "habré" | "habrás" | "habrá" | "habremos" | "habréis" | "habrán" |
            // Condicional
            "habría" | "habrías" | "habríamos" | "habríais" | "habrían" |
            // Subjuntivo presente
            "haya" | "hayas" | "hayamos" | "hayáis" | "hayan" |
            // Subjuntivo imperfecto
            "hubiera" | "hubieras" | "hubiéramos" | "hubierais" | "hubieran" |
            "hubiese" | "hubieses" | "hubiésemos" | "hubieseis" | "hubiesen"
        )
    }

    /// Verifica si la palabra es un pronombre relativo
    fn is_relative_pronoun(word: &str) -> bool {
        let lower = word.to_lowercase();
        matches!(
            lower.as_str(),
            "que" | "quien" | "quienes" | "cual" | "cuales"
        )
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

    /// Verifica si "que" es probablemente exhortativo/desiderativo, no relativo
    /// Ejemplos: "¡Que vengan todos!", "Que lo hagan", "Que sea lo que Dios quiera"
    /// Condiciones:
    /// 1. "que" está al inicio de oración o después de signo fuerte (. ! ? ; " »)
    /// 2. El verbo parece estar en subjuntivo presente
    fn is_likely_exhortative_que(
        word_tokens: &[(usize, &Token)],
        que_pos: usize,
        all_tokens: &[Token],
        verb: &str,
    ) -> bool {
        // Verificar si el verbo parece subjuntivo presente
        if !Self::looks_like_subjunctive_present(verb) {
            return false;
        }

        // Verificar si "que" está al inicio o después de signo fuerte
        let (que_idx, _) = word_tokens[que_pos];

        // Si es el primer token de palabra, verificar tokens anteriores
        if que_pos == 0 {
            // "que" es la primera palabra - verificar si hay signos de apertura antes
            if que_idx == 0 {
                return true; // Al inicio absoluto del texto
            }
            // Verificar tokens antes de "que"
            for idx in (0..que_idx).rev() {
                if let Some(tok) = all_tokens.get(idx) {
                    match tok.token_type {
                        TokenType::Whitespace => continue,
                        TokenType::Punctuation => {
                            // Signos que indican inicio de oración
                            if matches!(
                                tok.text.as_str(),
                                "." | "!" | "?" | ";" | "\"" | "»" | "¡" | "¿"
                            ) {
                                return true;
                            }
                            // Otros signos de puntuación - no es inicio
                            return false;
                        }
                        _ => return false,
                    }
                }
            }
            return true; // Solo espacios antes
        }

        // "que" no es la primera palabra - verificar si hay signo fuerte antes
        let (prev_word_idx, _) = word_tokens[que_pos - 1];
        for idx in (prev_word_idx + 1)..que_idx {
            if let Some(tok) = all_tokens.get(idx) {
                if tok.token_type == TokenType::Punctuation {
                    if matches!(tok.text.as_str(), "." | "!" | "?" | ";" | "\"" | "»") {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Verifica si un verbo parece estar en subjuntivo presente (3ª persona)
    /// Terminaciones: -e/-en (verbos -ar), -a/-an (verbos -er/-ir)
    fn looks_like_subjunctive_present(verb: &str) -> bool {
        // Formas irregulares comunes de subjuntivo presente
        if matches!(
            verb,
            "sea"
                | "sean"
                | "esté"
                | "estén"
                | "vaya"
                | "vayan"
                | "haya"
                | "hayan"
                | "tenga"
                | "tengan"
                | "venga"
                | "vengan"
                | "diga"
                | "digan"
                | "haga"
                | "hagan"
                | "ponga"
                | "pongan"
                | "salga"
                | "salgan"
                | "quiera"
                | "quieran"
                | "pueda"
                | "puedan"
                | "sepa"
                | "sepan"
                | "dé"
                | "den"
                | "traiga"
                | "traigan"
        ) {
            return true;
        }

        // Heurística por terminaciones (subjuntivo presente 3ª persona)
        // -ar → -e/-en, -er/-ir → -a/-an
        // Pero esto puede confundirse con indicativo, así que solo usamos
        // terminaciones menos ambiguas
        if verb.ends_with("en") && !verb.ends_with("ien") && !verb.ends_with("uen") {
            // Muchos subjuntivos terminan en -en: canten, coman, vivan
            // Pero también indicativos: tienen, vienen
            // Solo considerar si parece raíz + en sin diptongo
            let len = verb.len();
            if len >= 3 {
                let before_en = &verb[..len - 2];
                // Si termina en consonante + en, probablemente subjuntivo
                if before_en
                    .chars()
                    .last()
                    .map(|c| !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }

        false
    }

    /// Verifica si hay un inciso parentético con conector + verbo antes de "que"
    /// Ejemplo: "Las medidas, según explicó el ministro, que aprobó..."
    /// En estos casos, el verbo del relativo probablemente tiene su propio sujeto implícito
    ///
    /// Detecta conectores: según, como, tal como, tal y como, como indicó, como dijo, etc.
    /// Solo marca frontera si el conector va seguido de un verbo (1-5 tokens)
    ///
    /// Busca en una ventana de hasta 8 tokens hacia atrás desde before_que_pos
    fn has_parenthetical_clause(
        word_tokens: &[(usize, &Token)],
        before_que_pos: usize,
        que_pos: usize,
    ) -> bool {
        // Buscar hacia atrás desde la posición antes de "que"
        // hasta un máximo de 8 tokens (ventana razonable para un inciso)
        let search_start = before_que_pos.saturating_sub(8);

        // Conectores que introducen incisos parentéticos
        let connectors = ["según", "como"];

        // Buscar un conector en la ventana
        for pos in search_start..que_pos {
            if pos >= word_tokens.len() {
                continue;
            }
            let (_, token) = word_tokens[pos];
            let word_lower = token.effective_text().to_lowercase();

            // Verificar si es un conector
            let is_connector = connectors.contains(&word_lower.as_str());

            // También verificar "tal como", "tal y como"
            let is_tal_como = word_lower == "tal" && pos + 1 < que_pos && {
                let (_, next) = word_tokens[pos + 1];
                next.effective_text().to_lowercase() == "como"
            };

            if !is_connector && !is_tal_como {
                continue;
            }

            // Verificar que no sea "como" seguido de sustantivo (ej: "como base")
            // Solo activar frontera si hay verbo cercano
            let mut found_verb = false;
            let max_lookahead = 5.min(que_pos.saturating_sub(pos + 1));

            for offset in 1..=max_lookahead {
                let check_pos = pos + offset;
                if check_pos >= que_pos {
                    break;
                }

                let (_, check_token) = word_tokens[check_pos];
                let check_lower = check_token.effective_text().to_lowercase();

                // Verificar si es un verbo conjugado común en incisos
                // (explicó, dijo, indicó, señaló, apuntó, recordó, etc.)
                if Self::looks_like_parenthetical_verb(&check_lower) {
                    found_verb = true;
                    break;
                }

                // Si encontramos un sustantivo inmediatamente después del conector,
                // probablemente no es un inciso verbal ("como base", "según datos")
                if offset == 1 {
                    if let Some(ref info) = check_token.word_info {
                        if info.category == WordCategory::Sustantivo {
                            break; // No es inciso verbal
                        }
                    }
                }
            }

            if found_verb {
                return true;
            }
        }

        false
    }

    /// Verifica si una palabra parece un verbo típico de inciso parentético
    fn looks_like_parenthetical_verb(word: &str) -> bool {
        // Verbos de comunicación/percepción típicos en incisos
        matches!(
            word,
            // Formas de "explicar"
            "explicó" | "explica" | "explicaba" | "explicaron" |
            // Formas de "decir"
            "dijo" | "dice" | "decía" | "dijeron" |
            // Formas de "indicar"
            "indicó" | "indica" | "indicaba" | "indicaron" |
            // Formas de "señalar"
            "señaló" | "señala" | "señalaba" | "señalaron" |
            // Formas de "apuntar"
            "apuntó" | "apunta" | "apuntaba" | "apuntaron" |
            // Formas de "recordar"
            "recordó" | "recuerda" | "recordaba" | "recordaron" |
            // Formas de "afirmar"
            "afirmó" | "afirma" | "afirmaba" | "afirmaron" |
            // Formas de "asegurar"
            "aseguró" | "asegura" | "aseguraba" | "aseguraron" |
            // Formas de "comentar"
            "comentó" | "comenta" | "comentaba" | "comentaron" |
            // Formas de "añadir"
            "añadió" | "añade" | "añadía" | "añadieron" |
            // Formas de "sostener"
            "sostuvo" | "sostiene" | "sostenía" | "sostuvieron" |
            // Formas de "advertir"
            "advirtió" | "advierte" | "advertía" | "advirtieron"
        )
    }

    /// Verifica si después del verbo hay un sujeto propio (determinante/posesivo + sustantivo)
    /// Ejemplo: "que tiene nuestra población" → "nuestra población" es el sujeto
    /// También detecta nombres propios: "que negocia SoftBank" → SoftBank es el sujeto
    ///
    /// Busca en una ventana de hasta 5 tokens después del verbo, saltando:
    /// - Adverbios (rápidamente, ayer, ya, etc.)
    /// - Pronombres clíticos (lo, la, le, se, etc.)
    /// - Frases preposicionales temporales (en 2020, en enero, en ese momento)
    /// - Números (años como 2020, 1990)
    ///
    /// Ejemplos:
    /// - "criterios que fije rápidamente cada autonomía" → "cada autonomía" es sujeto
    /// - "normas que aprobó ayer la comisión" → "la comisión" es sujeto
    /// - "leyes que aprobó en 2020 la comisión" → "la comisión" es sujeto
    fn has_own_subject_after_verb(
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
        all_tokens: &[Token],
    ) -> bool {
        // Necesitamos al menos 1 palabra después del verbo
        if verb_pos + 1 >= word_tokens.len() {
            return false;
        }

        // Palabras que se pueden saltar al buscar el sujeto pospuesto
        // NOTA: NO incluir preposiciones como "a", "de" porque introducen complementos, no sujetos
        // "en" se maneja especialmente para frases temporales (en 2020, en enero)
        let skippable_words = [
            // Adverbios temporales
            "ayer",
            "hoy",
            "mañana",
            "ahora",
            "entonces",
            "luego",
            "después",
            "antes",
            "siempre",
            "nunca",
            "jamás",
            "todavía",
            "aún",
            "ya",
            // Adverbios de modo comunes
            "bien",
            "mal",
            "así",
            "solo",
            "sólo",
            "también",
            "tampoco",
            // Adverbios de cantidad
            "muy",
            "mucho",
            "poco",
            "bastante",
            "demasiado",
            "más",
            "menos",
            // Pronombres clíticos (pueden aparecer después del verbo en algunas construcciones)
            "lo",
            "la",
            "le",
            "los",
            "las",
            "les",
            "se",
            "me",
            "te",
            "nos",
            "os",
            // Meses (para frases temporales "en enero", etc.)
            "enero",
            "febrero",
            "marzo",
            "abril",
            "mayo",
            "junio",
            "julio",
            "agosto",
            "septiembre",
            "octubre",
            "noviembre",
            "diciembre",
            // Sustantivos temporales comunes (para "en ese momento", etc.)
            "momento",
            "tiempo",
            "época",
            "año",
            "día",
            "mes",
            "instante",
            "período",
            "periodo",
            "fecha",
            // Adverbios terminados en -mente (se verifican por sufijo más abajo)
        ];

        // Determinantes que introducen sujetos
        let subject_introducers = [
            // Posesivos
            "mi",
            "tu",
            "su",
            "nuestra",
            "nuestro",
            "vuestra",
            "vuestro",
            "mis",
            "tus",
            "sus",
            "nuestras",
            "nuestros",
            "vuestras",
            "vuestros",
            // Artículos
            "el",
            "la",
            "los",
            "las",
            "un",
            "una",
            "unos",
            "unas",
            // Demostrativos
            "este",
            "esta",
            "estos",
            "estas",
            "ese",
            "esa",
            "esos",
            "esas",
            "aquel",
            "aquella",
            "aquellos",
            "aquellas",
            // Distributivos e indefinidos
            "cada",
            "cualquier",
            "algún",
            "ningún",
            "otro",
            "otra",
            "cierto",
            "cierta",
            "ciertos",
            "ciertas",
            "varios",
            "varias",
            "muchos",
            "muchas",
            "pocos",
            "pocas",
            "algunos",
            "algunas",
            "todos",
            "todas",
        ];

        // Buscar en una ventana de hasta 5 tokens después del verbo
        let window_size = 5.min(word_tokens.len().saturating_sub(verb_pos + 1));

        let mut offset = 1;
        while offset <= window_size {
            let pos = verb_pos + offset;
            if pos >= word_tokens.len() {
                break;
            }

            let (_, current_token) = word_tokens[pos];
            let current_text = current_token.effective_text();
            let current_lower = current_text.to_lowercase();

            // Verificar si es un nombre propio (mayúscula inicial)
            // Ejemplo: "que negocia SoftBank" → SoftBank es el sujeto
            if current_text
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                // Verificar que no es simplemente una palabra común capitalizada
                // sino un nombre propio (no está en el diccionario como sustantivo común)
                if let Some(ref info) = current_token.word_info {
                    // Si está en el diccionario como sustantivo común, no es nombre propio
                    if info.category == WordCategory::Sustantivo {
                        // Podría ser "La comisión" donde "La" está capitalizada por contexto
                        // En ese caso, verificar si es un introductor de sujeto
                        if subject_introducers.contains(&current_lower.as_str()) {
                            // Verificar si la siguiente palabra es un sustantivo
                            if pos + 1 < word_tokens.len() {
                                let (_, next_token) = word_tokens[pos + 1];
                                if Self::is_noun(next_token) {
                                    return true;
                                }
                            }
                        }
                        // Nombre propio capitalizado etiquetado como sustantivo común en diccionario
                        // (ej: "María") también puede ser sujeto pospuesto.
                        if current_text.chars().skip(1).any(|c| c.is_lowercase()) {
                            return true;
                        }
                        offset += 1;
                        continue; // No es nombre propio, seguir buscando
                    }
                } else {
                    // No está en el diccionario, probablemente es nombre propio
                    return true;
                }
            }

            // Verificar si es un determinante que introduce sujeto
            if subject_introducers.contains(&current_lower.as_str()) {
                // Verificar si la siguiente palabra es un sustantivo
                if pos + 1 < word_tokens.len() {
                    let (_, next_token) = word_tokens[pos + 1];
                    if Self::is_noun(next_token) {
                        return true;
                    }
                    // También verificar si hay un adjetivo + sustantivo (det + adj + noun)
                    if Self::is_adjective(next_token) && pos + 2 < word_tokens.len() {
                        let (_, noun_token) = word_tokens[pos + 2];
                        if Self::is_noun(noun_token) {
                            return true;
                        }
                    }
                    // También aceptar determinante + adjetivo como sujeto nominalizado.
                    // Esto evita falsos positivos cuando el diccionario etiqueta el plural como adjetivo
                    // aunque se use como sustantivo (ej: "las panaderas").
                    if Self::is_adjective(next_token) {
                        return true;
                    }
                }
            }

            // Verificar si es una palabra que se puede saltar
            let is_skippable = skippable_words.contains(&current_lower.as_str())
                || current_lower.ends_with("mente"); // Adverbios en -mente

            // Caso especial: frases preposicionales temporales con "en"
            // "en 2020", "en enero", "en ese momento" son temporales y se pueden saltar
            if current_lower == "en" {
                let (current_orig_idx, _) = word_tokens[pos];

                // Buscar el siguiente token (puede ser número) en all_tokens
                // para verificar si es una frase temporal "en 2020"
                let mut found_number_temporal = false;
                for check_idx in (current_orig_idx + 1)..all_tokens.len() {
                    let check_token = &all_tokens[check_idx];
                    // Saltar espacios
                    if check_token.token_type == TokenType::Whitespace {
                        continue;
                    }
                    // "en" + número (año): "en 2020", "en 1990"
                    if check_token.token_type == crate::grammar::tokenizer::TokenType::Number {
                        found_number_temporal = true;
                    }
                    break;
                }

                if found_number_temporal {
                    // "en 2020" - saltar solo "en", el número no está en word_tokens
                    offset += 1;
                    continue;
                }

                // Verificar en word_tokens para meses y demostrativos temporales
                if pos + 1 < word_tokens.len() {
                    let (_, next_token) = word_tokens[pos + 1];
                    let next_lower = next_token.effective_text().to_lowercase();

                    // "en" + mes: "en enero", "en febrero", etc.
                    let months = [
                        "enero",
                        "febrero",
                        "marzo",
                        "abril",
                        "mayo",
                        "junio",
                        "julio",
                        "agosto",
                        "septiembre",
                        "octubre",
                        "noviembre",
                        "diciembre",
                    ];
                    if months.contains(&next_lower.as_str()) {
                        // Saltar "en" + mes (2 tokens)
                        offset += 2;
                        continue;
                    }

                    // "en" + demostrativo temporal: "en ese momento", "en aquel tiempo"
                    let temporal_demonstratives =
                        ["ese", "este", "aquel", "esa", "esta", "aquella"];
                    if temporal_demonstratives.contains(&next_lower.as_str()) {
                        // Verificar si la palabra después es un sustantivo temporal
                        if pos + 2 < word_tokens.len() {
                            let (_, third_token) = word_tokens[pos + 2];
                            let third_lower = third_token.effective_text().to_lowercase();
                            let temporal_nouns = [
                                "momento", "tiempo", "época", "año", "día", "mes", "instante",
                                "período", "periodo", "fecha",
                            ];
                            if temporal_nouns.contains(&third_lower.as_str()) {
                                // Saltar "en" + demostrativo + sustantivo temporal (3 tokens)
                                offset += 3;
                                continue;
                            }
                        }
                    }
                }
            }

            if !is_skippable {
                // No es saltable y no es introductor de sujeto, dejar de buscar
                // (probablemente es el objeto directo u otro complemento)
                break;
            }

            offset += 1;
        }

        false
    }

    /// Verifica si el verbo es copulativo (ser/estar) y va seguido de un predicativo plural
    /// En construcciones como "X que son Y", donde Y es plural, la concordancia puede ser
    /// con el predicativo en lugar del antecedente.
    /// Ejemplo: "la causa, que son las lluvias" - válido aunque "causa" es singular
    fn is_copulative_with_plural_predicate(
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
    ) -> bool {
        if verb_pos >= word_tokens.len() {
            return false;
        }

        let (_, verb) = word_tokens[verb_pos];
        let verb_lower = verb.effective_text().to_lowercase();

        // Formas del verbo "ser" en plural (3ª persona)
        let copulative_plural = [
            "son", "eran", "fueron", "serán", "serían", "sean", "fueran", "fuesen",
        ];

        if !copulative_plural.contains(&verb_lower.as_str()) {
            return false;
        }

        // Verificar si después del verbo hay un sustantivo (el predicativo)
        if verb_pos + 1 < word_tokens.len() {
            let (_, next_word) = word_tokens[verb_pos + 1];
            if Self::is_noun(next_word) {
                // Verificar si el sustantivo es plural
                if let Some(ref info) = next_word.word_info {
                    if info.number == Number::Plural {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Verifica si después del verbo hay un participio que concuerda con el antecedente
    /// En construcciones como "las misiones que tiene previstas", el antecedente es el
    /// objeto directo del verbo, no el sujeto. El sujeto es implícito.
    fn is_verb_with_agreeing_participle(
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
        antecedent: &Token,
    ) -> bool {
        if verb_pos + 1 >= word_tokens.len() {
            return false;
        }

        let (_, word_after_verb) = word_tokens[verb_pos + 1];
        let word_lower = word_after_verb.effective_text().to_lowercase();

        // Verificar si parece un participio (termina en -ado, -ido, -ada, -ida, etc.)
        let is_participle = word_lower.ends_with("ado")
            || word_lower.ends_with("ado")
            || word_lower.ends_with("ados")
            || word_lower.ends_with("ada")
            || word_lower.ends_with("adas")
            || word_lower.ends_with("ido")
            || word_lower.ends_with("idos")
            || word_lower.ends_with("ida")
            || word_lower.ends_with("idas")
            // Participios irregulares comunes
            || matches!(word_lower.as_str(),
                "previsto" | "previstos" | "prevista" | "previstas" |
                "hecho" | "hechos" | "hecha" | "hechas" |
                "dicho" | "dichos" | "dicha" | "dichas" |
                "escrito" | "escritos" | "escrita" | "escritas" |
                "visto" | "vistos" | "vista" | "vistas" |
                "puesto" | "puestos" | "puesta" | "puestas" |
                "abierto" | "abiertos" | "abierta" | "abiertas" |
                "cubierto" | "cubiertos" | "cubierta" | "cubiertas" |
                "muerto" | "muertos" | "muerta" | "muertas" |
                "vuelto" | "vueltos" | "vuelta" | "vueltas" |
                "roto" | "rotos" | "rota" | "rotas"
            );

        if !is_participle {
            return false;
        }

        // Verificar si el participio concuerda en número con el antecedente
        if let (Some(ant_info), Some(part_info)) =
            (&antecedent.word_info, &word_after_verb.word_info)
        {
            // Si ambos tienen el mismo número, probablemente el antecedente es el OD
            if ant_info.number == part_info.number && ant_info.number != Number::None {
                return true;
            }
        }

        // Si no tenemos info del diccionario, inferir del participio
        if let Some(ant_info) = &antecedent.word_info {
            let participle_plural = word_lower.ends_with('s');
            let antecedent_plural = ant_info.number == Number::Plural;
            if participle_plural == antecedent_plural {
                return true;
            }
        }

        false
    }

    /// Obtiene el número gramatical del antecedente
    fn get_antecedent_number(token: &Token) -> Option<Number> {
        token.word_info.as_ref().map(|info| info.number)
    }

    /// Verifica concordancia del verbo con el antecedente
    fn check_verb_agreement(
        verb_index: usize,
        antecedent: &Token,
        verb: &Token,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
        has_comma_before_que: bool,
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
        tokens: &[Token],
    ) -> Option<RelativeCorrection> {
        let antecedent_number = Self::get_antecedent_number(antecedent)?;

        // Solo procesar si el antecedente tiene número definido
        if antecedent_number == Number::None {
            return None;
        }

        // Excluir artículos y otras palabras que no son verbos
        // "las decisiones que una IA toma" - "una" no es verbo
        let non_verbs = [
            "un",
            "una",
            "unos",
            "unas",
            "el",
            "la",
            "los",
            "las",
            "mi",
            "tu",
            "su",
            "mis",
            "tus",
            "sus",
            "este",
            "esta",
            "estos",
            "estas",
            "ese",
            "esa",
            "esos",
            "esas",
            "aquel",
            "aquella",
            "aquellos",
            "aquellas",
            "nuestro",
            "nuestra",
            "nuestros",
            "nuestras",
            "vuestro",
            "vuestra",
            "vuestros",
            "vuestras",
            "cada",
            "todo",
            "toda",
            "todos",
            "todas",
            "otro",
            "otra",
            "otros",
            "otras",
            "mucho",
            "mucha",
            "muchos",
            "muchas",
            "poco",
            "poca",
            "pocos",
            "pocas",
            "algún",
            "alguno",
            "alguna",
            "algunos",
            "algunas",
            "ningún",
            "ninguno",
            "ninguna",
            "ningunos",
            "ningunas",
            "cualquier",
            "cualquiera",
            "cualesquiera",
        ];
        let verb_lower = verb.effective_text().to_lowercase();
        if non_verbs.contains(&verb_lower.as_str()) {
            return None;
        }

        // Excluir palabras que no son verbos según el diccionario.
        // Para homógrafos (p. ej. "cocina"), aceptar solo si el recognizer
        // confirma explícitamente la forma verbal; no usar heurísticas de sufijo
        // para evitar falsos positivos con nombres/pronombres.
        if let Some(ref info) = verb.word_info {
            if !matches!(info.category, WordCategory::Verbo) {
                let is_valid_verb = verb_recognizer
                    .map(|vr| vr.is_valid_verb_form(&verb_lower))
                    .unwrap_or(false);
                let allow_spurious_nominal = Self::is_spurious_nominal_verb_form(verb, &verb_lower);
                if !is_valid_verb && !allow_spurious_nominal {
                    return None;
                }
            }
        } else if verb
            .effective_text()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            let is_valid_verb = verb_recognizer
                .map(|vr| vr.is_valid_verb_form(&verb_lower))
                .unwrap_or(false);
            if !is_valid_verb {
                return None;
            }
        }

        // Excluir palabras que típicamente forman relativos de objeto (no sujeto)
        // En "los ratos que estaba", "ratos" es objeto, no sujeto del verbo
        // Estos sustantivos de tiempo/frecuencia típicamente no son sujeto del verbo subordinado
        let object_relative_nouns = [
            "ratos",
            "rato",
            "momento",
            "momentos",
            "tiempo",
            "tiempos",
            "día",
            "días",
            "vez",
            "veces",
            "hora",
            "horas",
            "minuto",
            "minutos",
            "segundo",
            "segundos",
            "año",
            "años",
            "mes",
            "meses",
            "semana",
            "semanas",
            "ocasión",
            "ocasiones",
            "instante",
            "instantes",
        ];

        let antecedent_lower = antecedent.effective_text().to_lowercase();
        if object_relative_nouns.contains(&antecedent_lower.as_str()) {
            return None;
        }

        // También excluir sustantivos que típicamente son objetos del verbo subordinado
        // "los agravios que pensaba deshacer" - "agravios" es objeto de "deshacer", no sujeto de "pensaba"
        // NOTA: No incluir "problema/problemas" etc. porque SÍ pueden ser sujetos legítimos
        let object_nouns = [
            "agravio", "agravios", "tuerto", "tuertos", "favor", "favores", "daño", "daños",
        ];

        if object_nouns.contains(&antecedent_lower.as_str()) {
            return None;
        }

        let verb_lower = verb.effective_text().to_lowercase();

        // Obtener información del verbo incluyendo tiempo
        let (verb_number, infinitive, tense) =
            Self::get_verb_info_with_tense(&verb_lower, verb_recognizer)?;

        if antecedent_number != verb_number
            && Self::should_skip_ambiguous_relative_mismatch(
                antecedent,
                antecedent_number,
                verb_number,
                &infinitive,
                has_comma_before_que,
                word_tokens,
                verb_pos,
                tokens,
            )
        {
            return None;
        }

        // Verificar si hay discordancia
        if antecedent_number != verb_number {
            // Generar la forma correcta del verbo en el mismo tiempo
            let correct_form =
                Self::get_correct_verb_form_with_tense(&infinitive, antecedent_number, tense)?;

            if correct_form.to_lowercase() != verb_lower {
                return Some(RelativeCorrection {
                    token_index: verb_index,
                    original: verb.text.clone(),
                    suggestion: correct_form,
                    message: format!(
                        "Concordancia relativo: el verbo '{}' debe concordar con '{}' ({})",
                        verb.text,
                        antecedent.text,
                        if antecedent_number == Number::Singular {
                            "singular"
                        } else {
                            "plural"
                        }
                    ),
                });
            }
        }

        None
    }

    fn should_skip_ambiguous_relative_mismatch(
        antecedent: &Token,
        antecedent_number: Number,
        verb_number: Number,
        infinitive: &str,
        has_comma_before_que: bool,
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
        tokens: &[Token],
    ) -> bool {
        if has_comma_before_que {
            return false;
        }

        let antecedent_lower = antecedent.effective_text().to_lowercase();
        if matches!(antecedent_lower.as_str(), "los" | "las") {
            // Caso eliptico "uno/una de los/las que ...":
            // debe mantenerse la concordancia plural del relativo.
            return false;
        }
        let is_human_antecedent = Self::is_human_like_antecedent(&antecedent_lower);
        let subject_biased_verb = Self::is_subject_biased_relative_verb(infinitive);
        let has_postverbal_object =
            Self::has_postverbal_object_like_phrase(word_tokens, verb_pos, tokens);

        if subject_biased_verb || has_postverbal_object {
            return false;
        }

        match (antecedent_number, verb_number) {
            // Direccion mas ambigua: antecedente singular + verbo plural.
            // Sin senales de sujeto, suele ser relativo de objeto con sujeto implicito plural.
            (Number::Singular, Number::Plural) => !is_human_antecedent,
            // Tambien puede ser relativo de objeto (los libros que compro).
            // Se mantiene conservador cuando el antecedente no es humano.
            (Number::Plural, Number::Singular) => !is_human_antecedent,
            _ => false,
        }
    }

    fn is_subject_biased_relative_verb(infinitive: &str) -> bool {
        matches!(
            infinitive,
            "ser"
                | "estar"
                | "parecer"
                | "resultar"
                | "quedar"
                | "venir"
                | "llegar"
                | "salir"
                | "morir"
                | "nacer"
                | "caer"
                | "pasar"
                | "ocurrir"
                | "suceder"
                | "existir"
                | "carecer"
                | "regir"
                | "reger"
                | "afectar"
                | "influir"
                | "depender"
                | "consistir"
        )
    }

    fn has_postverbal_object_like_phrase(
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
        tokens: &[Token],
    ) -> bool {
        if verb_pos + 1 >= word_tokens.len() {
            return false;
        }

        let (verb_idx, _) = word_tokens[verb_pos];
        let max_probe = (verb_pos + 8).min(word_tokens.len() - 1);
        let mut pos = verb_pos + 1;

        while pos <= max_probe {
            let (curr_idx, curr_token) = word_tokens[pos];
            if has_sentence_boundary(tokens, verb_idx, curr_idx) {
                break;
            }

            let curr_lower = curr_token.effective_text().to_lowercase();
            if curr_lower.ends_with("mente")
                || matches!(
                    curr_lower.as_str(),
                    "no" | "ya"
                        | "aun"
                        | "aún"
                        | "tambien"
                        | "también"
                        | "siempre"
                        | "nunca"
                        | "casi"
                        | "muy"
                        | "mas"
                        | "más"
                        | "menos"
                )
            {
                pos += 1;
                continue;
            }

            if matches!(
                curr_lower.as_str(),
                "me" | "te" | "se" | "nos" | "os" | "le" | "les" | "lo" | "la" | "los" | "las"
            ) {
                pos += 1;
                continue;
            }

            if matches!(curr_lower.as_str(), "a" | "al") {
                if pos + 1 < word_tokens.len() {
                    let (next_idx, next_token) = word_tokens[pos + 1];
                    if !has_sentence_boundary(tokens, curr_idx, next_idx) {
                        let next_lower = next_token.effective_text().to_lowercase();
                        let next_is_np_head = Self::is_noun(next_token)
                            || next_token.word_info.as_ref().is_some_and(|info| {
                                matches!(
                                    info.category,
                                    WordCategory::Articulo
                                        | WordCategory::Determinante
                                        | WordCategory::Pronombre
                                )
                            })
                            || next_lower
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false);
                        if next_is_np_head {
                            return true;
                        }
                    }
                }
                pos += 1;
                continue;
            }

            let looks_like_np_start = curr_token.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Articulo
                        | WordCategory::Determinante
                        | WordCategory::Sustantivo
                        | WordCategory::Pronombre
                )
            }) || curr_token.token_type == TokenType::Number;

            if looks_like_np_start {
                return true;
            }

            break;
        }

        false
    }

    fn is_human_like_token(token: &Token) -> bool {
        let lower = Self::normalize_spanish(&token.effective_text().to_lowercase());
        Self::is_human_like_antecedent(&lower)
    }

    fn is_temporal_like_noun_candidate(word: &str) -> bool {
        matches!(
            word,
            "enero"
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
                | "mes"
                | "meses"
                | "semana"
                | "semanas"
                | "dia"
                | "dias"
                | "ano"
                | "anos"
                | "trimestre"
                | "trimestres"
                | "semestre"
                | "semestres"
        )
    }

    fn is_preceded_by_de(word_tokens: &[(usize, &Token)], pos: usize) -> bool {
        if pos == 0 {
            return false;
        }
        let prev = Self::normalize_spanish(&word_tokens[pos - 1].1.effective_text().to_lowercase());
        prev == "de" || prev == "del"
    }

    fn is_preceded_by_determiner_like(word_tokens: &[(usize, &Token)], pos: usize) -> bool {
        if pos == 0 {
            return false;
        }
        let prev_token = word_tokens[pos - 1].1;
        if prev_token.word_info.as_ref().is_some_and(|info| {
            matches!(
                info.category,
                WordCategory::Articulo | WordCategory::Determinante
            )
        }) {
            return true;
        }
        let prev = Self::normalize_spanish(&prev_token.effective_text().to_lowercase());
        matches!(
            prev.as_str(),
            "el"
                | "la"
                | "los"
                | "las"
                | "un"
                | "una"
                | "unos"
                | "unas"
                | "este"
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
        )
    }

    fn antecedent_candidate_score(
        word_tokens: &[(usize, &Token)],
        pos: usize,
        token: &Token,
        target_number: Option<Number>,
        require_human: bool,
    ) -> i32 {
        let mut score = 0i32;
        let lower = Self::normalize_spanish(&token.effective_text().to_lowercase());

        if let Some(target) = target_number {
            if let Some(num) = Self::get_antecedent_number(token) {
                if num == target {
                    score += 4;
                } else if num != Number::None {
                    score -= 2;
                }
            }
        }

        if require_human {
            if Self::is_human_like_antecedent(&lower) {
                score += 4;
            } else {
                score -= 3;
            }
        }

        if Self::is_temporal_like_noun_candidate(&lower) {
            score -= 3;
        }
        if Self::is_preceded_by_de(word_tokens, pos) {
            score -= 2;
        } else {
            score += 1;
        }
        if Self::is_preceded_by_determiner_like(word_tokens, pos) {
            score += 1;
        }

        score
    }

    fn find_better_antecedent_before_relative<'a>(
        word_tokens: &[(usize, &'a Token)],
        relative_left_pos: usize,
        all_tokens: &[Token],
        target_number: Option<Number>,
        require_human: bool,
    ) -> Option<&'a Token> {
        if relative_left_pos + 1 >= word_tokens.len() {
            return None;
        }

        let rel_idx = word_tokens[relative_left_pos + 1].0;
        let start = relative_left_pos.saturating_sub(24);
        let mut best: Option<(&'a Token, i32, usize)> = None;

        for pos in (start..=relative_left_pos).rev() {
            let (cand_idx, cand_token) = word_tokens[pos];
            if has_sentence_boundary(all_tokens, cand_idx, rel_idx) {
                break;
            }
            if !Self::is_noun_or_nominalized(word_tokens, pos) {
                continue;
            }

            let mut score = Self::antecedent_candidate_score(
                word_tokens,
                pos,
                cand_token,
                target_number,
                require_human,
            );
            let distance = relative_left_pos - pos;
            score -= (distance as i32) / 6;

            match best {
                None => best = Some((cand_token, score, distance)),
                Some((_, best_score, best_distance)) => {
                    if score > best_score || (score == best_score && distance < best_distance) {
                        best = Some((cand_token, score, distance));
                    }
                }
            }
        }

        best.map(|(token, _, _)| token)
    }

    fn is_human_like_antecedent(word: &str) -> bool {
        matches!(
            word,
            "persona"
                | "personas"
                | "hombre"
                | "hombres"
                | "mujer"
                | "mujeres"
                | "niño"
                | "niños"
                | "niña"
                | "niñas"
                | "chico"
                | "chicos"
                | "chica"
                | "chicas"
                | "alumno"
                | "alumnos"
                | "alumna"
                | "alumnas"
                | "presidente"
                | "presidentes"
                | "director"
                | "directores"
                | "directora"
                | "directoras"
                | "ministro"
                | "ministros"
                | "ministra"
                | "ministras"
                | "profesor"
                | "profesores"
                | "profesora"
                | "profesoras"
                | "investigador"
                | "investigadores"
                | "investigadora"
                | "investigadoras"
                | "trabajador"
                | "trabajadores"
                | "trabajadora"
                | "trabajadoras"
        )
    }

    /// Verifica concordancia de "quien/quienes" con el antecedente
    fn check_quien_agreement(
        rel_index: usize,
        antecedent: &Token,
        relative: &Token,
    ) -> Option<RelativeCorrection> {
        let rel_lower = relative.effective_text().to_lowercase();

        // Solo verificar "quien" y "quienes"
        if rel_lower != "quien" && rel_lower != "quienes" {
            return None;
        }

        let antecedent_number = Self::get_antecedent_number(antecedent)?;

        if antecedent_number == Number::None {
            return None;
        }

        let rel_is_singular = rel_lower == "quien";
        let antecedent_is_singular = antecedent_number == Number::Singular;

        if rel_is_singular != antecedent_is_singular {
            let correct = if antecedent_is_singular {
                "quien"
            } else {
                "quienes"
            };

            // Preservar mayúsculas
            let suggestion = if relative
                .text
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                let mut chars = correct.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => correct.to_string(),
                }
            } else {
                correct.to_string()
            };

            return Some(RelativeCorrection {
                token_index: rel_index,
                original: relative.text.clone(),
                suggestion,
                message: format!(
                    "Concordancia: '{}' debe ser '{}' para concordar con '{}'",
                    relative.text, correct, antecedent.text
                ),
            });
        }

        None
    }

    /// Obtiene información del verbo (número, infinitivo, tiempo)
    /// Retorna (número, infinitivo, tiempo) con el infinitivo corregido
    fn get_verb_info_with_tense(
        verb: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Option<(Number, String, Tense)> {
        let (number, infinitive, tense) = Self::detect_verb_info(verb, verb_recognizer)?;
        let fixed = Self::fix_stem_changed_infinitive(&infinitive);
        Some((number, fixed, tense))
    }

    /// Corrige infinitivos extraídos de formas con cambio de raíz
    /// Ejemplo: "juegar" → "jugar", "sirver" → "servir", "durmir" → "dormir"
    fn fix_stem_changed_infinitive(candidate: &str) -> String {
        fix_stem_changed_infinitive_shared(candidate)
    }

    /// Detecta número, infinitivo (puede ser incorrecto para verbos con cambio de raíz) y tiempo
    fn detect_verb_info(
        verb: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Option<(Number, String, Tense)> {
        let get_infinitive = || -> Option<String> {
            if let Some(vr) = verb_recognizer {
                if let Some(mut inf) = vr.get_infinitive(verb) {
                    if let Some(base) = inf.strip_suffix("se") {
                        inf = base.to_string();
                    }
                    return Some(inf);
                }
            }
            None
        };

        // Verbos irregulares comunes - formas de tercera persona

        // ser - presente
        match verb {
            "es" => return Some((Number::Singular, "ser".to_string(), Tense::Present)),
            "son" => return Some((Number::Plural, "ser".to_string(), Tense::Present)),
            _ => {}
        }
        // ser - pretérito
        match verb {
            "fue" => return Some((Number::Singular, "ser".to_string(), Tense::Preterite)),
            "fueron" => return Some((Number::Plural, "ser".to_string(), Tense::Preterite)),
            _ => {}
        }
        // ser - imperfecto
        match verb {
            "era" => return Some((Number::Singular, "ser".to_string(), Tense::Imperfect)),
            "eran" => return Some((Number::Plural, "ser".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // estar - presente
        match verb {
            "está" | "esta" => {
                return Some((Number::Singular, "estar".to_string(), Tense::Present))
            }
            "están" | "estan" => {
                return Some((Number::Plural, "estar".to_string(), Tense::Present))
            }
            _ => {}
        }
        // estar - pretérito
        match verb {
            "estuvo" => return Some((Number::Singular, "estar".to_string(), Tense::Preterite)),
            "estuvieron" => return Some((Number::Plural, "estar".to_string(), Tense::Preterite)),
            _ => {}
        }
        // estar - imperfecto
        match verb {
            "estaba" => return Some((Number::Singular, "estar".to_string(), Tense::Imperfect)),
            "estaban" => return Some((Number::Plural, "estar".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // tener - presente
        match verb {
            "tiene" => return Some((Number::Singular, "tener".to_string(), Tense::Present)),
            "tienen" => return Some((Number::Plural, "tener".to_string(), Tense::Present)),
            _ => {}
        }
        // tener - pretérito
        match verb {
            "tuvo" => return Some((Number::Singular, "tener".to_string(), Tense::Preterite)),
            "tuvieron" => return Some((Number::Plural, "tener".to_string(), Tense::Preterite)),
            _ => {}
        }
        // tener - imperfecto
        match verb {
            "tenía" | "tenia" => {
                return Some((Number::Singular, "tener".to_string(), Tense::Imperfect))
            }
            "tenían" | "tenian" => {
                return Some((Number::Plural, "tener".to_string(), Tense::Imperfect))
            }
            _ => {}
        }

        // ir - presente
        match verb {
            "va" => return Some((Number::Singular, "ir".to_string(), Tense::Present)),
            "van" => return Some((Number::Plural, "ir".to_string(), Tense::Present)),
            _ => {}
        }
        // ir - pretérito (mismo que ser)
        match verb {
            "fue" => return Some((Number::Singular, "ir".to_string(), Tense::Preterite)),
            "fueron" => return Some((Number::Plural, "ir".to_string(), Tense::Preterite)),
            _ => {}
        }
        // ir - imperfecto
        match verb {
            "iba" => return Some((Number::Singular, "ir".to_string(), Tense::Imperfect)),
            "iban" => return Some((Number::Plural, "ir".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // hacer - presente
        match verb {
            "hace" => return Some((Number::Singular, "hacer".to_string(), Tense::Present)),
            "hacen" => return Some((Number::Plural, "hacer".to_string(), Tense::Present)),
            _ => {}
        }
        // hacer - pretérito
        match verb {
            "hizo" => return Some((Number::Singular, "hacer".to_string(), Tense::Preterite)),
            "hicieron" => return Some((Number::Plural, "hacer".to_string(), Tense::Preterite)),
            _ => {}
        }
        // hacer - imperfecto
        match verb {
            "hacía" | "hacia" => {
                return Some((Number::Singular, "hacer".to_string(), Tense::Imperfect))
            }
            "hacían" | "hacian" => {
                return Some((Number::Plural, "hacer".to_string(), Tense::Imperfect))
            }
            _ => {}
        }

        // venir - presente
        match verb {
            "viene" => return Some((Number::Singular, "venir".to_string(), Tense::Present)),
            "vienen" => return Some((Number::Plural, "venir".to_string(), Tense::Present)),
            _ => {}
        }
        // venir - pretérito
        match verb {
            "vino" => return Some((Number::Singular, "venir".to_string(), Tense::Preterite)),
            "vinieron" => return Some((Number::Plural, "venir".to_string(), Tense::Preterite)),
            _ => {}
        }
        // venir - imperfecto
        match verb {
            "venía" | "venia" => {
                return Some((Number::Singular, "venir".to_string(), Tense::Imperfect))
            }
            "venían" | "venian" => {
                return Some((Number::Plural, "venir".to_string(), Tense::Imperfect))
            }
            _ => {}
        }

        // poder - presente
        match verb {
            "puede" => return Some((Number::Singular, "poder".to_string(), Tense::Present)),
            "pueden" => return Some((Number::Plural, "poder".to_string(), Tense::Present)),
            _ => {}
        }
        // poder - pretérito
        match verb {
            "pudo" => return Some((Number::Singular, "poder".to_string(), Tense::Preterite)),
            "pudieron" => return Some((Number::Plural, "poder".to_string(), Tense::Preterite)),
            _ => {}
        }
        // poder - imperfecto
        match verb {
            "podía" | "podia" => {
                return Some((Number::Singular, "poder".to_string(), Tense::Imperfect))
            }
            "podían" | "podian" => {
                return Some((Number::Plural, "poder".to_string(), Tense::Imperfect))
            }
            _ => {}
        }

        // querer - presente
        match verb {
            "quiere" => return Some((Number::Singular, "querer".to_string(), Tense::Present)),
            "quieren" => return Some((Number::Plural, "querer".to_string(), Tense::Present)),
            _ => {}
        }
        // querer - pretérito
        match verb {
            "quiso" => return Some((Number::Singular, "querer".to_string(), Tense::Preterite)),
            "quisieron" => return Some((Number::Plural, "querer".to_string(), Tense::Preterite)),
            _ => {}
        }

        // decir - presente
        match verb {
            "dice" => return Some((Number::Singular, "decir".to_string(), Tense::Present)),
            "dicen" => return Some((Number::Plural, "decir".to_string(), Tense::Present)),
            _ => {}
        }
        // decir - pretérito
        match verb {
            "dijo" => return Some((Number::Singular, "decir".to_string(), Tense::Preterite)),
            "dijeron" => return Some((Number::Plural, "decir".to_string(), Tense::Preterite)),
            _ => {}
        }

        // saber - presente
        match verb {
            "sabe" => return Some((Number::Singular, "saber".to_string(), Tense::Present)),
            "saben" => return Some((Number::Plural, "saber".to_string(), Tense::Present)),
            _ => {}
        }
        // saber - pretérito
        match verb {
            "supo" => return Some((Number::Singular, "saber".to_string(), Tense::Preterite)),
            "supieron" => return Some((Number::Plural, "saber".to_string(), Tense::Preterite)),
            _ => {}
        }

        // ver - presente
        match verb {
            "ve" => return Some((Number::Singular, "ver".to_string(), Tense::Present)),
            "ven" => return Some((Number::Plural, "ver".to_string(), Tense::Present)),
            _ => {}
        }
        // ver - pretérito
        match verb {
            "vio" => return Some((Number::Singular, "ver".to_string(), Tense::Preterite)),
            "vieron" => return Some((Number::Plural, "ver".to_string(), Tense::Preterite)),
            _ => {}
        }
        // ver - imperfecto
        match verb {
            "veía" | "veia" => {
                return Some((Number::Singular, "ver".to_string(), Tense::Imperfect))
            }
            "veían" | "veian" => {
                return Some((Number::Plural, "ver".to_string(), Tense::Imperfect))
            }
            _ => {}
        }

        // dar - presente
        match verb {
            "da" => return Some((Number::Singular, "dar".to_string(), Tense::Present)),
            "dan" => return Some((Number::Plural, "dar".to_string(), Tense::Present)),
            _ => {}
        }
        // dar - pretérito
        match verb {
            "dio" => return Some((Number::Singular, "dar".to_string(), Tense::Preterite)),
            "dieron" => return Some((Number::Plural, "dar".to_string(), Tense::Preterite)),
            _ => {}
        }

        // llegar - presente
        match verb {
            "llega" => return Some((Number::Singular, "llegar".to_string(), Tense::Present)),
            "llegan" => return Some((Number::Plural, "llegar".to_string(), Tense::Present)),
            _ => {}
        }
        // llegar - pretérito
        match verb {
            "llegó" | "llego" => {
                return Some((Number::Singular, "llegar".to_string(), Tense::Preterite))
            }
            "llegaron" => return Some((Number::Plural, "llegar".to_string(), Tense::Preterite)),
            _ => {}
        }
        // llegar - imperfecto
        match verb {
            "llegaba" => return Some((Number::Singular, "llegar".to_string(), Tense::Imperfect)),
            "llegaban" => return Some((Number::Plural, "llegar".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // Verbos regulares - detectar tiempo y número

        // Pretérito perfecto simple -ar (cantó/cantaron)
        if let Some(stem) = verb.strip_suffix("aron") {
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}ar", stem), Tense::Preterite));
            }
        }
        if let Some(stem) = verb.strip_suffix("ó") {
            if !stem.is_empty() {
                if let Some(inf) = get_infinitive() {
                    if inf.ends_with("ar") {
                        return Some((Number::Singular, inf, Tense::Preterite));
                    }
                }
                if !stem.ends_with('i') {
                    return Some((Number::Singular, format!("{}ar", stem), Tense::Preterite));
                }
            }
        }

        // Pretérito perfecto simple -er/-ir (comió/comieron, vivió/vivieron)
        if let Some(stem) = verb.strip_suffix("ieron") {
            if !stem.is_empty() {
                if let Some(inf) = get_infinitive() {
                    if inf.ends_with("er") || inf.ends_with("ir") {
                        return Some((Number::Plural, inf, Tense::Preterite));
                    }
                }
                return Some((Number::Plural, format!("{}ir", stem), Tense::Preterite));
            }
        }
        if let Some(stem) = verb.strip_suffix("ió") {
            if !stem.is_empty() {
                if let Some(inf) = get_infinitive() {
                    if inf.ends_with("ar") || inf.ends_with("er") || inf.ends_with("ir") {
                        return Some((Number::Singular, inf, Tense::Preterite));
                    }
                }
                // Sin recognizer o sin desambiguación fiable, mantener fallback histórico.
                return Some((Number::Singular, format!("{}ir", stem), Tense::Preterite));
            }
        }

        // Segunda persona singular pretérito -er/-ir (comiste, leíste, viviste)
        // NOTA: Estas formas NO deben analizarse para concordancia de relativos
        // porque el sujeto es "tú", no el antecedente
        if verb.ends_with("iste") || verb.ends_with("íste") {
            // Retornar None para evitar análisis incorrecto
            return None;
        }
        // Segunda persona singular pretérito -ar (cantaste)
        if verb.ends_with("aste") {
            return None;
        }

        // Imperfecto -ar (cantaba/cantaban)
        if let Some(stem) = verb.strip_suffix("aban") {
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}ar", stem), Tense::Imperfect));
            }
        }
        if let Some(stem) = verb.strip_suffix("aba") {
            if !stem.is_empty() {
                return Some((Number::Singular, format!("{}ar", stem), Tense::Imperfect));
            }
        }

        // Imperfecto -er/-ir (comía/comían, vivía/vivían)
        if verb.ends_with("ían") || verb.ends_with("ian") {
            let stem = verb.trim_end_matches("ían").trim_end_matches("ian");
            if !stem.is_empty() {
                if let Some(inf) = get_infinitive() {
                    if inf.ends_with("er") || inf.ends_with("ir") {
                        return Some((Number::Plural, inf, Tense::Imperfect));
                    }
                }
                return Some((Number::Plural, format!("{}er", stem), Tense::Imperfect));
            }
        }
        if verb.ends_with("ía") || verb.ends_with("ia") {
            let stem = verb.trim_end_matches("ía").trim_end_matches("ia");
            if !stem.is_empty() {
                if let Some(inf) = get_infinitive() {
                    if inf.ends_with("er") || inf.ends_with("ir") {
                        return Some((Number::Singular, inf, Tense::Imperfect));
                    }
                }
                return Some((Number::Singular, format!("{}er", stem), Tense::Imperfect));
            }
        }

        // Presente indicativo -ar (canta/cantan)
        if verb.ends_with("an")
            && !verb.ends_with("ían")
            && !verb.ends_with("aban")
            && !verb.ends_with("aron")
        {
            let stem = &verb[..verb.len() - 2];
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}ar", stem), Tense::Present));
            }
        }
        if verb.ends_with("a") && !verb.ends_with("ía") && !verb.ends_with("aba") && verb.len() > 2
        {
            let stem = &verb[..verb.len() - 1];
            if !stem.is_empty() {
                return Some((Number::Singular, format!("{}ar", stem), Tense::Present));
            }
        }

        // Presente indicativo -er/-ir (come/comen, vive/viven)
        if verb.ends_with("en")
            && !verb.ends_with("ían")
            && !verb.ends_with("ien")
            && !verb.ends_with("ieron")
        {
            let stem = &verb[..verb.len() - 2];
            if !stem.is_empty() {
                if let Some(inf) = get_infinitive() {
                    if inf.ends_with("er") || inf.ends_with("ir") {
                        return Some((Number::Plural, inf, Tense::Present));
                    }
                }
                return Some((Number::Plural, format!("{}er", stem), Tense::Present));
            }
        }
        // Excluir adverbios terminados en -mente (probablemente, seguramente, etc.)
        if verb.ends_with("mente") {
            return None;
        }
        if verb.ends_with("e") && !verb.ends_with("ía") && verb.len() > 2 {
            let stem = &verb[..verb.len() - 1];
            if !stem.is_empty() {
                if let Some(inf) = get_infinitive() {
                    if inf.ends_with("er") || inf.ends_with("ir") {
                        return Some((Number::Singular, inf, Tense::Present));
                    }
                }
                return Some((Number::Singular, format!("{}er", stem), Tense::Present));
            }
        }

        None
    }

    /// Genera la forma correcta del verbo para el número y tiempo dados
    fn get_correct_verb_form_with_tense(
        infinitive: &str,
        number: Number,
        tense: Tense,
    ) -> Option<String> {
        match tense {
            Tense::Present => Self::get_correct_verb_form(infinitive, number),
            Tense::Preterite => Self::get_correct_verb_form_preterite(infinitive, number),
            Tense::Imperfect => Self::get_correct_verb_form_imperfect(infinitive, number),
            Tense::Future => Self::get_correct_verb_form_future(infinitive, number),
        }
    }

    /// Genera la forma correcta del verbo en pretérito
    fn get_correct_verb_form_preterite(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares en pretérito
        match infinitive {
            "ser" | "ir" => {
                return Some(
                    if number == Number::Singular {
                        "fue"
                    } else {
                        "fueron"
                    }
                    .to_string(),
                )
            }
            "estar" => {
                return Some(
                    if number == Number::Singular {
                        "estuvo"
                    } else {
                        "estuvieron"
                    }
                    .to_string(),
                )
            }
            "tener" => {
                return Some(
                    if number == Number::Singular {
                        "tuvo"
                    } else {
                        "tuvieron"
                    }
                    .to_string(),
                )
            }
            "hacer" => {
                return Some(
                    if number == Number::Singular {
                        "hizo"
                    } else {
                        "hicieron"
                    }
                    .to_string(),
                )
            }
            "venir" => {
                return Some(
                    if number == Number::Singular {
                        "vino"
                    } else {
                        "vinieron"
                    }
                    .to_string(),
                )
            }
            "poder" => {
                return Some(
                    if number == Number::Singular {
                        "pudo"
                    } else {
                        "pudieron"
                    }
                    .to_string(),
                )
            }
            "querer" => {
                return Some(
                    if number == Number::Singular {
                        "quiso"
                    } else {
                        "quisieron"
                    }
                    .to_string(),
                )
            }
            "decir" => {
                return Some(
                    if number == Number::Singular {
                        "dijo"
                    } else {
                        "dijeron"
                    }
                    .to_string(),
                )
            }
            "saber" => {
                return Some(
                    if number == Number::Singular {
                        "supo"
                    } else {
                        "supieron"
                    }
                    .to_string(),
                )
            }
            "ver" => {
                return Some(
                    if number == Number::Singular {
                        "vio"
                    } else {
                        "vieron"
                    }
                    .to_string(),
                )
            }
            "dar" => {
                return Some(
                    if number == Number::Singular {
                        "dio"
                    } else {
                        "dieron"
                    }
                    .to_string(),
                )
            }
            "poner" => {
                return Some(
                    if number == Number::Singular {
                        "puso"
                    } else {
                        "pusieron"
                    }
                    .to_string(),
                )
            }
            _ => {}
        }

        // Verbos regulares -ar
        if let Some(stem) = infinitive.strip_suffix("ar") {
            return Some(if number == Number::Singular {
                format!("{}ó", stem)
            } else {
                format!("{}aron", stem)
            });
        }

        // Verbos regulares -er/-ir
        if let Some(stem) = infinitive.strip_suffix("er") {
            return Some(if number == Number::Singular {
                format!("{}ió", stem)
            } else {
                format!("{}ieron", stem)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("ir") {
            // Verbos -ir con cambio de raíz tienen cambio especial en pretérito 3s/3p:
            // e→ie/e→i → e→i en pretérito (sentir→sintió, pedir→pidió)
            // o→ue → o→u en pretérito (dormir→durmió)
            let stem_changes = get_stem_changing_verbs();
            let preterite_stem = match stem_changes.get(infinitive).copied() {
                Some(StemChangeType::EToIe) | Some(StemChangeType::EToI) => {
                    Self::replace_last_occurrence(stem, "e", "i")
                }
                Some(StemChangeType::OToUe) => Self::replace_last_occurrence(stem, "o", "u"),
                _ => stem.to_string(),
            };
            return Some(if number == Number::Singular {
                format!("{}ió", preterite_stem)
            } else {
                format!("{}ieron", preterite_stem)
            });
        }

        None
    }

    /// Genera la forma correcta del verbo en imperfecto
    fn get_correct_verb_form_imperfect(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares en imperfecto
        match infinitive {
            "ser" => {
                return Some(
                    if number == Number::Singular {
                        "era"
                    } else {
                        "eran"
                    }
                    .to_string(),
                )
            }
            "ir" => {
                return Some(
                    if number == Number::Singular {
                        "iba"
                    } else {
                        "iban"
                    }
                    .to_string(),
                )
            }
            "ver" => {
                return Some(
                    if number == Number::Singular {
                        "veía"
                    } else {
                        "veían"
                    }
                    .to_string(),
                )
            }
            _ => {}
        }

        // Verbos regulares -ar
        if let Some(stem) = infinitive.strip_suffix("ar") {
            return Some(if number == Number::Singular {
                format!("{}aba", stem)
            } else {
                format!("{}aban", stem)
            });
        }

        // Verbos regulares -er/-ir
        if let Some(stem) = infinitive.strip_suffix("er") {
            return Some(if number == Number::Singular {
                format!("{}ía", stem)
            } else {
                format!("{}ían", stem)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("ir") {
            return Some(if number == Number::Singular {
                format!("{}ía", stem)
            } else {
                format!("{}ían", stem)
            });
        }

        None
    }

    /// Genera la forma correcta del verbo en futuro
    fn get_correct_verb_form_future(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares en futuro
        match infinitive {
            "tener" => {
                return Some(
                    if number == Number::Singular {
                        "tendrá"
                    } else {
                        "tendrán"
                    }
                    .to_string(),
                )
            }
            "poder" => {
                return Some(
                    if number == Number::Singular {
                        "podrá"
                    } else {
                        "podrán"
                    }
                    .to_string(),
                )
            }
            "querer" => {
                return Some(
                    if number == Number::Singular {
                        "querrá"
                    } else {
                        "querrán"
                    }
                    .to_string(),
                )
            }
            "hacer" => {
                return Some(
                    if number == Number::Singular {
                        "hará"
                    } else {
                        "harán"
                    }
                    .to_string(),
                )
            }
            "decir" => {
                return Some(
                    if number == Number::Singular {
                        "dirá"
                    } else {
                        "dirán"
                    }
                    .to_string(),
                )
            }
            "venir" => {
                return Some(
                    if number == Number::Singular {
                        "vendrá"
                    } else {
                        "vendrán"
                    }
                    .to_string(),
                )
            }
            "saber" => {
                return Some(
                    if number == Number::Singular {
                        "sabrá"
                    } else {
                        "sabrán"
                    }
                    .to_string(),
                )
            }
            _ => {}
        }

        // Verbos regulares - futuro usa el infinitivo completo
        Some(if number == Number::Singular {
            format!("{}á", infinitive)
        } else {
            format!("{}án", infinitive)
        })
    }

    /// Genera la forma correcta del verbo para el número dado (presente indicativo)
    fn get_correct_verb_form(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares
        match infinitive {
            "ser" => {
                return Some(
                    if number == Number::Singular {
                        "es"
                    } else {
                        "son"
                    }
                    .to_string(),
                )
            }
            "estar" => {
                return Some(
                    if number == Number::Singular {
                        "está"
                    } else {
                        "están"
                    }
                    .to_string(),
                )
            }
            "tener" => {
                return Some(
                    if number == Number::Singular {
                        "tiene"
                    } else {
                        "tienen"
                    }
                    .to_string(),
                )
            }
            "ir" => {
                return Some(
                    if number == Number::Singular {
                        "va"
                    } else {
                        "van"
                    }
                    .to_string(),
                )
            }
            "hacer" => {
                return Some(
                    if number == Number::Singular {
                        "hace"
                    } else {
                        "hacen"
                    }
                    .to_string(),
                )
            }
            "venir" => {
                return Some(
                    if number == Number::Singular {
                        "viene"
                    } else {
                        "vienen"
                    }
                    .to_string(),
                )
            }
            "poder" => {
                return Some(
                    if number == Number::Singular {
                        "puede"
                    } else {
                        "pueden"
                    }
                    .to_string(),
                )
            }
            "querer" => {
                return Some(
                    if number == Number::Singular {
                        "quiere"
                    } else {
                        "quieren"
                    }
                    .to_string(),
                )
            }
            "decir" => {
                return Some(
                    if number == Number::Singular {
                        "dice"
                    } else {
                        "dicen"
                    }
                    .to_string(),
                )
            }
            "saber" => {
                return Some(
                    if number == Number::Singular {
                        "sabe"
                    } else {
                        "saben"
                    }
                    .to_string(),
                )
            }
            "ver" => {
                return Some(
                    if number == Number::Singular {
                        "ve"
                    } else {
                        "ven"
                    }
                    .to_string(),
                )
            }
            "dar" => {
                return Some(
                    if number == Number::Singular {
                        "da"
                    } else {
                        "dan"
                    }
                    .to_string(),
                )
            }
            "llegar" => {
                return Some(
                    if number == Number::Singular {
                        "llega"
                    } else {
                        "llegan"
                    }
                    .to_string(),
                )
            }
            _ => {}
        }

        // Determinar si tiene cambio de raíz (presente 3s/3p)
        // CToZc solo afecta a 1s, no a 3s/3p
        let stem_changes = get_stem_changing_verbs();
        let change_type = stem_changes.get(infinitive).copied();
        let needs_stem_change =
            change_type.is_some() && !matches!(change_type, Some(StemChangeType::CToZc));

        // Verbos regulares (y con cambio de raíz)
        if let Some(stem) = infinitive.strip_suffix("ar") {
            let s = if needs_stem_change {
                Self::apply_stem_change(stem, change_type.unwrap())
            } else {
                stem.to_string()
            };
            return Some(if number == Number::Singular {
                format!("{}a", s)
            } else {
                format!("{}an", s)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("er") {
            let s = if needs_stem_change {
                Self::apply_stem_change(stem, change_type.unwrap())
            } else {
                stem.to_string()
            };
            return Some(if number == Number::Singular {
                format!("{}e", s)
            } else {
                format!("{}en", s)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("ir") {
            let s = if needs_stem_change {
                Self::apply_stem_change(stem, change_type.unwrap())
            } else {
                stem.to_string()
            };
            return Some(if number == Number::Singular {
                format!("{}e", s)
            } else {
                format!("{}en", s)
            });
        }

        None
    }

    /// Aplica el cambio de raíz a un stem (última ocurrencia de la vocal original)
    fn apply_stem_change(stem: &str, change_type: StemChangeType) -> String {
        let (original, changed) = change_type.change_pair();

        if change_type == StemChangeType::CToZc {
            if let Some(pos) = stem.rfind('c') {
                let mut result = String::with_capacity(stem.len() + 1);
                result.push_str(&stem[..pos]);
                result.push_str("zc");
                result.push_str(&stem[pos + 1..]);
                return result;
            }
            return stem.to_string();
        }

        if let Some(pos) = stem.rfind(original) {
            let mut result = String::with_capacity(stem.len() + changed.len());
            result.push_str(&stem[..pos]);
            result.push_str(changed);
            result.push_str(&stem[pos + original.len()..]);
            return result;
        }

        stem.to_string()
    }

    /// Reemplaza la última ocurrencia de `from` por `to` en el stem
    fn replace_last_occurrence(stem: &str, from: &str, to: &str) -> String {
        if let Some(pos) = stem.rfind(from) {
            let mut result = String::with_capacity(stem.len() + to.len());
            result.push_str(&stem[..pos]);
            result.push_str(to);
            result.push_str(&stem[pos + from.len()..]);
            result
        } else {
            stem.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::{DictionaryLoader, Trie};
    use crate::grammar::tokenizer::Tokenizer;
    use crate::languages::spanish::VerbRecognizer;

    fn setup_tokens(text: &str) -> Vec<Token> {
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };

        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize(text);

        // Enriquecer tokens con información del diccionario
        for token in &mut tokens {
            if token.token_type == TokenType::Word {
                if let Some(info) = dictionary.get(&token.effective_text().to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        tokens
    }

    fn analyze_with_dictionary(text: &str) -> Option<Vec<RelativeCorrection>> {
        let dict_path = std::path::Path::new("data/es/words.txt");
        if !dict_path.exists() {
            return None;
        }
        let dictionary =
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new());
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize(text);

        for token in &mut tokens {
            if token.token_type == TokenType::Word {
                if let Some(info) = dictionary.get(&token.effective_text().to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        Some(RelativeAnalyzer::analyze_with_recognizer(
            &tokens,
            Some(&recognizer),
        ))
    }

    #[test]
    fn test_persona_que_vinieron() {
        let tokens = setup_tokens("la persona que vinieron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "vinieron");
        assert_eq!(corrections[0].suggestion, "vino");
    }

    #[test]
    fn test_personas_que_vino() {
        // "vino" es pretérito, la corrección debe ser "vinieron" (pretérito plural)
        let tokens = setup_tokens("las personas que vino");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "vino");
        assert_eq!(corrections[0].suggestion, "vinieron");
    }

    #[test]
    fn test_nino_que_cantan() {
        let tokens = setup_tokens("el niño que cantan");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "cantan");
        assert_eq!(corrections[0].suggestion, "canta");
    }

    #[test]
    fn test_ninos_que_canta() {
        let tokens = setup_tokens("los niños que canta");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "canta");
        assert_eq!(corrections[0].suggestion, "cantan");
    }

    #[test]
    fn test_libro_que_fueron() {
        let tokens = setup_tokens("el libro que fueron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "fue");
    }

    #[test]
    fn test_libros_que_fue() {
        let tokens = setup_tokens("los libros que fue");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "fueron");
    }

    #[test]
    fn test_casa_que_tiene_correct() {
        let tokens = setup_tokens("la casa que tiene");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería haber correcciones para concordancia correcta"
        );
    }

    #[test]
    fn test_casas_que_tienen_correct() {
        let tokens = setup_tokens("las casas que tienen");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería haber correcciones para concordancia correcta"
        );
    }

    #[test]
    fn test_mujer_que_llegaron() {
        let tokens = setup_tokens("la mujer que llegaron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "llegó");
    }

    #[test]
    fn test_hombres_que_llegó() {
        // "llegó" es pretérito, la corrección debe ser "llegaron" (pretérito plural)
        let tokens = setup_tokens("los hombres que llegó");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "llegaron");
    }

    #[test]
    fn test_personas_quien() {
        let tokens = setup_tokens("las personas quien");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "quien");
        assert_eq!(corrections[0].suggestion, "quienes");
    }

    #[test]
    fn test_persona_quienes() {
        let tokens = setup_tokens("la persona quienes");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "quienes");
        assert_eq!(corrections[0].suggestion, "quien");
    }

    #[test]
    fn test_persona_quien_correct() {
        let tokens = setup_tokens("la persona quien");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        // Filtrar solo correcciones de quien/quienes
        let quien_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "quien" || c.original == "quienes")
            .collect();
        assert!(
            quien_corrections.is_empty(),
            "No debería corregir 'persona quien' que es correcto"
        );
    }

    #[test]
    fn test_problema_que_tienen() {
        let tokens = setup_tokens("el problema que tienen");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "Caso ambiguo de relativo de objeto: no debe forzar corrección",
        );
    }

    #[test]
    fn test_problemas_que_tiene() {
        let tokens = setup_tokens("los problemas que tiene");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "Caso ambiguo de relativo de objeto: no debe forzar corrección",
        );
    }

    #[test]
    fn test_verb_info_irregulars() {
        let info = RelativeAnalyzer::get_verb_info_with_tense("es", None);
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Singular);
        assert_eq!(inf, "ser");

        let info = RelativeAnalyzer::get_verb_info_with_tense("son", None);
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Plural);
        assert_eq!(inf, "ser");

        let info = RelativeAnalyzer::get_verb_info_with_tense("tiene", None);
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Singular);
        assert_eq!(inf, "tener");

        let info = RelativeAnalyzer::get_verb_info_with_tense("tienen", None);
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Plural);
        assert_eq!(inf, "tener");
    }

    #[test]
    fn test_preterite_iar_relative_uses_ar_with_recognizer() {
        let corrections = match analyze_with_dictionary("los hombres que cambió") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "cambió");
        assert!(
            correction.is_some(),
            "Debe corregir 'cambió' en relativo plural"
        );
        assert_eq!(correction.unwrap().suggestion, "cambiaron");

        let corrections = analyze_with_dictionary("los hombres que copió").unwrap();
        let correction = corrections.iter().find(|c| c.original == "copió");
        assert!(
            correction.is_some(),
            "Debe corregir 'copió' en relativo plural"
        );
        assert_eq!(correction.unwrap().suggestion, "copiaron");

        let corrections = analyze_with_dictionary("los hombres que envió").unwrap();
        let correction = corrections.iter().find(|c| c.original == "envió");
        if let Some(c) = correction {
            assert_ne!(
                c.suggestion, "envieron",
                "No debe generar la forma inexistente 'envieron'",
            );
        }
    }

    #[test]
    fn test_preterite_er_ir_relative_still_correct_with_recognizer() {
        let corrections = match analyze_with_dictionary("los hombres que comió") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "comió");
        assert!(
            correction.is_some(),
            "Debe corregir 'comió' en relativo plural"
        );
        assert_eq!(correction.unwrap().suggestion, "comieron");
    }

    #[test]
    fn test_transitive_regular_ir_present_not_forced_by_antecedent_number() {
        // "el libro que abren" suele ser relativo de objeto (sujeto implícito plural),
        // no debe forzar concordancia singular por el antecedente.
        let corrections = match analyze_with_dictionary("el libro que abren") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "abren");
        assert!(
            correction.is_none(),
            "No debe corregir 'abren' en relativo de objeto transitivo",
        );

        let corrections = analyze_with_dictionary("los libros que abre").unwrap();
        let correction = corrections.iter().find(|c| c.original == "abre");
        assert!(
            correction.is_none(),
            "No debe corregir 'abre' en relativo de objeto transitivo",
        );
    }

    #[test]
    fn test_transitive_regular_ir_imperfect_not_forced_by_antecedent_number() {
        // Misma idea en imperfecto.
        let corrections = match analyze_with_dictionary("el libro que abrían") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "abrían");
        assert!(
            correction.is_none(),
            "No debe corregir 'abrían' en relativo de objeto transitivo",
        );

        let corrections = analyze_with_dictionary("los libros que abría").unwrap();
        let correction = corrections.iter().find(|c| c.original == "abría");
        assert!(
            correction.is_none(),
            "No debe corregir 'abría' en relativo de objeto transitivo",
        );

        let corrections = analyze_with_dictionary("el plato que servían").unwrap();
        let correction = corrections.iter().find(|c| c.original == "servían");
        assert!(
            correction.is_none(),
            "No debe corregir 'servían' en relativo de objeto transitivo",
        );
    }

    #[test]
    fn test_transitive_regular_ar_preterite_not_forced_by_antecedent_number_redactar() {
        // "el acta que redactaron" puede ser relativo de objeto con sujeto implícito plural.
        let corrections = match analyze_with_dictionary("el acta que redactaron") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "redactaron");
        assert!(
            correction.is_none(),
            "No debe corregir 'redactaron' en relativo de objeto transitivo",
        );

        let corrections = analyze_with_dictionary("las actas que redactó").unwrap();
        let correction = corrections.iter().find(|c| c.original == "redactó");
        assert!(
            correction.is_none(),
            "No debe corregir 'redactó' en relativo de objeto transitivo",
        );
    }

    #[test]
    fn test_transitive_regular_ar_preterite_not_forced_by_antecedent_number_revisar() {
        let corrections = match analyze_with_dictionary("el informe que revisaron") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "revisaron");
        assert!(
            correction.is_none(),
            "No debe corregir 'revisaron' en relativo de objeto transitivo",
        );
    }

    #[test]
    fn test_transitive_regular_ir_preterite_not_forced_by_antecedent_number_definir() {
        let corrections = match analyze_with_dictionary("la estrategia que definieron") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "definieron");
        assert!(
            correction.is_none(),
            "No debe corregir 'definieron' en relativo de objeto transitivo",
        );
    }

    #[test]
    fn test_transitive_regular_ar_preterite_not_forced_by_antecedent_number_presentar() {
        let corrections = match analyze_with_dictionary("el resumen que presentaron") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "presentaron");
        assert!(
            correction.is_none(),
            "No debe corregir 'presentaron' en relativo de objeto transitivo",
        );
    }

    #[test]
    fn test_postposed_subject_proper_name_maria_no_infinite_loop() {
        let corrections = match analyze_with_dictionary("la lista que completaron María") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections.iter().find(|c| c.original == "completaron");
        assert!(
            correction.is_none(),
            "No debe forzar corrección en relativo con sujeto pospuesto de nombre propio",
        );
    }

    #[test]
    fn test_relative_skips_proper_name_or_pronoun_before_real_verb() {
        let corrections = match analyze_with_dictionary("los libros que María compró son buenos")
        {
            Some(c) => c,
            None => return,
        };
        assert!(
            corrections
                .iter()
                .all(|c| !c.original.eq_ignore_ascii_case("María")),
            "No debe tratar 'María' como verbo en relativo: {:?}",
            corrections
        );

        let corrections = match analyze_with_dictionary("los coches que ella conduce son rápidos")
        {
            Some(c) => c,
            None => return,
        };
        assert!(
            corrections
                .iter()
                .all(|c| !c.original.eq_ignore_ascii_case("ella")),
            "No debe tratar 'ella' como verbo en relativo: {:?}",
            corrections
        );
    }

    #[test]
    fn test_relative_after_como_example_apposition_keeps_plural_head() {
        let corrections = match analyze_with_dictionary(
            "Competiciones de resistencia como Hyrox, que combinan carrera y ejercicios",
        ) {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "combinan");
        assert!(
            correction.is_none(),
            "No debe corregir 'combinan' en relativo cuyo antecedente real es 'Competiciones': {corrections:?}"
        );
    }

    #[test]
    fn test_relative_after_como_example_apposition_with_spelling_suggestion_keeps_plural_head() {
        let mut tokens =
            setup_tokens("Competiciones de resistencia como Hyrox, que combinan carrera y ejercicios");
        if let Some((_, token)) = tokens
            .iter_mut()
            .enumerate()
            .find(|(_, t)| t.token_type == TokenType::Word && t.text.eq_ignore_ascii_case("Hyrox"))
        {
            token.corrected_spelling = Some("héroe".to_string());
            token.word_info = Some(crate::dictionary::WordInfo {
                category: WordCategory::Sustantivo,
                gender: crate::dictionary::Gender::Masculine,
                number: crate::dictionary::Number::Singular,
                extra: String::new(),
                frequency: 500,
            });
        }
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "combinan");
        assert!(
            correction.is_none(),
            "No debe corregir 'combinan' cuando el ejemplo tras 'como' recibe sugerencia ortográfica: {corrections:?}"
        );
    }

    #[test]
    fn test_get_correct_form_preterite_irregular_poner() {
        assert_eq!(
            RelativeAnalyzer::get_correct_verb_form_with_tense(
                "poner",
                Number::Singular,
                Tense::Preterite
            ),
            Some("puso".to_string())
        );
        assert_eq!(
            RelativeAnalyzer::get_correct_verb_form_with_tense(
                "poner",
                Number::Plural,
                Tense::Preterite
            ),
            Some("pusieron".to_string())
        );
    }

    #[test]
    fn test_get_correct_form() {
        assert_eq!(
            RelativeAnalyzer::get_correct_verb_form("ser", Number::Singular),
            Some("es".to_string())
        );
        assert_eq!(
            RelativeAnalyzer::get_correct_verb_form("ser", Number::Plural),
            Some("son".to_string())
        );
        assert_eq!(
            RelativeAnalyzer::get_correct_verb_form("cantar", Number::Singular),
            Some("canta".to_string())
        );
        assert_eq!(
            RelativeAnalyzer::get_correct_verb_form("cantar", Number::Plural),
            Some("cantan".to_string())
        );
    }

    #[test]
    fn test_sentence_boundary_prevents_false_positive() {
        // "Que vengan" es subjuntivo exhortativo, no relativo de "agresion"
        // El punto y comillas de cierre deben impedir que "agresion" sea antecedente
        let tokens = setup_tokens("no a otra agresion\". \"Que vengan todos");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        // No debe haber correccion de "vengan" a "venga"
        let vengan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "vengan")
            .collect();
        assert!(
            vengan_corrections.is_empty(),
            "No debe corregir 'vengan' cuando hay limite de oracion"
        );
    }

    #[test]
    fn test_exhortative_que_at_start() {
        // "Que vengan" al inicio es exhortativo, no relativo
        let tokens = setup_tokens("Que vengan todos");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vengan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "vengan")
            .collect();
        assert!(
            vengan_corrections.is_empty(),
            "No debe corregir subjuntivo exhortativo al inicio"
        );
    }

    #[test]
    fn test_exhortative_que_with_clitic() {
        // "Que lo hagan" es exhortativo
        let tokens = setup_tokens("Que lo hagan ellos");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let hagan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "hagan")
            .collect();
        assert!(
            hagan_corrections.is_empty(),
            "No debe corregir subjuntivo exhortativo con clítico"
        );
    }

    #[test]
    fn test_relative_with_subjunctive_corrected() {
        // "la persona que vengan" SÍ es relativo y debe corregirse
        let tokens = setup_tokens("la persona que vengan");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1, "Debe corregir relativo real");
        assert_eq!(corrections[0].original, "vengan");
        assert_eq!(corrections[0].suggestion, "venga");
    }

    #[test]
    fn test_explicative_clause_singular_antecedent() {
        // Cláusula explicativa con coma: "El presidente, que viajaron" → "viajó"
        let tokens = setup_tokens("El presidente, que viajaron ayer");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1, "Debe corregir cláusula explicativa");
        assert_eq!(corrections[0].original, "viajaron");
        assert_eq!(corrections[0].suggestion, "viajó");
    }

    #[test]
    fn test_explicative_clause_plural_antecedent() {
        // Cláusula explicativa con coma: "Los ministros, que viajó" → "viajaron"
        let tokens = setup_tokens("Los ministros, que viajó ayer");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(
            corrections.len(),
            1,
            "Debe corregir cláusula explicativa plural"
        );
        assert_eq!(corrections[0].original, "viajó");
        assert_eq!(corrections[0].suggestion, "viajaron");
    }

    #[test]
    fn test_explicative_clause_with_prep_phrase() {
        // Cláusula explicativa con frase preposicional: busca antecedente más atrás
        let tokens = setup_tokens("El director de empresa, que anunciaron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(
            corrections.len(),
            1,
            "Debe encontrar antecedente 'director'"
        );
        assert_eq!(corrections[0].original, "anunciaron");
        assert_eq!(corrections[0].suggestion, "anunció");
    }

    #[test]
    fn test_completive_que_with_verb_before_comma() {
        // "Juan dijo, que vendrían" - "que" es completivo, NO relativo
        // No debe usar ventana extendida porque "dijo" (verbo) precede a ", que"
        let tokens = setup_tokens("Juan dijo, que vendrían todos");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vendrian_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "vendrían")
            .collect();
        assert!(
            vendrian_corrections.is_empty(),
            "No debe corregir 'que' completivo tras verbo"
        );
    }

    #[test]
    fn test_completive_que_after_spurious_nominal_verb_form_not_corrected() {
        let corrections = analyze_with_dictionary("si viéramos que realmente ya está a la vuelta")
            .expect("Debe cargar diccionario para este test");
        let has_false_positive = corrections.iter().any(|c| {
            let original = c.original.to_lowercase();
            let suggestion = c.suggestion.to_lowercase();
            (original == "está" || original == "esta")
                && (suggestion == "están" || suggestion == "estan")
        });
        assert!(
            !has_false_positive,
            "No debe forzar 'está' -> 'están' cuando 'que' es completivo: {:?}",
            corrections
        );
    }

    #[test]
    fn test_completive_que_after_coordinated_indirect_object_not_corrected() {
        let corrections = analyze_with_dictionary(
            "se les dice a los narcisistas o a cualquier otra persona que son inteligentes",
        )
        .expect("Debe cargar diccionario para este test");

        let wrong = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("son")
                && (c.suggestion.eq_ignore_ascii_case("es")
                    || c.suggestion.eq_ignore_ascii_case("son"))
        });
        assert!(
            wrong.is_none(),
            "No debe tratar completiva coordinada como relativo singular: {:?}",
            corrections
        );
    }

    #[test]
    fn test_puesto_que_causal_not_treated_as_relative() {
        let corrections = analyze_with_dictionary(
            "puesto que apuntan a que la humedad convierte la casa en una incubadora",
        )
        .expect("Debe cargar diccionario para este test");

        let wrong = corrections.iter().find(|c| {
            c.original.eq_ignore_ascii_case("apuntan")
                && c.suggestion.eq_ignore_ascii_case("apunta")
        });
        assert!(
            wrong.is_none(),
            "No debe tratar 'puesto que' como relativo nominal: {:?}",
            corrections
        );
    }

    #[test]
    fn test_explicative_cuts_at_second_comma() {
        // "Los ministros, la directora, que viajó" - la segunda coma corta la búsqueda
        // Si el corte NO funcionara, encontraría "ministros" (plural) y corregiría "viajó" → "viajaron"
        // Con el corte, encuentra "directora" (singular) y NO corrige "viajó"
        let tokens = setup_tokens("Los ministros, la directora, que viajó bien");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let viajo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "viajó")
            .collect();
        assert!(
            viajo_corrections.is_empty(),
            "No debe corregir 'viajó' - el antecedente es 'directora' (singular), no 'ministros'"
        );
    }

    #[test]
    fn test_explicative_cuts_at_strong_punctuation() {
        // "Los ministros. El presidente, que viajaron" - el punto corta la búsqueda
        // Si el corte NO funcionara, encontraría "ministros" (plural) y NO corregiría "viajaron"
        // Con el corte, encuentra "presidente" (singular) y corrige "viajaron" → "viajó"
        let tokens = setup_tokens("Los ministros. El presidente, que viajaron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(
            corrections.len(),
            1,
            "Debe corregir porque 'presidente' es singular"
        );
        assert_eq!(corrections[0].original, "viajaron");
        assert_eq!(corrections[0].suggestion, "viajó");
    }

    #[test]
    fn test_noun_de_adj_noun_que_verb_pattern() {
        // En "acelerón de dos décimas que elevó", el antecedente es "acelerón" (singular)
        // No debe sugerir "elevaron" porque "décimas" (plural) no es el sujeto
        let tokens = setup_tokens("un acelerón de dos décimas que elevó el avance");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let elevo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "elevó")
            .collect();
        assert!(
            elevo_corrections.is_empty(),
            "No debe corregir 'elevó' - el antecedente es 'acelerón' (singular), no 'décimas'"
        );
    }

    #[test]
    fn test_noun_de_noun_que_verb_pattern() {
        // En "marcos de referencia que sirven", el antecedente es "marcos" (plural)
        // No debe sugerir "sirve" porque "referencia" (singular) no es el sujeto
        let tokens = setup_tokens("los marcos de referencia que sirven de guía");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let sirven_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "sirven")
            .collect();
        assert!(
            sirven_corrections.is_empty(),
            "No debe corregir 'sirven' - el antecedente es 'marcos' (plural), no 'referencia'"
        );
    }

    #[test]
    fn test_conjunto_de_plural_noun_relative_singular_not_corrected() {
        // "el conjunto de microorganismos que habita" -> el núcleo es "conjunto" (singular)
        let tokens = setup_tokens("el conjunto de microorganismos que habita en el intestino");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let habita_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "habita")
            .collect();
        assert!(
            habita_corrections.is_empty(),
            "No debe corregir 'habita' cuando el núcleo es 'conjunto': {:?}",
            corrections
        );
    }

    #[test]
    fn test_conjunto_de_plural_noun_relative_plural_also_not_corrected() {
        // También aceptar lectura semántica plural: "microorganismos ... habitan".
        let tokens = setup_tokens("el conjunto de microorganismos que habitan en el intestino");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let habitan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "habitan")
            .collect();
        assert!(
            habitan_corrections.is_empty(),
            "No debe corregir 'habitan' en lectura plural válida: {:?}",
            corrections
        );
    }

    #[test]
    fn test_conjunto_de_article_plural_relative_singular_not_corrected() {
        // Con artículo en el complemento: también debe aceptar concordancia con el núcleo.
        let tokens = setup_tokens("el conjunto de los microorganismos que habita en el intestino");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let habita_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "habita")
            .collect();
        assert!(
            habita_corrections.is_empty(),
            "No debe forzar plural en 'conjunto de los ... que habita': {:?}",
            corrections
        );
    }

    #[test]
    fn test_conjunto_de_article_plural_relative_plural_not_corrected() {
        // Y mantener válida la lectura plural con artículo en el complemento.
        let tokens = setup_tokens("el conjunto de los microorganismos que habitan en el intestino");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let habitan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "habitan")
            .collect();
        assert!(
            habitan_corrections.is_empty(),
            "No debe corregir la lectura plural en 'conjunto de los ... que habitan': {:?}",
            corrections
        );
    }

    #[test]
    fn test_partitive_relative_accepts_singular_agreement() {
        // "la mayoría ... que vino": concordancia con el núcleo partitivo (singular).
        let tokens = setup_tokens("la mayoría de estudiantes que vino a clase");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vino_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "vino")
            .collect();
        assert!(
            vino_corrections.is_empty(),
            "No debe forzar plural en concordancia partitiva singular: {:?}",
            corrections
        );
    }

    #[test]
    fn test_partitive_relative_accepts_singular_with_article_complement() {
        // "la mayoría de los estudiantes ... que vino": también puede concordar en singular.
        let tokens = setup_tokens("la mayoría de los estudiantes que vino a clase");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vino_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "vino")
            .collect();
        assert!(
            vino_corrections.is_empty(),
            "No debe forzar plural cuando hay artículo en complemento partitivo: {:?}",
            corrections
        );
    }

    #[test]
    fn test_partitive_relative_accepts_plural_agreement() {
        // Y mantener la lectura plural: "la mayoría ... que vinieron".
        let tokens = setup_tokens("la mayoría de estudiantes que vinieron a clase");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vinieron_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "vinieron")
            .collect();
        assert!(
            vinieron_corrections.is_empty(),
            "No debe corregir la lectura plural partitiva válida: {:?}",
            corrections
        );
    }

    #[test]
    fn test_uno_de_los_que_singular_relative_is_corrected() {
        let tokens = setup_tokens("es uno de los que vino temprano");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vino_correction = corrections.iter().find(|c| c.original == "vino");
        assert!(
            vino_correction.is_some(),
            "Debe corregir 'vino' en 'uno de los que ...': {:?}",
            corrections
        );
        assert_eq!(vino_correction.unwrap().suggestion, "vinieron");
    }

    #[test]
    fn test_una_de_las_que_singular_relative_is_corrected() {
        let tokens = setup_tokens("es una de las que vino temprano");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vino_correction = corrections.iter().find(|c| c.original == "vino");
        assert!(
            vino_correction.is_some(),
            "Debe corregir 'vino' en 'una de las que ...': {:?}",
            corrections
        );
        assert_eq!(vino_correction.unwrap().suggestion, "vinieron");
    }

    #[test]
    fn test_uno_de_los_que_plural_relative_not_corrected() {
        let tokens = setup_tokens("es uno de los que vinieron temprano");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let vinieron_correction = corrections.iter().find(|c| c.original == "vinieron");
        assert!(
            vinieron_correction.is_none(),
            "No debe corregir 'vinieron' cuando ya concuerda en 'uno de los que ...': {:?}",
            corrections
        );
    }

    #[test]
    fn test_uno_de_los_que_mejor_juega_is_corrected() {
        let tokens = setup_tokens("es uno de los que mejor juega");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let juega_correction = corrections.iter().find(|c| c.original == "juega");
        assert!(
            juega_correction.is_some(),
            "Debe corregir 'juega' en 'uno de los que mejor ...': {:?}",
            corrections
        );
        assert_eq!(juega_correction.unwrap().suggestion, "juegan");
    }

    #[test]
    fn test_uno_de_los_que_mejor_juegan_not_corrected() {
        let tokens = setup_tokens("es uno de los que mejor juegan");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let juegan_correction = corrections.iter().find(|c| c.original == "juegan");
        assert!(
            juegan_correction.is_none(),
            "No debe corregir 'juegan' cuando ya concuerda con 'uno de los que mejor ...': {:?}",
            corrections
        );
    }

    #[test]
    fn test_noun_de_article_noun_que_verb_pattern() {
        // En "actualización de los umbrales que determinan", el antecedente es "umbrales" (plural)
        // porque tiene artículo definido "los"
        // No debe sugerir "determina" porque "umbrales" es el verdadero sujeto
        let tokens = setup_tokens("la actualización de los umbrales que determinan el tamaño");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let det_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "determinan")
            .collect();
        assert!(det_corrections.is_empty(),
            "No debe corregir 'determinan' - el antecedente es 'umbrales' (con artículo 'los'), no 'actualización'");
    }

    #[test]
    fn test_indefinite_article_noun_que_verb() {
        // En "un escenario que contrarresta", el antecedente es "escenario" (singular)
        // El artículo indefinido "un" indica inicio de nuevo sintagma nominal
        // No debe buscar más atrás para encontrar un antecedente plural
        let tokens =
            setup_tokens("con modelos innovadores un escenario que contrarresta los efectos");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let contra_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "contrarresta")
            .collect();
        assert!(contra_corrections.is_empty(),
            "No debe corregir 'contrarresta' - el antecedente es 'escenario' (con artículo 'un'), no 'modelos'");
    }

    #[test]
    fn test_nominalized_adjective_as_antecedent() {
        // En "El estampado de lunares, que irrumpió", el antecedente es "estampado" (singular)
        // Aunque "estampado" es adjetivo en el diccionario, "El estampado" es un adjetivo nominalizado
        // No debe sugerir "irrumpieron" porque "lunares" (plural) no es el sujeto
        let tokens = setup_tokens("El estampado de lunares que irrumpió con fuerza");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let irrumpio_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "irrumpió")
            .collect();
        assert!(irrumpio_corrections.is_empty(),
            "No debe corregir 'irrumpió' - el antecedente es 'estampado' (nominalizado, singular), no 'lunares'");
    }

    #[test]
    fn test_noun_adjective_que_verb_pattern() {
        // En "enfoques integrales que incluyan", el antecedente es "enfoques" (plural)
        // No debe sugerir "incluya" porque el adjetivo "integrales" modifica a "enfoques",
        // no es el antecedente del relativo
        let tokens =
            setup_tokens("España abogó por enfoques integrales que incluyan mejores condiciones");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let incluyan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "incluyan")
            .collect();
        assert!(
            incluyan_corrections.is_empty(),
            "No debe corregir 'incluyan' - el antecedente es 'enfoques' (plural), no 'integrales'"
        );
    }

    #[test]
    fn test_noun_adjective_que_verb_singular_correction() {
        // En "el problema grave que afectan", el antecedente es "problema" (singular)
        // DEBE sugerir "afecta" porque "problema" es singular y "afectan" es plural
        let tokens = setup_tokens("el problema grave que afectan a la sociedad");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let afectan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "afectan")
            .collect();
        assert_eq!(
            afectan_corrections.len(),
            1,
            "Debe corregir 'afectan' a 'afecta' - el antecedente es 'problema' (singular)"
        );
        assert_eq!(afectan_corrections[0].suggestion, "afecta");
    }

    #[test]
    fn test_noun_multiple_adjectives_que_verb() {
        // En "los problemas graves internacionales que afectan", el antecedente es "problemas" (plural)
        // No debe sugerir corrección porque tanto sustantivo como verbo son plurales
        let tokens = setup_tokens("los problemas graves internacionales que afectan al país");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let afectan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "afectan")
            .collect();
        assert!(
            afectan_corrections.is_empty(),
            "No debe corregir 'afectan' - el antecedente es 'problemas' (plural)"
        );
    }

    // ==========================================================================
    // Tests de sujeto pospuesto (verb + adv/clitic + subject)
    // ==========================================================================

    #[test]
    fn test_postposed_subject_with_adverb() {
        // "criterios que fije rápidamente cada autonomía"
        // "fije" es singular porque el sujeto es "cada autonomía", no "criterios"
        let tokens = setup_tokens("los criterios que fije rápidamente cada autonomía");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let fije_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "fije")
            .collect();
        assert!(
            fije_corrections.is_empty(),
            "No debe corregir 'fije' - el sujeto pospuesto es 'cada autonomía' (singular)"
        );
    }

    #[test]
    fn test_postposed_subject_with_temporal_adverb() {
        // "normas que aprobó ayer la comisión"
        // "aprobó" es singular porque el sujeto es "la comisión", no "normas"
        let tokens = setup_tokens("las normas que aprobó ayer la comisión");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let aprobo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "aprobó")
            .collect();
        assert!(
            aprobo_corrections.is_empty(),
            "No debe corregir 'aprobó' - el sujeto pospuesto es 'la comisión' (singular)"
        );
    }

    #[test]
    fn test_postposed_subject_with_mente_adverb() {
        // "documentos que firma habitualmente el director"
        // "firma" es singular porque el sujeto es "el director", no "documentos"
        let tokens = setup_tokens("los documentos que firma habitualmente el director");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let firma_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "firma")
            .collect();
        assert!(
            firma_corrections.is_empty(),
            "No debe corregir 'firma' - el sujeto pospuesto es 'el director' (singular)"
        );
    }

    #[test]
    fn test_mente_modifier_before_que_keeps_plural_antecedent() {
        let cases = [
            (
                "los ratones modificados genéticamente que carecen de sensores",
                "carecen",
            ),
            (
                "los empleados despedidos injustamente que carecen de recursos",
                "carecen",
            ),
            (
                "los productos elaborados artesanalmente que compiten en precio",
                "compiten",
            ),
        ];

        for (text, verb) in cases {
            let tokens = setup_tokens(text);
            let corrections = RelativeAnalyzer::analyze(&tokens);
            let correction = corrections.iter().find(|c| c.original == verb);
            assert!(
                correction.is_none(),
                "No debe corregir '{verb}' cuando el antecedente plural va antes de adverbio -mente: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_mente_modifier_before_que_still_corrects_real_mismatch() {
        let tokens = setup_tokens("el ratón modificado genéticamente que carecen de sensores");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let correction = corrections.iter().find(|c| c.original == "carecen");
        assert!(
            correction.is_some(),
            "Debe corregir discordancia real con antecedente singular pese a adverbio -mente: {corrections:?}"
        );
        assert_eq!(correction.unwrap().suggestion, "carece");
    }

    #[test]
    fn test_find_noun_before_position_skips_participle_and_mente_chain() {
        let tokens = setup_tokens("los ratones modificados genéticamente que carecen de sensores");
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();
        let que_pos = word_tokens
            .iter()
            .position(|(_, t)| t.effective_text().eq_ignore_ascii_case("que"))
            .expect("Debe existir 'que'");
        assert!(que_pos > 0);
        let antecedent = RelativeAnalyzer::find_noun_before_position(&word_tokens, que_pos - 1);
        assert_eq!(
            antecedent.effective_text().to_lowercase(),
            "ratones",
            "Debe recuperar el antecedente nominal antes de participio + adverbio -mente"
        );
    }

    #[test]
    fn test_no_postposed_subject_with_preposition() {
        // "el problema que afectan a la sociedad" - "a la sociedad" es complemento, no sujeto
        // DEBE corregir "afectan" → "afecta" porque el antecedente es "problema" (singular)
        let tokens = setup_tokens("el problema grave que afectan a la sociedad");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let afectan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "afectan")
            .collect();
        assert_eq!(
            afectan_corrections.len(),
            1,
            "Debe corregir 'afectan' - 'a la sociedad' es complemento, no sujeto"
        );
    }

    #[test]
    fn test_postposed_subject_with_year() {
        // "leyes que aprobó en 2020 la comisión"
        // "aprobó" es singular porque el sujeto es "la comisión", no "leyes"
        // "en 2020" es frase temporal que se salta
        let tokens = setup_tokens("las leyes que aprobó en 2020 la comisión");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let aprobo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "aprobó")
            .collect();
        assert!(aprobo_corrections.is_empty(),
            "No debe corregir 'aprobó' - el sujeto pospuesto es 'la comisión', 'en 2020' es temporal");
    }

    #[test]
    fn test_postposed_subject_with_month() {
        // "leyes que aprobó en enero la comisión"
        // "en enero" es frase temporal que se salta
        let tokens = setup_tokens("las leyes que aprobó en enero la comisión");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let aprobo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "aprobó")
            .collect();
        assert!(aprobo_corrections.is_empty(),
            "No debe corregir 'aprobó' - el sujeto pospuesto es 'la comisión', 'en enero' es temporal");
    }

    #[test]
    fn test_postposed_subject_with_temporal_demonstrative() {
        // "leyes que aprobó en ese momento la comisión"
        // "en ese momento" es frase temporal que se salta completamente
        let tokens = setup_tokens("las leyes que aprobó en ese momento la comisión");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let aprobo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "aprobó")
            .collect();
        assert!(aprobo_corrections.is_empty(),
            "No debe corregir 'aprobó' - el sujeto pospuesto es 'la comisión', 'en ese momento' es temporal");
    }

    #[test]
    fn test_relative_without_postposed_subject_temporal() {
        // "las normas que regía en ese momento" - SÍ debe corregir porque no hay sujeto pospuesto
        // El sujeto es "las normas" (antecedente), debe concordar: regían
        let tokens = setup_tokens("las normas que regía en ese momento");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let regia_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "regía")
            .collect();
        assert!(
            !regia_corrections.is_empty(),
            "Debe corregir 'regía' a 'regían' - el antecedente 'las normas' es plural"
        );
    }

    #[test]
    fn test_no_que_no_correction() {
        // Sin "que" en la oración, RelativeAnalyzer no debe hacer ninguna corrección
        let tokens = setup_tokens("Las medidas, según explicó");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "Sin 'que' no debe hacer correcciones"
        );
    }

    #[test]
    fn test_parenthetical_segun_before_que() {
        // "Las medidas, según explicó el ministro, que aprobaron ayer"
        // No debe corregir "aprobaron" porque la cláusula "según explicó el ministro"
        // es parentética y "aprobaron" tiene su propio contexto
        let tokens = setup_tokens("Las medidas, según explicó el ministro, que aprobaron ayer");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let aprobaron_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "aprobaron")
            .collect();
        assert!(
            aprobaron_corrections.is_empty(),
            "No debe corregir 'aprobaron' - hay cláusula parentética antes de 'que'"
        );
    }

    #[test]
    fn test_relative_object_implicit_subject_proponen() {
        let tokens = setup_tokens("El avance que proponen en el IMB-CNM simplifica la deteccion.");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let proponen_correction = corrections.iter().find(|c| c.original == "proponen");
        assert!(
            proponen_correction.is_none(),
            "No debe corregir 'proponen' en relativo con sujeto implicito"
        );
    }

    // === Tests de verbos con cambio de raíz en cláusulas relativas ===

    #[test]
    fn test_stem_change_jugar_plural_to_singular() {
        // u→ue: "juegan" (plural) → "juega" (singular)
        let tokens = setup_tokens("la persona que juegan");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "juega");
    }

    #[test]
    fn test_stem_change_jugar_singular_to_plural() {
        // u→ue: "juega" (singular) → "juegan" (plural)
        let tokens = setup_tokens("los niños que juega");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "juegan");
    }

    #[test]
    fn test_stem_change_contar_present() {
        // o→ue: "cuentan" (plural) → "cuenta" (singular)
        let tokens = setup_tokens("la persona que cuentan");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cuenta");
    }

    #[test]
    fn test_stem_change_pensar_present() {
        // e→ie: "piensa" (singular) → "piensan" (plural)
        let tokens = setup_tokens("las personas que piensa");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "piensan");
    }

    #[test]
    fn test_stem_change_dormir_preterite() {
        // o→u en pretérito -ir: "durmieron" → "durmió"
        let tokens = setup_tokens("la persona que durmieron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "durmió");
    }

    #[test]
    fn test_stem_change_pedir_preterite() {
        // e→i en pretérito -ir: "pidieron" → "pidió"
        let tokens = setup_tokens("la persona que pidieron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "pidió");
    }

    #[test]
    fn test_object_relative_with_postposed_subject_not_corrected() {
        // "el pan que cuecen las panaderas"
        // "que" es OD y el sujeto real es "las panaderas" (pospuesto).
        // No debe forzar concordancia con "pan" (singular).
        let tokens = setup_tokens("el pan que cuecen las panaderas");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debe corregir cuando hay sujeto pospuesto explícito: {corrections:?}"
        );
    }

    #[test]
    fn test_impersonal_3p_relative_with_locative_phrase_not_corrected() {
        // "el pan que cuecen en esa panadería" suele ser 3p impersonal:
        // "en esa panadería cuecen el pan" (sujeto indefinido).
        // El antecedente es OD, así que no debe forzar "cuece".
        let tokens = setup_tokens("el pan que cuecen en esa panadería");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debe corregir 3p impersonal con complemento locativo: {corrections:?}"
        );
    }

    #[test]
    fn test_transitive_stem_changing_no_false_positive_servir() {
        // "servir" es transitivo con cambio de raíz — no debe corregir
        let tokens = setup_tokens("los platos que sirve");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let sirve_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "sirve")
            .collect();
        assert!(
            sirve_corrections.is_empty(),
            "No debe corregir 'sirve' — es transitivo, antecedente es objeto"
        );
    }

    #[test]
    fn test_transitive_stem_changing_no_false_positive_cerrar() {
        // "cerrar" es transitivo con cambio de raíz — no debe corregir
        let tokens = setup_tokens("las puertas que cierra");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let cierra_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "cierra")
            .collect();
        assert!(
            cierra_corrections.is_empty(),
            "No debe corregir 'cierra' — es transitivo, antecedente es objeto"
        );
    }

    #[test]
    fn test_fix_stem_changed_infinitive() {
        // Verifica que la corrección de infinitivos funciona
        assert_eq!(
            RelativeAnalyzer::fix_stem_changed_infinitive("juegar"),
            "jugar"
        );
        assert_eq!(
            RelativeAnalyzer::fix_stem_changed_infinitive("sirver"),
            "servir"
        );
        assert_eq!(
            RelativeAnalyzer::fix_stem_changed_infinitive("cierrar"),
            "cerrar"
        );
        assert_eq!(
            RelativeAnalyzer::fix_stem_changed_infinitive("durmir"),
            "dormir"
        );
        assert_eq!(
            RelativeAnalyzer::fix_stem_changed_infinitive("cuentar"),
            "contar"
        );
        // Verbo regular — no cambia
        assert_eq!(
            RelativeAnalyzer::fix_stem_changed_infinitive("cantar"),
            "cantar"
        );
        // Verbo que ya es correcto — no cambia
        assert_eq!(
            RelativeAnalyzer::fix_stem_changed_infinitive("pensar"),
            "pensar"
        );
    }
}
