//! Análisis de concordancia sujeto-verbo
//!
//! Detecta errores de concordancia entre pronombres personales y verbos conjugados.
//! Ejemplo: "yo cantas" → "yo canto", "tú canto" → "tú cantas"

use crate::dictionary::trie::Number;
use crate::dictionary::WordCategory;
use crate::grammar::tokenizer::TokenType;
use crate::grammar::has_sentence_boundary as has_sentence_boundary_slow;
use crate::grammar::{SentenceBoundaryIndex, Token};
use crate::languages::spanish::conjugation::enclitics::EncliticsAnalyzer;
use crate::languages::spanish::conjugation::prefixes::PrefixAnalyzer;
use crate::languages::spanish::conjugation::stem_changing::{
    get_stem_changing_verbs, StemChangeType,
};
use crate::languages::spanish::exceptions;
use crate::languages::VerbFormRecognizer;
use std::cell::RefCell;
use std::collections::HashSet;

/// Persona gramatical del sujeto (según la forma verbal que usa)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammaticalPerson {
    First,  // yo, nosotros
    Second, // tú, vosotros
    Third,  // él, ella, ellos, ellas, usted, ustedes (usted/ustedes usan forma de 3ª persona)
}

/// Número gramatical del sujeto
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammaticalNumber {
    Singular,
    Plural,
}
/// Tiempo verbal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbTense {
    Present,   // Presente de indicativo
    Preterite, // Pretérito indefinido / perfecto simple
}

/// Información del pronombre sujeto
#[derive(Debug, Clone)]
pub struct SubjectInfo {
    pub person: GrammaticalPerson,
    pub number: GrammaticalNumber,
}

/// Corrección de concordancia sujeto-verbo
#[derive(Debug, Clone)]
pub struct SubjectVerbCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub message: String,
}

/// Información de un sujeto nominal (sintagma nominal)
/// Ejemplo: "El Ministerio del Interior" → núcleo "Ministerio", singular
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct NominalSubject {
    /// Índice del token del núcleo del sintagma nominal
    nucleus_idx: usize,
    /// Número gramatical (singular/plural, considerando coordinación)
    number: GrammaticalNumber,
    /// Índice del último token del sintagma nominal (para buscar verbo después)
    end_idx: usize,
    /// True si el sintagma nominal incluye coordinación (y/e)
    is_coordinated: bool,
    /// True si la coordinación es correlativa "ni ... ni ...", donde la RAE
    /// admite concordancia en singular o plural.
    is_ni_correlative: bool,
}

/// Analizador de concordancia sujeto-verbo
pub struct SubjectVerbAnalyzer;

/// Resultado de saltar un complemento preposicional
struct PrepPhraseSkipResult {
    /// Posición del siguiente token candidato a verbo
    next_pos: usize,
    /// True si es complemento comitativo (con/sin) con contenido plural
    /// En ese caso, la concordancia sujeto-verbo es ambigua
    comitative_plural: bool,
}

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

impl SubjectVerbAnalyzer {
    const PREFIXABLE_IRREGULAR_BASES: [&'static str; 6] =
        ["hacer", "poner", "decir", "traer", "tener", "venir"];

    /// Analiza tokens buscando errores de concordancia sujeto-verbo
    pub fn analyze(tokens: &[Token]) -> Vec<SubjectVerbCorrection> {
        Self::analyze_with_recognizer(tokens, None)
    }

    /// Analiza tokens con VerbRecognizer opcional para desambiguar gerundios y verbos homógrafos
    pub fn analyze_with_recognizer(
        tokens: &[Token],
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<SubjectVerbCorrection> {
        let _boundary_cache_guard = BoundaryCacheGuard::new(tokens);
        let mut corrections = Vec::new();

        // Buscar patrones de pronombre + verbo
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        for i in 0..word_tokens.len() {
            let (idx1, token1) = word_tokens[i];

            // Usar effective_text() para ver correcciones de fases anteriores (ej: diacríticas)
            let text1 = token1.effective_text();

            // Verificar si el primer token es un pronombre personal sujeto
            if let Some(subject_info) = Self::get_subject_info(text1) {
                // "Ni ... ni ..." con pronombres forma sujeto coordinado;
                // no debemos forzar concordancia de un solo pronombre.
                if Self::is_pronoun_in_ni_correlative_subject(tokens, &word_tokens, i) {
                    continue;
                }
                // "Tanto ... como ..." con pronombres también forma sujeto coordinado;
                // no debemos forzar concordancia de un solo pronombre.
                if Self::is_pronoun_in_tanto_como_correlative_subject(tokens, &word_tokens, i) {
                    continue;
                }

                // Detectar casos donde el pronombre NO es sujeto
                if i >= 1 {
                    let (_, prev_token) = word_tokens[i - 1];
                    let prev_lower = prev_token.effective_text().to_lowercase();

                    // Caso 1: Pronombre precedido de preposición ? NO es sujeto
                    // "entre ellos estaba", "para ellos es", "sin ellos sería"
                    if Self::is_preposition(&prev_lower) {
                        continue;
                    }

                    // Caso 2: "X y pronombre" (pronombre precedido de conjunción)
                    // El verbo estará en forma plural, no debemos corregirlo
                    if prev_lower == "y" || prev_lower == "e" {
                        continue;
                    }
                }
                // Caso 2: "Pronombre y X" (pronombre seguido de conjunción)
                // El verbo estará en forma plural, no debemos corregirlo
                if i + 1 < word_tokens.len() {
                    let (_, next_token) = word_tokens[i + 1];
                    let next_lower = next_token.effective_text().to_lowercase();
                    if next_lower == "y" || next_lower == "e" {
                        // Es sujeto compuesto, skip
                        continue;
                    }
                }

                let allow_subjunctive =
                    Self::is_subjunctive_context_for_pronoun(tokens, &word_tokens, i);

                // Buscar el verbo después del pronombre, permitiendo adverbios/clíticos intercalados.
                // Ej: "ellos nunca olvido" -> "olvidan"
                let mut j = i + 1;
                while j < word_tokens.len() {
                    let (idx2, token2) = word_tokens[j];

                    if has_sentence_boundary(tokens, idx1, idx2) {
                        break;
                    }

                    let text2 = token2.effective_text();
                    let lower = text2.to_lowercase();

                    let is_adverb = if let Some(ref info) = token2.word_info {
                        info.category == WordCategory::Adverbio
                    } else {
                        Self::is_common_adverb(&lower)
                    };
                    if is_adverb || Self::is_clitic_pronoun(&lower) {
                        j += 1;
                        continue;
                    }

                    // Si el token candidato NO es verbo según el diccionario, no tratarlo como verbo
                    // salvo que el recognizer confirme que es una forma verbal válida.
                    //
                    // Esto evita falsos positivos con determinantes, preposiciones, pronombres, etc.
                    // Ejemplo: "él cuyo" - "cuyo" es determinante, no verbo
                    // Ejemplo: "él maravillas" - "maravillas" es sustantivo, no verbo
                    // Ejemplo: "él alto" - "alto" es adjetivo, no verbo
                    // Ejemplo: "él tampoco" - "tampoco" es adverbio, no verbo
                    if let Some(ref info) = token2.word_info {
                        if info.category != WordCategory::Verbo {
                            // Solo continuar si el recognizer confirma que es verbo.
                            let is_verb = verb_recognizer
                                .map(|vr| vr.is_valid_verb_form(&lower))
                                .unwrap_or(false);
                            if !is_verb {
                                break;
                            }
                        }
                    }

                    // Desambiguar homógrafos no verbales (ej. "nada" pronombre indefinido).
                    // Si la lectura no verbal es más fuerte en contexto, saltar y seguir buscando
                    // el verbo finito de la cláusula.
                    if Self::should_skip_ambiguous_nonverb_candidate(
                        tokens,
                        &word_tokens,
                        j,
                        &lower,
                        verb_recognizer,
                    ) {
                        j += 1;
                        continue;
                    }

                    if let Some(correction) = Self::check_verb_agreement(
                        idx2,
                        text2,
                        &subject_info,
                        verb_recognizer,
                        allow_subjunctive,
                    ) {
                        corrections.push(correction);
                    }
                    break;
                }
            }
        }

        // =========================================================================
        // Análisis de sujetos nominales (sintagmas nominales complejos)
        // Ejemplo: "El Ministerio del Interior intensifica" → núcleo "Ministerio"
        // =========================================================================
        // Verbos tipo "gustar" con clitico dativo y sujeto pospuesto:
        // "Me gusta los perros" -> "Me gustan los perros".
        // En esta construccion, el SN postverbal funciona como sujeto sintactico
        // y el verbo debe concordar en numero con ese SN.
        for vp in 0..word_tokens.len() {
            let (verb_idx, verb_token) = word_tokens[vp];
            let verb_text = verb_token.effective_text();
            let verb_lower = verb_text.to_lowercase();

            let Some((verb_person, verb_number, verb_tense, infinitive)) =
                Self::get_verb_info(&verb_lower, verb_recognizer)
            else {
                continue;
            };

            if verb_person != GrammaticalPerson::Third {
                continue;
            }
            if !Self::is_gustar_like_postposed_subject_infinitive(&infinitive) {
                continue;
            }
            if !Self::has_preverbal_dative_clitic(tokens, &word_tokens, vp) {
                continue;
            }

            let Some(postposed_number) =
                Self::detect_postposed_subject_number(tokens, &word_tokens, vp)
            else {
                continue;
            };
            if postposed_number == verb_number {
                continue;
            }

            if let Some(correct_form) = Self::get_correct_form(
                &infinitive,
                GrammaticalPerson::Third,
                postposed_number,
                verb_tense,
            ) {
                if correct_form.to_lowercase() != verb_lower
                    && !corrections.iter().any(|c| c.token_index == verb_idx)
                {
                    corrections.push(SubjectVerbCorrection {
                        token_index: verb_idx,
                        original: verb_text.to_string(),
                        suggestion: correct_form.clone(),
                        message: format!(
                            "Concordancia sujeto-verbo: '{}' deberÃ­a ser '{}'",
                            verb_text, correct_form
                        ),
                    });
                }
            }
        }

        // Pasiva refleja con "se" + verbo en singular + SN pospuesto plural:
        // "Se vende pisos" -> "Se venden pisos".
        for vp in 0..word_tokens.len() {
            let (verb_idx, verb_token) = word_tokens[vp];
            let verb_text = verb_token.effective_text();
            let verb_lower = verb_text.to_lowercase();

            let Some((verb_person, verb_number, verb_tense, infinitive)) =
                Self::get_verb_info(&verb_lower, verb_recognizer)
            else {
                continue;
            };

            if verb_person != GrammaticalPerson::Third || verb_number != GrammaticalNumber::Singular
            {
                continue;
            }
            if !Self::has_immediate_preverbal_se_clitic(tokens, &word_tokens, vp) {
                continue;
            }
            if Self::has_preverbal_explicit_subject_before_se(tokens, &word_tokens, vp) {
                continue;
            }
            if Self::is_reflexive_body_part_context(tokens, &word_tokens, vp, &infinitive) {
                continue;
            }

            let Some(postposed_number) =
                Self::detect_postposed_subject_number(tokens, &word_tokens, vp)
            else {
                continue;
            };
            if postposed_number != GrammaticalNumber::Plural {
                continue;
            }

            if let Some(correct_form) = Self::get_correct_form(
                &infinitive,
                GrammaticalPerson::Third,
                postposed_number,
                verb_tense,
            ) {
                if correct_form.to_lowercase() != verb_lower
                    && !corrections.iter().any(|c| c.token_index == verb_idx)
                {
                    corrections.push(SubjectVerbCorrection {
                        token_index: verb_idx,
                        original: verb_text.to_string(),
                        suggestion: correct_form.clone(),
                        message: format!(
                            "Concordancia sujeto-verbo: '{}' debería ser '{}'",
                            verb_text, correct_form
                        ),
                    });
                }
            }
        }

        let mut verbs_with_coordinated_subject: HashSet<usize> = HashSet::new();
        for i in 0..word_tokens.len() {
            // Verificar si esta posición está dentro de una cláusula parentética
            // "según explicó el ministro" o "como indicó el presidente"
            // En ese caso, "el ministro/presidente" es el sujeto del verbo de reporte,
            // no del verbo principal de la oración
            if Self::is_inside_parenthetical_clause(&word_tokens, tokens, i) {
                continue;
            }

            // Intentar detectar un sujeto nominal empezando en esta posición
            if let Some(nominal_subject) = Self::detect_nominal_subject(tokens, &word_tokens, i) {
                // Buscar el verbo después del sintagma nominal, saltando adverbios y complementos preposicionales
                let mut verb_pos = word_tokens
                    .iter()
                    .position(|(idx, _)| *idx > nominal_subject.end_idx);

                // Flag para detectar si hay un complemento comitativo plural (con/sin + plural)
                // En ese caso, la concordancia es ambigua y no debemos corregir verbo plural
                let mut has_comitative_plural = false;
                // Si hay una coma tras el SN y luego un adverbio antes del verbo,
                // probablemente empieza una nueva cláusula con sujeto implícito ("Un café, siempre canto").
                // En ese caso, NO debemos forzar 3ª persona a partir del SN anterior.
                let mut skipped_adverb_after_comma = false;

                // Saltar adverbios y complementos preposicionales entre el SN y el verbo
                // Ejemplo: "El Ministerio del Interior hoy intensifica" - saltar "hoy"
                // Ejemplo: "El Ministerio del Interior en 2020 intensifica" - saltar "en 2020"
                while let Some(vp) = verb_pos {
                    if vp >= word_tokens.len() {
                        break;
                    }
                    let (candidate_idx, candidate_token) = word_tokens[vp];
                    let lower = candidate_token.effective_text().to_lowercase();

                    // Si es adverbio conocido, saltar al siguiente token
                    let is_adverb = if let Some(ref info) = candidate_token.word_info {
                        info.category == WordCategory::Adverbio
                    } else {
                        // También verificar adverbios comunes sin word_info
                        Self::is_common_adverb(&lower)
                    };

                    if is_adverb {
                        if Self::has_comma_between(tokens, nominal_subject.end_idx, candidate_idx) {
                            skipped_adverb_after_comma = true;
                        }
                        verb_pos = if vp + 1 < word_tokens.len() {
                            Some(vp + 1)
                        } else {
                            None
                        };
                        continue;
                    }

                    // Números romanos (VI, XIV, etc.) tras un nombre/título no son verbos.
                    let candidate_text = candidate_token.effective_text();
                    if Self::is_roman_numeral(candidate_text) {
                        let prev_is_noun = if vp > 0 {
                            word_tokens[vp - 1]
                                .1
                                .word_info
                                .as_ref()
                                .map(|info| info.category == WordCategory::Sustantivo)
                                .unwrap_or(false)
                        } else {
                            false
                        };
                        let prev_is_capitalized = if vp > 0 {
                            word_tokens[vp - 1]
                                .1
                                .effective_text()
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false)
                        } else {
                            false
                        };

                        if prev_is_noun || prev_is_capitalized {
                            verb_pos = if vp + 1 < word_tokens.len() {
                                Some(vp + 1)
                            } else {
                                None
                            };
                            continue;
                        }
                    }

                    // Si es preposición, saltar el complemento preposicional completo
                    // Preposiciones comunes que inician complementos temporales/locativos
                    if Self::is_skippable_preposition(&lower) {
                        // Saltar la preposición y tokens siguientes hasta encontrar algo que parezca verbo
                        if let Some(skip_result) =
                            Self::skip_prepositional_phrase(&word_tokens, tokens, vp)
                        {
                            verb_pos = Some(skip_result.next_pos);
                            // Acumular flag de complemento comitativo plural
                            if skip_result.comitative_plural {
                                has_comitative_plural = true;
                            }
                        } else {
                            verb_pos = None;
                        }
                        continue;
                    }

                    // Saltar nombres propios en aposicion (p. ej., "El ministro Alberto Carrasquilla anuncio")
                    if let Some(vr) = verb_recognizer {
                        let candidate_text = candidate_token.effective_text();
                        let is_capitalized = candidate_text
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false);
                        let is_all_uppercase = candidate_text.chars().any(|c| c.is_alphabetic())
                            && candidate_text
                                .chars()
                                .all(|c| !c.is_alphabetic() || c.is_uppercase());

                        if is_capitalized && !is_all_uppercase {
                            let candidate_lower = candidate_text.to_lowercase();
                            let is_valid_verb = vr.is_valid_verb_form(&candidate_lower);
                            if !is_valid_verb {
                                let has_comma_before_name = Self::has_comma_between(
                                    tokens,
                                    nominal_subject.end_idx,
                                    word_tokens[vp].0,
                                );
                                let mut next_pos = vp + 1;
                                let mut last_idx = word_tokens[vp].0;
                                while next_pos < word_tokens.len() {
                                    let (next_idx, next_token) = word_tokens[next_pos];
                                    if has_sentence_boundary(tokens, last_idx, next_idx) {
                                        break;
                                    }
                                    let next_text = next_token.effective_text();
                                    let next_lower = next_text.to_lowercase();
                                    let next_is_capitalized = next_text
                                        .chars()
                                        .next()
                                        .map(|c| c.is_uppercase())
                                        .unwrap_or(false);
                                    let next_is_all_uppercase =
                                        next_text.chars().any(|c| c.is_alphabetic())
                                            && next_text
                                                .chars()
                                                .all(|c| !c.is_alphabetic() || c.is_uppercase());

                                    if next_is_capitalized && !next_is_all_uppercase {
                                        last_idx = next_idx;
                                        next_pos += 1;
                                        continue;
                                    }

                                    if Self::is_name_connector(&next_lower) {
                                        let mut lookahead = next_pos + 1;
                                        let mut steps = 0;
                                        let mut found_capitalized = false;
                                        while lookahead < word_tokens.len() && steps < 2 {
                                            let (look_idx, look_token) = word_tokens[lookahead];
                                            if has_sentence_boundary(tokens, last_idx, look_idx) {
                                                break;
                                            }
                                            let look_text = look_token.effective_text();
                                            let look_is_capitalized = look_text
                                                .chars()
                                                .next()
                                                .map(|c| c.is_uppercase())
                                                .unwrap_or(false);
                                            let look_is_all_uppercase =
                                                look_text.chars().any(|c| c.is_alphabetic())
                                                    && look_text.chars().all(|c| {
                                                        !c.is_alphabetic() || c.is_uppercase()
                                                    });
                                            if look_is_capitalized && !look_is_all_uppercase {
                                                found_capitalized = true;
                                                break;
                                            }
                                            let look_lower = look_text.to_lowercase();
                                            if !Self::is_name_connector(&look_lower) {
                                                break;
                                            }
                                            lookahead += 1;
                                            steps += 1;
                                        }

                                        if found_capitalized {
                                            last_idx = next_idx;
                                            next_pos += 1;
                                            continue;
                                        }
                                    }

                                    break;
                                }

                                if has_comma_before_name {
                                    let has_comma_after_name = if next_pos < word_tokens.len() {
                                        let (next_idx, _) = word_tokens[next_pos];
                                        Self::has_comma_between(tokens, last_idx, next_idx)
                                    } else {
                                        false
                                    };

                                    if !has_comma_after_name {
                                        // Nombre propio tras coma sin cierre: probable sujeto nuevo.
                                        // No vincular el SN previo con el verbo siguiente.
                                        verb_pos = None;
                                        break;
                                    }
                                }

                                verb_pos = if next_pos < word_tokens.len() {
                                    Some(next_pos)
                                } else {
                                    None
                                };
                                continue;
                            }
                        }
                    }

                    break;
                }

                if let Some(vp) = verb_pos {
                    let (verb_idx, verb_token) = word_tokens[vp];
                    let is_haber_aux_with_participle =
                        Self::is_haber_auxiliary_with_following_participle(
                            tokens,
                            &word_tokens,
                            vp,
                        );

                    // Verificar que no haya límite de oración entre el sujeto y el verbo
                    if has_sentence_boundary(tokens, nominal_subject.end_idx, verb_idx) {
                        continue;
                    }

                    // Si hay un paréntesis de cierre entre el sujeto y el verbo,
                    // el sujeto está dentro de un inciso y no debe concordar con el verbo externo.
                    // Ejemplo: "... el ajo, la cebolla y el puerro) tienen" - "puerro" no es sujeto de "tienen"
                    let has_closing_paren = tokens[nominal_subject.end_idx..verb_idx]
                        .iter()
                        .any(|t| t.text == ")");
                    if has_closing_paren {
                        continue;
                    }

                    let verb_text = verb_token.effective_text();

                    // Si el token NO es verbo según el diccionario, no tratarlo como verbo salvo:
                    // - Sustantivo/Adjetivo: solo permitimos en ALL-CAPS si el recognizer lo confirma (evita homógrafos).
                    // - Otras categorías (determinantes, preposiciones, etc.): solo si el recognizer lo confirma.
                    let is_all_uppercase = verb_token.text.chars().any(|c| c.is_alphabetic())
                        && verb_token
                            .text
                            .chars()
                            .all(|c| !c.is_alphabetic() || c.is_uppercase());
                    if let Some(ref info) = verb_token.word_info {
                        if info.category == WordCategory::Sustantivo
                            || info.category == WordCategory::Adjetivo
                        {
                            if !is_all_uppercase {
                                continue;
                            }
                            let is_valid_verb = verb_recognizer
                                .map(|vr| vr.is_valid_verb_form(verb_text))
                                .unwrap_or(false)
                                || is_haber_aux_with_participle;
                            if !is_valid_verb {
                                continue;
                            }
                        } else if info.category != WordCategory::Verbo {
                            let is_valid_verb = verb_recognizer
                                .map(|vr| vr.is_valid_verb_form(verb_text))
                                .unwrap_or(false)
                                || is_haber_aux_with_participle;
                            if !is_valid_verb {
                                continue;
                            }
                        }
                    }

                    // Skip all-uppercase words (acronyms) unless they are valid verb forms.
                    // This keeps corrections in ALL-CAPS text while avoiding acronyms like SATSE.
                    if is_all_uppercase {
                        let is_valid_verb = verb_recognizer
                            .map(|vr| vr.is_valid_verb_form(verb_text))
                            .unwrap_or(false)
                            || is_haber_aux_with_participle;
                        if !is_valid_verb {
                            continue;
                        }
                    }

                    if verbs_with_coordinated_subject.contains(&verb_idx) {
                        continue;
                    }

                    // Crear SubjectInfo con 3ª persona y el número detectado
                    let subject_info = SubjectInfo {
                        person: GrammaticalPerson::Third,
                        number: nominal_subject.number,
                    };
                    let allow_subjunctive_nominal =
                        Self::is_subjunctive_context_for_nominal_subject(tokens, &word_tokens, i);

                    // Si hay complemento comitativo plural y el sujeto es singular,
                    // no corregir en absoluto (ambas concordancias son aceptables)
                    // Ej: "El presidente con los ministros viajaron" - correcto
                    // Ej: "El presidente con los ministros viajó" - también correcto
                    if has_comitative_plural
                        && nominal_subject.number == GrammaticalNumber::Singular
                    {
                        continue;
                    }

                    let verb_lower = verb_text.to_lowercase();
                    if is_haber_aux_with_participle {
                        if let Some(correct_form) = Self::get_haber_auxiliary_third_person_for_number(
                            &verb_lower,
                            subject_info.number,
                        ) {
                            if Self::normalize_spanish(correct_form)
                                != Self::normalize_spanish(&verb_lower)
                            {
                                let correction = SubjectVerbCorrection {
                                    token_index: verb_idx,
                                    original: verb_text.to_string(),
                                    suggestion: correct_form.to_string(),
                                    message: format!(
                                        "Concordancia sujeto-verbo: '{}' debería ser '{}'",
                                        verb_text, correct_form
                                    ),
                                };

                                if !corrections.iter().any(|c| c.token_index == verb_idx) {
                                    corrections.push(correction);
                                }
                            } else if nominal_subject.is_coordinated {
                                verbs_with_coordinated_subject.insert(verb_idx);
                            }
                            continue;
                        }
                    }

                    if allow_subjunctive_nominal {
                        if let Some(vr) = verb_recognizer {
                            if let Some(correction) = Self::check_present_subjunctive_agreement(
                                verb_idx,
                                verb_text,
                                &verb_lower,
                                &subject_info,
                                vr,
                            ) {
                                if !corrections.iter().any(|c| c.token_index == verb_idx) {
                                    corrections.push(correction);
                                }
                                continue;
                            }
                            if Self::could_be_present_subjunctive(&verb_lower, &subject_info, vr) {
                                if nominal_subject.is_coordinated {
                                    verbs_with_coordinated_subject.insert(verb_idx);
                                }
                                continue;
                            }
                        }
                    }

                    if let Some((verb_person, verb_number, verb_tense, infinitive)) =
                        Self::get_verb_info(&verb_lower, verb_recognizer)
                    {
                        if Self::is_temporal_complement_with_postposed_subject(
                            tokens,
                            &word_tokens,
                            i,
                            vp,
                            verb_person,
                            verb_number,
                            &infinitive,
                        ) {
                            continue;
                        }

                        if skipped_adverb_after_comma
                            && matches!(
                                verb_person,
                                GrammaticalPerson::First | GrammaticalPerson::Second
                            )
                        {
                            continue;
                        }

                        // En coordinación correlativa "ni ... ni ..." con núcleo nominal,
                        // la concordancia en 3ª persona singular también es aceptable.
                        // Ej: "Ni el pan ni la leche está/están..."
                        if nominal_subject.is_ni_correlative
                            && verb_person == GrammaticalPerson::Third
                            && verb_number == GrammaticalNumber::Singular
                        {
                            verbs_with_coordinated_subject.insert(verb_idx);
                            continue;
                        }

                        // En copulativas con "ser", la concordancia con atributo plural
                        // se acepta cuando el sujeto es singular:
                        // "El problema fueron las lluvias", "La causa son los retrasos".
                        if Self::is_ser_copulative_with_postverbal_plural_attribute(
                            tokens,
                            &word_tokens,
                            &nominal_subject,
                            vp,
                            &verb_lower,
                            verb_person,
                            verb_number,
                            &infinitive,
                        ) {
                            continue;
                        }

                        if verb_person == subject_info.person && verb_number == subject_info.number
                        {
                            if nominal_subject.is_coordinated {
                                verbs_with_coordinated_subject.insert(verb_idx);
                            }
                        } else if let Some(correct_form) = Self::get_correct_form(
                            &infinitive,
                            subject_info.person,
                            subject_info.number,
                            verb_tense,
                        ) {
                            if correct_form.to_lowercase() != verb_lower {
                                let correction = SubjectVerbCorrection {
                                    token_index: verb_idx,
                                    original: verb_text.to_string(),
                                    suggestion: correct_form.clone(),
                                    message: format!(
                                        "Concordancia sujeto-verbo: '{}' debería ser '{}'",
                                        verb_text, correct_form
                                    ),
                                };

                                if !corrections.iter().any(|c| c.token_index == verb_idx) {
                                    corrections.push(correction);
                                }
                            }
                        }
                    }
                }
            }
        }

        corrections
    }

