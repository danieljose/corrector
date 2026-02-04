//! Análisis de concordancia sujeto-verbo
//!
//! Detecta errores de concordancia entre pronombres personales y verbos conjugados.
//! Ejemplo: "yo cantas" → "yo canto", "tú canto" → "tú cantas"

use crate::dictionary::trie::Number;
use crate::dictionary::WordCategory;
use crate::grammar::tokenizer::TokenType;
use crate::grammar::{has_sentence_boundary, Token};
use crate::languages::spanish::VerbRecognizer;
use crate::languages::spanish::conjugation::enclitics::EncliticsAnalyzer;
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
}

/// Sustantivos partitivos que admiten concordancia variable
/// "Un grupo de estudiantes llegó/llegaron" - ambos correctos
const PARTITIVE_NOUNS: &[&str] = &[
    "grupo", "conjunto", "serie", "mayoría", "minoría", "parte",
    "resto", "mitad", "tercio", "cuarto", "multitud", "infinidad",
    "cantidad", "sumatoria", "número", "totalidad", "porcentaje", "fracción",
    "docena", "decena", "centenar", "millar", "par",
];

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

impl SubjectVerbAnalyzer {
    /// Analiza tokens buscando errores de concordancia sujeto-verbo
    pub fn analyze(tokens: &[Token]) -> Vec<SubjectVerbCorrection> {
        Self::analyze_with_recognizer(tokens, None)
    }

    /// Analiza tokens con VerbRecognizer opcional para desambiguar gerundios y verbos homógrafos
    pub fn analyze_with_recognizer(
        tokens: &[Token],
        verb_recognizer: Option<&VerbRecognizer>,
    ) -> Vec<SubjectVerbCorrection> {
        let mut corrections = Vec::new();

        // Buscar patrones de pronombre + verbo
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        for i in 0..word_tokens.len().saturating_sub(1) {
            let (idx1, token1) = word_tokens[i];
            let (idx2, token2) = word_tokens[i + 1];

            // Verificar que no haya puntuación de fin de oración entre los dos tokens
            if has_sentence_boundary(tokens, idx1, idx2) {
                continue;
            }

            // Usar effective_text() para ver correcciones de fases anteriores (ej: diacríticas)
            let text1 = token1.effective_text();
            let text2 = token2.effective_text();

            // Verificar si el primer token es un pronombre personal sujeto
            if let Some(subject_info) = Self::get_subject_info(text1) {
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

                // Si el segundo token es un sustantivo, adjetivo o adverbio conocido, no tratarlo como verbo
                // Ejemplo: "él maravillas" - "maravillas" es sustantivo, no verbo
                // Ejemplo: "él alto" - "alto" es adjetivo, no verbo
                // Ejemplo: "él tampoco" - "tampoco" es adverbio, no verbo
                if let Some(ref info) = token2.word_info {
                    if info.category == WordCategory::Sustantivo
                        || info.category == WordCategory::Adjetivo
                        || info.category == WordCategory::Adverbio
                    {
                        // Solo continuar si el recognizer confirma que es verbo
                        let text2_lower = text2.to_lowercase();
                        let is_verb = verb_recognizer
                            .map(|vr| vr.is_valid_verb_form(&text2_lower))
                            .unwrap_or(false);
                        if !is_verb {
                            continue;
                        }
                    }
                }

                // Verificar si el segundo token es un verbo conjugado
                if let Some(correction) = Self::check_verb_agreement(
                    idx2,
                    text2,
                    &subject_info,
                    verb_recognizer,
                ) {
                    corrections.push(correction);
                }
            }
        }

        // =========================================================================
        // Análisis de sujetos nominales (sintagmas nominales complejos)
        // Ejemplo: "El Ministerio del Interior intensifica" → núcleo "Ministerio"
        // =========================================================================
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
                let mut verb_pos = word_tokens.iter().position(|(idx, _)| *idx > nominal_subject.end_idx);

                // Flag para detectar si hay un complemento comitativo plural (con/sin + plural)
                // En ese caso, la concordancia es ambigua y no debemos corregir verbo plural
                let mut has_comitative_plural = false;

                // Saltar adverbios y complementos preposicionales entre el SN y el verbo
                // Ejemplo: "El Ministerio del Interior hoy intensifica" - saltar "hoy"
                // Ejemplo: "El Ministerio del Interior en 2020 intensifica" - saltar "en 2020"
                while let Some(vp) = verb_pos {
                    if vp >= word_tokens.len() {
                        break;
                    }
                    let (_, candidate_token) = word_tokens[vp];
                    let lower = candidate_token.effective_text().to_lowercase();

                    // Si es adverbio conocido, saltar al siguiente token
                    let is_adverb = if let Some(ref info) = candidate_token.word_info {
                        info.category == WordCategory::Adverbio
                    } else {
                        // También verificar adverbios comunes sin word_info
                        Self::is_common_adverb(&lower)
                    };

                    if is_adverb {
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
                        if let Some(skip_result) = Self::skip_prepositional_phrase(&word_tokens, tokens, vp) {
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
                                    let next_is_all_uppercase = next_text.chars().any(|c| c.is_alphabetic())
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
                                            let look_is_all_uppercase = look_text.chars().any(|c| c.is_alphabetic())
                                                && look_text
                                                    .chars()
                                                    .all(|c| !c.is_alphabetic() || c.is_uppercase());
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

                    // Si el token es un sustantivo o adjetivo conocido, no tratarlo como verbo
                    // salvo en texto ALL-CAPS cuando el recognizer confirma que es forma verbal.
                    // Ejemplo: "LA POLITICA INTENSIFICA" debe permitir "INTENSIFICA" como verbo.
                    let is_all_uppercase = verb_token.text.chars().any(|c| c.is_alphabetic())
                        && verb_token.text.chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
                    if let Some(ref info) = verb_token.word_info {
                        if info.category == WordCategory::Sustantivo
                            || info.category == WordCategory::Adjetivo
                        {
                            if !is_all_uppercase {
                                continue;
                            }
                            let is_valid_verb = verb_recognizer
                                .map(|vr| vr.is_valid_verb_form(verb_text))
                                .unwrap_or(false);
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
                            .unwrap_or(false);
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
                    if let Some((verb_person, verb_number, verb_tense, infinitive)) =
                        Self::get_verb_info(&verb_lower, verb_recognizer)
                    {
                        if verb_person == subject_info.person && verb_number == subject_info.number {
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
        matches!(word,
            "a" | "ante" | "bajo" | "con" | "contra" | "de" | "desde" |
            "en" | "entre" | "hacia" | "hasta" | "para" | "por" |
            "según" | "sin" | "sobre" | "tras"
        )
    }

    /// Verifica si una palabra es un adverbio común (para saltar entre SN y verbo)
    fn is_common_adverb(word: &str) -> bool {
        matches!(word,
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

    /// Verifica si una preposición puede iniciar un complemento que debemos saltar
    fn is_skippable_preposition(word: &str) -> bool {
        matches!(word,
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
                if matches!(lower.as_str(), "los" | "las" | "unos" | "unas" |
                    "estos" | "estas" | "esos" | "esas" | "aquellos" | "aquellas" |
                    "mis" | "tus" | "sus" | "nuestros" | "nuestras" | "vuestros" | "vuestras") {
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
                || matches!(lower.as_str(),
                    // Meses, años, palabras comunes en complementos temporales
                    "enero" | "febrero" | "marzo" | "abril" | "mayo" | "junio" |
                    "julio" | "agosto" | "septiembre" | "octubre" | "noviembre" | "diciembre" |
                    "año" | "mes" | "día" | "semana" | "hora" | "momento" |
                    "de" | "del" |  // Preposiciones internas del complemento
                    "y" | "e"  // Coordinación
                )
                || token.word_info.as_ref().map(|i| i.category == WordCategory::Sustantivo).unwrap_or(false);

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
        word.ends_with("an") || word.ends_with("en") || word.ends_with("on")
            || word.ends_with("ó") || word.ends_with("aron") || word.ends_with("ieron")
            || word.ends_with("aban") || word.ends_with("ían")
            || word.ends_with("ará") || word.ends_with("erá") || word.ends_with("irá")
            || word.ends_with("arán") || word.ends_with("erán") || word.ends_with("irán")
    }

    /// Verifica si la preposición introduce cláusulas parentéticas de cita
    /// Ejemplo: "según explicó", "como indicó"
    fn is_parenthetical_preposition(word: &str) -> bool {
        matches!(word, "según" | "como")
    }

    /// Verifica si una palabra es un verbo de comunicación/percepción/opinión
    /// típico de cláusulas parentéticas de cita
    fn is_reporting_verb(word: &str) -> bool {
        matches!(word,
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

        false
    }

    /// Verifica si una palabra es determinante (artículo o demostrativo)
    fn is_determiner(word: &str) -> bool {
        let lower = word.to_lowercase();
        matches!(lower.as_str(),
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

    /// Obtiene el número gramatical de un determinante
    fn get_determiner_number(word: &str) -> GrammaticalNumber {
        let lower = word.to_lowercase();
        if matches!(lower.as_str(),
            "los" | "las" | "unos" | "unas" |
            "estos" | "estas" | "esos" | "esas" |
            "aquellos" | "aquellas"
        ) {
            GrammaticalNumber::Plural
        } else {
            GrammaticalNumber::Singular
        }
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
            if Self::is_preposition(&prev_text) && !Self::has_nonword_between(tokens, prev_idx, det_idx) {
                return None;
            }
            // Si el token anterior es un verbo (sin coma/puntuación entre ellos),
            // este SN es probablemente el objeto directo, no un nuevo sujeto
            // Ejemplo: "Los estudiantes que aprobaron el examen celebraron"
            // "el examen" es OD de "aprobaron", no sujeto de "celebraron"
            if Self::looks_like_verb(&prev_text) && !Self::has_nonword_between(tokens, prev_idx, det_idx) {
                return None;
            }
        }

        // Debe empezar con un determinante
        if !Self::is_determiner(det_text) {
            return None;
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

        // Evitar partitivos (concordancia variable)
        if PARTITIVE_NOUNS.contains(&noun_text.as_str()) {
            return None;
        }

        let mut number = Self::get_determiner_number(det_text);
        let mut end_idx = noun_idx;
        let mut has_coordination = false;

        // Coordinacion previa con nombre propio sin determinante: "Google y el Movimiento"
        if start_pos > 0 {
            let (_, prev_token) = word_tokens[start_pos - 1];
            let prev_lower = prev_token.effective_text().to_lowercase();
            if (prev_lower == "y" || prev_lower == "e")
                && Self::has_proper_name_before_conjunction(word_tokens, tokens, start_pos - 1)
            {
                has_coordination = true;
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

            // Coordinación con "y/e" → plural (solo si realmente inicia otro SN)
            if curr_text == "y" || curr_text == "e" {
                if pos + 1 >= word_tokens.len() {
                    break;
                }
                let (_, next_token) = word_tokens[pos + 1];
                let next_text = next_token.effective_text().to_lowercase();

                let mut starts_noun_phrase = Self::is_determiner(&next_text);
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

                has_coordination = true;
                end_idx = curr_idx;
                pos += 1;
                // Seguir buscando el siguiente elemento coordinado
                continue;
            }

            // Preposición "de" o contracción "del"
            if curr_text == "de" || curr_text == "del" {
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

                        // Si es artículo, avanzar
                        if Self::is_determiner(next_text) {
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
                if Self::is_determiner(&curr_text) {
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

        // Si hubo coordinación, el sujeto es plural
        if has_coordination {
            number = GrammaticalNumber::Plural;
        }

        Some(NominalSubject {
            nucleus_idx: noun_idx,
            number,
            end_idx,
            is_coordinated: has_coordination,
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
        verb_recognizer: Option<&VerbRecognizer>,
    ) -> Option<SubjectVerbCorrection> {
        let verb_lower = verb.to_lowercase();

        // Obtener información de la conjugación del verbo
        if let Some((verb_person, verb_number, verb_tense, infinitive)) =
            Self::get_verb_info(&verb_lower, verb_recognizer)
        {
            // Verificar concordancia
            if verb_person != subject.person || verb_number != subject.number {
                // Generar la forma correcta (preservando el tiempo verbal)
                if let Some(correct_form) = Self::get_correct_form(
                    &infinitive,
                    subject.person,
                    subject.number,
                    verb_tense,
                ) {
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

    /// Obtiene información de persona/número/tiempo del verbo conjugado
    /// Devuelve (persona, número, tiempo, infinitivo)
    fn get_verb_info(
        verb: &str,
        verb_recognizer: Option<&VerbRecognizer>,
    ) -> Option<(GrammaticalPerson, GrammaticalNumber, VerbTense, String)> {
        // Excluir palabras compuestas con guión (adjetivos como "ruso-colombiano")
        // Estos no son verbos y no deben tratarse como formas verbales
        if verb.contains('-') {
            return None;
        }

        // Excluir preposiciones y otras palabras que no son verbos pero podrían parecer formas verbales
        let non_verbs = [
            // Adverbios cortos que terminan en -o/-a/-e
            "no", "ya", "nunca", "ahora", "luego", "antes", "después",
            // Artículos y pronombres átonos
            "el", "la", "los", "las", "lo",
            "un", "una", "unos", "unas",
            "me", "te", "se", "nos", "os", "le", "les",
            // Participios de presente usados como sustantivos/adjetivos (-ante, -ente)
            "pensante", "amante", "estudiante", "cantante", "brillante",
            "importante", "constante", "instante", "distante", "elegante",
            "gigante", "ambulante", "abundante", "dominante", "fascinante",
            "ente", "paciente", "pendiente", "presente", "ausente",
            "consciente", "inconsciente", "evidente", "diferente", "excelente",
            // Preposiciones
            "de", "a", "en", "con", "por", "para", "sin",
            "sobre", "ante", "entre", "desde", "durante", "mediante", "según",
            "contra", "hacia", "hasta", "mediante", "tras",
            // Conjunciones y relativos
            "que", "porque", "aunque", "mientras", "donde", "como", "cuando",
            "sino", "pero", "mas", "pues", "luego",
            // Demostrativos y adjetivos que terminan en -o/-a
            "este", "ese", "aquel", "grande",
            "mismo", "misma", "mismos", "mismas",
            "otro", "otra", "otros", "otras",
            "poco", "poca", "pocos", "pocas",
            "mucho", "mucha", "muchos", "muchas",
            "tanto", "tanta", "tantos", "tantas",
            "cuanto", "cuanta", "cuantos", "cuantas",
            "todo", "toda", "todos", "todas",
            "alguno", "alguna", "algunos", "algunas",
            "ninguno", "ninguna", "ningunos", "ningunas",
            "cierto", "cierta", "ciertos", "ciertas",
            "propio", "propia", "propios", "propias",
            "solo", "sola", "solos", "solas",
            "medio", "media", "medios", "medias",
            "doble", "triple",
            // Subjuntivo imperfecto - formas comunes que se confunden con verbos -ar
            // ser/ir
            "fuera", "fueras", "fuéramos", "fuerais", "fueran",
            "fuese", "fueses", "fuésemos", "fueseis", "fuesen",
            // tener
            "tuviera", "tuvieras", "tuviéramos", "tuvierais", "tuvieran",
            "tuviese", "tuvieses", "tuviésemos", "tuvieseis", "tuviesen",
            // estar
            "estuviera", "estuvieras", "estuviéramos", "estuvierais", "estuvieran",
            "estuviese", "estuvieses", "estuviésemos", "estuvieseis", "estuviesen",
            // hacer
            "hiciera", "hicieras", "hiciéramos", "hicierais", "hicieran",
            "hiciese", "hicieses", "hiciésemos", "hicieseis", "hiciesen",
            // poder
            "pudiera", "pudieras", "pudiéramos", "pudierais", "pudieran",
            "pudiese", "pudieses", "pudiésemos", "pudieseis", "pudiesen",
            // poner
            "pusiera", "pusieras", "pusiéramos", "pusierais", "pusieran",
            "pusiese", "pusieses", "pusiésemos", "pusieseis", "pusiesen",
            // saber
            "supiera", "supieras", "supiéramos", "supierais", "supieran",
            "supiese", "supieses", "supiésemos", "supieseis", "supiesen",
            // querer
            "quisiera", "quisieras", "quisiéramos", "quisierais", "quisieran",
            "quisiese", "quisieses", "quisiésemos", "quisieseis", "quisiesen",
            // venir
            "viniera", "vinieras", "viniéramos", "vinierais", "vinieran",
            "viniese", "vinieses", "viniésemos", "vinieseis", "viniesen",
            // decir
            "dijera", "dijeras", "dijéramos", "dijerais", "dijeran",
            "dijese", "dijeses", "dijésemos", "dijeseis", "dijesen",
            // Imperfecto de ser (se confunde con verbos -ar)
            "era", "eras", "éramos", "erais", "eran",
            // Imperfecto de ir
            "iba", "ibas", "íbamos", "ibais", "iban",
            // Participios irregulares (no terminan en -ado/-ido)
            // Estos NO son formas conjugadas y no deben corregirse por concordancia sujeto-verbo
            "visto", "vista", "vistos", "vistas",       // ver
            "hecho", "hecha", "hechos", "hechas",       // hacer
            "dicho", "dicha", "dichos", "dichas",       // decir
            "puesto", "puesta", "puestos", "puestas",   // poner
            "escrito", "escrita", "escritos", "escritas", // escribir
            "abierto", "abierta", "abiertos", "abiertas", // abrir
            "vuelto", "vuelta", "vueltos", "vueltas",   // volver
            "roto", "rota", "rotos", "rotas",           // romper
            "muerto", "muerta", "muertos", "muertas",   // morir
            "cubierto", "cubierta", "cubiertos", "cubiertas", // cubrir
            "frito", "frita", "fritos", "fritas",       // freír
            "impreso", "impresa", "impresos", "impresas", // imprimir
            "preso", "presa", "presos", "presas",       // prender
            "provisto", "provista", "provistos", "provistas", // proveer
            "satisfecho", "satisfecha", "satisfechos", "satisfechas", // satisfacer
            "deshecho", "deshecha", "deshechos", "deshechas", // deshacer
            "devuelto", "devuelta", "devueltos", "devueltas", // devolver
            "resuelto", "resuelta", "resueltos", "resueltas", // resolver
            "revuelto", "revuelta", "revueltos", "revueltas", // revolver
            "absuelto", "absuelta", "absueltos", "absueltas", // absolver
            "disuelto", "disuelta", "disueltos", "disueltas", // disolver
            "envuelto", "envuelta", "envueltos", "envueltas", // envolver
            "compuesto", "compuesta", "compuestos", "compuestas", // componer
            "dispuesto", "dispuesta", "dispuestos", "dispuestas", // disponer
            "expuesto", "expuesta", "expuestos", "expuestas", // exponer
            "impuesto", "impuesta", "impuestos", "impuestas", // imponer
            "opuesto", "opuesta", "opuestos", "opuestas", // oponer
            "propuesto", "propuesta", "propuestos", "propuestas", // proponer
            "repuesto", "repuesta", "repuestos", "repuestas", // reponer
            "supuesto", "supuesta", "supuestos", "supuestas", // suponer
            "antepuesto", "antepuesta", "antepuestos", "antepuestas", // anteponer
            "pospuesto", "pospuesta", "pospuestos", "pospuestas", // posponer
            "contrapuesto", "contrapuesta", "contrapuestos", "contrapuestas", // contraponer
            "interpuesto", "interpuesta", "interpuestos", "interpuestas", // interponer
            "yuxtapuesto", "yuxtapuesta", "yuxtapuestos", "yuxtapuestas", // yuxtaponer
            "inscrito", "inscrita", "inscritos", "inscritas", // inscribir
            "descrito", "descrita", "descritos", "descritas", // describir
            "prescrito", "prescrita", "prescritos", "prescritas", // prescribir
            "proscrito", "proscrita", "proscritos", "proscritas", // proscribir
            "transcrito", "transcrita", "transcritos", "transcritas", // transcribir
            "suscrito", "suscrita", "suscritos", "suscritas", // suscribir
            "circunscrito", "circunscrita", "circunscritos", "circunscritas", // circunscribir
            "adscrito", "adscrita", "adscritos", "adscritas", // adscribir
            "manuscrito", "manuscrita", "manuscritos", "manuscritas", // manuscribir
            "entreabierto", "entreabierta", "entreabiertos", "entreabiertas", // entreabrir
            "encubierto", "encubierta", "encubiertos", "encubiertas", // encubrir
            "descubierto", "descubierta", "descubiertos", "descubiertas", // descubrir
            "recubierto", "recubierta", "recubiertos", "recubiertas", // recubrir
            "contradicho", "contradicha", "contradichos", "contradichas", // contradecir
            "predicho", "predicha", "predichos", "predichas", // predecir
            "bendito", "bendita", "benditos", "benditas", // bendecir (doble participio)
            "maldito", "maldita", "malditos", "malditas", // maldecir (doble participio)
            "rehecho", "rehecha", "rehechos", "rehechas", // rehacer
            "previsto", "prevista", "previstos", "previstas", // prever
            "revisto", "revista", "revistos", "revistas", // rever (rare)
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

        // Excluir participios usados como adjetivos (terminados en -ado/-ada/-ido/-ida y plurales)
        // "ellas unidas" - "unidas" es participio/adjetivo, no verbo conjugado
        // No deben tratarse como formas verbales conjugadas
        if verb.ends_with("ado") || verb.ends_with("ada") ||
           verb.ends_with("ados") || verb.ends_with("adas") ||
           verb.ends_with("ido") || verb.ends_with("ida") ||
           verb.ends_with("idos") || verb.ends_with("idas") {
            return None;
        }

        // Verbos irregulares comunes - ser
        match verb {
            // Presente
            "soy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "ser".to_string())),
            "eres" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "ser".to_string())),
            "es" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "ser".to_string())),
            "somos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "ser".to_string())),
            "sois" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "ser".to_string())),
            "son" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "ser".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - estar
        match verb {
            // Presente
            "estoy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "estar".to_string())),
            "estás" | "estas" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "estar".to_string())),
            "está" | "esta" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "estar".to_string())),
            "estamos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "estar".to_string())),
            "estáis" | "estais" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "estar".to_string())),
            "están" | "estan" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "estar".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - tener
        match verb {
            // Presente
            "tengo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "tener".to_string())),
            "tienes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "tener".to_string())),
            "tiene" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "tener".to_string())),
            "tenemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "tener".to_string())),
            "tenéis" | "teneis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "tener".to_string())),
            "tienen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "tener".to_string())),
            // Pretérito
            "tuve" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "tener".to_string())),
            "tuviste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "tener".to_string())),
            "tuvo" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "tener".to_string())),
            "tuvimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "tener".to_string())),
            "tuvisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "tener".to_string())),
            "tuvieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "tener".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - ir
        // NOTA: Las formas del pretérito (fui, fue, fueron) son compartidas con "ser"
        match verb {
            // Presente
            "voy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "ir".to_string())),
            "vas" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "ir".to_string())),
            "va" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "ir".to_string())),
            "vamos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "ir".to_string())),
            "vais" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "ir".to_string())),
            "van" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "ir".to_string())),
            // Pretérito (compartido con "ser")
            "fui" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "ir".to_string())),
            "fuiste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "ir".to_string())),
            "fue" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "ir".to_string())),
            "fuimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "ir".to_string())),
            "fuisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "ir".to_string())),
            "fueron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "ir".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - hacer
        match verb {
            // Presente
            "hago" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "hacer".to_string())),
            "haces" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "hacer".to_string())),
            "hace" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "hacer".to_string())),
            "hacemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "hacer".to_string())),
            "hacéis" | "haceis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "hacer".to_string())),
            "hacen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "hacer".to_string())),
            // Pretérito
            "hice" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "hacer".to_string())),
            "hiciste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "hacer".to_string())),
            "hizo" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "hacer".to_string())),
            "hicimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "hacer".to_string())),
            "hicisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "hacer".to_string())),
            "hicieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "hacer".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - poder
        match verb {
            // Presente
            "puedo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "poder".to_string())),
            "puedes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "poder".to_string())),
            "puede" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "poder".to_string())),
            "podemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "poder".to_string())),
            "podéis" | "podeis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "poder".to_string())),
            "pueden" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "poder".to_string())),
            // Pretérito
            "pude" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "poder".to_string())),
            "pudiste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "poder".to_string())),
            "pudo" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "poder".to_string())),
            "pudimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "poder".to_string())),
            "pudisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "poder".to_string())),
            "pudieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "poder".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - querer
        match verb {
            // Presente
            "quiero" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "querer".to_string())),
            "quieres" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "querer".to_string())),
            "quiere" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "querer".to_string())),
            "queremos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "querer".to_string())),
            "queréis" | "quereis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "querer".to_string())),
            "quieren" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "querer".to_string())),
            // Pretérito
            "quise" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "querer".to_string())),
            "quisiste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "querer".to_string())),
            "quiso" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "querer".to_string())),
            "quisimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "querer".to_string())),
            "quisisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "querer".to_string())),
            "quisieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "querer".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - decir
        match verb {
            // Presente
            "digo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "decir".to_string())),
            "dices" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "decir".to_string())),
            "dice" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "decir".to_string())),
            "decimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "decir".to_string())),
            "decís" | "decis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "decir".to_string())),
            "dicen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "decir".to_string())),
            // Pretérito
            "dije" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "decir".to_string())),
            "dijiste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "decir".to_string())),
            "dijo" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "decir".to_string())),
            "dijimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "decir".to_string())),
            "dijisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "decir".to_string())),
            "dijeron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "decir".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - saber
        // NOTA: Solo "sé" con tilde es verbo; "se" sin tilde es pronombre reflexivo
        match verb {
            // Presente
            "sé" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "saber".to_string())),
            "sabes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "saber".to_string())),
            "sabe" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "saber".to_string())),
            "sabemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "saber".to_string())),
            "sabéis" | "sabeis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "saber".to_string())),
            "saben" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "saber".to_string())),
            // Pretérito
            "supe" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "saber".to_string())),
            "supiste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "saber".to_string())),
            "supo" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "saber".to_string())),
            "supimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "saber".to_string())),
            "supisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "saber".to_string())),
            "supieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "saber".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - venir
        match verb {
            // Presente
            "vengo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "venir".to_string())),
            "vienes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "venir".to_string())),
            "viene" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "venir".to_string())),
            "venimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "venir".to_string())),
            "venís" | "venis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "venir".to_string())),
            "vienen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "venir".to_string())),
            // Pretérito
            "vine" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "venir".to_string())),
            "viniste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "venir".to_string())),
            "vino" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "venir".to_string())),
            "vinimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "venir".to_string())),
            "vinisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "venir".to_string())),
            "vinieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "venir".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - dar
        match verb {
            // Presente
            "doy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "dar".to_string())),
            "das" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "dar".to_string())),
            "da" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "dar".to_string())),
            "damos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "dar".to_string())),
            "dais" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "dar".to_string())),
            "dan" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "dar".to_string())),
            // Pretérito
            "di" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "dar".to_string())),
            "diste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "dar".to_string())),
            "dio" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "dar".to_string())),
            "dimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "dar".to_string())),
            "disteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "dar".to_string())),
            "dieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "dar".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - ver
        match verb {
            // Presente
            "veo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "ver".to_string())),
            "ves" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "ver".to_string())),
            "ve" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "ver".to_string())),
            "vemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "ver".to_string())),
            "veis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "ver".to_string())),
            "ven" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "ver".to_string())),
            // Pretérito
            "vi" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "ver".to_string())),
            "viste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "ver".to_string())),
            "vio" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "ver".to_string())),
            "vimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "ver".to_string())),
            "visteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "ver".to_string())),
            "vieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "ver".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - poner
        match verb {
            // Presente
            "pongo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "poner".to_string())),
            "pones" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "poner".to_string())),
            "pone" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "poner".to_string())),
            "ponemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "poner".to_string())),
            "ponéis" | "poneis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "poner".to_string())),
            "ponen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "poner".to_string())),
            // Pretérito
            "puse" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "poner".to_string())),
            "pusiste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "poner".to_string())),
            "puso" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "poner".to_string())),
            "pusimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "poner".to_string())),
            "pusisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "poner".to_string())),
            "pusieron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "poner".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - traer
        match verb {
            // Presente
            "traigo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, "traer".to_string())),
            "traes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, "traer".to_string())),
            "trae" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, "traer".to_string())),
            "traemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, "traer".to_string())),
            "traéis" | "traeis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, "traer".to_string())),
            "traen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, "traer".to_string())),
            // Pretérito
            "traje" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, "traer".to_string())),
            "trajiste" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, "traer".to_string())),
            "trajo" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, "traer".to_string())),
            "trajimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Preterite, "traer".to_string())),
            "trajisteis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, "traer".to_string())),
            "trajeron" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, "traer".to_string())),
            _ => {}
        }

        // Verbos regulares -ar (presente indicativo)
        if let Some(stem) = verb.strip_suffix("o") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Present, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("as") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("a") {
            if !stem.is_empty() && !verb.ends_with("ía") {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("amos") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("áis") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("ais") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("an") {
            if !stem.is_empty() && !verb.ends_with("ían") {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, format!("{}ar", stem)));
            }
        }

        // Verbos regulares -er (presente indicativo)
        // NOTA: Excluimos stems que terminan en "c" porque probablemente son
        // subjuntivo de verbos -zar (ej: "garantice" es subjuntivo de "garantizar",
        // no indicativo de hipotético "garanticer")
        if let Some(stem) = verb.strip_suffix("es") {
            if !stem.is_empty() && !verb.ends_with("as") && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Present, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("e") {
            if !stem.is_empty() && !verb.ends_with("a") && !verb.ends_with("ie") && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Present, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("emos") {
            if !stem.is_empty() && !stem.ends_with('c') {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("éis") {
            if !stem.is_empty() && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("eis") {
            if !stem.is_empty() && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("en") {
            if !stem.is_empty() && !verb.ends_with("an") && !verb.ends_with("ien") && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Present, format!("{}er", stem)));
            }
        }

        // Verbos regulares -ir (presente indicativo)
        if let Some(stem) = verb.strip_suffix("imos") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, VerbTense::Present, format!("{}ir", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("ís") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, format!("{}ir", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("is") {
            if !stem.is_empty() && !verb.ends_with("ais") && !verb.ends_with("eis") {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Present, format!("{}ir", stem)));
            }
        }

        // ========== PRETÉRITO REGULAR ==========

        // Verbos regulares -ar (pretérito)
        // Nota: -ó y -é llevan tilde obligatoria
        if let Some(stem) = verb.strip_suffix("aron") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("asteis") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("aste") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("ó") {
            // Solo si no termina en -ió (que sería -er/-ir)
            if !stem.is_empty() && !stem.ends_with('i') {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("é") {
            // Solo si no termina en -ié (que sería otra cosa)
            if !stem.is_empty() && !stem.ends_with('i') {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, format!("{}ar", stem)));
            }
        }

        // Verbos regulares -er/-ir (pretérito)
        // Comparten las mismas terminaciones
        if let Some(stem) = verb.strip_suffix("ieron") {
            if !stem.is_empty() {
                // Intentar determinar si es -er o -ir (difícil sin diccionario)
                // Por defecto asumimos -er
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, VerbTense::Preterite, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("isteis") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, VerbTense::Preterite, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("iste") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, VerbTense::Preterite, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("ió") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, VerbTense::Preterite, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("í") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, VerbTense::Preterite, format!("{}er", stem)));
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
        // Verbos irregulares - ser
        if infinitive == "ser" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "soy",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "eres",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "es",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "somos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "sois",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "son",
                // Pretérito (compartido con ir)
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "fui",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "fuiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "fue",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "fuimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "fuisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "fueron",
            }.to_string());
        }

        // Verbos irregulares - estar
        if infinitive == "estar" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "estoy",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "estás",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "está",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "estamos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "estáis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "están",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "estuve",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "estuviste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "estuvo",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "estuvimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "estuvisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "estuvieron",
            }.to_string());
        }

        // Verbos irregulares - tener
        if infinitive == "tener" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "tengo",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "tienes",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "tiene",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "tenemos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "tenéis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "tienen",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "tuve",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "tuviste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "tuvo",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "tuvimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "tuvisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "tuvieron",
            }.to_string());
        }

        // Verbos irregulares - ir
        if infinitive == "ir" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "voy",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "vas",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "va",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "vamos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "vais",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "van",
                // Pretérito (compartido con ser)
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "fui",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "fuiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "fue",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "fuimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "fuisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "fueron",
            }.to_string());
        }

        // Verbos irregulares - hacer
        if infinitive == "hacer" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "hago",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "haces",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "hace",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "hacemos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "hacéis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "hacen",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "hice",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "hiciste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "hizo",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "hicimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "hicisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "hicieron",
            }.to_string());
        }

        // Verbos irregulares - poder
        if infinitive == "poder" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "puedo",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "puedes",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "puede",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "podemos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "podéis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "pueden",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "pude",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "pudiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "pudo",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "pudimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "pudisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "pudieron",
            }.to_string());
        }

        // Verbos irregulares - querer
        if infinitive == "querer" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "quiero",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "quieres",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "quiere",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "queremos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "queréis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "quieren",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "quise",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "quisiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "quiso",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "quisimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "quisisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "quisieron",
            }.to_string());
        }

        // Verbos irregulares - decir
        if infinitive == "decir" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "digo",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "dices",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "dice",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "decimos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "decís",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "dicen",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "dije",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "dijiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "dijo",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "dijimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "dijisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "dijeron",
            }.to_string());
        }

        // Verbos irregulares - saber
        if infinitive == "saber" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "sé",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "sabes",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "sabe",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "sabemos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "sabéis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "saben",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "supe",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "supiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "supo",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "supimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "supisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "supieron",
            }.to_string());
        }

        // Verbos irregulares - venir
        if infinitive == "venir" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "vengo",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "vienes",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "viene",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "venimos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "venís",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "vienen",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "vine",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "viniste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "vino",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "vinimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "vinisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "vinieron",
            }.to_string());
        }

        // Verbos irregulares - dar
        if infinitive == "dar" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "doy",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "das",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "da",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "damos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "dais",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "dan",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "di",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "diste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "dio",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "dimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "disteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "dieron",
            }.to_string());
        }

        // Verbos irregulares - ver
        if infinitive == "ver" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "veo",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "ves",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "ve",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "vemos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "veis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "ven",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "vi",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "viste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "vio",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "vimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "visteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "vieron",
            }.to_string());
        }

        // Verbos irregulares - poner
        if infinitive == "poner" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "pongo",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "pones",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "pone",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "ponemos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "ponéis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "ponen",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "puse",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "pusiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "puso",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "pusimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "pusisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "pusieron",
            }.to_string());
        }

        // Verbos irregulares - traer
        if infinitive == "traer" {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => "traigo",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "traes",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "trae",
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => "traemos",
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "traéis",
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "traen",
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => "traje",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => "trajiste",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => "trajo",
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => "trajimos",
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => "trajisteis",
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => "trajeron",
            }.to_string());
        }

        // Verbos regulares -ar (solo presente por ahora)
        if let Some(stem) = infinitive.strip_suffix("ar") {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}o", stem),
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}as", stem),
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}a", stem),
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}amos", stem),
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}áis", stem),
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}an", stem),
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}é", stem),
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}aste", stem),
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}ó", stem),
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}amos", stem),
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}asteis", stem),
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}aron", stem),
            });
        }

        // Verbos regulares -er (solo presente por ahora)
        if let Some(stem) = infinitive.strip_suffix("er") {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}o", stem),
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}es", stem),
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}e", stem),
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}emos", stem),
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}éis", stem),
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}en", stem),
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}í", stem),
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}iste", stem),
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}ió", stem),
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}imos", stem),
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}isteis", stem),
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}ieron", stem),
            });
        }

        // Verbos regulares -ir
        if let Some(stem) = infinitive.strip_suffix("ir") {
            return Some(match (tense, person, number) {
                // Presente
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}o", stem),
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}es", stem),
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}e", stem),
                (VerbTense::Present, GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}imos", stem),
                (VerbTense::Present, GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}ís", stem),
                (VerbTense::Present, GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}en", stem),
                // Pretérito
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}í", stem),
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}iste", stem),
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}ió", stem),
                (VerbTense::Preterite, GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}imos", stem),
                (VerbTense::Preterite, GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}isteis", stem),
                (VerbTense::Preterite, GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}ieron", stem),
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::{DictionaryLoader, Trie};
    use crate::grammar::tokenizer::Tokenizer;

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
        assert_eq!(corrections.len(), 1, "Should detect mismatch: tú + mando (1st person)");
        assert_eq!(corrections[0].suggestion, "mandas");
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
        assert_eq!(corrections.len(), 1, "Coma después no debería afectar detección");
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
        assert!(corrections.is_empty(), "Forma correcta no debe generar corrección");
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
        assert!(corrections.is_empty(), "Pretérito correcto no debe generar corrección");
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
        let explico_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "explicó")
            .collect();
        assert!(explico_corrections.is_empty(),
            "No debe corregir 'explicó' - es verbo de cláusula parentética");
    }

    #[test]
    fn test_parenthetical_segun_dijo() {
        // "Las medidas, según dijo" - no debe corregir "dijo"
        let tokens = tokenize("Las medidas, según dijo la portavoz, mejoran la situación.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let dijo_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "dijo")
            .collect();
        assert!(dijo_corrections.is_empty(),
            "No debe corregir 'dijo' - es verbo de cláusula parentética");
    }

    #[test]
    fn test_parenthetical_como_indico() {
        // "Las cifras, como indicó el presidente, muestran mejoría"
        // No debe corregir "indicó" ni "muestran"
        let tokens = tokenize("Las cifras, como indicó el presidente, muestran mejoría.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let indico_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "indicó")
            .collect();
        let muestran_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "muestran")
            .collect();
        assert!(indico_corrections.is_empty(),
            "No debe corregir 'indicó' - es verbo de cláusula parentética");
        assert!(muestran_corrections.is_empty(),
            "No debe corregir 'muestran' - concuerda con 'cifras'");
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
        let revelan_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "revelan")
            .collect();
        assert!(revelan_corrections.is_empty(),
            "No debe corregir 'revelan' - 'el presidente' está dentro de cláusula parentética");
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
        let abandonando_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "abandonando")
            .collect();
        assert!(abandonando_corrections.is_empty(),
            "No debe corregir gerundio 'abandonando' - es forma verbal invariable");
    }

    #[test]
    fn test_gerund_comiendo_not_corrected() {
        // Gerundio con terminación -iendo
        let tokens = tokenize("estaba comiendo su cena");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let comiendo_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "comiendo")
            .collect();
        assert!(comiendo_corrections.is_empty(),
            "No debe corregir gerundio 'comiendo' - es forma verbal invariable");
    }

    #[test]
    fn test_gerund_viviendo_not_corrected() {
        // Gerundio con terminación -iendo
        let tokens = tokenize("seguía viviendo en Madrid");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let viviendo_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "viviendo")
            .collect();
        assert!(viviendo_corrections.is_empty(),
            "No debe corregir gerundio 'viviendo' - es forma verbal invariable");
    }

    #[test]
    fn test_gerund_cayendo_not_corrected() {
        // Gerundio con terminación -yendo
        let tokens = tokenize("estaba cayendo la lluvia");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let cayendo_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "cayendo")
            .collect();
        assert!(cayendo_corrections.is_empty(),
            "No debe corregir gerundio 'cayendo' - es forma verbal invariable");
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
        let satse_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "SATSE")
            .collect();
        assert!(satse_corrections.is_empty(),
            "No debe corregir el acrónimo 'SATSE' - es acrónimo, no verbo");
    }

    #[test]
    fn test_acronym_ccoo_not_corrected() {
        // Multiple acronyms after noun should not be corrected
        let tokens = tokenize("Los sindicatos CCOO y UGT firmaron el acuerdo.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        let ccoo_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "CCOO" || c.original == "UGT")
            .collect();
        assert!(ccoo_corrections.is_empty(),
            "No debe corregir acrónimos 'CCOO' y 'UGT'");
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
        assert!(correction.is_some(), "Should correct 'CANTA' in all-caps text");
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
        assert!(abre_correction.is_none(), "No debe corregir 'abre' en coordinaciÃ³n verbal");
    }



    #[test]
    fn test_coordinated_subject_no_false_singular_correction() {
        let mut tokens = tokenize("La Dirección Nacional y el Consejo de Fundadores del MAIS escogieron.");
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
        assert!(correction.is_none(), "No debe sugerir singular en sujeto coordinado");
    }

    #[test]
    fn test_date_before_subject_does_not_block_coordination() {
        let mut tokens = tokenize("El 25 de julio de 2021 la Dirección Nacional y el Consejo escogieron.");
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
        assert!(correction.is_none(), "No debe corregir verbo plural tras fecha con números");
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
        assert!(correction.is_none(), "No debe corregir verbo con sujeto propio tras coma");
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
        assert!(correction.is_none(), "No debe corregir sujeto coordinado con nombre propio");
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
        assert!(numeral_correction.is_none(), "No debe corregir número romano como verbo");
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
        assert!(correction.is_none(), "Partitivo 'sumatoria' no debe forzar singular");
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
        assert!(mauricio_correction.is_none(), "No debe corregir nombre propio como verbo");
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
        assert!(name_correction.is_none(), "No debe corregir nombre propio en aposicion");
        let verb_correction = corrections.iter().find(|c| c.original == "anunciaron");
        assert!(verb_correction.is_some(), "Debe corregir el verbo despues del nombre propio");
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
        assert!(correction.is_some(), "Debe corregir verbo capitalizado despues del SN");
    }

    #[test]
    fn test_compound_adjective_with_hyphen_not_treated_as_verb() {
        // Adjetivos compuestos con guión NO deben tratarse como verbos
        // Bug anterior: "ruso-colombiano" se trataba como verbo y se generaban
        // correcciones incorrectas como "ruso-colombiana" o "ruso-colombianan"
        let tokens = tokenize("El caso ruso-colombiano fue importante");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);

        // No debe haber correcciones para "ruso-colombiano"
        let compound_correction = corrections.iter().find(|c| c.original.contains("ruso-colombiano"));
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

        let compound_correction = corrections.iter().find(|c| c.original.contains("ruso-colombianas"));
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

}