    /// Verifica si una palabra es preposición
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
                | "durante"
                | "mediante"
                | "según"
                | "sin"
                | "sobre"
                | "tras"
        )
    }

    /// Verifica si una palabra es un adverbio común (para saltar entre SN y verbo)
    fn is_common_adverb(word: &str) -> bool {
        matches!(
            word,
            // Adverbios temporales
            "hoy" | "ayer" | "mañana" | "ahora" | "antes" | "después" |
            "luego" | "pronto" | "tarde" | "temprano" | "siempre" | "nunca" |
            "todavía" | "aún" | "ya" | "entonces" | "anoche" | "anteayer" |
            // Adverbios de frecuencia
            "frecuentemente" | "raramente" | "habitualmente" | "normalmente" |
            "generalmente" | "usualmente" | "regularmente" | "ocasionalmente" |
            // Adverbios de modo comunes
            "también" | "tampoco" | "solo" | "solamente" | "incluso" |
            // Adverbios de lugar que pueden intercalarse
            "aquí" | "allí" | "ahí"
        )
    }

    /// Normaliza tildes y diacríticos frecuentes para comparaciones léxicas.
    fn is_likely_mente_adverb(word: &str) -> bool {
        let normalized = Self::normalize_spanish(word);
        normalized.len() > 6
            && normalized.ends_with("mente")
            && !matches!(normalized.as_str(), "demente" | "clemente")
    }

    fn is_haber_auxiliary_with_following_participle(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        aux_pos: usize,
    ) -> bool {
        if aux_pos >= word_tokens.len() {
            return false;
        }

        let aux_text = Self::normalize_spanish(word_tokens[aux_pos].1.effective_text());
        if !Self::is_haber_finite_form(aux_text.as_str()) {
            return false;
        }

        let (aux_idx, _) = word_tokens[aux_pos];
        let mut probe_pos = aux_pos + 1;
        let mut skipped = 0usize;
        const MAX_SKIPPED: usize = 2;

        while probe_pos < word_tokens.len() {
            let (candidate_idx, candidate_token) = word_tokens[probe_pos];
            if has_sentence_boundary(tokens, aux_idx, candidate_idx) {
                return false;
            }

            let candidate_lower = Self::normalize_spanish(candidate_token.effective_text());
            if Self::is_adverb_token(candidate_token) || Self::is_clitic_pronoun(&candidate_lower)
            {
                skipped += 1;
                if skipped > MAX_SKIPPED {
                    return false;
                }
                probe_pos += 1;
                continue;
            }

            return Self::looks_like_compound_participle(candidate_lower.as_str());
        }

        false
    }

    fn looks_like_compound_participle(word: &str) -> bool {
        matches!(
            word,
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
                | "abierto"
                | "abierta"
                | "abiertos"
                | "abiertas"
                | "escrito"
                | "escrita"
                | "escritos"
                | "escritas"
                | "roto"
                | "rota"
                | "rotos"
                | "rotas"
                | "vuelto"
                | "vuelta"
                | "vueltos"
                | "vueltas"
                | "muerto"
                | "muerta"
                | "muertos"
                | "muertas"
                | "cubierto"
                | "cubierta"
                | "cubiertos"
                | "cubiertas"
                | "resuelto"
                | "resuelta"
                | "resueltos"
                | "resueltas"
                | "devuelto"
                | "devuelta"
                | "devueltos"
                | "devueltas"
                | "frito"
                | "frita"
                | "fritos"
                | "fritas"
                | "impreso"
                | "impresa"
                | "impresos"
                | "impresas"
                | "satisfecho"
                | "satisfecha"
                | "satisfechos"
                | "satisfechas"
                | "deshecho"
                | "deshecha"
                | "deshechos"
                | "deshechas"
        ) || word.ends_with("ado")
            || word.ends_with("ada")
            || word.ends_with("ados")
            || word.ends_with("adas")
            || word.ends_with("ido")
            || word.ends_with("ida")
            || word.ends_with("idos")
            || word.ends_with("idas")
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

    fn is_haber_finite_form(word: &str) -> bool {
        matches!(
            word,
            "he"
                | "has"
                | "ha"
                | "hemos"
                | "habeis"
                | "han"
                | "habia"
                | "habias"
                | "habiamos"
                | "habiais"
                | "habian"
                | "hube"
                | "hubiste"
                | "hubo"
                | "hubimos"
                | "hubisteis"
                | "hubieron"
                | "habre"
                | "habras"
                | "habra"
                | "habremos"
                | "habreis"
                | "habran"
                | "habria"
                | "habrias"
                | "habriamos"
                | "habriais"
                | "habrian"
                | "haya"
                | "hayas"
                | "hayamos"
                | "hayais"
                | "hayan"
                | "hubiera"
                | "hubieras"
                | "hubieramos"
                | "hubierais"
                | "hubieran"
                | "hubiese"
                | "hubieses"
                | "hubiesemos"
                | "hubieseis"
                | "hubiesen"
        )
    }

    fn get_haber_auxiliary_third_person_for_number(
        word: &str,
        number: GrammaticalNumber,
    ) -> Option<&'static str> {
        let normalized = Self::normalize_spanish(word);
        let singular = match normalized.as_str() {
            "ha" | "han" => "ha",
            "habia" | "habian" => "había",
            "hubo" | "hubieron" => "hubo",
            "habra" | "habran" => "habrá",
            "habria" | "habrian" => "habría",
            "haya" | "hayan" => "haya",
            "hubiera" | "hubieran" => "hubiera",
            "hubiese" | "hubiesen" => "hubiese",
            _ => return None,
        };

        let plural = match singular {
            "ha" => "han",
            "había" => "habían",
            "hubo" => "hubieron",
            "habrá" => "habrán",
            "habría" => "habrían",
            "haya" => "hayan",
            "hubiera" => "hubieran",
            "hubiese" => "hubiesen",
            _ => return None,
        };

        Some(match number {
            GrammaticalNumber::Singular => singular,
            GrammaticalNumber::Plural => plural,
        })
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

    fn is_adverb_token(token: &Token) -> bool {
        let lower = token.effective_text().to_lowercase();
        token
            .word_info
            .as_ref()
            .map(|info| info.category == WordCategory::Adverbio)
            .unwrap_or(false)
            || Self::is_common_adverb(&lower)
            || Self::is_likely_mente_adverb(&lower)
    }

    fn is_clitic_pronoun(word: &str) -> bool {
        matches!(
            word,
            "me" | "te" | "se" | "lo" | "la" | "le" | "nos" | "os" | "los" | "las" | "les"
        )
    }

    fn is_dative_clitic(word: &str) -> bool {
        matches!(word, "me" | "te" | "le" | "nos" | "os" | "les" | "se")
    }

    fn has_preverbal_dative_clitic(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
    ) -> bool {
        if verb_pos == 0 {
            return false;
        }

        let (verb_idx, _) = word_tokens[verb_pos];
        let mut scanned = 0usize;
        const MAX_LOOKBACK: usize = 6;

        for j in (0..verb_pos).rev() {
            if scanned >= MAX_LOOKBACK {
                break;
            }
            let (prev_idx, prev_token) = word_tokens[j];
            if has_sentence_boundary(tokens, prev_idx, verb_idx) {
                break;
            }

            let lower = Self::normalize_spanish(prev_token.effective_text());
            if Self::is_dative_clitic(lower.as_str()) {
                return true;
            }

            scanned += 1;
        }

        false
    }

    fn has_immediate_preverbal_se_clitic(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
    ) -> bool {
        if verb_pos == 0 {
            return false;
        }
        let (verb_idx, _) = word_tokens[verb_pos];
        let (prev_idx, prev_token) = word_tokens[verb_pos - 1];
        if has_sentence_boundary(tokens, prev_idx, verb_idx) {
            return false;
        }
        Self::normalize_spanish(prev_token.effective_text()) == "se"
    }

    /// Detecta sujeto explícito pre-verbal antes de "se" en la misma cláusula.
    /// Si existe, estamos ante una estructura reflexiva/pronominal y no pasiva refleja.
    fn has_preverbal_explicit_subject_before_se(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
    ) -> bool {
        if verb_pos < 2 {
            return false;
        }

        let (se_idx, se_token) = word_tokens[verb_pos - 1];
        if Self::normalize_spanish(se_token.effective_text()) != "se" {
            return false;
        }

        let mut probe_pos = verb_pos - 1;
        while probe_pos > 0 {
            let candidate_pos = probe_pos - 1;
            let (candidate_idx, candidate_token) = word_tokens[candidate_pos];
            if Self::is_clause_break_between(tokens, candidate_idx, se_idx) {
                break;
            }

            if Self::is_adverb_token(candidate_token) {
                probe_pos -= 1;
                continue;
            }

            let candidate_lower = Self::normalize_spanish(candidate_token.effective_text());
            if Self::is_subject_pronoun_form(candidate_lower.as_str()) {
                return true;
            }
            if Self::is_proper_name_like_token(candidate_token) {
                return true;
            }

            let is_nominal_candidate = candidate_token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Sustantivo)
                .unwrap_or_else(|| {
                    !Self::looks_like_verb(&candidate_lower) && !Self::is_common_adverb(&candidate_lower)
                });

            if is_nominal_candidate
                && Self::is_nominal_subject_candidate_before_se(tokens, word_tokens, candidate_pos)
            {
                return true;
            }

            break;
        }

        false
    }

    fn is_subject_pronoun_form(word: &str) -> bool {
        matches!(
            word,
            "yo"
                | "tu"
                | "el"
                | "ella"
                | "usted"
                | "nosotros"
                | "nosotras"
                | "vosotros"
                | "vosotras"
                | "ellos"
                | "ellas"
                | "ustedes"
        )
    }

    fn is_nominal_subject_candidate_before_se(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        noun_pos: usize,
    ) -> bool {
        if noun_pos == 0 {
            return true;
        }

        let (noun_idx, _) = word_tokens[noun_pos];
        let (prev_idx, prev_token) = word_tokens[noun_pos - 1];
        if Self::has_nonword_between(tokens, prev_idx, noun_idx) {
            return true;
        }

        let prev_lower = Self::normalize_spanish(prev_token.effective_text());
        if Self::is_preposition(&prev_lower) {
            return false;
        }

        let prev_is_det = Self::is_determiner(prev_token.effective_text())
            || Self::is_possessive_determiner(prev_token.effective_text());
        if prev_is_det && noun_pos >= 2 {
            let (prev_prev_idx, prev_prev_token) = word_tokens[noun_pos - 2];
            if !Self::has_nonword_between(tokens, prev_prev_idx, prev_idx) {
                let prev_prev_lower = Self::normalize_spanish(prev_prev_token.effective_text());
                if Self::is_preposition(&prev_prev_lower) {
                    return false;
                }
            }
        }

        true
    }

    /// Reflexivos de cuidado corporal: "se lava las manos", "se corta las uñas".
    /// En estos casos "las manos/uñas" es CD, no sujeto de pasiva refleja.
    fn is_reflexive_body_part_context(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
        infinitive: &str,
    ) -> bool {
        if !Self::is_reflexive_body_part_verb(infinitive) {
            return false;
        }
        if verb_pos + 2 >= word_tokens.len() {
            return false;
        }

        let (verb_idx, _) = word_tokens[verb_pos];
        let mut probe_pos = verb_pos + 1;
        let mut skipped_adverbs = 0usize;
        const MAX_SKIPPED_ADVERBS: usize = 2;

        while probe_pos < word_tokens.len() {
            let (det_idx, det_token) = word_tokens[probe_pos];
            if has_sentence_boundary(tokens, verb_idx, det_idx) {
                return false;
            }
            if Self::is_adverb_token(det_token) {
                skipped_adverbs += 1;
                if skipped_adverbs > MAX_SKIPPED_ADVERBS {
                    return false;
                }
                probe_pos += 1;
                continue;
            }

            let det_lower = Self::normalize_spanish(det_token.effective_text());
            if !matches!(det_lower.as_str(), "el" | "la" | "los" | "las") {
                return false;
            }
            if probe_pos + 1 >= word_tokens.len() {
                return false;
            }

            let (noun_idx, noun_token) = word_tokens[probe_pos + 1];
            if has_sentence_boundary(tokens, det_idx, noun_idx) {
                return false;
            }
            let noun_lower = Self::normalize_spanish(noun_token.effective_text());
            return Self::is_body_part_noun(noun_lower.as_str());
        }

        false
    }

    fn is_reflexive_body_part_verb(infinitive: &str) -> bool {
        matches!(
            Self::normalize_spanish(infinitive).as_str(),
            "lavar"
                | "cepillar"
                | "peinar"
                | "cortar"
                | "pintar"
                | "secar"
                | "arreglar"
                | "limpiar"
        )
    }

    fn is_body_part_noun(noun: &str) -> bool {
        matches!(
            noun,
            "mano"
                | "manos"
                | "diente"
                | "dientes"
                | "una"
                | "unas"
                | "cabello"
                | "cabellos"
                | "pelo"
                | "pelos"
                | "cara"
                | "caras"
                | "ojo"
                | "ojos"
                | "oreja"
                | "orejas"
                | "labio"
                | "labios"
                | "pierna"
                | "piernas"
                | "brazo"
                | "brazos"
                | "pie"
                | "pies"
        )
    }

    fn is_gustar_like_postposed_subject_infinitive(infinitive: &str) -> bool {
        matches!(
            Self::normalize_spanish(infinitive).as_str(),
            "gustar"
                | "molestar"
                | "preocupar"
                | "interesar"
                | "doler"
                | "faltar"
                | "sobrar"
                | "encantar"
                | "fascinar"
                | "apetecer"
                | "agradar"
                | "disgustar"
                | "importar"
                | "convenir"
                | "corresponder"
                | "pertenecer"
                | "bastar"
        )
    }

    fn should_skip_ambiguous_nonverb_candidate(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        candidate_pos: usize,
        candidate_lower: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let candidate_token = word_tokens[candidate_pos].1;
        let candidate_is_nonverb = candidate_token
            .word_info
            .as_ref()
            .map(|info| info.category != WordCategory::Verbo)
            .unwrap_or(false);

        if candidate_lower == "nada"
            && Self::is_indefinite_nada_context(tokens, word_tokens, candidate_pos, verb_recognizer)
        {
            return true;
        }

        if !candidate_is_nonverb {
            return false;
        }

        Self::has_following_finite_verb_in_clause(
            tokens,
            word_tokens,
            candidate_pos,
            verb_recognizer,
        )
    }

    fn has_following_finite_verb_in_clause(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        candidate_pos: usize,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let (candidate_idx, _) = word_tokens[candidate_pos];
        for k in (candidate_pos + 1)..word_tokens.len() {
            let (next_idx, next_token) = word_tokens[k];
            if has_sentence_boundary(tokens, candidate_idx, next_idx) {
                break;
            }

            let next_lower = next_token.effective_text().to_lowercase();
            if Self::is_common_adverb(&next_lower) || Self::is_clitic_pronoun(&next_lower) {
                continue;
            }

            if Self::get_verb_info(&next_lower, verb_recognizer).is_some() {
                return true;
            }
        }
        false
    }

    fn is_indefinite_nada_context(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        candidate_pos: usize,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let (candidate_idx, _) = word_tokens[candidate_pos];

        // "... se nada" suele ser "sé nada" o pronombre indefinido "nada",
        // no el verbo "nadar".
        if candidate_pos > 0 {
            let (prev_idx, prev_token) = word_tokens[candidate_pos - 1];
            if !has_sentence_boundary(tokens, prev_idx, candidate_idx) {
                let prev_norm = Self::normalize_spanish(prev_token.effective_text());
                if prev_norm == "se" {
                    return true;
                }
            }
        }

        // "Yo nada sé": si hay otro verbo finito en la misma cláusula,
        // tratar "nada" como pronombre y no como verbo.
        Self::has_following_finite_verb_in_clause(
            tokens,
            word_tokens,
            candidate_pos,
            verb_recognizer,
        )
    }

    /// Verifica si una preposición puede iniciar un complemento que debemos saltar
    fn is_skippable_preposition(word: &str) -> bool {
        matches!(
            word,
            // Preposiciones que inician complementos temporales/locativos/circunstanciales
            "en" | "desde" | "hasta" | "durante" | "tras" | "mediante" |
            "por" | "para" | "sobre" | "bajo" | "ante" | "según" | "como" |
            // Preposiciones comitativas (con/sin) - manejadas especialmente por ambigüedad
            "con" | "sin"
        )
    }

    /// Verifica si una palabra puede actuar como conector en nombres propios compuestos
    fn is_name_connector(word: &str) -> bool {
        matches!(word, "de" | "del" | "la" | "las" | "los" | "y" | "e")
    }

    /// Verifica si una preposición es comitativa (con/sin)
    /// Estas preposiciones pueden afectar la concordancia percibida cuando el complemento es plural
    fn is_comitative_preposition(word: &str) -> bool {
        matches!(word, "con" | "sin")
    }

    /// Salta un complemento preposicional y devuelve la posición del siguiente token candidato a verbo
    /// Ej: "en 2020" -> salta "en" y "2020"
    /// Ej: "en el año 2020" -> salta "en", "el", "año", "2020"
    /// También detecta si es un complemento comitativo plural (para marcar ambigüedad)
    ///
    /// Para cláusulas parentéticas (según/como + verbo de reporte), salta todo el inciso
    /// hasta la coma de cierre y continúa buscando el verbo principal.
    fn skip_prepositional_phrase(
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
        prep_pos: usize,
    ) -> Option<PrepPhraseSkipResult> {
        let prep_token = word_tokens.get(prep_pos)?;
        let prep_word = prep_token.1.effective_text().to_lowercase();
        let is_comitative = Self::is_comitative_preposition(&prep_word);

        // =======================================================================
        // Detección anticipada de cláusula parentética (según/como + verbo)
        // Buscar verbo de reporte en ventana amplia, saltando adverbios
        // Ejemplo: "según ayer explicó el ministro" - "ayer" no debe bloquear
        // =======================================================================
        if Self::is_parenthetical_preposition(&prep_word) {
            let max_lookahead = 4; // Buscar verbo en hasta 4 tokens después de la prep
            for offset in 1..=max_lookahead {
                let check_pos = prep_pos + offset;
                if check_pos >= word_tokens.len() {
                    break;
                }
                let (token_idx, token) = word_tokens[check_pos];
                let lower = token.effective_text().to_lowercase();

                if Self::is_reporting_verb(&lower) {
                    // Encontramos verbo de reporte. Saltar todo el inciso hasta la coma.
                    for idx in token_idx..all_tokens.len() {
                        if all_tokens[idx].token_type == TokenType::Punctuation
                            && all_tokens[idx].text == ","
                        {
                            // Encontramos la coma de cierre. Buscar el siguiente word_token
                            for next_pos in (check_pos + 1)..word_tokens.len() {
                                let (next_idx, _) = word_tokens[next_pos];
                                if next_idx > idx {
                                    return Some(PrepPhraseSkipResult {
                                        next_pos,
                                        comitative_plural: false,
                                    });
                                }
                            }
                            // No hay más word_tokens después de la coma
                            return None;
                        }
                    }
                    // No encontramos coma de cierre, no hay verbo principal
                    return None;
                }
            }
        }

        // =======================================================================
        // Lógica normal para complementos preposicionales
        // =======================================================================
        let mut pos = prep_pos + 1; // Saltar la preposición

        // Saltar hasta 5 tokens del complemento preposicional
        // (ej: "en el mes de enero" = prep + art + sust + prep + sust)
        // (ej: "según los expertos del sector" = prep + art + sust + prep + art + sust)
        let max_skip = 5;
        let mut skipped = 0;
        let mut found_plural = false;

        while pos < word_tokens.len() && skipped < max_skip {
            let (_, token) = word_tokens[pos];
            let text = token.effective_text();
            let lower = text.to_lowercase();

            // Si encontramos algo que parece verbo (termina en formas verbales comunes), parar
            if Self::looks_like_verb(&lower) {
                return Some(PrepPhraseSkipResult {
                    next_pos: pos,
                    comitative_plural: is_comitative && found_plural,
                });
            }

            // Detectar plurales en el complemento (para con/sin)
            if is_comitative && !found_plural {
                // Determinantes plurales
                if matches!(
                    lower.as_str(),
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
                ) {
                    found_plural = true;
                }
                // Sustantivos con word_info plural
                if let Some(ref info) = token.word_info {
                    if info.number == Number::Plural {
                        found_plural = true;
                    }
                }
                // Coordinación con "y" o "e" (ej: "con Juan y María")
                if matches!(lower.as_str(), "y" | "e") {
                    found_plural = true;
                }
            }
            // Si es número, artículo, sustantivo conocido, o palabra corta, seguir saltando
            let is_part_of_complement = text.chars().all(|c| c.is_ascii_digit())
                || Self::is_determiner(&lower)
                || matches!(
                    lower.as_str(),
                    // Meses, años, palabras comunes en complementos temporales
                    "enero" | "febrero" | "marzo" | "abril" | "mayo" | "junio" |
                    "julio" | "agosto" | "septiembre" | "octubre" | "noviembre" | "diciembre" |
                    "año" | "mes" | "día" | "semana" | "hora" | "momento" |
                    "de" | "del" |  // Preposiciones internas del complemento
                    "y" | "e" // Coordinación
                )
                || token
                    .word_info
                    .as_ref()
                    .map(|i| i.category == WordCategory::Sustantivo)
                    .unwrap_or(false);

            if is_part_of_complement {
                pos += 1;
                skipped += 1;
            } else {
                // Encontramos algo que no es parte del complemento
                break;
            }
        }

        if pos < word_tokens.len() {
            Some(PrepPhraseSkipResult {
                next_pos: pos,
                comitative_plural: is_comitative && found_plural,
            })
        } else {
            None
        }
    }

    /// Heurística simple para detectar si una palabra parece forma verbal
    fn looks_like_verb(word: &str) -> bool {
        // Terminaciones verbales comunes (presente, pretérito, etc.)
        word.ends_with("an")
            || word.ends_with("en")
            || word.ends_with("on")
            || word.ends_with("ó")
            || word.ends_with("aron")
            || word.ends_with("ieron")
            || word.ends_with("aban")
            || word.ends_with("ían")
            || word.ends_with("ará")
            || word.ends_with("erá")
            || word.ends_with("irá")
            || word.ends_with("arán")
            || word.ends_with("erán")
            || word.ends_with("irán")
    }

    /// Detecta si un pronombre está dentro de un sujeto correlativo "ni ... ni ..."
    /// previo al verbo principal ("Ni tú ni yo podemos", "Ni ella ni él quieren").
    ///
    /// Regla conservadora:
    /// - Requiere que el pronombre esté adyacente a "ni".
    /// - Requiere al menos dos "ni" antes del primer verbo de la secuencia.
    /// - Si ya apareció un verbo antes del pronombre, no lo tratamos como sujeto correlativo.
    fn is_pronoun_in_ni_correlative_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pronoun_pos: usize,
    ) -> bool {
        if word_tokens.is_empty() || pronoun_pos >= word_tokens.len() {
            return false;
        }

        let prev_is_ni = if pronoun_pos > 0 {
            Self::normalize_spanish(word_tokens[pronoun_pos - 1].1.effective_text()) == "ni"
        } else {
            false
        };
        let next_is_ni = if pronoun_pos + 1 < word_tokens.len() {
            Self::normalize_spanish(word_tokens[pronoun_pos + 1].1.effective_text()) == "ni"
        } else {
            false
        };
        if !prev_is_ni && !next_is_ni {
            return false;
        }

        // Delimitar inicio de cláusula por frontera de oración.
        let mut clause_start = pronoun_pos;
        while clause_start > 0 {
            let (prev_idx, _) = word_tokens[clause_start - 1];
            let (curr_idx, _) = word_tokens[clause_start];
            if Self::is_clause_break_between(tokens, prev_idx, curr_idx) {
                break;
            }
            clause_start -= 1;
        }

        // Si ya apareció un verbo antes del pronombre, probablemente no es
        // el sujeto correlativo del verbo que sigue.
        for pos in clause_start..pronoun_pos {
            let (_, token) = word_tokens[pos];
            let lower = token.effective_text().to_lowercase();
            let is_verb = token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Verbo)
                .unwrap_or(false)
                || Self::looks_like_verb(&lower);
            if is_verb {
                return false;
            }
        }

        // Buscar el primer verbo después del pronombre.
        let mut first_verb_pos = None;
        for pos in (pronoun_pos + 1)..word_tokens.len() {
            let (curr_idx, token) = word_tokens[pos];
            let (pronoun_idx, _) = word_tokens[pronoun_pos];
            if Self::is_clause_break_between(tokens, pronoun_idx, curr_idx) {
                break;
            }

            let lower = token.effective_text().to_lowercase();
            let is_verb = token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Verbo)
                .unwrap_or(false)
                || Self::looks_like_verb(&lower);
            if is_verb {
                first_verb_pos = Some(pos);
                break;
            }
        }

        let Some(verb_pos) = first_verb_pos else {
            return false;
        };

        let ni_count = (clause_start..=verb_pos)
            .filter(|&pos| Self::normalize_spanish(word_tokens[pos].1.effective_text()) == "ni")
            .count();

        ni_count >= 2
    }

    /// Detecta si un pronombre está dentro de un sujeto correlativo
    /// "tanto ... como ..." previo al verbo principal.
    ///
    /// Ejemplos:
    /// - "Tanto él como ella son..."
    /// - "Tanto yo como tú sabemos..."
    ///
    /// Regla conservadora:
    /// - Requiere adyacencia local al patrón ("tanto*" o "como").
    /// - Requiere al menos un marcador "tanto*" y un "como" antes del primer verbo.
    /// - Si ya apareció un verbo antes del pronombre, no lo tratamos como sujeto correlativo.
    fn is_pronoun_in_tanto_como_correlative_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pronoun_pos: usize,
    ) -> bool {
        if word_tokens.is_empty() || pronoun_pos >= word_tokens.len() {
            return false;
        }

        let prev_is_marker = if pronoun_pos > 0 {
            let prev_norm = Self::normalize_spanish(word_tokens[pronoun_pos - 1].1.effective_text());
            Self::is_tanto_correlative_marker(prev_norm.as_str()) || prev_norm == "como"
        } else {
            false
        };
        let next_is_como = if pronoun_pos + 1 < word_tokens.len() {
            Self::normalize_spanish(word_tokens[pronoun_pos + 1].1.effective_text()) == "como"
        } else {
            false
        };
        if !prev_is_marker && !next_is_como {
            return false;
        }

        // Delimitar inicio de cláusula por frontera de oración.
        let mut clause_start = pronoun_pos;
        while clause_start > 0 {
            let (prev_idx, _) = word_tokens[clause_start - 1];
            let (curr_idx, _) = word_tokens[clause_start];
            if Self::is_clause_break_between(tokens, prev_idx, curr_idx) {
                break;
            }
            clause_start -= 1;
        }

        // Si el correlativo aparece tras una coordinación interna
        // ("... y tanto ella como él ..."), limitar la búsqueda al bloque local.
        let mut local_start = clause_start;
        for pos in (clause_start..=pronoun_pos).rev() {
            let norm = Self::normalize_spanish(word_tokens[pos].1.effective_text());
            if Self::is_tanto_correlative_marker(norm.as_str()) {
                local_start = pos;
                break;
            }
        }

        // Si ya apareció un verbo antes del pronombre dentro del bloque local,
        // probablemente no es el sujeto correlativo del verbo que sigue.
        for pos in local_start..pronoun_pos {
            let (_, token) = word_tokens[pos];
            let lower = token.effective_text().to_lowercase();
            let is_verb = token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Verbo)
                .unwrap_or(false)
                || Self::looks_like_verb(&lower);
            if is_verb {
                return false;
            }
        }

        // Buscar el primer verbo después del pronombre.
        let mut first_verb_pos = None;
        for pos in (pronoun_pos + 1)..word_tokens.len() {
            let (curr_idx, token) = word_tokens[pos];
            let (pronoun_idx, _) = word_tokens[pronoun_pos];
            if Self::is_clause_break_between(tokens, pronoun_idx, curr_idx) {
                break;
            }

            let lower = token.effective_text().to_lowercase();
            let is_verb = token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Verbo)
                .unwrap_or(false)
                || Self::looks_like_verb(&lower);
            if is_verb {
                first_verb_pos = Some(pos);
                break;
            }
        }

        let Some(verb_pos) = first_verb_pos else {
            return false;
        };

        let mut has_tanto = false;
        let mut has_como = false;
        for pos in local_start..=verb_pos {
            let norm = Self::normalize_spanish(word_tokens[pos].1.effective_text());
            if Self::is_tanto_correlative_marker(norm.as_str()) {
                has_tanto = true;
            } else if norm == "como" {
                has_como = true;
            }
        }

        has_tanto && has_como
    }

    fn is_tanto_correlative_marker(word: &str) -> bool {
        matches!(word, "tanto" | "tanta" | "tantos" | "tantas")
    }

    /// Verifica si la preposición introduce cláusulas parentéticas de cita
    /// Ejemplo: "según explicó", "como indicó"
    fn is_parenthetical_preposition(word: &str) -> bool {
        matches!(word, "según" | "como")
    }

    /// Verifica si una palabra es un verbo de comunicación/percepción/opinión
    /// típico de cláusulas parentéticas de cita
    fn is_reporting_verb(word: &str) -> bool {
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
            "advirtió" | "advierte" | "advertía" | "advirtieron" |
            // Formas de "expresar"
            "expresó" | "expresa" | "expresaba" | "expresaron" |
            // Formas de "manifestar"
            "manifestó" | "manifiesta" | "manifestaba" | "manifestaron" |
            // Formas de "destacar"
            "destacó" | "destaca" | "destacaba" | "destacaron" |
            // Formas de "subrayar"
            "subrayó" | "subraya" | "subrayaba" | "subrayaron" |
            // Formas de "reconocer"
            "reconoció" | "reconoce" | "reconocía" | "reconocieron"
        )
    }

    /// Verifica si la posición actual está dentro de una cláusula parentética de cita
    /// Ejemplo: en "Las cifras, como indicó *el presidente*, muestran..."
    /// las posiciones de "el" y "presidente" están dentro de la cláusula "como indicó..."
    fn is_inside_parenthetical_clause(
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
        pos: usize,
    ) -> bool {
        // Buscar hacia atrás "según/como" + verbo de reporte
        // La cláusula parentética empieza con "según/como + reporting_verb"
        // y termina en la siguiente coma o límite de oración
        let max_lookback = 10.min(pos); // Ventana amplia para incisos largos
        let (current_idx, _) = word_tokens[pos];

        for offset in 1..=max_lookback {
            let check_pos = pos - offset;
            let (check_idx, token) = word_tokens[check_pos];
            let lower = token.effective_text().to_lowercase();

            // Verificar si hay coma entre check_pos y pos en all_tokens
            // Si la hay, hemos cruzado el límite del inciso
            let mut found_comma = false;
            for idx in check_idx..current_idx {
                if all_tokens[idx].token_type == TokenType::Punctuation
                    && all_tokens[idx].text == ","
                {
                    found_comma = true;
                    break;
                }
            }
            if found_comma {
                break;
            }

            // ¿Es un verbo de reporte?
            if Self::is_reporting_verb(&lower) {
                // Buscar "según/como" antes de este verbo
                if check_pos > 0 {
                    let (_, prep_token) = word_tokens[check_pos - 1];
                    let prep_lower = prep_token.effective_text().to_lowercase();
                    if Self::is_parenthetical_preposition(&prep_lower) {
                        // Encontramos "según/como + reporting_verb" antes de nuestra posición
                        // Estamos dentro de la cláusula parentética
                        return true;
                    }
                }
            }
        }

        Self::is_inside_como_apposition(word_tokens, all_tokens, pos)
    }

    /// Verifica si una palabra es determinante (artículo o demostrativo)
    fn is_determiner(word: &str) -> bool {
        let lower = word.to_lowercase();
        matches!(
            lower.as_str(),
            // Artículos definidos
            "el" | "la" | "los" | "las" |
            // Artículos indefinidos
            "un" | "una" | "unos" | "unas" |
            // Demostrativos
            "este" | "esta" | "estos" | "estas" |
            "ese" | "esa" | "esos" | "esas" |
            "aquel" | "aquella" | "aquellos" | "aquellas"
        )
    }

    /// Determinantes posesivos que pueden iniciar un sujeto nominal pospuesto.
    fn is_possessive_determiner(word: &str) -> bool {
        matches!(
            Self::normalize_spanish(word).as_str(),
            "mi" | "mis"
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

    /// Sustantivos temporalmente frecuentes en complementos circunstanciales.
    fn is_temporal_noun(word: &str) -> bool {
        matches!(
            Self::normalize_spanish(word).as_str(),
            "lunes"
                | "martes"
                | "miercoles"
                | "jueves"
                | "viernes"
                | "sabado"
                | "sabados"
                | "domingo"
                | "domingos"
                | "enero"
                | "febrero"
                | "marzo"
                | "abril"
                | "mayo"
                | "junio"
                | "julio"
                | "agosto"
                | "septiembre"
                | "octubre"
                | "noviembre"
                | "diciembre"
                | "verano"
                | "invierno"
                | "inviernos"
                | "primavera"
                | "primaveras"
                | "otono"
                | "otonos"
                | "ano"
                | "anos"
                | "mes"
                | "meses"
                | "semana"
                | "semanas"
                | "dia"
                | "dias"
                | "manana"
                | "mananas"
                | "tarde"
                | "tardes"
                | "noche"
                | "noches"
                | "madrugada"
                | "madrugadas"
        )
    }

    fn is_temporal_quantifier(word: &str) -> bool {
        matches!(
            Self::normalize_spanish(word).as_str(),
            "todo" | "todos" | "toda" | "todas"
        )
    }

    /// Verbos meteorológicos impersonales (normalmente 3ª singular).
    fn is_impersonal_weather_infinitive(infinitive: &str) -> bool {
        matches!(
            Self::normalize_spanish(infinitive).as_str(),
            "llover" | "nevar" | "granizar" | "lloviznar" | "tronar" | "relampaguear"
        )
    }

    /// Obtiene el número gramatical de un determinante
    fn get_determiner_number(word: &str) -> GrammaticalNumber {
        let lower = word.to_lowercase();
        if matches!(
            lower.as_str(),
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
        ) {
            GrammaticalNumber::Plural
        } else {
            GrammaticalNumber::Singular
        }
    }

    /// Cuantificadores/numerales que pueden iniciar sujeto pospuesto sin determinante.
    fn get_quantifier_number(word: &str) -> Option<GrammaticalNumber> {
        let normalized = Self::normalize_spanish(word);
        if normalized.chars().all(|c| c.is_ascii_digit()) {
            return Some(if normalized == "1" {
                GrammaticalNumber::Singular
            } else {
                GrammaticalNumber::Plural
            });
        }

        if matches!(normalized.as_str(), "un" | "una" | "uno") {
            return Some(GrammaticalNumber::Singular);
        }

        if matches!(
            normalized.as_str(),
            "dos"
                | "tres"
                | "cuatro"
                | "cinco"
                | "seis"
                | "siete"
                | "ocho"
                | "nueve"
                | "diez"
                | "once"
                | "doce"
                | "trece"
                | "catorce"
                | "quince"
                | "dieciseis"
                | "diecisiete"
                | "dieciocho"
                | "diecinueve"
                | "veinte"
                | "treinta"
                | "cuarenta"
                | "cincuenta"
                | "sesenta"
                | "setenta"
                | "ochenta"
                | "noventa"
                | "cien"
                | "cientos"
                | "varios"
                | "varias"
                | "muchos"
                | "muchas"
                | "pocos"
                | "pocas"
                | "algunos"
                | "algunas"
                | "numerosos"
                | "numerosas"
                | "diversos"
                | "diversas"
        ) {
            return Some(GrammaticalNumber::Plural);
        }

        None
    }

    /// Devuelve true si hay un token no-palabra (número/puntuación/etc.) entre dos índices.
    fn has_nonword_between(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        let (start, end) = if start_idx < end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };

        for i in (start + 1)..end {
            match tokens[i].token_type {
                TokenType::Whitespace | TokenType::Word => continue,
                _ => return true,
            }
        }
        false
    }

    /// Devuelve true si hay una coma entre dos índices de tokens.
    fn has_comma_between(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        let (start, end) = if start_idx < end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };

        for i in (start + 1)..end {
            if tokens[i].token_type == TokenType::Punctuation && tokens[i].text == "," {
                return true;
            }
        }
        false
    }

    fn is_clause_break_between(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        has_sentence_boundary(tokens, start_idx, end_idx)
            || Self::has_comma_between(tokens, start_idx, end_idx)
    }

    /// Verifica si un token es un número romano en mayúsculas (I, V, X, L, C, D, M).
    fn is_roman_numeral(word: &str) -> bool {
        let mut has_alpha = false;
        for ch in word.chars() {
            if !ch.is_alphabetic() {
                return false;
            }
            has_alpha = true;
            if ch.is_lowercase() {
                return false;
            }
            let upper = ch.to_ascii_uppercase();
            if !matches!(upper, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M') {
                return false;
            }
        }
        has_alpha
    }

    /// Devuelve true si hay un nombre propio antes de una conjuncion (y/e).
    fn has_proper_name_before_conjunction(
        word_tokens: &[(usize, &Token)],
        tokens: &[Token],
        conj_pos: usize,
    ) -> bool {
        if conj_pos == 0 {
            return false;
        }
        let (conj_idx, _) = word_tokens[conj_pos];
        let max_lookback = 6.min(conj_pos);

        for offset in 1..=max_lookback {
            let check_pos = conj_pos - offset;
            let (check_idx, token) = word_tokens[check_pos];
            if has_sentence_boundary(tokens, check_idx, conj_idx) {
                break;
            }

            let text = token.effective_text();
            let lower = text.to_lowercase();
            if Self::is_name_connector(&lower) {
                continue;
            }

            let is_capitalized = text
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false);
            let is_all_uppercase = text.chars().any(|c| c.is_alphabetic())
                && text.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());

            if !is_capitalized && !is_all_uppercase {
                break;
            }

            if let Some(ref info) = token.word_info {
                if matches!(
                    info.category,
                    WordCategory::Articulo
                        | WordCategory::Determinante
                        | WordCategory::Preposicion
                        | WordCategory::Conjuncion
                        | WordCategory::Pronombre
                ) {
                    continue;
                }
            }

            return true;
        }

        false
    }

    fn is_proper_name_like_token(token: &Token) -> bool {
        let text = token.effective_text();
        let is_capitalized = text
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false);
        let is_all_uppercase = text.chars().any(|c| c.is_alphabetic())
            && text.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
        if !is_capitalized && !is_all_uppercase {
            return false;
        }

        if let Some(ref info) = token.word_info {
            if matches!(
                info.category,
                WordCategory::Articulo
                    | WordCategory::Determinante
                    | WordCategory::Preposicion
                    | WordCategory::Conjuncion
                    | WordCategory::Pronombre
                    | WordCategory::Verbo
            ) {
                return false;
            }
        }

        true
    }

    fn is_proper_name_linker(word: &str) -> bool {
        matches!(
            Self::normalize_spanish(word).as_str(),
            "de" | "del" | "la" | "las" | "los"
        )
    }

    fn consume_proper_name_sequence(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        start_pos: usize,
    ) -> Option<(usize, usize)> {
        if start_pos >= word_tokens.len() {
            return None;
        }
        if !Self::is_proper_name_like_token(word_tokens[start_pos].1) {
            return None;
        }

        let mut end_pos = start_pos;
        let mut end_idx = word_tokens[start_pos].0;
        let mut pos = start_pos + 1;

        while pos < word_tokens.len() {
            let (curr_idx, curr_token) = word_tokens[pos];
            if has_sentence_boundary(tokens, end_idx, curr_idx) {
                break;
            }

            let curr_lower = curr_token.effective_text().to_lowercase();
            if Self::is_proper_name_linker(&curr_lower) {
                if pos + 1 >= word_tokens.len() {
                    break;
                }
                let (next_idx, next_token) = word_tokens[pos + 1];
                if has_sentence_boundary(tokens, curr_idx, next_idx)
                    || !Self::is_proper_name_like_token(next_token)
                {
                    break;
                }
                end_pos = pos + 1;
                end_idx = next_idx;
                pos += 2;
                continue;
            }

            if Self::is_proper_name_like_token(curr_token) {
                end_pos = pos;
                end_idx = curr_idx;
                pos += 1;
                continue;
            }

            break;
        }

        Some((end_pos, end_idx))
    }

    fn detect_proper_name_coordinated_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        start_pos: usize,
    ) -> Option<NominalSubject> {
        let (first_end_pos, mut end_idx) =
            Self::consume_proper_name_sequence(tokens, word_tokens, start_pos)?;

        if first_end_pos + 1 >= word_tokens.len() {
            return None;
        }
        let (mut conj_idx, conj_token) = word_tokens[first_end_pos + 1];
        if has_sentence_boundary(tokens, end_idx, conj_idx) {
            return None;
        }
        let conj_lower = Self::normalize_spanish(conj_token.effective_text());
        if conj_lower != "y" && conj_lower != "e" {
            return None;
        }

        let mut next_start = first_end_pos + 2;
        if next_start >= word_tokens.len() {
            return None;
        }
        let (next_idx, _) = word_tokens[next_start];
        if has_sentence_boundary(tokens, conj_idx, next_idx) {
            return None;
        }
        let (mut seq_end_pos, seq_end_idx) =
            Self::consume_proper_name_sequence(tokens, word_tokens, next_start)?;
        end_idx = seq_end_idx;

        // Permitir más elementos coordinados: "María y Pedro y Juan ..."
        loop {
            if seq_end_pos + 1 >= word_tokens.len() {
                break;
            }
            let (maybe_conj_idx, maybe_conj_token) = word_tokens[seq_end_pos + 1];
            if has_sentence_boundary(tokens, end_idx, maybe_conj_idx) {
                break;
            }
            let maybe_conj = Self::normalize_spanish(maybe_conj_token.effective_text());
            if maybe_conj != "y" && maybe_conj != "e" {
                break;
            }
            let maybe_next_start = seq_end_pos + 2;
            if maybe_next_start >= word_tokens.len() {
                break;
            }
            let (maybe_next_idx, _) = word_tokens[maybe_next_start];
            if has_sentence_boundary(tokens, maybe_conj_idx, maybe_next_idx) {
                break;
            }
            let (next_seq_end_pos, next_seq_end_idx) =
                match Self::consume_proper_name_sequence(tokens, word_tokens, maybe_next_start) {
                    Some(v) => v,
                    None => break,
                };
            seq_end_pos = next_seq_end_pos;
            end_idx = next_seq_end_idx;
            conj_idx = maybe_conj_idx;
            next_start = maybe_next_start;
        }

        let _ = (conj_idx, next_start); // variables kept for readability while parsing chain

        Some(NominalSubject {
            nucleus_idx: word_tokens[start_pos].0,
            number: GrammaticalNumber::Plural,
            end_idx,
            is_coordinated: true,
            is_ni_correlative: false,
        })
    }

    /// Detecta incisos ejemplificativos delimitados por comas:
    /// ", como ... ,"
    /// Ejemplos:
    /// - "Las enfermedades, como la diabetes, requieren..."
    /// - "Los edificios, como el colegio o la biblioteca, necesitan..."
    ///
    /// Los SN dentro del inciso no deben tratarse como sujetos del verbo principal.
    fn is_inside_como_apposition(
        word_tokens: &[(usize, &Token)],
        all_tokens: &[Token],
        pos: usize,
    ) -> bool {
        if pos >= word_tokens.len() {
            return false;
        }

        let (current_idx, _) = word_tokens[pos];

        // Buscar la coma de apertura más cercana sin cruzar límite fuerte.
        let mut left_comma_idx = None;
        for idx in (0..current_idx).rev() {
            let token = &all_tokens[idx];
            if token.token_type != TokenType::Punctuation {
                continue;
            }
            if token.text == "," {
                left_comma_idx = Some(idx);
                break;
            }
            if matches!(token.text.as_str(), "." | "!" | "?" | ";" | ":") {
                return false;
            }
        }
        let Some(left_comma_idx) = left_comma_idx else {
            return false;
        };

        // Buscar la coma de cierre sin cruzar límite fuerte.
        let mut right_comma_idx = None;
        for idx in (current_idx + 1)..all_tokens.len() {
            let token = &all_tokens[idx];
            if token.token_type != TokenType::Punctuation {
                continue;
            }
            if token.text == "," {
                right_comma_idx = Some(idx);
                break;
            }
            if matches!(token.text.as_str(), "." | "!" | "?" | ";" | ":") {
                return false;
            }
        }
        let Some(right_comma_idx) = right_comma_idx else {
            return false;
        };

        // El segmento debe contener "como" tras la coma de apertura.
        let has_como = word_tokens.iter().any(|(idx, token)| {
            *idx > left_comma_idx
                && *idx < current_idx
                && Self::normalize_spanish(token.effective_text()) == "como"
        });

        has_como && current_idx < right_comma_idx
    }

    /// Detecta patrón de relativa con sujeto pospuesto tras adverbio(s)
    /// o cuantificadores temporales:
    /// "... que [verbo] [adverbio/cuanti]* [det+sust] ..."
    /// En ese caso, el SN es sujeto del verbo relativo, no del verbo principal.
    fn is_relative_postposed_subject_context(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        start_pos: usize,
    ) -> bool {
        if start_pos < 2 {
            return false;
        }

        let (det_idx, _) = word_tokens[start_pos];
        let mut probe_pos = start_pos;
        let mut skipped_fillers = 0usize;
        const MAX_SKIPPED_FILLERS: usize = 3;

        while probe_pos > 0 {
            if let Some(chunk_len) =
                Self::relative_temporal_prepositional_bridge_len(word_tokens, probe_pos)
            {
                let (bridge_start_idx, _) = word_tokens[probe_pos - chunk_len];
                if has_sentence_boundary(tokens, bridge_start_idx, det_idx)
                    || Self::has_nonword_between(tokens, bridge_start_idx, det_idx)
                {
                    return false;
                }

                skipped_fillers += 1;
                if skipped_fillers > MAX_SKIPPED_FILLERS {
                    return false;
                }
                probe_pos -= chunk_len;
                continue;
            }

            let (candidate_idx, candidate_token) = word_tokens[probe_pos - 1];

            if has_sentence_boundary(tokens, candidate_idx, det_idx)
                || Self::has_nonword_between(tokens, candidate_idx, det_idx)
            {
                return false;
            }

            let candidate_lower = candidate_token.effective_text().to_lowercase();
            let is_filler = Self::is_adverb_token(candidate_token)
                || Self::is_clitic_pronoun(&candidate_lower)
                || Self::is_temporal_quantifier(&candidate_lower)
                // Permitir puente preposicional en relativas:
                // "que pintaron durante toda la noche ..."
                || Self::is_preposition(&candidate_lower);
            if is_filler {
                skipped_fillers += 1;
                if skipped_fillers > MAX_SKIPPED_FILLERS {
                    return false;
                }
                probe_pos -= 1;
                continue;
            }

            let candidate_is_verb = candidate_token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Verbo)
                .unwrap_or(false)
                || Self::looks_like_verb(&candidate_lower);
            if !candidate_is_verb {
                return false;
            }

            if probe_pos < 2 {
                return false;
            }
            let (before_verb_idx, before_verb_token) = word_tokens[probe_pos - 2];
            if has_sentence_boundary(tokens, before_verb_idx, candidate_idx)
                || Self::has_nonword_between(tokens, before_verb_idx, candidate_idx)
            {
                return false;
            }

            return before_verb_token.effective_text().to_lowercase() == "que";
        }

        false
    }

    fn relative_temporal_prepositional_bridge_len(
        word_tokens: &[(usize, &Token)],
        probe_pos: usize,
    ) -> Option<usize> {
        if probe_pos < 3 {
            return None;
        }

        let noun_lower = Self::normalize_spanish(word_tokens[probe_pos - 1].1.effective_text());
        let det_lower = Self::normalize_spanish(word_tokens[probe_pos - 2].1.effective_text());
        if !Self::is_temporal_noun(&noun_lower) || !Self::is_determiner(&det_lower) {
            return None;
        }

        // "durante la noche", "en la tarde", ...
        let prep_lower = Self::normalize_spanish(word_tokens[probe_pos - 3].1.effective_text());
        if Self::is_preposition(&prep_lower) {
            return Some(3);
        }

        // "durante toda la noche", "en todos los dias", ...
        if probe_pos < 4 {
            return None;
        }
        let quant_lower = Self::normalize_spanish(word_tokens[probe_pos - 3].1.effective_text());
        if !Self::is_temporal_quantifier(&quant_lower) {
            return None;
        }

        let prep_with_quant =
            Self::normalize_spanish(word_tokens[probe_pos - 4].1.effective_text());
        if Self::is_preposition(&prep_with_quant) {
            return Some(4);
        }

        None
    }

    /// Detecta patrones tipo:
    /// "El lunes empiezan las vacaciones", "La semana pasada vinieron mis primos".
    /// En estos casos, el primer SN temporal es complemento circunstancial, no sujeto.
    fn is_temporal_complement_with_postposed_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        start_pos: usize,
        verb_pos: usize,
        verb_person: GrammaticalPerson,
        verb_number: GrammaticalNumber,
        verb_infinitive: &str,
    ) -> bool {
        if start_pos + 1 >= word_tokens.len() || verb_pos <= start_pos + 1 {
            return false;
        }

        // Heurística conservadora: solo evitar el falso positivo típico
        // (SN temporal al inicio + verbo que no concuerda con sujeto nominal real).
        //
        // Regla segura: un SN temporal no puede ser sujeto de verbos en 1ª/2ª persona.
        // Ej: "El lunes vamos/viajo/vienes".
        if matches!(
            verb_person,
            GrammaticalPerson::First | GrammaticalPerson::Second
        ) {
            return true;
        }
        let (det_idx, det_token) = word_tokens[start_pos];
        let (_, noun_token) = word_tokens[start_pos + 1];

        if !Self::is_determiner(det_token.effective_text()) {
            return false;
        }
        if !Self::is_temporal_noun(noun_token.effective_text()) {
            return false;
        }

        // Requerir inicio de cláusula para no afectar SN internos.
        if start_pos > 0 {
            let (prev_idx, prev_token) = word_tokens[start_pos - 1];
            let separated_by_boundary_or_comma = has_sentence_boundary(tokens, prev_idx, det_idx)
                || Self::has_comma_between(tokens, prev_idx, det_idx);
            if !separated_by_boundary_or_comma {
                // Permitir cuantificador temporal inmediatamente antes del determinante:
                // "Todos los días...", "Todas las noches..."
                let has_temporal_quantifier_prefix =
                    Self::is_temporal_quantifier(&prev_token.effective_text().to_lowercase())
                        && (start_pos == 1 || {
                            let (before_prev_idx, _) = word_tokens[start_pos - 2];
                            has_sentence_boundary(tokens, before_prev_idx, prev_idx)
                                || Self::has_comma_between(tokens, before_prev_idx, prev_idx)
                        });
                if !has_temporal_quantifier_prefix {
                    return false;
                }
            }
        }

        // "Todos los días llueve", "Las noches nieva": el SN temporal inicial
        // no es sujeto cuando el verbo es impersonal meteorológico en 3ª singular.
        if verb_person == GrammaticalPerson::Third
            && verb_number == GrammaticalNumber::Singular
            && Self::is_impersonal_weather_infinitive(verb_infinitive)
        {
            return true;
        }

        // Con complemento temporal plural al inicio ("todos los días", "los sábados"...),
        // también es frecuente 3ª singular con sujeto implícito:
        // "Todos los días sale a correr".
        //
        // Si aparece un sujeto pospuesto explícito y plural, NO relajamos la regla
        // para conservar correcciones como "Todos los días llega mis amigos" → "llegan".
        if verb_person == GrammaticalPerson::Third
            && verb_number == GrammaticalNumber::Singular
            && Self::get_determiner_number(det_token.effective_text()) == GrammaticalNumber::Plural
        {
            match Self::detect_postposed_subject_number(tokens, word_tokens, verb_pos) {
                Some(GrammaticalNumber::Plural) => return false,
                Some(GrammaticalNumber::Singular) | None => return true,
            }
        }

        // Para 3ª persona plural aplicamos la detección fuerte con sujeto pospuesto.
        if verb_person != GrammaticalPerson::Third || verb_number != GrammaticalNumber::Plural {
            return false;
        }
        Self::detect_postposed_subject_number(tokens, word_tokens, verb_pos).is_some()
    }

    fn get_possessive_determiner_number(word: &str) -> Option<GrammaticalNumber> {
        match Self::normalize_spanish(word).as_str() {
            "mis" | "tus" | "sus" | "nuestros" | "nuestras" | "vuestros" | "vuestras" => {
                Some(GrammaticalNumber::Plural)
            }
            "mi" | "tu" | "su" | "nuestro" | "nuestra" | "vuestro" | "vuestra" => {
                Some(GrammaticalNumber::Singular)
            }
            _ => None,
        }
    }

    /// Intenta detectar sujeto nominal pospuesto tras el verbo y devuelve su número.
    /// Patrón: [adverbio]* + (det/posesivo) + sustantivo
    fn detect_postposed_subject_number(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
    ) -> Option<GrammaticalNumber> {
        if verb_pos + 1 >= word_tokens.len() {
            return None;
        }

        let (verb_idx, _) = word_tokens[verb_pos];
        let mut probe_pos = verb_pos + 1;
        let mut skipped_adverbs = 0usize;
        const MAX_SKIPPED_ADVERBS: usize = 2;

        while probe_pos < word_tokens.len() {
            let (candidate_idx, candidate_token) = word_tokens[probe_pos];
            if has_sentence_boundary(tokens, verb_idx, candidate_idx) {
                return None;
            }

            let candidate_lower = candidate_token.effective_text().to_lowercase();
            if Self::is_adverb_token(candidate_token) {
                skipped_adverbs += 1;
                if skipped_adverbs > MAX_SKIPPED_ADVERBS {
                    return None;
                }
                probe_pos += 1;
                continue;
            }

            let determiner_number = if Self::is_determiner(&candidate_lower) {
                Some(Self::get_determiner_number(&candidate_lower))
            } else if Self::is_possessive_determiner(&candidate_lower) {
                Self::get_possessive_determiner_number(&candidate_lower)
            } else {
                Self::get_quantifier_number(&candidate_lower)
            };

            let (_noun_idx, noun_token, number_hint) = if let Some(det_number) = determiner_number {
                if probe_pos + 1 >= word_tokens.len() {
                    return None;
                }
                let (noun_idx, noun_token) = word_tokens[probe_pos + 1];
                if has_sentence_boundary(tokens, candidate_idx, noun_idx) {
                    return None;
                }
                (noun_idx, noun_token, Some(det_number))
            } else {
                // Fallback conservador: sustantivo plural sin determinante.
                // Ej: "Le sobra motivos", "Nos falta recursos".
                (candidate_idx, candidate_token, None)
            };

            if number_hint.is_none() && probe_pos > verb_pos + 1 {
                let prev_lower =
                    Self::normalize_spanish(word_tokens[probe_pos - 1].1.effective_text());
                let noun_lower = Self::normalize_spanish(noun_token.effective_text());
                if Self::is_subordinate_clause_intro(&prev_lower)
                    && Self::looks_like_finite_nonthird_form(&noun_lower)
                {
                    return None;
                }
            }

            let noun_like = noun_token
                .word_info
                .as_ref()
                .map(|info| {
                    matches!(
                        info.category,
                        WordCategory::Sustantivo | WordCategory::Adjetivo | WordCategory::Otro
                    )
                })
                .unwrap_or_else(|| {
                    let lower = noun_token.effective_text().to_lowercase();
                    !Self::looks_like_verb(&lower) && !Self::is_common_adverb(&lower)
                });
            if !noun_like {
                return None;
            }

            let noun_number = noun_token
                .word_info
                .as_ref()
                .and_then(|info| match info.number {
                    Number::Singular => Some(GrammaticalNumber::Singular),
                    Number::Plural => Some(GrammaticalNumber::Plural),
                    Number::None => None,
                })
                .or_else(|| {
                    let lower = Self::normalize_spanish(noun_token.effective_text());
                    if lower.ends_with('s') && lower.len() > 2 && !Self::looks_like_verb(&lower) {
                        Some(GrammaticalNumber::Plural)
                    } else {
                        None
                    }
                });

            if number_hint.is_none() && noun_number != Some(GrammaticalNumber::Plural) {
                return None;
            }

            // Priorizar el número del determinante para evitar falsos plurales
            // con sustantivos invariables en -s (ej: "su cumpleaños").
            return number_hint.or(noun_number);
        }

        None
    }

    fn is_subordinate_clause_intro(word: &str) -> bool {
        matches!(
            word,
            "que"
                | "como"
                | "si"
                | "cuando"
                | "donde"
                | "adonde"
                | "quien"
                | "quienes"
                | "cual"
                | "cuales"
                | "cuanto"
                | "cuanta"
                | "cuantos"
                | "cuantas"
        )
    }

    fn looks_like_finite_nonthird_form(word: &str) -> bool {
        matches!(
            word,
            "soy"
                | "eres"
                | "somos"
                | "estoy"
                | "estas"
                | "estamos"
                | "vamos"
                | "vas"
                | "puedo"
                | "puedes"
                | "podemos"
                | "quiero"
                | "quieres"
                | "queremos"
                | "tengo"
                | "tienes"
                | "tenemos"
                | "se"
        ) || word.ends_with("as")
            || word.ends_with("es")
            || word.ends_with("amos")
            || word.ends_with("emos")
            || word.ends_with("imos")
            || word.ends_with("ais")
            || word.ends_with("eis")
    }

    /// Detecta copulativas con "ser" donde la concordancia plural con atributo
    /// posverbal también es válida, aunque el sujeto nominal previo sea singular.
    /// Ejemplos:
    /// - "El problema fueron las lluvias"
    /// - "La causa son los retrasos"
    fn is_ser_copulative_with_postverbal_plural_attribute(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        nominal_subject: &NominalSubject,
        verb_pos: usize,
        verb_lower: &str,
        verb_person: GrammaticalPerson,
        verb_number: GrammaticalNumber,
        infinitive: &str,
    ) -> bool {
        if nominal_subject.number != GrammaticalNumber::Singular
            || verb_person != GrammaticalPerson::Third
            || verb_number != GrammaticalNumber::Plural
            || !Self::is_likely_copulative_ser_form(verb_lower, infinitive)
        {
            return false;
        }

        if verb_pos + 1 >= word_tokens.len() {
            return false;
        }

        let (verb_idx, _) = word_tokens[verb_pos];
        let mut probe_pos = verb_pos + 1;
        let mut skipped_adverbs = 0usize;
        const MAX_SKIPPED_ADVERBS: usize = 2;

        while probe_pos < word_tokens.len() {
            let (candidate_idx, candidate_token) = word_tokens[probe_pos];
            if has_sentence_boundary(tokens, verb_idx, candidate_idx) {
                return false;
            }

            if Self::is_adverb_token(candidate_token) {
                skipped_adverbs += 1;
                if skipped_adverbs > MAX_SKIPPED_ADVERBS {
                    return false;
                }
                probe_pos += 1;
                continue;
            }

            // Atributo pronominal plural sin determinante:
            // "El problema son ellos/ustedes/nosotros".
            if Self::is_plural_attribute_pronoun(candidate_token) {
                return true;
            }

            let candidate_lower = candidate_token.effective_text().to_lowercase();
            if !Self::is_determiner(&candidate_lower)
                || Self::get_determiner_number(&candidate_lower) != GrammaticalNumber::Plural
            {
                return false;
            }

            if probe_pos + 1 >= word_tokens.len() {
                return false;
            }
            let (head_idx, head_token) = word_tokens[probe_pos + 1];
            if has_sentence_boundary(tokens, candidate_idx, head_idx) {
                return false;
            }

            if Self::is_plural_nominal_attribute_head(head_token) {
                return true;
            }

            // Permitir adjetivo plural antes del núcleo nominal:
            // "fueron las intensas lluvias".
            if Self::is_plural_adjective_token(head_token) && probe_pos + 2 < word_tokens.len() {
                let (noun_idx, noun_token) = word_tokens[probe_pos + 2];
                if !has_sentence_boundary(tokens, head_idx, noun_idx)
                    && Self::is_plural_nominal_attribute_head(noun_token)
                {
                    return true;
                }
            }

            return false;
        }

        false
    }

    fn is_likely_copulative_ser_form(verb_lower: &str, infinitive: &str) -> bool {
        if infinitive == "ser" {
            return true;
        }

        // En pretérito "fue/fueron" se mapea a "ir" por ambigüedad formal,
        // pero en copulativas nominales pueden ser formas de "ser".
        matches!(
            Self::normalize_spanish(verb_lower).as_str(),
            "fue" | "fueron"
        )
    }

    fn is_plural_adjective_token(token: &Token) -> bool {
        if let Some(ref info) = token.word_info {
            return info.category == WordCategory::Adjetivo && info.number == Number::Plural;
        }

        let lower = Self::normalize_spanish(token.effective_text());
        lower.ends_with('s') && !Self::looks_like_verb(&lower)
    }

    fn is_plural_nominal_attribute_head(token: &Token) -> bool {
        if let Some(ref info) = token.word_info {
            return matches!(
                info.category,
                WordCategory::Sustantivo | WordCategory::Adjetivo | WordCategory::Otro
            ) && info.number == Number::Plural;
        }

        let lower = Self::normalize_spanish(token.effective_text());
        lower.ends_with('s') && lower.len() > 2 && !Self::looks_like_verb(&lower)
    }

    fn is_plural_attribute_pronoun(token: &Token) -> bool {
        let lower = Self::normalize_spanish(token.effective_text());

        if let Some(ref info) = token.word_info {
            if info.category == WordCategory::Pronombre && info.number == Number::Plural {
                return true;
            }
            if info.category == WordCategory::Determinante
                && matches!(
                    lower.as_str(),
                    "estos" | "estas" | "esos" | "esas" | "aquellos" | "aquellas"
                )
            {
                return true;
            }
        }

        matches!(
            lower.as_str(),
            "nosotros"
                | "nosotras"
                | "vosotros"
                | "vosotras"
                | "ellos"
                | "ellas"
                | "ustedes"
                | "quienes"
                | "cuales"
                | "estos"
                | "estas"
                | "esos"
                | "esas"
                | "aquellos"
                | "aquellas"
        )
    }

    /// Detecta un sujeto nominal (sintagma nominal) empezando en la posición dada
    /// Patrón: Det + Sust + (de/del/de la...)?
    /// Ejemplo: "El Ministerio del Interior" → núcleo "Ministerio", singular
    fn detect_nominal_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        start_pos: usize,
    ) -> Option<NominalSubject> {
        if start_pos >= word_tokens.len() {
            return None;
        }

        let (det_idx, det_token) = word_tokens[start_pos];
        let det_text = det_token.effective_text();

        // Si el token anterior es una preposición inmediata, este no es un sujeto nominal
        // (es un complemento preposicional, ej: "con los ministros")
        if start_pos > 0 {
            let (prev_idx, prev_token) = word_tokens[start_pos - 1];
            let prev_text = prev_token.effective_text().to_lowercase();
            if Self::is_preposition(&prev_text)
                && !Self::has_nonword_between(tokens, prev_idx, det_idx)
            {
                return None;
            }
            // Si el token anterior es un verbo (sin coma/puntuación entre ellos),
            // este SN es probablemente el objeto directo, no un nuevo sujeto
            // Ejemplo: "Los estudiantes que aprobaron el examen celebraron"
            // "el examen" es OD de "aprobaron", no sujeto de "celebraron"
            let prev_is_verb = prev_token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Verbo)
                .unwrap_or(false)
                || Self::looks_like_verb(&prev_text);
            if prev_is_verb && !Self::has_nonword_between(tokens, prev_idx, det_idx) {
                return None;
            }
        }

        // En relativas de objeto, un sujeto pospuesto (con adverbios intermedios) no debe
        // reinterpretarse como sujeto nominal del verbo principal.
        // Ej: "Las cosas que dijo ayer el ministro son..."
        if Self::is_relative_postposed_subject_context(tokens, word_tokens, start_pos) {
            return None;
        }

        // Debe empezar con un determinante (articulo/demostrativo/posesivo)
        let starts_with_determiner =
            Self::is_determiner(det_text) || Self::is_possessive_determiner(det_text);
        if !starts_with_determiner {
            return Self::detect_proper_name_coordinated_subject(tokens, word_tokens, start_pos);
        }

        // Siguiente token debe ser sustantivo
        if start_pos + 1 >= word_tokens.len() {
            return None;
        }

        let (noun_idx, noun_token) = word_tokens[start_pos + 1];

        // Verificar que no hay límite de oración
        if has_sentence_boundary(tokens, det_idx, noun_idx) {
            return None;
        }

        // Verificar que es un sustantivo
        let is_noun = if let Some(ref info) = noun_token.word_info {
            info.category == WordCategory::Sustantivo
        } else {
            false
        };

        if !is_noun {
            return None;
        }

        let noun_text = noun_token.effective_text().to_lowercase();
        let is_variable_collective_head = exceptions::is_variable_collective_noun(&noun_text);

        let mut number = Self::get_possessive_determiner_number(det_text)
            .unwrap_or_else(|| Self::get_determiner_number(det_text));
        let mut end_idx = noun_idx;
        let mut has_coordination = false;
        let mut has_subject_coordination = false;
        let mut has_ni_coordination = false;
        let mut has_tanto_correlative = false;
        let mut has_ni_correlative = false;
        let mut in_de_complement = false;

        // Coordinacion previa con nombre propio sin determinante: "Google y el Movimiento"
        if start_pos > 0 {
            let (_, prev_token) = word_tokens[start_pos - 1];
            let prev_lower = prev_token.effective_text().to_lowercase();
            let prev_normalized = Self::normalize_spanish(&prev_lower);
            has_tanto_correlative = matches!(
                prev_normalized.as_str(),
                "tanto" | "tanta" | "tantos" | "tantas"
            );
            has_ni_correlative = prev_normalized == "ni";
            if (prev_lower == "y" || prev_lower == "e")
                && Self::has_proper_name_before_conjunction(word_tokens, tokens, start_pos - 1)
            {
                has_coordination = true;
                has_subject_coordination = true;
                number = GrammaticalNumber::Plural;
            }
        }

        // Buscar patrón "de/del/de la" o coordinación "y/e"
        let mut pos = start_pos + 2;
        while pos < word_tokens.len() {
            let (curr_idx, curr_token) = word_tokens[pos];

            // Verificar que no hay límite de oración
            if has_sentence_boundary(tokens, end_idx, curr_idx) {
                break;
            }

            // Si hay coordinación previa y aparece una coma, no extender el sujeto
            // Ejemplo: "la casa y el coche, el CNE autorizó"
            if has_coordination && Self::has_comma_between(tokens, end_idx, curr_idx) {
                break;
            }

            let curr_text = curr_token.effective_text().to_lowercase();

            // Adjetivos postnominales dentro del sintagma nominal
            if let Some(ref info) = curr_token.word_info {
                if info.category == WordCategory::Adjetivo {
                    end_idx = curr_idx;
                    pos += 1;
                    continue;
                }
            }

            // Coordinación nominal:
            // - "y/e"
            // - "tanto ... como ..."
            // - "ni ... ni ..."
            // Solo si realmente inicia otro SN.
            let is_coord_conjunction = curr_text == "y"
                || curr_text == "e"
                || (curr_text == "como" && has_tanto_correlative)
                || (curr_text == "ni" && has_ni_correlative);
            if is_coord_conjunction {
                if pos + 1 >= word_tokens.len() {
                    break;
                }
                let (_, next_token) = word_tokens[pos + 1];
                let next_text = next_token.effective_text().to_lowercase();
                let next_is_determiner =
                    Self::is_determiner(&next_text) || Self::is_possessive_determiner(&next_text);

                let mut starts_noun_phrase = next_is_determiner;
                if !starts_noun_phrase {
                    if let Some(ref info) = next_token.word_info {
                        if info.category == WordCategory::Sustantivo {
                            starts_noun_phrase = true;
                        }
                    }
                }

                // Si no empieza otro SN, no es coordinación nominal (ej: "y abre")
                if !starts_noun_phrase {
                    break;
                }

                // Coordinación interna de complemento en "de X y Y" (sin determinante explícito):
                // "la asociación de padres y madres", "la dirección de ventas y marketing".
                // No debe pluralizar el núcleo singular del sujeto.
                let is_internal_de_coordination = in_de_complement && !next_is_determiner;
                has_coordination = true;
                if !is_internal_de_coordination {
                    has_subject_coordination = true;
                    if curr_text == "ni" && has_ni_correlative {
                        has_ni_coordination = true;
                    }
                }
                end_idx = curr_idx;
                pos += 1;
                // Seguir buscando el siguiente elemento coordinado
                continue;
            }

            // Preposición "de" o contracción "del"
            if curr_text == "de" || curr_text == "del" {
                in_de_complement = true;
                end_idx = curr_idx;
                pos += 1;

                // Puede seguir artículo + sustantivo o solo sustantivo
                if pos < word_tokens.len() {
                    let (next_idx, next_token) = word_tokens[pos];
                    if !has_sentence_boundary(tokens, curr_idx, next_idx) {
                        let next_text = next_token.effective_text();
                        let is_capitalized = next_text
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false);

                        // Si es determinante (artículo/demostrativo/posesivo), avanzar
                        if Self::is_determiner(next_text)
                            || Self::is_possessive_determiner(next_text)
                        {
                            end_idx = next_idx;
                            pos += 1;

                            // Luego debe venir sustantivo
                            if pos < word_tokens.len() {
                                let (sust_idx, _) = word_tokens[pos];
                                if !has_sentence_boundary(tokens, next_idx, sust_idx) {
                                    end_idx = sust_idx;
                                    pos += 1;
                                }
                            }
                        } else if let Some(ref info) = next_token.word_info {
                            // Si es sustantivo directo
                            if info.category == WordCategory::Sustantivo {
                                end_idx = next_idx;
                                pos += 1;
                            } else if info.category == WordCategory::Otro && is_capitalized {
                                // Palabra capitalizada sin categoria fiable: tratar como nombre propio
                                end_idx = next_idx;
                                pos += 1;

                                // Consumir nombres propios compuestos (Juan Carlos, etc.)
                                while pos < word_tokens.len() {
                                    let (cap_idx, cap_token) = word_tokens[pos];
                                    if has_sentence_boundary(tokens, end_idx, cap_idx) {
                                        break;
                                    }
                                    let cap_text = cap_token.effective_text();
                                    let cap_is_capitalized = cap_text
                                        .chars()
                                        .next()
                                        .map(|c| c.is_uppercase())
                                        .unwrap_or(false);
                                    if cap_is_capitalized {
                                        end_idx = cap_idx;
                                        pos += 1;
                                        continue;
                                    }
                                    break;
                                }
                            }
                        } else if is_capitalized {
                            // Si es nombre propio (capitalizado), tratarlo como sustantivo
                            end_idx = next_idx;
                            pos += 1;

                            // Consumir nombres propios compuestos (Juan Carlos, etc.)
                            while pos < word_tokens.len() {
                                let (cap_idx, cap_token) = word_tokens[pos];
                                if has_sentence_boundary(tokens, end_idx, cap_idx) {
                                    break;
                                }
                                let cap_text = cap_token.effective_text();
                                let cap_is_capitalized = cap_text
                                    .chars()
                                    .next()
                                    .map(|c| c.is_uppercase())
                                    .unwrap_or(false);
                                if cap_is_capitalized {
                                    end_idx = cap_idx;
                                    pos += 1;
                                    continue;
                                }
                                break;
                            }
                        }
                    }
                }
                continue;
            }

            // Si es otro determinante o sustantivo después de coordinación, continuar
            if has_coordination {
                if Self::is_determiner(&curr_text) || Self::is_possessive_determiner(&curr_text) {
                    end_idx = curr_idx;
                    pos += 1;
                    // Siguiente debería ser sustantivo
                    if pos < word_tokens.len() {
                        let (next_idx, _) = word_tokens[pos];
                        if !has_sentence_boundary(tokens, curr_idx, next_idx) {
                            end_idx = next_idx;
                            pos += 1;
                        }
                    }
                    continue;
                }

                // También aceptar sustantivo sin determinante tras coordinación
                // Ejemplo: "El ministro y presidente dijeron" - "presidente" es sustantivo sin artículo
                if let Some(ref info) = curr_token.word_info {
                    if info.category == WordCategory::Sustantivo {
                        end_idx = curr_idx;
                        pos += 1;
                        continue;
                    }
                }
            }

            // Si no es preposición ni coordinación, el sintagma termina aquí
            break;
        }

        // En cabezas colectivas/partitivas de concordancia variable
        // ("la mayoría de alumnos", "el grupo de estudiantes"),
        // la concordancia puede seguir al complemento introducido por "de".
        // Solo relajamos en ese patrón; sin "de", mantenemos concordancia singular.
        if is_variable_collective_head && in_de_complement {
            return None;
        }

        // Si hubo coordinación, el sujeto es plural
        if has_subject_coordination {
            number = GrammaticalNumber::Plural;
        }

        Some(NominalSubject {
            nucleus_idx: noun_idx,
            number,
            end_idx,
            is_coordinated: has_subject_coordination,
            is_ni_correlative: has_ni_coordination,
        })
    }

    /// Obtiene información gramatical de un pronombre personal sujeto
    fn get_subject_info(word: &str) -> Option<SubjectInfo> {
        let lower = word.to_lowercase();
        match lower.as_str() {
            "yo" => Some(SubjectInfo {
                person: GrammaticalPerson::First,
                number: GrammaticalNumber::Singular,
            }),
            // NOTA: Solo "tú" con tilde es pronombre personal; "tu" sin tilde es posesivo
            "tú" => Some(SubjectInfo {
                person: GrammaticalPerson::Second,
                number: GrammaticalNumber::Singular,
            }),
            // "usted" usa forma de tercera persona singular
            // NOTA: Solo "él" con tilde es pronombre personal; "el" sin tilde es artículo
            "él" | "ella" | "usted" => Some(SubjectInfo {
                person: GrammaticalPerson::Third,
                number: GrammaticalNumber::Singular,
            }),
            "nosotros" | "nosotras" => Some(SubjectInfo {
                person: GrammaticalPerson::First,
                number: GrammaticalNumber::Plural,
            }),
            "vosotros" | "vosotras" => Some(SubjectInfo {
                person: GrammaticalPerson::Second,
                number: GrammaticalNumber::Plural,
            }),
            // "ustedes" usa forma de tercera persona plural
            "ellos" | "ellas" | "ustedes" => Some(SubjectInfo {
                person: GrammaticalPerson::Third,
                number: GrammaticalNumber::Plural,
            }),
            _ => None,
        }
    }

    /// Verifica si el verbo concuerda con el sujeto y devuelve corrección si no
    fn check_verb_agreement(
        verb_index: usize,
        verb: &str,
        subject: &SubjectInfo,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
        allow_subjunctive: bool,
    ) -> Option<SubjectVerbCorrection> {
        let verb_lower = verb.to_lowercase();

        if allow_subjunctive {
            if let Some(vr) = verb_recognizer {
                if let Some(correction) = Self::check_present_subjunctive_agreement(
                    verb_index,
                    verb,
                    &verb_lower,
                    subject,
                    vr,
                ) {
                    return Some(correction);
                }
                if Self::could_be_present_subjunctive(&verb_lower, subject, vr) {
                    return None;
                }
            }
        }

        // Obtener información de la conjugación del verbo
        if let Some((verb_person, verb_number, verb_tense, infinitive)) =
            Self::get_verb_info(&verb_lower, verb_recognizer)
        {
            // Verificar concordancia
            if verb_person != subject.person || verb_number != subject.number {
                // Generar la forma correcta (preservando el tiempo verbal)
                if let Some(correct_form) =
                    Self::get_correct_form(&infinitive, subject.person, subject.number, verb_tense)
                {
                    // Solo corregir si tenemos una forma diferente
                    if correct_form.to_lowercase() != verb_lower {
                        return Some(SubjectVerbCorrection {
                            token_index: verb_index,
                            original: verb.to_string(),
                            suggestion: correct_form.clone(),
                            message: format!(
                                "Concordancia sujeto-verbo: '{}' debería ser '{}'",
                                verb, correct_form
                            ),
                        });
                    }
                }
            }
        }

        None
    }

    fn is_subjunctive_context_for_pronoun(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pronoun_pos: usize,
    ) -> bool {
        if pronoun_pos == 0 {
            return false;
        }

        let (pronoun_idx, _) = word_tokens[pronoun_pos];
        for j in (0..pronoun_pos).rev() {
            let (prev_idx, prev_token) = word_tokens[j];
            if has_sentence_boundary(tokens, prev_idx, pronoun_idx) {
                return false;
            }

            let lower = prev_token.effective_text().to_lowercase();
            let normalized = Self::normalize_spanish(&lower);
            if Self::is_subjunctive_trigger(word_tokens, j, &normalized) {
                return true;
            }
            if Self::is_subjunctive_bridge_token(prev_token, &normalized) {
                continue;
            }
            return false;
        }

        false
    }

    fn is_subjunctive_context_for_nominal_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        subject_start_pos: usize,
    ) -> bool {
        if subject_start_pos == 0 {
            return false;
        }

        let (subject_idx, _) = word_tokens[subject_start_pos];
        for j in (0..subject_start_pos).rev() {
            let (prev_idx, prev_token) = word_tokens[j];
            if has_sentence_boundary(tokens, prev_idx, subject_idx) {
                return false;
            }

            let lower = prev_token.effective_text().to_lowercase();
            let normalized = Self::normalize_spanish(&lower);
            if Self::is_subjunctive_trigger(word_tokens, j, &normalized) {
                return true;
            }
            if Self::is_subjunctive_bridge_token(prev_token, &normalized) {
                continue;
            }
            return false;
        }

        false
    }

    fn is_subjunctive_trigger(
        word_tokens: &[(usize, &Token)],
        pos: usize,
        normalized: &str,
    ) -> bool {
        if matches!(normalized, "que" | "ojala" | "quizas" | "acaso" | "talvez") {
            return true;
        }

        if normalized == "vez" && pos > 0 {
            let tal_lower = word_tokens[pos - 1].1.effective_text().to_lowercase();
            return Self::normalize_spanish(&tal_lower) == "tal";
        }

        false
    }

    fn is_subjunctive_bridge_token(token: &Token, normalized: &str) -> bool {
        if Self::is_adverb_token(token) {
            return true;
        }

        matches!(normalized, "no")
    }

    fn could_be_present_subjunctive(
        verb: &str,
        subject: &SubjectInfo,
        verb_recognizer: &dyn VerbFormRecognizer,
    ) -> bool {
        let (endings_ar, endings_er_ir): (&[&str], &[&str]) = match (subject.person, subject.number)
        {
            (GrammaticalPerson::First, GrammaticalNumber::Singular)
            | (GrammaticalPerson::Third, GrammaticalNumber::Singular) => (&["e"], &["a"]),
            (GrammaticalPerson::Second, GrammaticalNumber::Singular) => (&["es"], &["as"]),
            (GrammaticalPerson::First, GrammaticalNumber::Plural) => (&["emos"], &["amos"]),
            (GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                (&["éis", "eis"], &["áis", "ais"])
            }
            (GrammaticalPerson::Third, GrammaticalNumber::Plural) => (&["en"], &["an"]),
        };

        for ending in endings_ar {
            if let Some(stem) = verb.strip_suffix(ending) {
                if stem.is_empty() {
                    continue;
                }
                if verb_recognizer.knows_infinitive(&format!("{stem}ar")) {
                    return true;
                }
            }
        }

        for ending in endings_er_ir {
            if let Some(stem) = verb.strip_suffix(ending) {
                if stem.is_empty() {
                    continue;
                }
                if verb_recognizer.knows_infinitive(&format!("{stem}er")) {
                    return true;
                }
                if verb_recognizer.knows_infinitive(&format!("{stem}ir")) {
                    return true;
                }
            }
        }

        false
    }

    /// Obtiene información de persona/número/tiempo del verbo conjugado
    /// Devuelve (persona, número, tiempo, infinitivo)
    fn get_verb_info(
        verb: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Option<(GrammaticalPerson, GrammaticalNumber, VerbTense, String)> {
        // Excluir palabras compuestas con guión (adjetivos como "ruso-colombiano")
        // Estos no son verbos y no deben tratarse como formas verbales
        if verb.contains('-') {
            return None;
        }

        // Excluir preposiciones y otras palabras que no son verbos pero podrían parecer formas verbales
        let non_verbs = [
            // Adverbios cortos que terminan en -o/-a/-e
            "no",
            "ya",
            "nunca",
            "ahora",
            "luego",
            "antes",
            "después",
            // Artículos y pronombres átonos
            "el",
            "la",
            "los",
            "las",
            "lo",
            "un",
            "una",
            "unos",
            "unas",
            "me",
            "te",
            "se",
            "nos",
            "os",
            "le",
            "les",
            // Participios de presente usados como sustantivos/adjetivos (-ante, -ente)
            "pensante",
            "amante",
            "estudiante",
            "cantante",
            "brillante",
            "importante",
            "constante",
            "instante",
            "distante",
            "elegante",
            "gigante",
            "ambulante",
            "abundante",
            "dominante",
            "fascinante",
            "ente",
            "paciente",
            "pendiente",
            "presente",
            "ausente",
            "consciente",
            "inconsciente",
            "evidente",
            "diferente",
            "excelente",
            // Preposiciones
            "de",
            "a",
            "en",
            "con",
            "por",
            "para",
            "sin",
            "sobre",
            "ante",
            "entre",
            "desde",
            "durante",
            "mediante",
            "según",
            "contra",
            "hacia",
            "hasta",
            "mediante",
            "tras",
            // Conjunciones y relativos
            "que",
            "porque",
            "aunque",
            "mientras",
            "donde",
            "como",
            "cuando",
            "sino",
            "pero",
            "mas",
            "pues",
            "luego",
            // Determinante posesivo relativo (concuerda con lo poseído, no con antecedente)
            "cuyo",
            "cuya",
            "cuyos",
            "cuyas",
            // Demostrativos y adjetivos que terminan en -o/-a
            "este",
            "ese",
            "aquel",
            "grande",
            "mismo",
            "misma",
            "mismos",
            "mismas",
            "otro",
            "otra",
            "otros",
            "otras",
            "poco",
            "poca",
            "pocos",
            "pocas",
            "mucho",
            "mucha",
            "muchos",
            "muchas",
            "tanto",
            "tanta",
            "tantos",
            "tantas",
            "cuanto",
            "cuanta",
            "cuantos",
            "cuantas",
            "todo",
            "toda",
            "todos",
            "todas",
            "alguno",
            "alguna",
            "algunos",
            "algunas",
            "ninguno",
            "ninguna",
            "ningunos",
            "ningunas",
            "cierto",
            "cierta",
            "ciertos",
            "ciertas",
            "propio",
            "propia",
            "propios",
            "propias",
            "solo",
            "sola",
            "solos",
            "solas",
            "medio",
            "media",
            "medios",
            "medias",
            "doble",
            "triple",
            // Subjuntivo imperfecto - formas comunes que se confunden con verbos -ar
            // ser/ir
            "fuera",
            "fueras",
            "fuéramos",
            "fuerais",
            "fueran",
            "fuese",
            "fueses",
            "fuésemos",
            "fueseis",
            "fuesen",
            // tener
            "tuviera",
            "tuvieras",
            "tuviéramos",
            "tuvierais",
            "tuvieran",
            "tuviese",
            "tuvieses",
            "tuviésemos",
            "tuvieseis",
            "tuviesen",
            // estar
            "estuviera",
            "estuvieras",
            "estuviéramos",
            "estuvierais",
            "estuvieran",
            "estuviese",
            "estuvieses",
            "estuviésemos",
            "estuvieseis",
            "estuviesen",
            // hacer
            "hiciera",
            "hicieras",
            "hiciéramos",
            "hicierais",
            "hicieran",
            "hiciese",
            "hicieses",
            "hiciésemos",
            "hicieseis",
            "hiciesen",
            // poder
            "pudiera",
            "pudieras",
            "pudiéramos",
            "pudierais",
            "pudieran",
            "pudiese",
            "pudieses",
            "pudiésemos",
            "pudieseis",
            "pudiesen",
            // poner
            "pusiera",
            "pusieras",
            "pusiéramos",
            "pusierais",
            "pusieran",
            "pusiese",
            "pusieses",
            "pusiésemos",
            "pusieseis",
            "pusiesen",
            // saber
            "supiera",
            "supieras",
            "supiéramos",
            "supierais",
            "supieran",
            "supiese",
            "supieses",
            "supiésemos",
            "supieseis",
            "supiesen",
            // querer
            "quisiera",
            "quisieras",
            "quisiéramos",
            "quisierais",
            "quisieran",
            "quisiese",
            "quisieses",
            "quisiésemos",
            "quisieseis",
            "quisiesen",
            // venir
            "viniera",
            "vinieras",
            "viniéramos",
            "vinierais",
            "vinieran",
            "viniese",
            "vinieses",
            "viniésemos",
            "vinieseis",
            "viniesen",
            // decir
            "dijera",
            "dijeras",
            "dijéramos",
            "dijerais",
            "dijeran",
            "dijese",
            "dijeses",
            "dijésemos",
            "dijeseis",
            "dijesen",
            // Imperfecto de ser (se confunde con verbos -ar)
            "era",
            "eras",
            "éramos",
            "erais",
            "eran",
            // Imperfecto de ir
            "iba",
            "ibas",
            "íbamos",
            "ibais",
            "iban",
            // Participios irregulares (no terminan en -ado/-ido)
            // Estos NO son formas conjugadas y no deben corregirse por concordancia sujeto-verbo
            "visto",
            "vista",
            "vistos",
            "vistas", // ver
            "hecho",
            "hecha",
            "hechos",
            "hechas", // hacer
            "dicho",
            "dicha",
            "dichos",
            "dichas", // decir
            "puesto",
            "puesta",
            "puestos",
            "puestas", // poner
            "escrito",
            "escrita",
            "escritos",
            "escritas", // escribir
            "abierto",
            "abierta",
            "abiertos",
            "abiertas", // abrir
            "vuelto",
            "vuelta",
            "vueltos",
            "vueltas", // volver
            "roto",
            "rota",
            "rotos",
            "rotas", // romper
            "muerto",
            "muerta",
            "muertos",
            "muertas", // morir
            "cubierto",
            "cubierta",
            "cubiertos",
            "cubiertas", // cubrir
            "frito",
            "frita",
            "fritos",
            "fritas", // freír
            "impreso",
            "impresa",
            "impresos",
            "impresas", // imprimir
            "preso",
            "presa",
            "presos",
            "presas", // prender
            "provisto",
            "provista",
            "provistos",
            "provistas", // proveer
            "satisfecho",
            "satisfecha",
            "satisfechos",
            "satisfechas", // satisfacer
            "deshecho",
            "deshecha",
            "deshechos",
            "deshechas", // deshacer
            "devuelto",
            "devuelta",
            "devueltos",
            "devueltas", // devolver
            "resuelto",
            "resuelta",
            "resueltos",
            "resueltas", // resolver
            "revuelto",
            "revuelta",
            "revueltos",
            "revueltas", // revolver
            "absuelto",
            "absuelta",
            "absueltos",
            "absueltas", // absolver
            "disuelto",
            "disuelta",
            "disueltos",
            "disueltas", // disolver
            "envuelto",
            "envuelta",
            "envueltos",
            "envueltas", // envolver
            "compuesto",
            "compuesta",
            "compuestos",
            "compuestas", // componer
            "dispuesto",
            "dispuesta",
            "dispuestos",
            "dispuestas", // disponer
            "expuesto",
            "expuesta",
            "expuestos",
            "expuestas", // exponer
            "impuesto",
            "impuesta",
            "impuestos",
            "impuestas", // imponer
            "opuesto",
            "opuesta",
            "opuestos",
            "opuestas", // oponer
            "propuesto",
            "propuesta",
            "propuestos",
            "propuestas", // proponer
            "repuesto",
            "repuesta",
            "repuestos",
            "repuestas", // reponer
            "supuesto",
            "supuesta",
            "supuestos",
            "supuestas", // suponer
            "antepuesto",
            "antepuesta",
            "antepuestos",
            "antepuestas", // anteponer
            "pospuesto",
            "pospuesta",
            "pospuestos",
            "pospuestas", // posponer
            "contrapuesto",
            "contrapuesta",
            "contrapuestos",
            "contrapuestas", // contraponer
            "interpuesto",
            "interpuesta",
            "interpuestos",
            "interpuestas", // interponer
            "yuxtapuesto",
            "yuxtapuesta",
            "yuxtapuestos",
            "yuxtapuestas", // yuxtaponer
            "inscrito",
            "inscrita",
            "inscritos",
            "inscritas", // inscribir
            "descrito",
            "descrita",
            "descritos",
            "descritas", // describir
            "prescrito",
            "prescrita",
            "prescritos",
            "prescritas", // prescribir
            "proscrito",
            "proscrita",
            "proscritos",
            "proscritas", // proscribir
            "transcrito",
            "transcrita",
            "transcritos",
            "transcritas", // transcribir
            "suscrito",
            "suscrita",
            "suscritos",
            "suscritas", // suscribir
            "circunscrito",
            "circunscrita",
            "circunscritos",
            "circunscritas", // circunscribir
            "adscrito",
            "adscrita",
            "adscritos",
            "adscritas", // adscribir
            "manuscrito",
            "manuscrita",
            "manuscritos",
            "manuscritas", // manuscribir
            "entreabierto",
            "entreabierta",
            "entreabiertos",
            "entreabiertas", // entreabrir
            "encubierto",
            "encubierta",
            "encubiertos",
            "encubiertas", // encubrir
            "descubierto",
            "descubierta",
            "descubiertos",
            "descubiertas", // descubrir
            "recubierto",
            "recubierta",
            "recubiertos",
            "recubiertas", // recubrir
            "contradicho",
            "contradicha",
            "contradichos",
            "contradichas", // contradecir
            "predicho",
            "predicha",
            "predichos",
            "predichas", // predecir
            "bendito",
            "bendita",
            "benditos",
            "benditas", // bendecir (doble participio)
            "maldito",
            "maldita",
            "malditos",
            "malditas", // maldecir (doble participio)
            "rehecho",
            "rehecha",
            "rehechos",
            "rehechas", // rehacer
            "previsto",
            "prevista",
            "previstos",
            "previstas", // prever
            "revisto",
            "revista",
            "revistos",
            "revistas", // rever (rare)
        ];
        if non_verbs.contains(&verb) {
            return None;
        }

        // Excluir gerundios (formas invariables)
        if let Some(vr) = verb_recognizer {
            if vr.is_gerund(verb) {
                return None;
            }
        } else if EncliticsAnalyzer::is_gerund(verb) {
            return None;
        }

        // Excluir participios usados como adjetivos.
        //
        // Importante: no basta con mirar el sufijo (-ado/-ido/-ada/-ida) porque hay formas
        // finitas que coinciden ("pido", "cuida", "olvida", "nada", etc.). Para evitar
        // falsos negativos, solo excluimos:
        // - Plurales participiales (no pueden ser formas finitas)
        // - Singular solo si el recognizer confirma que es participio del infinitivo
        //
        // Ej: "ellas unidas" - "unidas" es participio/adjetivo, no verbo conjugado.
        if verb.ends_with("adas")
            || verb.ends_with("idas")
            || verb.ends_with("ados")
            || verb.ends_with("idos")
        {
            return None;
        }
        if verb.ends_with("ado")
            || verb.ends_with("ido")
            || verb.ends_with("ada")
            || verb.ends_with("ida")
        {
            if let Some(vr) = verb_recognizer {
                if let Some(mut inf) = vr.get_infinitive(verb) {
                    if let Some(base) = inf.strip_suffix("se") {
                        inf = base.to_string();
                    }
                    let (participle_masc, participle_fem) = if inf.ends_with("ar") {
                        (
                            format!("{}ado", &inf[..inf.len() - 2]),
                            format!("{}ada", &inf[..inf.len() - 2]),
                        )
                    } else if inf.ends_with("er") || inf.ends_with("ir") {
                        (
                            format!("{}ido", &inf[..inf.len() - 2]),
                            format!("{}ida", &inf[..inf.len() - 2]),
                        )
                    } else {
                        (String::new(), String::new())
                    };
                    if participle_masc == verb || participle_fem == verb {
                        return None;
                    }
                }
            } else {
                // Sin recognizer, ser conservador para evitar falsos positivos.
                return None;
            }
        }

        // Auxiliar "haber" en tiempos compuestos
        match verb {
            // Presente
            "he" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "haber".to_string(),
                ))
            }
            "has" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "haber".to_string(),
                ))
            }
            "ha" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "haber".to_string(),
                ))
            }
            "hemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "haber".to_string(),
                ))
            }
            "habeis" | "habéis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "haber".to_string(),
                ))
            }
            "han" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "haber".to_string(),
                ))
            }
            // Pretérito
            "hube" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "haber".to_string(),
                ))
            }
            "hubiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "haber".to_string(),
                ))
            }
            "hubo" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "haber".to_string(),
                ))
            }
            "hubimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "haber".to_string(),
                ))
            }
            "hubisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "haber".to_string(),
                ))
            }
            "hubieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "haber".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - ser
        match verb {
            // Presente
            "soy" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ser".to_string(),
                ))
            }
            "eres" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ser".to_string(),
                ))
            }
            "es" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ser".to_string(),
                ))
            }
            "somos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ser".to_string(),
                ))
            }
            "sois" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ser".to_string(),
                ))
            }
            "son" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ser".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - estar
        match verb {
            // Presente
            "estoy" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "estar".to_string(),
                ))
            }
            "estás" | "estas" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "estar".to_string(),
                ))
            }
            "está" | "esta" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "estar".to_string(),
                ))
            }
            "estamos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "estar".to_string(),
                ))
            }
            "estáis" | "estais" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "estar".to_string(),
                ))
            }
            "están" | "estan" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "estar".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - tener
        match verb {
            // Presente
            "tengo" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "tener".to_string(),
                ))
            }
            "tienes" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "tener".to_string(),
                ))
            }
            "tiene" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "tener".to_string(),
                ))
            }
            "tenemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "tener".to_string(),
                ))
            }
            "tenéis" | "teneis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "tener".to_string(),
                ))
            }
            "tienen" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "tener".to_string(),
                ))
            }
            // Pretérito
            "tuve" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "tener".to_string(),
                ))
            }
            "tuviste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "tener".to_string(),
                ))
            }
            "tuvo" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "tener".to_string(),
                ))
            }
            "tuvimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "tener".to_string(),
                ))
            }
            "tuvisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "tener".to_string(),
                ))
            }
            "tuvieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "tener".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - ir
        // NOTA: Las formas del pretérito (fui, fue, fueron) son compartidas con "ser"
        match verb {
            // Presente
            "voy" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ir".to_string(),
                ))
            }
            "vas" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ir".to_string(),
                ))
            }
            "va" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ir".to_string(),
                ))
            }
            "vamos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ir".to_string(),
                ))
            }
            "vais" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ir".to_string(),
                ))
            }
            "van" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ir".to_string(),
                ))
            }
            // Pretérito (compartido con "ser")
            "fui" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "ir".to_string(),
                ))
            }
            "fuiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "ir".to_string(),
                ))
            }
            "fue" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "ir".to_string(),
                ))
            }
            "fuimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "ir".to_string(),
                ))
            }
            "fuisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "ir".to_string(),
                ))
            }
            "fueron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "ir".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - hacer
        match verb {
            // Presente
            "hago" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "hacer".to_string(),
                ))
            }
            "haces" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "hacer".to_string(),
                ))
            }
            "hace" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "hacer".to_string(),
                ))
            }
            "hacemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "hacer".to_string(),
                ))
            }
            "hacéis" | "haceis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "hacer".to_string(),
                ))
            }
            "hacen" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "hacer".to_string(),
                ))
            }
            // Pretérito
            "hice" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "hacer".to_string(),
                ))
            }
            "hiciste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "hacer".to_string(),
                ))
            }
            "hizo" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "hacer".to_string(),
                ))
            }
            "hicimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "hacer".to_string(),
                ))
            }
            "hicisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "hacer".to_string(),
                ))
            }
            "hicieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "hacer".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - poder
        match verb {
            // Presente
            "puedo" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "poder".to_string(),
                ))
            }
            "puedes" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "poder".to_string(),
                ))
            }
            "puede" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "poder".to_string(),
                ))
            }
            "podemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "poder".to_string(),
                ))
            }
            "podéis" | "podeis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "poder".to_string(),
                ))
            }
            "pueden" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "poder".to_string(),
                ))
            }
            // Pretérito
            "pude" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "poder".to_string(),
                ))
            }
            "pudiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "poder".to_string(),
                ))
            }
            "pudo" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "poder".to_string(),
                ))
            }
            "pudimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "poder".to_string(),
                ))
            }
            "pudisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "poder".to_string(),
                ))
            }
            "pudieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "poder".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - querer
        match verb {
            // Presente
            "quiero" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "querer".to_string(),
                ))
            }
            "quieres" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "querer".to_string(),
                ))
            }
            "quiere" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "querer".to_string(),
                ))
            }
            "queremos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "querer".to_string(),
                ))
            }
            "queréis" | "quereis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "querer".to_string(),
                ))
            }
            "quieren" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "querer".to_string(),
                ))
            }
            // Pretérito
            "quise" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "querer".to_string(),
                ))
            }
            "quisiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "querer".to_string(),
                ))
            }
            "quiso" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "querer".to_string(),
                ))
            }
            "quisimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "querer".to_string(),
                ))
            }
            "quisisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "querer".to_string(),
                ))
            }
            "quisieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "querer".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - decir
        match verb {
            // Presente
            "digo" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "decir".to_string(),
                ))
            }
            "dices" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "decir".to_string(),
                ))
            }
            "dice" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "decir".to_string(),
                ))
            }
            "decimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "decir".to_string(),
                ))
            }
            "decís" | "decis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "decir".to_string(),
                ))
            }
            "dicen" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "decir".to_string(),
                ))
            }
            // Pretérito
            "dije" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "decir".to_string(),
                ))
            }
            "dijiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "decir".to_string(),
                ))
            }
            "dijo" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "decir".to_string(),
                ))
            }
            "dijimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "decir".to_string(),
                ))
            }
            "dijisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "decir".to_string(),
                ))
            }
            "dijeron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "decir".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - saber
        // NOTA: Solo "sé" con tilde es verbo; "se" sin tilde es pronombre reflexivo
        match verb {
            // Presente
            "sé" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "saber".to_string(),
                ))
            }
            "sabes" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "saber".to_string(),
                ))
            }
            "sabe" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "saber".to_string(),
                ))
            }
            "sabemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "saber".to_string(),
                ))
            }
            "sabéis" | "sabeis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "saber".to_string(),
                ))
            }
            "saben" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "saber".to_string(),
                ))
            }
            // Pretérito
            "supe" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "saber".to_string(),
                ))
            }
            "supiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "saber".to_string(),
                ))
            }
            "supo" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "saber".to_string(),
                ))
            }
            "supimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "saber".to_string(),
                ))
            }
            "supisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "saber".to_string(),
                ))
            }
            "supieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "saber".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - venir
        match verb {
            // Presente
            "vengo" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "venir".to_string(),
                ))
            }
            "vienes" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "venir".to_string(),
                ))
            }
            "viene" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "venir".to_string(),
                ))
            }
            "venimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "venir".to_string(),
                ))
            }
            "venís" | "venis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "venir".to_string(),
                ))
            }
            "vienen" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "venir".to_string(),
                ))
            }
            // Pretérito
            "vine" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "venir".to_string(),
                ))
            }
            "viniste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "venir".to_string(),
                ))
            }
            "vino" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "venir".to_string(),
                ))
            }
            "vinimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "venir".to_string(),
                ))
            }
            "vinisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "venir".to_string(),
                ))
            }
            "vinieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "venir".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - dar
        match verb {
            // Presente
            "doy" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "dar".to_string(),
                ))
            }
            "das" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "dar".to_string(),
                ))
            }
            "da" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "dar".to_string(),
                ))
            }
            "damos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "dar".to_string(),
                ))
            }
            "dais" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "dar".to_string(),
                ))
            }
            "dan" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "dar".to_string(),
                ))
            }
            // Pretérito
            "di" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "dar".to_string(),
                ))
            }
            "diste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "dar".to_string(),
                ))
            }
            "dio" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "dar".to_string(),
                ))
            }
            "dimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "dar".to_string(),
                ))
            }
            "disteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "dar".to_string(),
                ))
            }
            "dieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "dar".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - ver
        match verb {
            // Presente
            "veo" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ver".to_string(),
                ))
            }
            "ves" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ver".to_string(),
                ))
            }
            "ve" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "ver".to_string(),
                ))
            }
            "vemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ver".to_string(),
                ))
            }
            "veis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ver".to_string(),
                ))
            }
            "ven" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "ver".to_string(),
                ))
            }
            // Pretérito
            "vi" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "ver".to_string(),
                ))
            }
            "viste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "ver".to_string(),
                ))
            }
            "vio" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "ver".to_string(),
                ))
            }
            "vimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "ver".to_string(),
                ))
            }
            "visteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "ver".to_string(),
                ))
            }
            "vieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "ver".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - poner
        match verb {
            // Presente
            "pongo" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "poner".to_string(),
                ))
            }
            "pones" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "poner".to_string(),
                ))
            }
            "pone" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "poner".to_string(),
                ))
            }
            "ponemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "poner".to_string(),
                ))
            }
            "ponéis" | "poneis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "poner".to_string(),
                ))
            }
            "ponen" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "poner".to_string(),
                ))
            }
            // Pretérito
            "puse" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "poner".to_string(),
                ))
            }
            "pusiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "poner".to_string(),
                ))
            }
            "puso" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "poner".to_string(),
                ))
            }
            "pusimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "poner".to_string(),
                ))
            }
            "pusisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "poner".to_string(),
                ))
            }
            "pusieron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "poner".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares comunes - traer
        match verb {
            // Presente
            "traigo" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "traer".to_string(),
                ))
            }
            "traes" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "traer".to_string(),
                ))
            }
            "trae" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    "traer".to_string(),
                ))
            }
            "traemos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "traer".to_string(),
                ))
            }
            "traéis" | "traeis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "traer".to_string(),
                ))
            }
            "traen" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    "traer".to_string(),
                ))
            }
            // Pretérito
            "traje" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "traer".to_string(),
                ))
            }
            "trajiste" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "traer".to_string(),
                ))
            }
            "trajo" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    "traer".to_string(),
                ))
            }
            "trajimos" => {
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "traer".to_string(),
                ))
            }
            "trajisteis" => {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "traer".to_string(),
                ))
            }
            "trajeron" => {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    "traer".to_string(),
                ))
            }
            _ => {}
        }

        // Verbos irregulares con prefijo (deshacer/rehacer, imponer/componer, predecir, etc.)
        // Debe evaluarse antes de las heurísticas regulares por sufijo (p. ej. "-o" presente),
        // para no confundir pretéritos como "rehizo/deshizo/predijo/impuso".
        if let Some(vr) = verb_recognizer {
            if let Some(mut inf) = vr.get_infinitive(verb) {
                if let Some(base) = inf.strip_suffix("se") {
                    inf = base.to_string();
                }
                if let Some(info) = Self::match_prefixed_irregular_form(verb, &inf) {
                    return Some(info);
                }
            }
            if let Some(info) = Self::match_prefixed_irregular_form_from_surface(verb, vr) {
                return Some(info);
            }
        }

        let get_infinitive_for = |allowed_endings: &[&str]| -> Option<String> {
            if let Some(vr) = verb_recognizer {
                let normalized = Self::normalize_spanish(verb);
                let forms = if normalized != verb {
                    vec![verb.to_string(), normalized]
                } else {
                    vec![verb.to_string()]
                };

                for form in forms {
                    if let Some(mut inf) = vr.get_infinitive(&form) {
                        if let Some(base) = inf.strip_suffix("se") {
                            inf = base.to_string();
                        }
                        if allowed_endings.is_empty()
                            || allowed_endings.iter().any(|ending| inf.ends_with(ending))
                        {
                            return Some(inf);
                        }
                    }
                }
            }
            None
        };

        // Verbos regulares -ar (presente indicativo)
        if let Some(stem) = verb.strip_suffix("o") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ar", "er", "ir"]) {
                        return Some((
                            GrammaticalPerson::First,
                            GrammaticalNumber::Singular,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("as") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ar"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Singular,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("a") {
            if !stem.is_empty() && !verb.ends_with("ía") {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ar"]) {
                        return Some((
                            GrammaticalPerson::Third,
                            GrammaticalNumber::Singular,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("amos") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ar"]) {
                        return Some((
                            GrammaticalPerson::First,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("áis") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ar"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("ais") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ar"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("an") {
            if !stem.is_empty() && !verb.ends_with("ían") {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ar"]) {
                        return Some((
                            GrammaticalPerson::Third,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}ar", stem),
                ));
            }
        }

        // Verbos regulares -er (presente indicativo)
        // NOTA: Excluimos stems que terminan en "c" porque probablemente son
        // subjuntivo de verbos -zar (ej: "garantice" es subjuntivo de "garantizar",
        // no indicativo de hipotético "garanticer")
        if let Some(stem) = verb.strip_suffix("es") {
            if !stem.is_empty()
                && !verb.ends_with("as")
                && (verb_recognizer.is_some() || !stem.ends_with('c'))
            {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Singular,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("e") {
            if !stem.is_empty()
                && !verb.ends_with("a")
                && !verb.ends_with("ie")
                // Evitar confundir pretérito 2s (-iste) con presente 3s (-e)
                && !verb.ends_with("iste")
                && (verb_recognizer.is_some() || !stem.ends_with('c'))
            {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::Third,
                            GrammaticalNumber::Singular,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Present,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("emos") {
            if !stem.is_empty() && (verb_recognizer.is_some() || !stem.ends_with('c')) {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er"]) {
                        return Some((
                            GrammaticalPerson::First,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("éis") {
            if !stem.is_empty() && (verb_recognizer.is_some() || !stem.ends_with('c')) {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("eis") {
            if !stem.is_empty()
                // Evitar confundir pretérito 2p (-isteis) con presente 2p (-eis)
                && !verb.ends_with("isteis")
                && (verb_recognizer.is_some() || !stem.ends_with('c'))
            {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("en") {
            if !stem.is_empty()
                && !verb.ends_with("an")
                && !verb.ends_with("ien")
                && (verb_recognizer.is_some() || !stem.ends_with('c'))
            {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::Third,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}er", stem),
                ));
            }
        }

        // Verbos regulares -ir (presente indicativo)
        if let Some(stem) = verb.strip_suffix("imos") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ir"]) {
                        return Some((
                            GrammaticalPerson::First,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}ir", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("ís") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ir"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}ir", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("is") {
            if !stem.is_empty() && !verb.ends_with("ais") && !verb.ends_with("eis") {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["ir"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Plural,
                            VerbTense::Present,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Present,
                    format!("{}ir", stem),
                ));
            }
        }

        // ========== PRETÉRITO REGULAR ==========

        // Verbos regulares -ar (pretérito)
        // Nota: -ó y -é llevan tilde obligatoria
        if let Some(stem) = verb.strip_suffix("aron") {
            if !stem.is_empty() {
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("asteis") {
            if !stem.is_empty() {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("aste") {
            if !stem.is_empty() {
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    format!("{}ar", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("ó") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&[]) {
                        // Si el recognizer identifica un infinitivo -ar (incluye -iar),
                        // priorizarlo antes de la heurística por sufijo.
                        if inf.ends_with("ar") {
                            return Some((
                                GrammaticalPerson::Third,
                                GrammaticalNumber::Singular,
                                VerbTense::Preterite,
                                inf,
                            ));
                        }
                    } else if !stem.ends_with('i') {
                        // Fallback sin infinitivo: conservar heurística anterior.
                        return Some((
                            GrammaticalPerson::Third,
                            GrammaticalNumber::Singular,
                            VerbTense::Preterite,
                            format!("{}ar", stem),
                        ));
                    }
                } else if !stem.ends_with('i') {
                    // Sin recognizer, mantener el comportamiento histórico.
                    return Some((
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                        VerbTense::Preterite,
                        format!("{}ar", stem),
                    ));
                }
            }
        }
        if let Some(stem) = verb.strip_suffix("é") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&[]) {
                        // Misma lógica para 1ª singular pretérita.
                        if inf.ends_with("ar") {
                            return Some((
                                GrammaticalPerson::First,
                                GrammaticalNumber::Singular,
                                VerbTense::Preterite,
                                inf,
                            ));
                        }
                    } else if !stem.ends_with('i') {
                        return Some((
                            GrammaticalPerson::First,
                            GrammaticalNumber::Singular,
                            VerbTense::Preterite,
                            format!("{}ar", stem),
                        ));
                    }
                } else if !stem.ends_with('i') {
                    return Some((
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                        VerbTense::Preterite,
                        format!("{}ar", stem),
                    ));
                }
            }
        }

        // Verbos regulares -er/-ir (pretérito)
        // Comparten las mismas terminaciones
        if let Some(stem) = verb.strip_suffix("ieron") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::Third,
                            GrammaticalNumber::Plural,
                            VerbTense::Preterite,
                            inf,
                        ));
                    }
                    return None;
                }
                // Sin diccionario: difícil distinguir -er/-ir. Por defecto asumimos -er.
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("isteis") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Plural,
                            VerbTense::Preterite,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Plural,
                    VerbTense::Preterite,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("iste") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::Second,
                            GrammaticalNumber::Singular,
                            VerbTense::Preterite,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Second,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("ió") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::Third,
                            GrammaticalNumber::Singular,
                            VerbTense::Preterite,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::Third,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    format!("{}er", stem),
                ));
            }
        }
        if let Some(stem) = verb.strip_suffix("í") {
            if !stem.is_empty() {
                if verb_recognizer.is_some() {
                    if let Some(inf) = get_infinitive_for(&["er", "ir"]) {
                        return Some((
                            GrammaticalPerson::First,
                            GrammaticalNumber::Singular,
                            VerbTense::Preterite,
                            inf,
                        ));
                    }
                    return None;
                }
                return Some((
                    GrammaticalPerson::First,
                    GrammaticalNumber::Singular,
                    VerbTense::Preterite,
                    format!("{}er", stem),
                ));
            }
        }

        None
    }

    fn check_present_subjunctive_agreement(
        verb_index: usize,
        verb_original: &str,
        verb_lower: &str,
        subject: &SubjectInfo,
        verb_recognizer: &dyn VerbFormRecognizer,
    ) -> Option<SubjectVerbCorrection> {
        let mut infinitive = verb_recognizer.get_infinitive(verb_lower)?;
        if let Some(base) = infinitive.strip_suffix("se") {
            infinitive = base.to_string();
        }

        let class_ending = if infinitive.ends_with("ar") {
            "ar"
        } else if infinitive.ends_with("er") || infinitive.ends_with("ir") {
            &infinitive[infinitive.len() - 2..]
        } else {
            return None;
        };

        let target_ending = if class_ending == "ar" {
            match (subject.person, subject.number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular)
                | (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "e",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "es",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "emos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "éis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "en",
            }
        } else {
            match (subject.person, subject.number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular)
                | (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "a",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "as",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "amos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "áis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "an",
            }
        };

        let observed_endings: &[&str] = if class_ending == "ar" {
            &["e", "es", "emos", "éis", "eis", "en"]
        } else {
            &["a", "as", "amos", "áis", "ais", "an"]
        };

        for observed_ending in observed_endings.iter().copied() {
            if let Some(stem) = verb_lower.strip_suffix(observed_ending) {
                if stem.is_empty() {
                    continue;
                }

                let target_form = format!("{stem}{target_ending}");
                if target_form == verb_lower {
                    return None;
                }

                if let Some(mut target_infinitive) = verb_recognizer.get_infinitive(&target_form) {
                    if let Some(base) = target_infinitive.strip_suffix("se") {
                        target_infinitive = base.to_string();
                    }
                    if target_infinitive == infinitive {
                        return Some(SubjectVerbCorrection {
                            token_index: verb_index,
                            original: verb_original.to_string(),
                            suggestion: target_form,
                            message: format!(
                                "Concordancia sujeto-verbo: '{}' debería ser '{}'",
                                verb_original,
                                stem.to_string() + target_ending
                            ),
                        });
                    }
                } else {
                    // Fallback conservador: para formas claramente regulares en subjuntivo,
                    // aceptar reconstrucción por stem+terminación cuando el infinitivo base
                    // coincide con la clase verbal.
                    let fallback_infinitive = format!("{stem}{class_ending}");
                    if fallback_infinitive == infinitive {
                        return Some(SubjectVerbCorrection {
                            token_index: verb_index,
                            original: verb_original.to_string(),
                            suggestion: target_form,
                            message: format!(
                                "Concordancia sujeto-verbo: '{}' debería ser '{}'",
                                verb_original,
                                stem.to_string() + target_ending
                            ),
                        });
                    }
                }
            }
        }

        None
    }

    fn get_prefixed_irregular_family(infinitive: &str) -> Option<(&str, &'static str)> {
        for base in Self::PREFIXABLE_IRREGULAR_BASES {
            if let Some(prefix) = infinitive.strip_suffix(base) {
                if !prefix.is_empty() {
                    return Some((prefix, base));
                }
            }
        }
        None
    }

    fn build_prefixed_irregular_form(
        prefix: &str,
        base: &str,
        person: GrammaticalPerson,
        number: GrammaticalNumber,
        tense: VerbTense,
    ) -> Option<String> {
        let base_form = Self::get_correct_form(base, person, number, tense)?;
        Some(format!("{prefix}{base_form}"))
    }

    fn extract_prefixed_surface_prefix(verb: &str, base_form: &str) -> Option<String> {
        if let Some(prefix) = verb.strip_suffix(base_form) {
            if !prefix.is_empty() {
                return Some(prefix.to_string());
            }
        }

        let base_len = base_form.chars().count();
        let verb_chars: Vec<char> = verb.chars().collect();
        if verb_chars.len() <= base_len {
            return None;
        }

        let split = verb_chars.len() - base_len;
        let observed_suffix: String = verb_chars[split..].iter().collect();
        if !Self::same_form_with_optional_accents(&observed_suffix, base_form) {
            return None;
        }

        let prefix: String = verb_chars[..split].iter().collect();
        if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        }
    }

    fn match_prefixed_irregular_form_from_surface(
        verb: &str,
        verb_recognizer: &dyn VerbFormRecognizer,
    ) -> Option<(GrammaticalPerson, GrammaticalNumber, VerbTense, String)> {
        let slots = [
            (GrammaticalPerson::First, GrammaticalNumber::Singular),
            (GrammaticalPerson::Second, GrammaticalNumber::Singular),
            (GrammaticalPerson::Third, GrammaticalNumber::Singular),
            (GrammaticalPerson::First, GrammaticalNumber::Plural),
            (GrammaticalPerson::Second, GrammaticalNumber::Plural),
            (GrammaticalPerson::Third, GrammaticalNumber::Plural),
        ];

        for base in Self::PREFIXABLE_IRREGULAR_BASES {
            for tense in [VerbTense::Present, VerbTense::Preterite] {
                for (person, number) in slots {
                    let Some(base_form) = Self::get_correct_form(base, person, number, tense)
                    else {
                        continue;
                    };
                    let Some(prefix) = Self::extract_prefixed_surface_prefix(verb, &base_form)
                    else {
                        continue;
                    };
                    let infinitive = format!("{prefix}{base}");
                    if verb_recognizer.knows_infinitive(&infinitive) {
                        return Some((person, number, tense, infinitive));
                    }
                    // Si el verbo de superficie coincide claramente con una forma irregular
                    // prefijada conocida (prefijo + base irregular), preservar ese prefijo
                    // aunque el infinitivo no figure en el diccionario cargado.
                    // Esto evita correcciones semánticamente peligrosas como:
                    // "sobreponen" -> "pone" (pérdida de prefijo).
                    if let Some((known_prefix, _)) = PrefixAnalyzer::strip_prefix(verb) {
                        if known_prefix == prefix {
                            return Some((person, number, tense, infinitive));
                        }
                    }
                }
            }
        }

        None
    }

    fn same_form_with_optional_accents(observed: &str, expected: &str) -> bool {
        observed == expected
            || Self::normalize_spanish(observed) == Self::normalize_spanish(expected)
    }

    fn match_prefixed_irregular_form(
        verb: &str,
        infinitive: &str,
    ) -> Option<(GrammaticalPerson, GrammaticalNumber, VerbTense, String)> {
        let (prefix, base) = Self::get_prefixed_irregular_family(infinitive)?;
        for tense in [VerbTense::Present, VerbTense::Preterite] {
            let slots = [
                (GrammaticalPerson::First, GrammaticalNumber::Singular),
                (GrammaticalPerson::Second, GrammaticalNumber::Singular),
                (GrammaticalPerson::Third, GrammaticalNumber::Singular),
                (GrammaticalPerson::First, GrammaticalNumber::Plural),
                (GrammaticalPerson::Second, GrammaticalNumber::Plural),
                (GrammaticalPerson::Third, GrammaticalNumber::Plural),
            ];
            for (person, number) in slots {
                let Some(candidate) =
                    Self::build_prefixed_irregular_form(prefix, base, person, number, tense)
                else {
                    continue;
                };
                if Self::same_form_with_optional_accents(verb, &candidate) {
                    return Some((person, number, tense, infinitive.to_string()));
                }
            }
        }
        None
    }

    /// Obtiene la forma correcta del verbo para la persona, número y tiempo dados
    fn get_correct_form(
        infinitive: &str,
        person: GrammaticalPerson,
        number: GrammaticalNumber,
        tense: VerbTense,
    ) -> Option<String> {
        if let Some((prefix, base)) = Self::get_prefixed_irregular_family(infinitive) {
            if let Some(form) =
                Self::build_prefixed_irregular_form(prefix, base, person, number, tense)
            {
                return Some(form);
            }
        }

        // Verbos irregulares - ser
        if infinitive == "ser" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "soy"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "eres",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "es"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "somos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "sois"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "son"
                    }
                    // Pretérito (compartido con ir)
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "fui",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "fuiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "fue",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "fuimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "fuisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "fueron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - estar
        if infinitive == "estar" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "estoy"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "estás",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "está"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "estamos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "estáis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "están"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "estuve",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "estuviste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "estuvo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "estuvimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "estuvisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "estuvieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - tener
        if infinitive == "tener" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "tengo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "tienes",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "tiene"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "tenemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "tenéis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "tienen"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "tuve",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "tuviste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "tuvo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "tuvimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "tuvisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "tuvieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - ir
        if infinitive == "ir" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "voy"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "vas",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "va"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "vamos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "vais"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "van"
                    }
                    // Pretérito (compartido con ser)
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "fui",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "fuiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "fue",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "fuimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "fuisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "fueron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - hacer
        if infinitive == "hacer" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "hago"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "haces",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "hace"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "hacemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "hacéis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "hacen"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "hice",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "hiciste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "hizo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "hicimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "hicisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "hicieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbo irregular - haber (auxiliar de tiempos compuestos)
        if infinitive == "haber" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "he"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "has",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "ha"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "hemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "habéis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "han"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "hube",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "hubiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "hubo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "hubimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "hubisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "hubieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbo con hiato en presente: prohibir (prohíbo, prohíbes, prohíbe, prohíben)
        if Self::normalize_spanish(infinitive) == "prohibir" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "proh\u{00ED}bo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "proh\u{00ED}bes",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "proh\u{00ED}be"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "prohibimos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "prohib\u{00ED}s"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "proh\u{00ED}ben"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "prohib\u{00ED}",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "prohibiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "prohibi\u{00F3}",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "prohibimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "prohibisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "prohibieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - poder
        if infinitive == "poder" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "puedo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "puedes",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "puede"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "podemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "podéis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "pueden"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "pude",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "pudiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "pudo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "pudimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "pudisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "pudieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - querer
        if infinitive == "querer" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "quiero"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "quieres",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "quiere"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "queremos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "queréis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "quieren"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "quise",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "quisiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "quiso",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "quisimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "quisisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "quisieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - decir
        if infinitive == "decir" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "digo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "dices",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "dice"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "decimos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "decís"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "dicen"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "dije",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "dijiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "dijo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "dijimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "dijisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "dijeron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - saber
        if infinitive == "saber" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "sé"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "sabes",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "sabe"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "sabemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "sabéis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "saben"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "supe",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "supiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "supo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "supimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "supisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "supieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - venir
        if infinitive == "venir" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "vengo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "vienes",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "viene"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "venimos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "venís"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "vienen"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "vine",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "viniste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "vino",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "vinimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "vinisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "vinieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - dar
        if infinitive == "dar" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "doy"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "das",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "da"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "damos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "dais"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "dan"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "di",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "diste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "dio",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "dimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "disteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "dieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - ver
        if infinitive == "ver" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "veo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "ves",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "ve"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "vemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "veis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "ven"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "vi",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "viste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "vio",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "vimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "visteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "vieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - poner
        if infinitive == "poner" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "pongo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "pones",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "pone"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "ponemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "ponéis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "ponen"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "puse",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "pusiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "puso",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "pusimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "pusisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "pusieron"
                    }
                }
                .to_string(),
            );
        }

        // Verbos irregulares - traer
        if infinitive == "traer" {
            return Some(
                match (tense, person, number) {
                    // Presente
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        "traigo"
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "traes",
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        "trae"
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "traemos"
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        "traéis"
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "traen"
                    }
                    // Pretérito
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => "traje",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => "trajiste",
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => "trajo",
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        "trajimos"
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => "trajisteis",
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        "trajeron"
                    }
                }
                .to_string(),
            );
        }

        // Determinar si el verbo tiene cambio de raíz
        let stem_changes = get_stem_changing_verbs();
        let change_type = stem_changes.get(infinitive).copied();

        // En presente indicativo, el cambio de raíz aplica a: 1s, 2s, 3s, 3p
        let needs_stem_change = match change_type {
            Some(StemChangeType::CToZc) => {
                tense == VerbTense::Present
                    && person == GrammaticalPerson::First
                    && number == GrammaticalNumber::Singular
            }
            Some(_) => {
                tense == VerbTense::Present
                    && !(person == GrammaticalPerson::First && number == GrammaticalNumber::Plural)
                    && !(person == GrammaticalPerson::Second && number == GrammaticalNumber::Plural)
            }
            None => false,
        };

        // Verbos regulares -ar
        if let Some(stem) = infinitive.strip_suffix("ar") {
            let mut s = if needs_stem_change {
                Self::apply_stem_change(stem, change_type.unwrap())
            } else {
                stem.to_string()
            };
            if tense == VerbTense::Present
                && person == GrammaticalPerson::First
                && number == GrammaticalNumber::Singular
            {
                s = Self::apply_present_1s_orthography(infinitive, &s, change_type);
            }
            return Some(match (tense, person, number) {
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                    format!("{}o", s)
                }
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => {
                    format!("{}as", s)
                }
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                    format!("{}a", s)
                }
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                    format!("{}amos", s)
                }
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                    format!("{}áis", s)
                }
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                    format!("{}an", s)
                }
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                    format!("{}é", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => {
                    format!("{}aste", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                    format!("{}ó", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                    format!("{}amos", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                    format!("{}asteis", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                    format!("{}aron", stem)
                }
            });
        }

        // Verbos regulares -er
        if let Some(stem) = infinitive.strip_suffix("er") {
            let mut s = if needs_stem_change {
                Self::apply_stem_change(stem, change_type.unwrap())
            } else {
                stem.to_string()
            };
            if tense == VerbTense::Present
                && person == GrammaticalPerson::First
                && number == GrammaticalNumber::Singular
            {
                s = Self::apply_present_1s_orthography(infinitive, &s, change_type);
            }
            return Some(match (tense, person, number) {
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                    format!("{}o", s)
                }
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => {
                    format!("{}es", s)
                }
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                    format!("{}e", s)
                }
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                    format!("{}emos", s)
                }
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                    format!("{}éis", s)
                }
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                    format!("{}en", s)
                }
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                    format!("{}í", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => {
                    format!("{}iste", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                    format!("{}ió", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                    format!("{}imos", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                    format!("{}isteis", stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                    format!("{}ieron", stem)
                }
            });
        }

        // Verbos -uir (i→y), excepto -guir: incluir, concluir, confluir, huir...
        if infinitive.ends_with("uir") && !infinitive.ends_with("guir") {
            if let Some(stem) = infinitive.strip_suffix("ir") {
                return Some(match (tense, person, number) {
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                        format!("{}yo", stem)
                    }
                    (
                        VerbTense::Present,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => format!("{}yes", stem),
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                        format!("{}ye", stem)
                    }
                    (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        format!("{}imos", stem)
                    }
                    (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                        format!("{}ís", stem)
                    }
                    (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        format!("{}yen", stem)
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::First,
                        GrammaticalNumber::Singular,
                    ) => format!("{}í", stem),
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Singular,
                    ) => format!("{}iste", stem),
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Third,
                        GrammaticalNumber::Singular,
                    ) => format!("{}yó", stem),
                    (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                        format!("{}imos", stem)
                    }
                    (
                        VerbTense::Preterite,
                        GrammaticalPerson::Second,
                        GrammaticalNumber::Plural,
                    ) => format!("{}isteis", stem),
                    (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                        format!("{}yeron", stem)
                    }
                });
            }
        }

        // Verbos regulares -ir
        if let Some(stem) = infinitive.strip_suffix("ir") {
            let mut s = if needs_stem_change {
                Self::apply_stem_change(stem, change_type.unwrap())
            } else {
                stem.to_string()
            };
            if tense == VerbTense::Present
                && person == GrammaticalPerson::First
                && number == GrammaticalNumber::Singular
            {
                s = Self::apply_present_1s_orthography(infinitive, &s, change_type);
            }
            let preterite_stem =
                Self::apply_preterite_ir_stem_change(stem, tense, person, number, change_type);
            return Some(match (tense, person, number) {
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                    format!("{}o", s)
                }
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => {
                    format!("{}es", s)
                }
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                    format!("{}e", s)
                }
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                    format!("{}imos", s)
                }
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                    format!("{}ís", s)
                }
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                    format!("{}en", s)
                }
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => {
                    format!("{}í", preterite_stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => {
                    format!("{}iste", preterite_stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => {
                    format!("{}ió", preterite_stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => {
                    format!("{}imos", preterite_stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => {
                    format!("{}isteis", preterite_stem)
                }
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => {
                    format!("{}ieron", preterite_stem)
                }
            });
        }

        None
    }

    fn apply_preterite_ir_stem_change(
        stem: &str,
        tense: VerbTense,
        person: GrammaticalPerson,
        number: GrammaticalNumber,
        change_type: Option<StemChangeType>,
    ) -> String {
        if tense != VerbTense::Preterite {
            return stem.to_string();
        }

        // En pretérito, el cambio aplica solo a 3s y 3p en verbos -ir:
        // - e→i: pedir→pidió/pidieron, servir→sirvió/sirvieron
        // - e→ie (en presente) también usa e→i: sentir→sintió/sintieron
        // - o→ue (en presente) usa o→u: dormir→durmió/durmieron
        let is_third_person = person == GrammaticalPerson::Third;
        let is_singular_or_plural =
            number == GrammaticalNumber::Singular || number == GrammaticalNumber::Plural;
        if !is_third_person || !is_singular_or_plural {
            return stem.to_string();
        }

        match change_type {
            Some(StemChangeType::EToI) | Some(StemChangeType::EToIe) => {
                Self::replace_last_occurrence(stem, "e", "i")
            }
            Some(StemChangeType::OToUe) => Self::replace_last_occurrence(stem, "o", "u"),
            _ => stem.to_string(),
        }
    }

    fn replace_last_occurrence(stem: &str, from: &str, to: &str) -> String {
        if let Some(pos) = stem.rfind(from) {
            let mut result = String::with_capacity(stem.len() - from.len() + to.len());
            result.push_str(&stem[..pos]);
            result.push_str(to);
            result.push_str(&stem[pos + from.len()..]);
            result
        } else {
            stem.to_string()
        }
    }

    fn apply_present_1s_orthography(
        infinitive: &str,
        stem: &str,
        change_type: Option<StemChangeType>,
    ) -> String {
        let mut s = stem.to_string();

        // -guir: en 1s presente se elimina la 'u' (seguir→sigo, distinguir→distingo)
        if infinitive.ends_with("guir") && s.ends_with("gu") {
            s.pop();
        }

        // -ger/-gir: en 1s presente g→j (escoger→escojo, corregir→corrijo)
        if (infinitive.ends_with("ger") || infinitive.ends_with("gir"))
            && !infinitive.ends_with("guir")
            && s.ends_with('g')
        {
            s.pop();
            s.push('j');
        }

        // -cer/-cir: en 1s presente c→z (vencer→venzo, torcer→tuerzo)
        // Excluir c→zc (conocer→conozco, lucir→luzco, etc.)
        if (infinitive.ends_with("cer") || infinitive.ends_with("cir"))
            && change_type != Some(StemChangeType::CToZc)
            && s.ends_with('c')
            && !s.ends_with("zc")
        {
            s.pop();
            s.push('z');
        }

        s
    }

    /// Aplica el cambio de raíz a un stem (última ocurrencia de la vocal original)
    fn apply_stem_change(stem: &str, change_type: StemChangeType) -> String {
        let (original, changed) = change_type.change_pair();

        // c→zc: reemplazar última 'c' por 'zc'
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

        // Cambios vocálicos: reemplazar última ocurrencia de la vocal original
        if let Some(pos) = stem.rfind(original) {
            let mut result = String::with_capacity(stem.len() + changed.len());
            result.push_str(&stem[..pos]);
            result.push_str(changed);
            result.push_str(&stem[pos + original.len()..]);
            return result;
        }

        stem.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::{DictionaryLoader, Trie};
    use crate::grammar::tokenizer::Tokenizer;
    use crate::languages::spanish::VerbRecognizer;

    fn tokenize(text: &str) -> Vec<Token> {
        Tokenizer::new().tokenize(text)
    }

    #[test]
    fn test_yo_with_wrong_verb() {
        // "yo cantas" debería sugerir "canto"
        let tokens = tokenize("yo cantas");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canto");
    }

    #[test]
    fn test_yo_cree_corrected_in_main_clause() {
        let mut tokens = tokenize("Yo cree");
        let dict_path = std::path::Path::new("data/es/words.txt");
        if !dict_path.exists() {
            return;
        }
        let dictionary = DictionaryLoader::load_from_file(dict_path).unwrap();
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "creo");
    }

    #[test]
    fn test_no_correction_for_subjunctive_cree_after_que() {
        let mut tokens = tokenize("Espero que yo cree un modelo.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        if !dict_path.exists() {
            return;
        }
        let dictionary = DictionaryLoader::load_from_file(dict_path).unwrap();
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert!(
            corrections.is_empty(),
            "No debe corregir 'yo cree' tras 'que' (posible subjuntivo de 'crear')"
        );
    }

    #[test]
    fn test_tu_with_wrong_verb() {
        // "tú canto" debería sugerir "cantas"
        let tokens = tokenize("tú canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantas");
    }

    #[test]
    fn test_tu_mando_not_gerund() {
        // "tú mando" debería sugerir "mandas"
        // "mando" termina en -ando pero NO es gerundio; es 1ª persona de "mandar"
        let mut tokens = tokenize("tú mando");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert_eq!(
            corrections.len(),
            1,
            "Should detect mismatch: tú + mando (1st person)"
        );
        assert_eq!(corrections[0].suggestion, "mandas");
    }

    #[test]
    fn test_tu_temo_uses_recognizer_infinitive() {
        // "tú temo" debería sugerir "temes" (no "temas")
        let mut tokens = tokenize("tú temo");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "temes");
    }

    #[test]
    fn test_el_with_wrong_verb() {
        // "él cantamos" debería sugerir "canta"
        let tokens = tokenize("él cantamos");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canta");
    }

    #[test]
    fn test_nosotros_with_wrong_verb() {
        // "nosotros canta" debería sugerir "cantamos"
        let tokens = tokenize("nosotros canta");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantamos");
    }

    #[test]
    fn test_ellos_with_wrong_verb() {
        // "ellos canto" debería sugerir "cantan"
        let tokens = tokenize("ellos canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantan");
    }

    #[test]
    fn test_correct_agreement() {
        // "yo canto" es correcto, no debería haber correcciones
        let tokens = tokenize("yo canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_irregular_ser_yo() {
        // "yo eres" debería sugerir "soy"
        let tokens = tokenize("yo eres");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "soy");
    }

    #[test]
    fn test_irregular_ser_tu() {
        // "tú soy" debería sugerir "eres"
        let tokens = tokenize("tú soy");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "eres");
    }

    #[test]
    fn test_irregular_estar() {
        // "yo estás" debería sugerir "estoy"
        let tokens = tokenize("yo estás");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "estoy");
    }

    #[test]
    fn test_irregular_tener() {
        // "yo tienes" debería sugerir "tengo"
        let tokens = tokenize("yo tienes");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tengo");
    }

    #[test]
    fn test_irregular_ir() {
        // "yo vas" debería sugerir "voy"
        let tokens = tokenize("yo vas");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "voy");
    }

    #[test]
    fn test_irregular_hacer() {
        // "yo haces" debería sugerir "hago"
        let tokens = tokenize("yo haces");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hago");
    }

    #[test]
    fn test_correct_irregular() {
        // "yo soy" es correcto
        let tokens = tokenize("yo soy");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_er_verb() {
        // "yo comes" debería sugerir "como"
        let tokens = tokenize("yo comes");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "como");
    }

    #[test]
    fn test_ir_verb() {
        // "yo vives" debería sugerir "vivo"
        let tokens = tokenize("yo vives");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vivo");
    }

    #[test]
    fn test_vosotros() {
        // "vosotros canto" debería sugerir "cantáis"
        let tokens = tokenize("vosotros canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantáis");
    }

    #[test]
    fn test_ella_with_nosotros_form() {
        // "ella cantamos" debería sugerir "canta"
        let tokens = tokenize("ella cantamos");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canta");
    }

    #[test]
    fn test_multiple_errors_in_sentence() {
        // Una oración con múltiples errores potenciales
        // Solo detectamos el patrón pronombre + verbo inmediato
        let tokens = tokenize("yo cantas bien");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canto");
    }

    // =========================================================================
    // Tests de límites de oración - no debe cruzar puntuación
    // =========================================================================

    #[test]
    fn test_no_cross_period() {
        // "él. canto" - el punto separa oraciones, no debe emparejar "él" con "canto"
        let tokens = tokenize("él. canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar punto: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_question_mark() {
        // "¿él? canto" - el signo de interrogación separa
        let tokens = tokenize("¿él? canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar signo de interrogación: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_exclamation() {
        // "¡él! canto" - el signo de exclamación separa
        let tokens = tokenize("¡él! canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar signo de exclamación: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_semicolon() {
        // "él; canto" - el punto y coma separa
        let tokens = tokenize("él; canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar punto y coma: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_colon() {
        // "él: canto" - los dos puntos separan
        let tokens = tokenize("él: canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar dos puntos: {:?}",
            corrections
        );
    }

    #[test]
    fn test_still_detects_within_sentence() {
        // "Dijo que yo cantas mal" - error dentro de la misma oración
        let tokens = tokenize("Dijo que yo cantas mal");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canto");
    }

    #[test]
    fn test_comma_does_not_block() {
        // "yo, sinceramente, cantas" - las comas no bloquean (no son fin de oración)
        // pero tampoco debería detectar error porque "cantas" no es inmediato a "yo"
        // Nota: el analizador solo mira word tokens consecutivos
        let tokens = tokenize("yo cantas, bien");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(
            corrections.len(),
            1,
            "Coma después no debería afectar detección"
        );
    }

    #[test]
    fn test_multiple_sentences_independent() {
        // "Yo canto. Él cantas." - cada oración es independiente
        // Primera oración correcta, segunda tiene error
        let tokens = tokenize("Yo canto. Él cantas.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canta");
    }

    // Tests para pretéritos irregulares
    #[test]
    fn test_irregular_preterite_decir() {
        // "ellos dijo" → "ellos dijeron" (pretérito de decir)
        let tokens = tokenize("Ellos dijo la verdad.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "dijeron");
    }

    #[test]
    fn test_irregular_preterite_hacer() {
        // "él hicieron" → "él hizo" (pretérito de hacer)
        let tokens = tokenize("Él hicieron el trabajo.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hizo");
    }

    #[test]
    fn test_irregular_preterite_poner() {
        // "ellos puso" → "ellos pusieron" (pretérito de poner)
        let tokens = tokenize("Ellos puso la mesa.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "pusieron");
    }

    #[test]
    fn test_irregular_preterite_tener() {
        // "ella tuvieron" → "ella tuvo" (pretérito de tener)
        let tokens = tokenize("Ella tuvieron suerte.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tuvo");
    }

    #[test]
    fn test_irregular_preterite_venir() {
        // "ellos vino" → "ellos vinieron" (pretérito de venir)
        let tokens = tokenize("Ellos vino a la fiesta.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vinieron");
    }

    #[test]
    fn test_irregular_preterite_ir_ser() {
        // "él fueron" → "él fue" (pretérito de ir/ser)
        let tokens = tokenize("Él fueron al cine.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "fue");
    }

    #[test]
    fn test_irregular_preterite_correct_no_change() {
        // "ella dijo" es correcto, no debe haber correcciones
        let tokens = tokenize("Ella dijo la verdad.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "Forma correcta no debe generar corrección"
        );
    }

    #[test]
    fn test_prefixed_hacer_preterite_pronoun_no_false_positive() {
        let corrections = match analyze_with_dictionary("Ella rehizo el trabajo") {
            Some(c) => c,
            None => return,
        };
        assert!(
            corrections.is_empty(),
            "No debe corregir 'rehizo' con sujeto singular: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Él deshizo el nudo").unwrap();
        assert!(
            corrections.is_empty(),
            "No debe corregir 'deshizo' con sujeto singular: {corrections:?}"
        );
    }

    #[test]
    fn test_prefixed_hacer_preterite_plural_suggestion() {
        let corrections = match analyze_with_dictionary("Ellos rehizo el trabajo") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "rehicieron");

        let corrections = analyze_with_dictionary("Ellos deshizo el nudo").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "deshicieron");
    }

    #[test]
    fn test_prefixed_venir_preterite_pronoun_no_false_positive() {
        let corrections = match analyze_with_dictionary("Ella previno riesgos") {
            Some(c) => c,
            None => return,
        };
        assert!(
            corrections.is_empty(),
            "No debe corregir 'previno' con sujeto singular: {corrections:?}"
        );
    }

    #[test]
    fn test_prefixed_venir_preterite_plural_suggestion() {
        let corrections = match analyze_with_dictionary("Ellos previno riesgos") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "previnieron");

        let corrections = analyze_with_dictionary("Ellos convino en el trato").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "convinieron");
    }

    #[test]
    fn test_prefixed_tener_preterite_plural_suggestion() {
        let corrections = match analyze_with_dictionary("Ellos mantuvo la calma") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "mantuvieron");
    }

    #[test]
    fn test_prefixed_irregular_preserves_prefix_when_infinitive_missing() {
        let corrections = match analyze_with_dictionary("Ella sobreponen objeciones") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sobrepone");

        let corrections = analyze_with_dictionary("Ellos antepone reparos").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "anteponen");
    }

    #[test]
    fn test_prefixed_irregular_surface_recognition_nonclassic_prefixes() {
        let corrections = match analyze_with_dictionary("Ellos depuso el cargo") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "depusieron");

        let corrections = analyze_with_dictionary("Ellos opuso resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opusieron");

        let corrections = analyze_with_dictionary("Ella atienen reservas").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "atiene");

        let corrections = analyze_with_dictionary("Ellos avino un acuerdo").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "avinieron");

        let corrections = analyze_with_dictionary("Ella bendicen").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "bendice");

        let corrections = analyze_with_dictionary("Ellos maldijo").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "maldijeron");
    }

    #[test]
    fn test_subjunctive_number_agreement_with_prefixed_irregulars() {
        let corrections = match analyze_with_dictionary("Que ellos oponga resistencia") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongan");

        let corrections = analyze_with_dictionary("Que ella opongan resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "oponga");

        let corrections = analyze_with_dictionary("Que ellos decaiga en su ánimo").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "decaigan");

        let corrections = analyze_with_dictionary("Que nosotros oponga resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongamos");

        let corrections = analyze_with_dictionary("Que vosotros oponga resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongáis");

        let corrections = analyze_with_dictionary("Que mañana ellos oponga resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongan");

        let corrections = analyze_with_dictionary("Que mañana ella opongan resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "oponga");

        let corrections = analyze_with_dictionary("Tal vez ellos oponga resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongan");

        let corrections = analyze_with_dictionary("Quizás ella opongan resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "oponga");
    }

    #[test]
    fn test_subjunctive_agreement_with_nominal_subject_prefixed_irregulars() {
        let corrections = match analyze_with_dictionary("Que los alumnos oponga resistencia") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongan");

        let corrections = analyze_with_dictionary("Que el alumno opongan resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "oponga");

        let corrections = analyze_with_dictionary("Que los estudiantes decaiga su ánimo").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "decaigan");

        let corrections =
            analyze_with_dictionary("Que mañana los alumnos oponga resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongan");

        let corrections =
            analyze_with_dictionary("Que mañana el alumno opongan resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "oponga");

        let corrections =
            analyze_with_dictionary("Tal vez los alumnos oponga resistencia").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "opongan");
    }

    #[test]
    fn test_nada_pronoun_not_treated_as_verb_with_other_finite_verb() {
        let corrections = match analyze_with_dictionary("Yo nada sé") {
            Some(c) => c,
            None => return,
        };
        let nada_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "nada")
            .collect();
        assert!(
            nada_corrections.is_empty(),
            "No debe tratar 'nada' como verbo cuando hay otro verbo finito: {corrections:?}"
        );
    }

    #[test]
    fn test_nada_pronoun_not_treated_as_verb_after_se() {
        let corrections = match analyze_with_dictionary("Yo no se nada") {
            Some(c) => c,
            None => return,
        };
        let nada_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "nada")
            .collect();
        assert!(
            nada_corrections.is_empty(),
            "No debe tratar 'nada' como verbo en patrón 'se nada': {corrections:?}"
        );
    }

    #[test]
    fn test_nada_verb_still_detected_when_actual_verb() {
        let corrections = match analyze_with_dictionary("Ellos nada en la piscina") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "nadan");
    }

    // Tests para pretéritos regulares (con pronombre explícito, no necesita diccionario)
    #[test]
    fn test_regular_preterite_ar_ellos() {
        // "ellos cantó" → "cantaron" (pretérito regular -ar)
        let tokens = tokenize("Ellos cantó muy bien.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantaron");
    }

    #[test]
    fn test_regular_preterite_er_ellos() {
        // "ellos comió" → "comieron" (pretérito regular -er)
        let tokens = tokenize("Ellos comió mucho.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "comieron");
    }

    #[test]
    fn test_regular_preterite_correct() {
        // "ellos cantaron" es correcto
        let tokens = tokenize("Ellos cantaron muy bien.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "Pretérito correcto no debe generar corrección"
        );
    }

    // ==========================================================================
    // Tests para cláusulas parentéticas (según/como + verbo de reporte)
    // ==========================================================================

    #[test]
    fn test_parenthetical_segun_explico() {
        // "Las medidas, según explicó" - no debe corregir "explicó"
        // porque es parte de cláusula parentética con sujeto implícito
        let tokens = tokenize("Las medidas, según explicó el ministro, son importantes.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let explico_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "explicó")
            .collect();
        assert!(
            explico_corrections.is_empty(),
            "No debe corregir 'explicó' - es verbo de cláusula parentética"
        );
    }

    #[test]
    fn test_parenthetical_segun_dijo() {
        // "Las medidas, según dijo" - no debe corregir "dijo"
        let tokens = tokenize("Las medidas, según dijo la portavoz, mejoran la situación.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let dijo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "dijo")
            .collect();
        assert!(
            dijo_corrections.is_empty(),
            "No debe corregir 'dijo' - es verbo de cláusula parentética"
        );
    }

    #[test]
    fn test_parenthetical_como_indico() {
        // "Las cifras, como indicó el presidente, muestran mejoría"
        // No debe corregir "indicó" ni "muestran"
        let tokens = tokenize("Las cifras, como indicó el presidente, muestran mejoría.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let indico_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "indicó")
            .collect();
        let muestran_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "muestran")
            .collect();
        assert!(
            indico_corrections.is_empty(),
            "No debe corregir 'indicó' - es verbo de cláusula parentética"
        );
        assert!(
            muestran_corrections.is_empty(),
            "No debe corregir 'muestran' - concuerda con 'cifras'"
        );
    }

    // Nota: Los siguientes tests requieren enrichment completo del diccionario
    // para detectar sujetos nominales. Se verifican manualmente:
    //
    // cargo run --release -- "Los presidentes viajó a España"
    // Output esperado: Los presidentes viajó [viajaron] a España
    //
    // cargo run --release -- "Las medidas, según explicó el ministro, es importante"
    // Output esperado: Las medidas, según explicó el ministro, es [son] importante
    //
    // cargo run --release -- "Las cifras, como indicó el presidente, muestra mejoría"
    // Output esperado: Las cifras, como indicó el presidente, muestra [muestran] mejoría

    #[test]
    fn test_parenthetical_subject_inside_clause() {
        // "el presidente" dentro de "como indicó el presidente" no debe
        // analizarse como sujeto del verbo principal
        let tokens = tokenize("Los datos, como indicó el presidente, revelan mejoras.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let revelan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "revelan")
            .collect();
        assert!(
            revelan_corrections.is_empty(),
            "No debe corregir 'revelan' - 'el presidente' está dentro de cláusula parentética"
        );
    }

    #[test]
    fn test_como_apposition_example_not_treated_as_main_subject() {
        let tokens =
            tokenize("Las enfermedades crónicas, como la diabetes, requieren seguimiento.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let requieren_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "requieren")
            .collect();
        assert!(
            requieren_corrections.is_empty(),
            "No debe corregir 'requieren' por SN interno en inciso ', como ... ,': {corrections:?}"
        );
    }

    #[test]
    fn test_como_apposition_with_or_not_treated_as_main_subject() {
        let tokens =
            tokenize("Los edificios, como el colegio o la biblioteca, necesitan reformas.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let necesitan_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original.to_lowercase() == "necesitan")
            .collect();
        assert!(
            necesitan_corrections.is_empty(),
            "No debe corregir 'necesitan' por SN coordinado interno en inciso ', como ... ,': {corrections:?}"
        );
    }

    // ==========================================================================
    // Tests para gerundios (formas verbales invariables)
    // ==========================================================================

    #[test]
    fn test_gerund_abandonando_not_corrected() {
        // Los gerundios (-ando, -iendo, -yendo) son formas verbales invariables
        // que no tienen concordancia de persona/número con el sujeto
        // "abandonando" NO debe corregirse a "abandonanda"
        let tokens = tokenize("abandonando su consideración");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let abandonando_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "abandonando")
            .collect();
        assert!(
            abandonando_corrections.is_empty(),
            "No debe corregir gerundio 'abandonando' - es forma verbal invariable"
        );
    }

    #[test]
    fn test_gerund_comiendo_not_corrected() {
        // Gerundio con terminación -iendo
        let tokens = tokenize("estaba comiendo su cena");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let comiendo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "comiendo")
            .collect();
        assert!(
            comiendo_corrections.is_empty(),
            "No debe corregir gerundio 'comiendo' - es forma verbal invariable"
        );
    }

    #[test]
    fn test_gerund_viviendo_not_corrected() {
        // Gerundio con terminación -iendo
        let tokens = tokenize("seguía viviendo en Madrid");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let viviendo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "viviendo")
            .collect();
        assert!(
            viviendo_corrections.is_empty(),
            "No debe corregir gerundio 'viviendo' - es forma verbal invariable"
        );
    }

    #[test]
    fn test_gerund_cayendo_not_corrected() {
        // Gerundio con terminación -yendo
        let tokens = tokenize("estaba cayendo la lluvia");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let cayendo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "cayendo")
            .collect();
        assert!(
            cayendo_corrections.is_empty(),
            "No debe corregir gerundio 'cayendo' - es forma verbal invariable"
        );
    }

    // ==========================================================================
    // Tests para acrónimos (palabras en mayúsculas)
    // ==========================================================================

    #[test]
    fn test_acronym_satse_not_corrected() {
        // Acronyms like SATSE should not be treated as verbs needing agreement
        // "los sindicatos SATSE" - SATSE is an acronym, not a verb
        let tokens = tokenize("Los sindicatos SATSE convocaron la huelga.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let satse_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "SATSE")
            .collect();
        assert!(
            satse_corrections.is_empty(),
            "No debe corregir el acrónimo 'SATSE' - es acrónimo, no verbo"
        );
    }

    #[test]
    fn test_acronym_ccoo_not_corrected() {
        // Multiple acronyms after noun should not be corrected
        let tokens = tokenize("Los sindicatos CCOO y UGT firmaron el acuerdo.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let ccoo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.original == "CCOO" || c.original == "UGT")
            .collect();
        assert!(
            ccoo_corrections.is_empty(),
            "No debe corregir acrónimos 'CCOO' y 'UGT'"
        );
    }

    #[test]
    fn test_all_caps_subject_verb_corrected() {
        // All-caps headlines should still be corrected
        let mut tokens = tokenize("LOS PERROS CANTA");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "CANTA");
        assert!(
            correction.is_some(),
            "Should correct 'CANTA' in all-caps text"
        );
        assert_eq!(correction.unwrap().suggestion.to_lowercase(), "cantan");
    }

    #[test]
    fn test_coordination_conjunction_not_nominal_subject() {
        // "y abre" coordina verbos, no SN; no debe forzar plural en el sujeto nominal previo
        let mut tokens = tokenize("Esta tecnologia es una via para simplificar el control de la calidad del aire y abre la puerta.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let abre_correction = corrections.iter().find(|c| c.original == "abre");
        assert!(
            abre_correction.is_none(),
            "No debe corregir 'abre' en coordinaciÃ³n verbal"
        );
    }

    #[test]
    fn test_coordinated_subject_no_false_singular_correction() {
        let mut tokens =
            tokenize("La Dirección Nacional y el Consejo de Fundadores del MAIS escogieron.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();
        let nominal_subject = SubjectVerbAnalyzer::detect_nominal_subject(&tokens, &word_tokens, 0)
            .expect("Debe detectar sujeto nominal coordinado");
        assert!(nominal_subject.is_coordinated, "Debe marcar coordinación");

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "escogieron");
        assert!(
            correction.is_none(),
            "No debe sugerir singular en sujeto coordinado"
        );
    }

    #[test]
    fn test_date_before_subject_does_not_block_coordination() {
        let mut tokens =
            tokenize("El 25 de julio de 2021 la Dirección Nacional y el Consejo escogieron.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "escogieron");
        assert!(
            correction.is_none(),
            "No debe corregir verbo plural tras fecha con números"
        );
    }

    #[test]
    fn test_proper_name_after_comma_blocks_previous_subject() {
        let mut tokens = tokenize("Los resultados, Juan anunció que su voto sería por Rodolfo.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "anunció");
        assert!(
            correction.is_none(),
            "No debe corregir verbo con sujeto propio tras coma"
        );
    }

    #[test]
    fn test_common_noun_after_comma_blocks_previous_coordinated_subject() {
        let mut tokens = tokenize("La casa y el coche, el niño corrió.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "corrió");
        assert!(
            correction.is_none(),
            "No debe corregir cuando hay nuevo sujeto nominal tras coma"
        );
    }

    #[test]
    fn test_common_noun_after_comma_blocks_previous_coordinated_subject_with_acronym() {
        let mut tokens = tokenize("La salida y la ausencia, el CNE autorizó.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "autorizó");
        assert!(
            correction.is_none(),
            "No debe corregir cuando hay nuevo sujeto nominal tras coma con sigla"
        );
    }

    #[test]
    fn test_proper_name_coordination_without_determiner() {
        let mut tokens = tokenize("Google y el Movimiento anunciaron su voto.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "anunciaron");
        assert!(
            correction.is_none(),
            "No debe corregir sujeto coordinado con nombre propio"
        );
    }

    #[test]
    fn test_proper_names_coordination_without_determiner_corrects_singular_verb() {
        let corrections = match analyze_with_dictionary("Maria y Pedro sale") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "sale");
        assert!(
            correction.is_some(),
            "Debe corregir verbo singular con sujeto coordinado de nombres propios: {corrections:?}"
        );
        assert_eq!(
            SubjectVerbAnalyzer::normalize_spanish(&correction.unwrap().suggestion),
            "salen"
        );
    }

    #[test]
    fn test_roman_numeral_not_treated_as_verb() {
        let mut tokens = tokenize("El Rey VI envió un mensaje.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let numeral_correction = corrections.iter().find(|c| c.original == "VI");
        assert!(
            numeral_correction.is_none(),
            "No debe corregir número romano como verbo"
        );
    }

    #[test]
    fn test_partitive_sumatoria_does_not_force_singular() {
        let mut tokens = tokenize("La sumatoria de los porcentajes daban.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "daban");
        assert!(
            correction.is_none(),
            "Partitivo 'sumatoria' no debe forzar singular"
        );
    }

    #[test]
    fn test_proper_name_after_preposition_not_treated_as_verb() {
        let mut tokens = tokenize("La presidencia de Mauricio");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let mauricio_correction = corrections.iter().find(|c| c.original == "Mauricio");
        assert!(
            mauricio_correction.is_none(),
            "No debe corregir nombre propio como verbo"
        );
    }

    #[test]
    fn test_proper_name_apposition_skipped() {
        let mut tokens = tokenize("El ministro Alberto Carrasquilla anunciaron.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let name_correction = corrections.iter().find(|c| c.original == "Alberto");
        assert!(
            name_correction.is_none(),
            "No debe corregir nombre propio en aposicion"
        );
        let verb_correction = corrections.iter().find(|c| c.original == "anunciaron");
        assert!(
            verb_correction.is_some(),
            "Debe corregir el verbo despues del nombre propio"
        );
    }

    #[test]
    fn test_capitalized_verb_after_nominal_subject_still_corrected() {
        let mut tokens = tokenize("El ministro Anuncian.");
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        let correction = corrections.iter().find(|c| c.original == "Anuncian");
        assert!(
            correction.is_some(),
            "Debe corregir verbo capitalizado despues del SN"
        );
    }

    #[test]
    fn test_compound_adjective_with_hyphen_not_treated_as_verb() {
        // Adjetivos compuestos con guión NO deben tratarse como verbos
        // Bug anterior: "ruso-colombiano" se trataba como verbo y se generaban
        // correcciones incorrectas como "ruso-colombiana" o "ruso-colombianan"
        let tokens = tokenize("El caso ruso-colombiano fue importante");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);

        // No debe haber correcciones para "ruso-colombiano"
        let compound_correction = corrections
            .iter()
            .find(|c| c.original.contains("ruso-colombiano"));
        assert!(
            compound_correction.is_none(),
            "No debe tratar adjetivos compuestos con guión como verbos: {:?}",
            compound_correction
        );
    }

    #[test]
    fn test_compound_adjective_plural_not_treated_as_verb() {
        // Adjetivos compuestos en plural tampoco deben tratarse como verbos
        let tokens = tokenize("Las relaciones ruso-colombianas son buenas");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);

        let compound_correction = corrections
            .iter()
            .find(|c| c.original.contains("ruso-colombianas"));
        assert!(
            compound_correction.is_none(),
            "No debe tratar adjetivos compuestos plurales con guión como verbos: {:?}",
            compound_correction
        );
    }

    #[test]
    fn test_object_after_verb_not_treated_as_subject() {
        // Un SN inmediatamente después de un verbo es objeto directo, no sujeto
        // "Los estudiantes que aprobaron el examen celebraron"
        // "el examen" es OD de "aprobaron", no sujeto de "celebraron"
        let tokens = tokenize("Los estudiantes que aprobaron el examen celebraron");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let celebraron_correction = corrections.iter().find(|c| c.original == "celebraron");
        assert!(
            celebraron_correction.is_none(),
            "No debe tratar OD como sujeto: {:?}",
            corrections
        );
    }

    // ====== Tests de verbos con cambio de raíz ======

    #[test]
    fn test_stem_change_u_to_ue_jugar() {
        // jugar (u→ue): "ellos juega" → "juegan", no "jugan"
        let tokens = tokenize("ellos juega");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "juegan");
    }

    #[test]
    fn test_stem_change_o_to_ue_dormir() {
        // dormir (o→ue): "ellos duerme" → "duermen", no "dormen"
        let tokens = tokenize("ellos duerme");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "duermen");
    }

    #[test]
    fn test_stem_change_e_to_ie_pensar() {
        // pensar (e→ie): "ellos piensa" → "piensan", no "pensan"
        let tokens = tokenize("ellos piensa");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "piensan");
    }

    #[test]
    fn test_stem_change_e_to_i_pedir() {
        // pedir (e→i): "ellos pide" → "piden", no "peden"
        let tokens = tokenize("ellos pide");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "piden");
    }

    #[test]
    fn test_stem_change_singular_jugar() {
        // jugar (u→ue): "él juegan" → "juega", no "juga"
        let tokens = tokenize("él juegan");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "juega");
    }

    #[test]
    fn test_tu_conocen_corrected_to_conoces() {
        let dict_path = std::path::Path::new("data/es/words.txt");
        if !dict_path.exists() {
            return;
        }
        let dictionary =
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new());
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenize("tú conocen");
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "conoces");
    }

    #[test]
    fn test_ellos_conozco_corrected_to_conocen() {
        let dict_path = std::path::Path::new("data/es/words.txt");
        if !dict_path.exists() {
            return;
        }
        let dictionary =
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new());
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenize("ellos conozco");
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "conocen");
    }

    fn analyze_with_dictionary(text: &str) -> Option<Vec<SubjectVerbCorrection>> {
        let dict_path = std::path::Path::new("data/es/words.txt");
        if !dict_path.exists() {
            return None;
        }
        let dictionary =
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new());
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenize(text);
        for token in tokens.iter_mut() {
            if token.token_type == crate::grammar::tokenizer::TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        Some(SubjectVerbAnalyzer::analyze_with_recognizer(
            &tokens,
            Some(&recognizer),
        ))
    }

    #[test]
    fn test_recognizer_regular_preterite_iar_3s_is_detected_as_ar() {
        let corrections = match analyze_with_dictionary("ellos cambió de opinión") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cambiaron");

        let corrections = analyze_with_dictionary("ellos copió el examen").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "copiaron");

        let corrections = analyze_with_dictionary("ellos envió el paquete").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "enviaron");
    }

    #[test]
    fn test_recognizer_regular_preterite_iar_1s_is_detected_as_ar() {
        let corrections = match analyze_with_dictionary("ellos cambié de idea") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cambiaron");
    }

    #[test]
    fn test_recognizer_regular_preterite_er_ir_remains_correct() {
        let corrections = match analyze_with_dictionary("ellos comió mucho") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "comieron");
    }

    #[test]
    fn test_participle_suffix_words_not_filtered_out() {
        // Bug: filtro de participios descartaba "pido/cuido/nado" por terminar en -ido/-ado
        let corrections = match analyze_with_dictionary("ellos pido ayuda") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "piden");

        let corrections = analyze_with_dictionary("nosotros cuido el jardín").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cuidamos");

        let corrections = analyze_with_dictionary("ellos nado en la piscina").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "nadan");
    }

    #[test]
    fn test_participle_suffix_ada_ida_words_not_filtered_out() {
        // Bug: filtro de participios descartaba formas finitas en -ada/-ida.
        let corrections = match analyze_with_dictionary("yo olvida las llaves") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "olvido");

        let corrections = analyze_with_dictionary("ellos cuida las plantas").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cuidan");

        let corrections = analyze_with_dictionary("yo nada en la piscina").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "nado");
    }

    #[test]
    fn test_postposed_subject_in_relative_clause_not_used_for_main_verb() {
        let corrections =
            match analyze_with_dictionary("Las puertas que cierra el vigilante son grandes") {
                Some(c) => c,
                None => return,
            };
        let son_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "son");
        assert!(
            son_correction.is_none(),
            "No debe forzar singular en verbo principal tras relativa: {corrections:?}"
        );

        let corrections =
            analyze_with_dictionary("Los problemas que resuelve el equipo son graves").unwrap();
        let son_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "son");
        assert!(
            son_correction.is_none(),
            "No debe forzar singular en verbo principal tras relativa: {corrections:?}"
        );

        let corrections =
            analyze_with_dictionary("Las leyes que aprueba el parlamento son justas").unwrap();
        let son_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "son");
        assert!(
            son_correction.is_none(),
            "No debe forzar singular en verbo principal tras relativa: {corrections:?}"
        );
    }

    #[test]
    fn test_postposed_subject_in_relative_clause_with_adverb_not_used_for_main_verb() {
        let corrections =
            match analyze_with_dictionary("Las cosas que dijo ayer el ministro son ciertas") {
                Some(c) => c,
                None => return,
            };
        let son_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "son");
        assert!(
            son_correction.is_none(),
            "No debe forzar singular con adverbio entre verbo relativo y sujeto pospuesto: {corrections:?}"
        );

        let corrections =
            analyze_with_dictionary("Los problemas que resuelve siempre el equipo son graves")
                .unwrap();
        let son_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "son");
        assert!(
            son_correction.is_none(),
            "No debe forzar singular con adverbio entre verbo relativo y sujeto pospuesto: {corrections:?}"
        );
    }

    #[test]
    fn test_postposed_subject_in_relative_clause_with_todo_quantifier_not_used_for_main_verb() {
        let cases = [
            "Los perros que ladraban toda la noche están dormidos",
            "Los niños que limpiaron toda la casa están cansados",
            "Los atletas que corrieron toda la carrera están agotados",
            "Las paredes que pintaron durante toda la noche están secas",
        ];

        for text in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let estar_correction = corrections.iter().find(|c| {
                SubjectVerbAnalyzer::normalize_spanish(&c.original) == "estan"
                    || SubjectVerbAnalyzer::normalize_spanish(&c.original) == "esta"
            });
            assert!(
                estar_correction.is_none(),
                "No debe forzar singular en verbo principal con patrón 'que + verbo + toda la ...': {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_postposed_object_after_relative_temporal_preposition_not_used_for_main_verb() {
        let corrections = match analyze_with_dictionary(
            "El comité que revisó durante toda la mañana los informes confirma los resultados",
        ) {
            Some(c) => c,
            None => return,
        };
        let confirma_correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "confirma");
        assert!(
            confirma_correction.is_none(),
            "No debe forzar plural en verbo principal por objeto dentro de relativa temporal: {corrections:?}"
        );
    }

    #[test]
    fn test_stem_changing_reventar_escocer_supported() {
        let mut trie = Trie::new();
        let verb_info = crate::dictionary::WordInfo {
            category: crate::dictionary::WordCategory::Verbo,
            gender: crate::dictionary::Gender::None,
            number: crate::dictionary::trie::Number::None,
            extra: String::new(),
            frequency: 100,
        };
        trie.insert("reventar", verb_info.clone());
        trie.insert("escocer", verb_info);
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        let tokens = tokenize("ellos revienta por dentro");
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "revientan");

        let tokens = tokenize("ellos escuece la lengua");
        let corrections = SubjectVerbAnalyzer::analyze_with_recognizer(&tokens, Some(&recognizer));
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "escuecen");
    }

    #[test]
    fn test_uir_confluir_y_forms_supported() {
        let corrections = match analyze_with_dictionary("yo confluyen en ese punto") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "confluyo");

        let corrections = analyze_with_dictionary("ellos confluye en ese punto").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "confluyen");
    }

    #[test]
    fn test_recognizer_orthography_sigo_elijo_corrijo() {
        // Bug: el recognizer no reconocía gu→g, g→j y z/c en presente 1s
        let corrections = match analyze_with_dictionary("ellos sigo adelante") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "siguen");

        let corrections = analyze_with_dictionary("ellos elijo la mejor opción").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "eligen");

        let corrections = analyze_with_dictionary("nosotros corrijo los errores").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "corregimos");
    }

    #[test]
    fn test_pronoun_with_intervening_adverb_is_corrected() {
        // Bug: el patrón pronombre + verbo solo funcionaba con verbos adyacentes.
        // Ej: "ellos nunca olvido" -> "olvidan"
        let corrections = match analyze_with_dictionary("ellos nunca olvido nada") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "olvidan");
    }

    #[test]
    fn test_preterite_iste_not_misclassified_as_present() {
        // Bug: "dormiste" (pretérito 2s) se interpretaba como presente 3s por el sufijo "-e".
        let corrections = match analyze_with_dictionary("ellos dormiste") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "durmieron");

        // Bug similar: "-isteis" (pretérito 2p) se interpretaba como presente 2p por "-eis".
        let corrections = analyze_with_dictionary("ellos comisteis").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "comieron");
    }

    #[test]
    fn test_generation_present_1s_orthography() {
        // Bug: generación de 1s inventaba "siguo/eligo/corrigo/tuerco/venco"
        let corrections = match analyze_with_dictionary("yo siguen el camino") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sigo");

        let corrections = analyze_with_dictionary("yo eligen mal").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "elijo");

        let corrections = analyze_with_dictionary("yo corrigen los textos").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "corrijo");

        let corrections = analyze_with_dictionary("yo persiguen sus sueños").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "persigo");

        let corrections = analyze_with_dictionary("yo tuercen el alambre").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tuerzo");

        let corrections = analyze_with_dictionary("yo vencen hoy").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "venzo");
    }

    #[test]
    fn test_preterite_ir_e_to_i_pedir() {
        // Bug: "yo pidieron" → "pidí" (debe ser "pedí")
        let corrections = match analyze_with_dictionary("yo pidieron") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "pedí");

        // Bug: "ellos pedí" → "pedieron" (debe ser "pidieron")
        let corrections = analyze_with_dictionary("ellos pedí").unwrap();
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "pidieron");
    }

    #[test]
    fn test_cocer_cuecen_cuezo() {
        // Bug: spell-check interfiere y se corrige como "crecen"/"crezco".
        // Aquí testeamos la parte sujeto-verbo: "yo cuecen" → "cuezo".
        let corrections = match analyze_with_dictionary("yo cuecen") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cuezo");
    }

    #[test]
    fn test_prefixed_unknown_infinitive_falls_back_to_base() {
        // Bug: "yo recorrigen" → "recorrejo" (inventado por reconstruir "recorregir").
        // Si el infinitivo con prefijo no existe, preferimos el infinitivo base.
        let corrections = match analyze_with_dictionary("yo recorrigen") {
            Some(c) => c,
            None => return,
        };
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "corrijo");
    }

    #[test]
    fn test_requerir_inquirir_not_mapped_to_querer() {
        let corrections = match analyze_with_dictionary("ella requieren ayuda") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "requieren")
            .expect("Debe detectar discordancia en 'ella requieren'");
        assert_eq!(correction.suggestion, "requiere");

        let corrections = analyze_with_dictionary("ellos requiere ayuda").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "requiere")
            .expect("Debe detectar discordancia en 'ellos requiere'");
        assert_eq!(correction.suggestion, "requieren");

        let corrections = analyze_with_dictionary("ella inquieren detalles").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "inquieren")
            .expect("Debe detectar discordancia en 'ella inquieren'");
        assert_eq!(correction.suggestion, "inquiere");

        let corrections = analyze_with_dictionary("ellos inquiere detalles").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "inquiere")
            .expect("Debe detectar discordancia en 'ellos inquiere'");
        assert_eq!(correction.suggestion, "inquieren");

        let corrections = analyze_with_dictionary("ella adquieren recursos").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "adquieren")
            .expect("Debe detectar discordancia en 'ella adquieren'");
        assert_eq!(correction.suggestion, "adquiere");

        let corrections = analyze_with_dictionary("ellos adquiere recursos").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "adquiere")
            .expect("Debe detectar discordancia en 'ellos adquiere'");
        assert_eq!(correction.suggestion, "adquieren");

        let corrections = analyze_with_dictionary("ella convienen soluciones").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "convienen")
            .expect("Debe detectar discordancia en 'ella convienen'");
        assert_eq!(correction.suggestion, "conviene");

        let corrections = analyze_with_dictionary("ellos conviene soluciones").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "conviene")
            .expect("Debe detectar discordancia en 'ellos conviene'");
        assert_eq!(correction.suggestion, "convienen");
    }

    #[test]
    fn test_nominal_subject_comma_adverb_does_not_force_third_person() {
        // "Un café, siempre canto" suele ser tópico + cláusula nueva con sujeto implícito (yo).
        // No debe forzar "canta" por haber visto un SN singular antes de la coma.
        let corrections = match analyze_with_dictionary("un café, siempre canto") {
            Some(c) => c,
            None => return,
        };
        assert!(
            corrections.is_empty(),
            "No debe corregir 'canto' tras coma+adverbio: {corrections:?}"
        );
    }

    #[test]
    fn test_temporal_complement_det_noun_not_forced_as_subject() {
        let corrections = match analyze_with_dictionary("El lunes empiezan las vacaciones") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "empiezan");
        assert!(
            correction.is_none(),
            "No debe corregir 'empiezan' cuando 'El lunes' es complemento temporal: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("El verano florecen las rosas").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "florecen");
        assert!(
            correction.is_none(),
            "No debe corregir 'florecen' cuando 'El verano' es complemento temporal: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("La semana pasada vinieron mis primos").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "vinieron");
        assert!(
            correction.is_none(),
            "No debe corregir 'vinieron' cuando hay sujeto pospuesto: {corrections:?}"
        );
    }

    #[test]
    fn test_temporal_complement_plural_not_forced_as_subject() {
        let corrections = match analyze_with_dictionary("Los domingos vamos a misa") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "vamos");
        assert!(
            correction.is_none(),
            "No debe corregir 'vamos' cuando 'Los domingos' es complemento temporal: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Los sábados hacemos deporte").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "hacemos");
        assert!(
            correction.is_none(),
            "No debe corregir 'hacemos' cuando 'Los sábados' es complemento temporal: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Las mañanas entreno en el parque").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "entreno");
        assert!(
            correction.is_none(),
            "No debe corregir 'entreno' cuando 'Las mañanas' es complemento temporal: {corrections:?}"
        );
    }

    #[test]
    fn test_temporal_complement_singular_with_first_second_person_not_forced() {
        let corrections = match analyze_with_dictionary("El lunes vamos al cine") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "vamos");
        assert!(
            correction.is_none(),
            "No debe corregir 'vamos' con complemento temporal singular: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("El lunes viajo a Madrid").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "viajo");
        assert!(
            correction.is_none(),
            "No debe corregir 'viajo' con complemento temporal singular: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("El martes vienes a casa").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "vienes");
        assert!(
            correction.is_none(),
            "No debe corregir 'vienes' con complemento temporal singular: {corrections:?}"
        );
    }

    #[test]
    fn test_temporal_complement_plural_with_third_person_singular_not_forced() {
        let corrections = match analyze_with_dictionary("Todos los días sale a correr") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "sale");
        assert!(
            correction.is_none(),
            "No debe corregir 'sale' cuando 'todos los días' es complemento temporal: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Todos los sábados juega al fútbol").unwrap();
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "juega");
        assert!(
            correction.is_none(),
            "No debe corregir 'juega' con complemento temporal plural: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Todos los meses paga el alquiler").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "paga");
        assert!(
            correction.is_none(),
            "No debe corregir 'paga' con complemento temporal plural: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Todos los años celebra su cumpleaños").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "celebra");
        assert!(
            correction.is_none(),
            "No debe corregir 'celebra' con complemento temporal plural: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Todos los días estudia mucho").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "estudia");
        assert!(
            correction.is_none(),
            "No debe corregir 'estudia' con complemento temporal plural: {corrections:?}"
        );
    }

    #[test]
    fn test_temporal_complement_plural_with_explicit_postposed_plural_subject_still_corrects() {
        let corrections = match analyze_with_dictionary("Todos los días llega mis amigos") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "llega");
        assert!(
            correction.is_some(),
            "Debe corregir 3ª singular cuando hay sujeto pospuesto plural explícito: {corrections:?}"
        );
        assert_eq!(correction.unwrap().suggestion, "llegan");
    }

    #[test]
    fn test_gustar_like_verbs_with_postposed_plural_subject_are_corrected() {
        let cases = [
            ("Me gusta los perros", "gusta", "gustan"),
            ("Le molesta los ruidos", "molesta", "molestan"),
            ("Nos preocupa las noticias", "preocupa", "preocupan"),
            ("Te interesa los libros", "interesa", "interesan"),
            ("Me duele las piernas", "duele", "duelen"),
            ("Nos falta dos días", "falta", "faltan"),
            ("Le sobra motivos", "sobra", "sobran"),
            ("Me encanta los planes", "encanta", "encantan"),
            ("Le fascina los documentales", "fascina", "fascinan"),
            ("Nos apetece unas vacaciones", "apetece", "apetecen"),
            ("Te agrada los cambios", "agrada", "agradan"),
            ("Me disgusta los ruidos", "disgusta", "disgustan"),
            ("Le importa los detalles", "importa", "importan"),
            ("Nos conviene las medidas", "conviene", "convienen"),
            ("Les corresponde los premios", "corresponde", "corresponden"),
            ("Le pertenece esos terrenos", "pertenece", "pertenecen"),
            ("Nos basta dos ejemplos", "basta", "bastan"),
        ];

        for (text, wrong, expected) in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let correction = corrections.iter().find(|c| {
                SubjectVerbAnalyzer::normalize_spanish(&c.original)
                    == SubjectVerbAnalyzer::normalize_spanish(wrong)
            });
            assert!(
                correction.is_some(),
                "Debe corregir '{wrong}' en construccion tipo gustar: {text} -> {corrections:?}"
            );
            assert_eq!(
                SubjectVerbAnalyzer::normalize_spanish(&correction.unwrap().suggestion),
                expected
            );
        }
    }

    #[test]
    fn test_gustar_like_verbs_without_dative_clitic_are_not_forced() {
        let corrections = match analyze_with_dictionary("La crisis preocupa los mercados") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "preocupa");
        assert!(
            correction.is_none(),
            "No debe forzar plural en uso transitivo sin clitico dativo: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("La empresa importa coches").unwrap();
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "importa");
        assert!(
            correction.is_none(),
            "No debe forzar plural en uso transitivo de 'importar': {corrections:?}"
        );
    }

    #[test]
    fn test_gustar_like_clause_or_infinitive_subject_not_forced_plural() {
        let corrections = match analyze_with_dictionary("Me importa que vengas") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "importa");
        assert!(
            correction.is_none(),
            "No debe corregir cuando el sujeto es una subordinada: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Me encanta correr").unwrap();
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "encanta");
        assert!(
            correction.is_none(),
            "No debe corregir cuando el sujeto es infinitivo singular: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Le gusta como cocinas").unwrap();
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "gusta");
        assert!(
            correction.is_none(),
            "No debe corregir cuando el sujeto es subordinada interrogativa: {corrections:?}"
        );
    }

    #[test]
    fn test_reflexive_passive_se_singular_with_postposed_plural_is_corrected() {
        let cases = [
            ("Se vende pisos", "vende", "venden"),
            ("Se busca empleados", "busca", "buscan"),
            (
                "Se proh\u{00ED}be las motos",
                "proh\u{00ED}be",
                "proh\u{00ED}ben",
            ),
            ("Se busca urgentemente empleados", "busca", "buscan"),
        ];

        for (text, wrong, expected) in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let correction = corrections.iter().find(|c| {
                SubjectVerbAnalyzer::normalize_spanish(&c.original)
                    == SubjectVerbAnalyzer::normalize_spanish(wrong)
            });
            assert!(
                correction.is_some(),
                "Debe corregir pasiva refleja en '{text}': {corrections:?}"
            );
            assert_eq!(
                correction.unwrap().suggestion,
                expected
            );
        }
    }

    #[test]
    fn test_reflexive_passive_se_singular_with_postposed_singular_not_forced_plural() {
        let corrections = match analyze_with_dictionary("Se vende piso") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "vende");
        assert!(
            correction.is_none(),
            "No debe corregir cuando el SN pospuesto es singular: {corrections:?}"
        );
    }

    #[test]
    fn test_reflexive_body_part_patterns_not_treated_as_passive() {
        let cases = [
            ("Maria se lava las manos", "lava"),
            ("Juan se pone los zapatos", "pone"),
            ("Ana se cepilla los dientes", "cepilla"),
            ("Se corta las unas", "corta"),
            ("Se pinta las unas", "pinta"),
        ];

        for (text, verb) in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let correction = corrections
                .iter()
                .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == verb);
            assert!(
                correction.is_none(),
                "No debe tratar reflexivo corporal como pasiva refleja en '{text}': {corrections:?}"
            );
        }
    }

    #[test]
    fn test_temporal_complement_with_impersonal_weather_not_forced_plural() {
        let corrections = analyze_with_dictionary("Todos los días llueve").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "llueve");
        assert!(
            correction.is_none(),
            "No debe corregir 'llueve' en patrón temporal + verbo impersonal: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Todas las noches nieva").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "nieva");
        assert!(
            correction.is_none(),
            "No debe corregir 'nieva' en patrón temporal + verbo impersonal: {corrections:?}"
        );
    }

    #[test]
    fn test_correlative_coordination_tanto_como_not_forced_singular() {
        let corrections = match analyze_with_dictionary("Tanto el pan como la leche están caros") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "están");
        assert!(
            correction.is_none(),
            "No debe corregir 'están' en coordinación 'tanto...como...': {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Tanto el perro como el gato duermen").unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "duermen");
        assert!(
            correction.is_none(),
            "No debe corregir 'duermen' en coordinación 'tanto...como...': {corrections:?}"
        );
    }

    #[test]
    fn test_correlative_coordination_ni_ni_not_forced_singular() {
        let corrections = match analyze_with_dictionary("Ni el padre ni la madre quieren ir") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "quieren");
        assert!(
            correction.is_none(),
            "No debe corregir 'quieren' en coordinación 'ni...ni...': {corrections:?}"
        );
    }

    #[test]
    fn test_correlative_coordination_ni_ni_singular_is_also_accepted() {
        let corrections = match analyze_with_dictionary("Ni el pan ni la leche está caro") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "esta");
        assert!(
            correction.is_none(),
            "No debe forzar plural en 'ni...ni' cuando va en singular: {corrections:?}"
        );
    }

    #[test]
    fn test_coordinated_subject_with_y_still_requires_plural() {
        let corrections = match analyze_with_dictionary("El pan y la leche está caro") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "esta");
        assert!(
            correction.is_some(),
            "Debe corregir singular en coordinación con 'y': {corrections:?}"
        );
        assert_eq!(correction.unwrap().suggestion, "están");
    }

    #[test]
    fn test_coordinated_subject_with_possessives_requires_plural() {
        let corrections = match analyze_with_dictionary("Mi hermano y mi hermana estudia") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "estudia");
        assert!(
            correction.is_some(),
            "Debe corregir verbo singular con sujeto coordinado posesivo: {corrections:?}"
        );
        assert_eq!(
            SubjectVerbAnalyzer::normalize_spanish(&correction.unwrap().suggestion),
            "estudian"
        );
    }

    #[test]
    fn test_coordinated_subject_with_possessives_plural_not_corrected() {
        let corrections = match analyze_with_dictionary("Mi hermano y mi hermana estudian") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "estudian");
        assert!(
            correction.is_none(),
            "No debe corregir cuando ya esta en plural con sujeto coordinado posesivo: {corrections:?}"
        );
    }

    #[test]
    fn test_de_complement_internal_coordination_not_forced_plural() {
        let cases = [
            (
                "La asociación de padres y madres solicitó apoyo",
                "solicitó",
            ),
            ("El comité de padres y madres aprobó el plan", "aprobó"),
            (
                "La dirección de ventas y marketing decidió cambios",
                "decidió",
            ),
            (
                "El consejo de ministros y ministras aprobó la medida",
                "aprobó",
            ),
        ];

        for (text, verb) in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let correction = corrections.iter().find(|c| {
                SubjectVerbAnalyzer::normalize_spanish(&c.original)
                    == SubjectVerbAnalyzer::normalize_spanish(verb)
            });
            assert!(
                correction.is_none(),
                "No debe forzar plural con coordinación interna de 'de ... y ...': {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_de_complement_internal_coordination_still_corrects_plural_verb() {
        let corrections =
            match analyze_with_dictionary("La asociación de padres y madres solicitaron apoyo") {
                Some(c) => c,
                None => return,
            };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "solicitaron");
        assert!(
            correction.is_some(),
            "Debe corregir plural en sujeto singular con complemento 'de ... y ...': {corrections:?}"
        );
        assert_eq!(
            SubjectVerbAnalyzer::normalize_spanish(&correction.unwrap().suggestion),
            "solicito"
        );
    }

    #[test]
    fn test_de_complement_with_possessive_determiner_still_corrects_plural_verb() {
        let corrections =
            match analyze_with_dictionary("La hermana de mis amigos trabajan en casa") {
                Some(c) => c,
                None => return,
            };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "trabajan");
        assert!(
            correction.is_some(),
            "Debe corregir plural en sujeto singular con complemento 'de mis ...': {corrections:?}"
        );
        assert_eq!(
            SubjectVerbAnalyzer::normalize_spanish(&correction.unwrap().suggestion),
            "trabaja"
        );
    }

    #[test]
    fn test_pronoun_correlative_ni_ni_not_forced_singular_or_person() {
        let corrections = match analyze_with_dictionary("Ni ella ni él quieren ir") {
            Some(c) => c,
            None => return,
        };
        let quieren_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "quieren");
        assert!(
            quieren_correction.is_none(),
            "No debe corregir 'quieren' en coordinación pronominal 'ni...ni...': {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Ni tú ni yo podemos hacerlo").unwrap();
        let podemos_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "podemos");
        assert!(
            podemos_correction.is_none(),
            "No debe corregir 'podemos' en coordinación pronominal 'ni...ni...': {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Ni tú ni ella queréis ir").unwrap();
        let quereis_correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "quereis");
        assert!(
            quereis_correction.is_none(),
            "No debe corregir 'queréis' en coordinación pronominal 'ni...ni...': {corrections:?}"
        );
    }

    #[test]
    fn test_pronoun_correlative_tanto_como_not_forced_singular_or_person() {
        let corrections = match analyze_with_dictionary("Tanto él como ella son buenos") {
            Some(c) => c,
            None => return,
        };
        let son_correction = corrections.iter().find(|c| c.original.to_lowercase() == "son");
        assert!(
            son_correction.is_none(),
            "No debe corregir 'son' en coordinación pronominal 'tanto...como...': {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Tanto yo como tú sabemos la verdad").unwrap();
        let sabemos_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "sabemos");
        assert!(
            sabemos_correction.is_none(),
            "No debe corregir 'sabemos' en coordinación pronominal 'tanto...como...': {corrections:?}"
        );

        let corrections = analyze_with_dictionary("Tanto ella como él vienen mañana").unwrap();
        let vienen_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "vienen");
        assert!(
            vienen_correction.is_none(),
            "No debe corregir 'vienen' en coordinación pronominal 'tanto...como...': {corrections:?}"
        );
    }

    #[test]
    fn test_pronoun_correlative_tanto_como_after_comma_clause_not_forced() {
        let corrections = analyze_with_dictionary(
            "Tanto \u{00E9}l como ella son buenos, y tanto yo como t\u{00FA} sabemos la verdad",
        )
        .unwrap();
        let sabemos_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "sabemos");
        assert!(
            sabemos_correction.is_none(),
            "No debe corregir 'sabemos' tras coma y nueva coordinaci\u{00F3}n 'tanto...como...': {corrections:?}"
        );
    }

    #[test]
    fn test_pronoun_correlative_tanto_como_after_previous_clause_without_comma_not_forced() {
        let corrections = analyze_with_dictionary(
            "Tanto Pedro como Juan vienen temprano y tanto ella como \u{00E9}l est\u{00E1}n listos",
        )
        .unwrap();
        let estan_correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "estan");
        assert!(
            estan_correction.is_none(),
            "No debe corregir 'est\u{00E1}n' cuando hay nueva coordinaci\u{00F3}n 'tanto...como...' tras otra cl\u{00E1}usula: {corrections:?}"
        );
    }
    #[test]
    fn test_single_tanto_pronoun_still_checked_for_agreement() {
        let corrections = match analyze_with_dictionary("Tanto yo cantas bien") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "cantas");
        assert!(
            correction.is_some(),
            "Debe seguir corrigiendo concordancia sin correlativo completo 'tanto...como': {corrections:?}"
        );
        assert_eq!(correction.unwrap().suggestion, "canto");
    }

    #[test]
    fn test_single_ni_pronoun_still_checked_for_agreement() {
        let corrections = match analyze_with_dictionary("Ni yo cantas bien") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "cantas");
        assert!(
            correction.is_some(),
            "Debe seguir corrigiendo concordancia con un solo 'ni': {corrections:?}"
        );
        assert_eq!(correction.unwrap().suggestion, "canto");
    }

    #[test]
    fn test_durante_mediante_prepositional_phrase_not_treated_as_subject() {
        let corrections =
            match analyze_with_dictionary("Las flores durante la primavera florecieron") {
                Some(c) => c,
                None => return,
            };
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "florecieron");
        assert!(
            correction.is_none(),
            "No debe corregir 'florecieron' por SN tras 'durante': {corrections:?}"
        );

        let corrections =
            analyze_with_dictionary("Los científicos mediante la investigación descubrieron")
                .unwrap();
        let correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "descubrieron");
        assert!(
            correction.is_none(),
            "No debe corregir 'descubrieron' por SN tras 'mediante': {corrections:?}"
        );
    }

    #[test]
    fn test_copulative_ser_plural_attribute_with_plural_determiner_is_accepted() {
        let corrections = match analyze_with_dictionary("El problema fueron las lluvias") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "fueron");
        assert!(
            correction.is_none(),
            "No debe forzar singular en copulativa con atributo plural: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("La causa fueron los retrasos").unwrap();
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "fueron");
        assert!(
            correction.is_none(),
            "No debe forzar singular en copulativa con atributo plural: {corrections:?}"
        );

        let corrections = analyze_with_dictionary("La causa son los retrasos").unwrap();
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "son");
        assert!(
            correction.is_none(),
            "No debe forzar singular en copulativa presente con atributo plural: {corrections:?}"
        );
    }

    #[test]
    fn test_copulative_ser_plural_attribute_with_plural_pronoun_is_accepted() {
        let cases = [
            "El problema son ellos",
            "El motivo son ellos",
            "La causa son ellos",
            "El problema son ustedes",
            "El problema son nosotros",
            "El problema son estos",
            "El problema son esos",
            "El problema son aquellos",
        ];

        for text in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let correction = corrections
                .iter()
                .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "son");
            assert!(
                correction.is_none(),
                "No debe forzar singular en copulativa con atributo pronominal plural: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_compound_haber_auxiliary_agreement_with_singular_subject() {
        let cases = [
            ("El gobierno han decidido", "ha"),
            ("La empresa han despedido", "ha"),
            ("El equipo han jugado", "ha"),
            ("La gente han protestado", "ha"),
        ];

        for (text, expected) in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let correction = corrections
                .iter()
                .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "han");
            assert!(
                correction.is_some(),
                "Debe corregir auxiliar plural con sujeto singular en: {text} -> {corrections:?}"
            );
            assert_eq!(correction.unwrap().suggestion, expected);
        }
    }

    #[test]
    fn test_compound_haber_auxiliary_agreement_with_plural_subject_no_correction() {
        let corrections = match analyze_with_dictionary("Los equipos han jugado") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "han");
        assert!(
            correction.is_none(),
            "No debe corregir auxiliar bien concordado con sujeto plural: {corrections:?}"
        );
    }

    #[test]
    fn test_compound_haber_auxiliary_agreement_extended_tenses_with_singular_subject() {
        let cases = [
            ("El gobierno habían decidido", "había"),
            ("La empresa habrán despedido", "habrá"),
            ("El equipo habrían jugado", "habría"),
            ("La gente hayan protestado", "haya"),
            ("El comité hubieran aprobado", "hubiera"),
            ("La dirección hubiesen contestado", "hubiese"),
            ("El comité aún no habían aprobado", "había"),
        ];

        for (text, expected) in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let correction = corrections.iter().find(|c| {
                matches!(
                    SubjectVerbAnalyzer::normalize_spanish(&c.original).as_str(),
                    "han"
                        | "habian"
                        | "habran"
                        | "habrian"
                        | "hayan"
                        | "hubieran"
                        | "hubiesen"
                )
            });
            assert!(
                correction.is_some(),
                "Debe corregir auxiliar plural de haber en: {text} -> {corrections:?}"
            );
            assert_eq!(correction.unwrap().suggestion, expected);
        }
    }

    #[test]
    fn test_compound_haber_auxiliary_agreement_extended_tenses_with_plural_subject_no_correction() {
        let cases = [
            "Los equipos habían jugado",
            "Las empresas habrán despedido",
            "Los comités hubieran aprobado",
        ];

        for text in cases {
            let corrections = match analyze_with_dictionary(text) {
                Some(c) => c,
                None => return,
            };
            let has_haber_correction = corrections.iter().any(|c| {
                matches!(
                    SubjectVerbAnalyzer::normalize_spanish(&c.original).as_str(),
                    "ha"
                        | "han"
                        | "habia"
                        | "habian"
                        | "habra"
                        | "habran"
                        | "habria"
                        | "habrian"
                        | "haya"
                        | "hayan"
                        | "hubiera"
                        | "hubieran"
                        | "hubiese"
                        | "hubiesen"
                        | "hubo"
                        | "hubieron"
                )
            });
            assert!(
                !has_haber_correction,
                "No debe corregir auxiliar de haber bien concordado en: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_non_copulative_plural_after_singular_subject_still_corrects() {
        let corrections = match analyze_with_dictionary("El problema fueron al cine") {
            Some(c) => c,
            None => return,
        };
        let correction = corrections
            .iter()
            .find(|c| SubjectVerbAnalyzer::normalize_spanish(&c.original) == "fueron");
        assert!(
            correction.is_some(),
            "Debe seguir corrigiendo cuando no hay atributo plural posverbal: {corrections:?}"
        );
        assert_eq!(correction.unwrap().suggestion, "fue");
    }
}

