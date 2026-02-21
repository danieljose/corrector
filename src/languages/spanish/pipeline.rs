//! Pipeline de fases específicas del español.

use crate::dictionary::{Gender, Number, ProperNames, Trie, WordCategory};
use crate::grammar::{has_sentence_boundary, Token, TokenType};
use crate::languages::spanish::common_gender::CommonGenderAction;
use crate::languages::spanish::dequeismo::DequeismoErrorType;
use crate::languages::spanish::punctuation::PunctuationErrorType;
use crate::languages::spanish::{
    CapitalizationAnalyzer, CommonGenderAnalyzer, CompoundVerbAnalyzer, DequeismoAnalyzer,
    DiacriticAnalyzer, FossilizedPrepositionAnalyzer, GerundPosteriorityAnalyzer,
    HomophoneAnalyzer, ImperativeInfinitiveAnalyzer, ImpersonalAnalyzer,
    IrrealisConditionalAnalyzer, PleonasmAnalyzer, PronounAnalyzer, PunctuationAnalyzer,
    RelativeAnalyzer, SubjectVerbAnalyzer, VocativeAnalyzer,
};
use crate::languages::VerbFormRecognizer;

pub fn apply_spanish_corrections(
    tokens: &mut [Token],
    dictionary: &Trie,
    proper_names: &ProperNames,
    verb_recognizer: Option<&dyn VerbFormRecognizer>,
) {
    // Fase 3: Concordancia de género común con referente
    let common_gender_corrections = CommonGenderAnalyzer::analyze(tokens, dictionary, proper_names);
    for correction in common_gender_corrections {
        if correction.token_index < tokens.len() {
            match correction.action {
                CommonGenderAction::Correct(ref suggestion) => {
                    tokens[correction.token_index].corrected_grammar = Some(suggestion.clone());
                }
                CommonGenderAction::ClearCorrection => {
                    tokens[correction.token_index].corrected_grammar = None;
                }
            }
        }
    }

    // Fase 3.5: vulgarismo en pretérito 2ª persona con -s espuria
    // ("cantastes" -> "cantaste", "dijistes" -> "dijiste").
    apply_second_person_preterite_extra_s(tokens, verb_recognizer);
    // Fase 3.6: enclíticos sin tilde cuya mejor sugerencia ya es forma verbal válida
    // (digame -> dígame, escuchame -> escúchame).
    promote_enclitic_missing_accent_from_spelling(tokens, dictionary, verb_recognizer);

    // Fase 4: Tildes diacríticas
    let diacritic_corrections =
        DiacriticAnalyzer::analyze(tokens, verb_recognizer, Some(proper_names));
    for correction in diacritic_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
        }
    }

    // Fase 5: Homófonos
    let homophone_corrections = HomophoneAnalyzer::analyze(tokens);
    for correction in homophone_corrections {
        if correction.token_index < tokens.len() {
            if correction.suggestion == "sobra" {
                tokens[correction.token_index].strikethrough = true;
                tokens[correction.token_index].corrected_spelling = None;
            } else if tokens[correction.token_index].corrected_grammar.is_none() {
                tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
                tokens[correction.token_index].corrected_spelling = None;
            } else if tokens[correction.token_index]
                .corrected_grammar
                .as_deref()
                .is_some_and(|existing| {
                    should_override_with_homophone(
                        &tokens[correction.token_index],
                        existing,
                        correction.suggestion.as_str(),
                    )
                })
            {
                tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
                tokens[correction.token_index].corrected_spelling = None;
            }
        }
    }
    promote_esta_with_unknown_predicative_tail(tokens);

    // Fase 5.5: Saneamiento de participios tras auxiliar "haber".
    // En tiempos compuestos el participio es invariable y no debe recibir
    // correcciones de concordancia de género/número.
    clear_participle_agreement_after_haber(tokens);

    // Fase 6: Locuciones preposicionales fosilizadas
    let fossilized_preposition_corrections = FossilizedPrepositionAnalyzer::analyze(tokens);
    for correction in fossilized_preposition_corrections {
        if correction.token_index < tokens.len() {
            if correction.suggestion == "sobra" {
                tokens[correction.token_index].strikethrough = true;
            } else if tokens[correction.token_index].corrected_grammar.is_none() {
                tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
            }
        }
    }

    // Fase 7: Dequeísmo/queísmo
    let deq_corrections = DequeismoAnalyzer::analyze(tokens);
    for correction in deq_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            match correction.error_type {
                DequeismoErrorType::Dequeismo => {
                    tokens[correction.token_index].strikethrough = true;
                }
                DequeismoErrorType::Queismo => {
                    tokens[correction.token_index].corrected_grammar =
                        Some(correction.suggestion.clone());
                }
            }
        }
    }

    // Fase 8: Laísmo/leísmo/loísmo
    let pronoun_corrections = PronounAnalyzer::analyze(tokens);
    for correction in pronoun_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 9: Tiempos compuestos
    let compound_analyzer = CompoundVerbAnalyzer::new();
    let compound_corrections = compound_analyzer.analyze_with_recognizer(tokens, verb_recognizer);
    for correction in compound_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
            // Si ya hay corrección gramatical específica del participio/tiempo compuesto,
            // evitamos ruido duplicado de ortografía para el mismo token.
            tokens[correction.token_index].corrected_spelling = None;
        }
    }

    // Fase 10: Verbos impersonales pluralizados (haber existencial + hacer temporal)
    let impersonal_corrections = ImpersonalAnalyzer::analyze(tokens);
    for correction in impersonal_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 11: Condicional irreal tras "si" (si + condicional -> subjuntivo imperfecto)
    let irrealis_corrections = IrrealisConditionalAnalyzer::analyze(tokens, verb_recognizer);
    for correction in irrealis_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 12: Infinitivo usado como imperativo en contexto exhortativo
    let imperative_infinitive_corrections =
        ImperativeInfinitiveAnalyzer::analyze(tokens, verb_recognizer);
    for correction in imperative_infinitive_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 13: Concordancia sujeto-verbo
    let subject_verb_corrections =
        SubjectVerbAnalyzer::analyze_with_recognizer(tokens, verb_recognizer);
    for correction in subject_verb_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 14: Concordancia de relativos
    let relative_corrections = RelativeAnalyzer::analyze_with_recognizer(tokens, verb_recognizer);
    for correction in relative_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 15: Gerundio de posterioridad (patrones claros)
    let gerund_posteriority_corrections =
        GerundPosteriorityAnalyzer::analyze(tokens, verb_recognizer);
    for correction in gerund_posteriority_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
        }
    }

    // Fase 16: Pleonasmos
    let pleonasm_corrections = PleonasmAnalyzer::analyze(tokens);
    for correction in pleonasm_corrections {
        if correction.token_index < tokens.len() {
            if correction.suggestion == "sobra" {
                tokens[correction.token_index].strikethrough = true;
            } else if tokens[correction.token_index].corrected_grammar.is_none() {
                tokens[correction.token_index].corrected_grammar =
                    Some(correction.suggestion.clone());
            }
        }
    }

    // Fase 17: Mayúsculas
    let cap_corrections = CapitalizationAnalyzer::analyze(tokens);
    for correction in cap_corrections {
        if correction.token_index < tokens.len() {
            if is_compact_dotted_abbreviation_token(tokens[correction.token_index].text.as_str()) {
                continue;
            }
            if tokens[correction.token_index].strikethrough {
                continue;
            }
            if is_part_of_url(tokens, correction.token_index) {
                continue;
            }
            if let Some(existing) = tokens[correction.token_index].corrected_grammar.as_mut() {
                *existing = capitalize_if_needed(existing);
            } else {
                tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
            }
        }
    }

    // Fase 17.5: Tratamientos + nombre propio
    // "Sr. lópez", "Dr. pérez", "Don pedro" -> capitalizar el nombre siguiente.
    apply_honorific_name_capitalization(tokens);

    // Fase 18: Puntuación
    let punct_errors = PunctuationAnalyzer::analyze(tokens);
    for error in punct_errors {
        let target_idx =
            punctuation_annotation_target_index(tokens, error.error_type, error.token_index);
        if target_idx < tokens.len() && tokens[target_idx].corrected_grammar.is_none() {
            let suggestion = match error.error_type {
                PunctuationErrorType::MissingOpening => {
                    format!("falta {}", get_opening_sign(&error.original))
                }
                PunctuationErrorType::MissingClosing => {
                    format!("falta {}", get_closing_sign(&error.original))
                }
                PunctuationErrorType::Unbalanced => "desbalanceado".to_string(),
            };
            tokens[target_idx].corrected_grammar = Some(suggestion);
        }
    }

    // Fase 19: Comas vocativas
    let vocative_corrections = VocativeAnalyzer::analyze(tokens);
    for correction in vocative_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
        }
    }

    // Fase 20: Concordancia de "cuyo/cuya/cuyos/cuyas" con el sustantivo poseído.
    apply_cuyo_agreement(tokens, dictionary);

    // Fase 21: Apócope adjetival ante sustantivo singular.
    apply_apocope_before_singular_noun(tokens, dictionary);

    // Fase 22: Apocope de 'alguno/ninguno' ante sustantivo masculino singular.
    apply_apocope_alguno_ninguno_before_noun(tokens, dictionary);

    clear_determiner_corrections_with_following_noun(tokens, dictionary);
}

fn capitalize_if_needed(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_lowercase() => {
            first.to_uppercase().collect::<String>() + chars.as_str()
        }
        _ => text.to_string(),
    }
}

fn is_compact_dotted_abbreviation_token(text: &str) -> bool {
    if !text.ends_with('.') {
        return false;
    }

    let mut chars = text.chars().peekable();
    let mut chunks = 0usize;
    loop {
        let Some(letter) = chars.next() else {
            break;
        };
        if !letter.is_alphabetic() {
            return false;
        }
        let Some(dot) = chars.next() else {
            return false;
        };
        if dot != '.' {
            return false;
        }
        chunks += 1;
        if chars.peek().is_none() {
            break;
        }
    }
    chunks >= 2
}

fn normalize_spanish_simple(word: &str) -> String {
    word.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            _ => c,
        })
        .collect()
}

fn is_subject_like_for_esta(word: &str) -> bool {
    matches!(
        word,
        "yo"
            | "tu"
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
            | "todo"
            | "toda"
            | "todos"
            | "todas"
    )
}

fn is_function_word_after_esta(word: &str) -> bool {
    matches!(
        word,
        "el"
            | "la"
            | "los"
            | "las"
            | "un"
            | "una"
            | "unos"
            | "unas"
            | "mi"
            | "mis"
            | "tu"
            | "tus"
            | "su"
            | "sus"
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
            | "a"
            | "ante"
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
            | "segun"
            | "sin"
            | "sobre"
            | "tras"
            | "me"
            | "te"
            | "se"
            | "nos"
            | "os"
            | "lo"
            | "le"
            | "les"
            | "que"
            | "quien"
            | "quienes"
            | "cual"
            | "cuales"
    )
}

fn previous_word_index_in_sentence(tokens: &[Token], idx: usize) -> Option<usize> {
    if idx == 0 {
        return None;
    }
    for i in (0..idx).rev() {
        if tokens[i].token_type == TokenType::Word {
            if has_sentence_boundary(tokens, i, idx) {
                return None;
            }
            return Some(i);
        }
        if tokens[i].is_sentence_boundary() {
            return None;
        }
    }
    None
}

fn next_word_index_in_sentence(tokens: &[Token], idx: usize) -> Option<usize> {
    for i in (idx + 1)..tokens.len() {
        if tokens[i].token_type == TokenType::Word {
            if has_sentence_boundary(tokens, idx, i) {
                return None;
            }
            return Some(i);
        }
        if tokens[i].is_sentence_boundary() {
            return None;
        }
    }
    None
}

fn promote_esta_with_unknown_predicative_tail(tokens: &mut [Token]) {
    for idx in 0..tokens.len() {
        if tokens[idx].token_type != TokenType::Word || tokens[idx].corrected_grammar.is_some() {
            continue;
        }
        let token_norm = normalize_spanish_simple(tokens[idx].text.as_str());
        if !matches!(token_norm.as_str(), "esta" | "estas") {
            continue;
        }

        let Some(prev_idx) = previous_word_index_in_sentence(tokens, idx) else {
            continue;
        };
        let prev_norm = normalize_spanish_simple(tokens[prev_idx].effective_text());
        if !is_subject_like_for_esta(prev_norm.as_str()) {
            continue;
        }

        let Some(next_idx) = next_word_index_in_sentence(tokens, idx) else {
            continue;
        };
        let next = &tokens[next_idx];
        let next_norm = normalize_spanish_simple(next.effective_text());
        if is_function_word_after_esta(next_norm.as_str()) {
            continue;
        }
        if next.corrected_spelling.is_none() {
            continue;
        }

        let replacement = if token_norm == "estas" {
            "estás"
        } else {
            "está"
        };
        tokens[idx].corrected_grammar = Some(preserve_initial_case(tokens[idx].text.as_str(), replacement));
    }
}

fn should_override_with_homophone(token: &Token, existing: &str, suggestion: &str) -> bool {
    let token_norm = normalize_spanish_simple(token.text.as_str());
    if !matches!(token_norm.as_str(), "esta" | "estas") {
        return false;
    }

    let existing_norm = normalize_spanish_simple(existing);
    let suggestion_norm = normalize_spanish_simple(suggestion);
    let existing_is_demonstrative = matches!(existing_norm.as_str(), "este" | "esta" | "estos" | "estas");
    let suggestion_is_estar = matches!(suggestion_norm.as_str(), "esta" | "estas")
        && suggestion.chars().any(|c| c == 'á' || c == 'Á');

    existing_is_demonstrative && suggestion_is_estar
}

fn get_opening_sign(closing: &str) -> &'static str {
    match closing {
        "?" => "¿",
        "!" => "¡",
        _ => "¿",
    }
}

fn get_closing_sign(opening: &str) -> &'static str {
    match opening {
        "¿" => "?",
        "¡" => "!",
        _ => "?",
    }
}

fn punctuation_annotation_target_index(
    tokens: &[Token],
    error_type: PunctuationErrorType,
    token_index: usize,
) -> usize {
    if token_index >= tokens.len() {
        return token_index;
    }

    match error_type {
        PunctuationErrorType::MissingClosing => {
            // "¿Cómo estás" -> anotar en el final de la cláusula, no sobre "¿".
            let mut end = token_index;
            let mut i = token_index + 1;
            while i < tokens.len() {
                if tokens[i].is_sentence_boundary() {
                    break;
                }
                if tokens[i].token_type != TokenType::Whitespace {
                    end = i;
                }
                i += 1;
            }
            end
        }
        PunctuationErrorType::MissingOpening => {
            // "Qué bueno!" -> anotar al inicio de la cláusula, no sobre "!".
            let mut start = 0usize;
            let mut i = token_index;
            while i > 0 {
                let prev = i - 1;
                if tokens[prev].is_sentence_boundary() {
                    start = i;
                    break;
                }
                i -= 1;
            }

            for j in start..token_index {
                if tokens[j].token_type != TokenType::Whitespace {
                    return j;
                }
            }

            token_index
        }
        PunctuationErrorType::Unbalanced => token_index,
    }
}

fn is_part_of_url(tokens: &[Token], idx: usize) -> bool {
    let word = &tokens[idx].text;
    let word_lower = word.to_lowercase();

    if matches!(
        word_lower.as_str(),
        "http" | "https" | "ftp" | "www" | "mailto"
    ) {
        return true;
    }

    let common_tlds = [
        "com", "org", "net", "edu", "gov", "io", "co", "es", "mx", "ar", "cl", "pe", "ve", "ec",
        "bo", "py", "uy", "br", "uk", "de", "fr", "it", "pt", "ru", "cn", "jp", "kr", "au", "nz",
        "ca", "us", "info", "biz", "tv", "me", "app", "dev", "wiki", "html", "htm", "php", "asp",
        "jsp", "xml", "json", "css", "js",
    ];
    if common_tlds.contains(&word_lower.as_str()) && is_tld_token_in_domain_context(tokens, idx) {
        return true;
    }

    let context_range = 10;
    let start = idx.saturating_sub(context_range);
    let end = (idx + context_range).min(tokens.len());

    for i in start..end {
        let t = &tokens[i];
        if t.token_type == TokenType::Punctuation {
            if t.text == ":"
                && i + 2 < tokens.len()
                && tokens[i + 1].text == "/"
                && tokens[i + 2].text == "/"
            {
                return true;
            }
        }
        if t.token_type == TokenType::Word {
            let lower = t.text.to_lowercase();
            if lower == "http" || lower == "https" || lower == "www" {
                return true;
            }
        }
    }

    false
}

fn normalize_spanish_word(word: &str) -> String {
    word.to_lowercase()
        .replace('á', "a")
        .replace('é', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ú', "u")
        .replace('ü', "u")
}

fn clear_participle_agreement_after_haber(tokens: &mut [Token]) {
    for i in 0..tokens.len() {
        if tokens[i].token_type != TokenType::Word {
            continue;
        }
        if tokens[i].corrected_grammar.is_none() {
            continue;
        }

        let Some(prev_idx) = previous_word_index(tokens, i) else {
            continue;
        };
        let prev = normalize_spanish_word(tokens[prev_idx].effective_text());
        if !is_haber_aux_form(prev.as_str()) {
            continue;
        }

        let original = normalize_spanish_word(tokens[i].text.as_str());
        if !is_meteorological_participle_word(original.as_str()) {
            continue;
        }

        tokens[i].corrected_grammar = None;
    }
}

fn previous_word_index(tokens: &[Token], idx: usize) -> Option<usize> {
    if idx == 0 {
        return None;
    }
    for i in (0..idx).rev() {
        if tokens[i].is_sentence_boundary() {
            break;
        }
        if tokens[i].token_type == TokenType::Word {
            return Some(i);
        }
    }
    None
}

fn is_haber_aux_form(word: &str) -> bool {
    matches!(
        word,
        "haber"
            | "he"
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
            | "habra"
            | "habras"
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
            // forma vulgar corregida por homófonos
            | "haiga"
    )
}

fn is_meteorological_participle_word(word: &str) -> bool {
    matches!(
        word,
        "llovido" | "nevado" | "helado" | "granizado" | "tronado"
    )
}

fn is_tld_token_in_domain_context(tokens: &[Token], idx: usize) -> bool {
    if idx == 0 {
        return false;
    }

    let dot_idx = idx.saturating_sub(1);
    if tokens[dot_idx].token_type != TokenType::Punctuation || tokens[dot_idx].text != "." {
        return false;
    }

    if dot_idx == 0 {
        return false;
    }

    matches!(
        tokens[dot_idx - 1].token_type,
        TokenType::Word | TokenType::Number
    )
}

fn apply_honorific_name_capitalization(tokens: &mut [Token]) {
    for i in 0..tokens.len() {
        if tokens[i].token_type != TokenType::Word {
            continue;
        }

        let honorific = normalize_spanish_word(tokens[i].effective_text());
        if !is_honorific_word(honorific.as_str()) {
            continue;
        }

        let Some(name_idx) = next_word_after_optional_dot(tokens, i) else {
            continue;
        };
        let candidate = tokens[name_idx].effective_text().to_string();
        let first = candidate.chars().next();
        if first.is_none_or(|c| !c.is_alphabetic()) || first.is_some_and(|c| c.is_uppercase()) {
            continue;
        }

        let candidate_norm = normalize_spanish_word(candidate.as_str());
        if is_lowercase_name_particle(candidate_norm.as_str()) {
            continue;
        }

        if let Some(existing) = tokens[name_idx].corrected_grammar.as_mut() {
            *existing = capitalize_if_needed(existing);
        } else {
            tokens[name_idx].corrected_grammar = Some(capitalize_if_needed(candidate.as_str()));
        }
    }
}

fn next_word_after_optional_dot(tokens: &[Token], start_idx: usize) -> Option<usize> {
    let mut i = start_idx + 1;
    while i < tokens.len() && tokens[i].token_type == TokenType::Whitespace {
        i += 1;
    }
    if i < tokens.len() && tokens[i].token_type == TokenType::Punctuation && tokens[i].text == "." {
        i += 1;
        while i < tokens.len() && tokens[i].token_type == TokenType::Whitespace {
            i += 1;
        }
    }
    if i < tokens.len() && tokens[i].token_type == TokenType::Word {
        Some(i)
    } else {
        None
    }
}

fn is_honorific_word(word: &str) -> bool {
    matches!(
        word,
        "sr"
            | "sra"
            | "srta"
            | "dr"
            | "dra"
            | "don"
            | "dona"
            | "senor"
            | "senora"
            | "senorita"
            | "lic"
            | "ing"
    )
}

fn is_lowercase_name_particle(word: &str) -> bool {
    matches!(word, "de" | "del" | "la" | "las" | "los" | "y" | "e" | "da" | "do")
}

fn clear_determiner_corrections_with_following_noun(tokens: &mut [Token], dictionary: &Trie) {
    for i in 0..tokens.len() {
        if tokens[i].corrected_grammar.is_none() || tokens[i].token_type != TokenType::Word {
            continue;
        }

        if let Some(ref correction) = tokens[i].corrected_grammar {
            if correction.to_lowercase() == tokens[i].text.to_lowercase() {
                continue;
            }
        }

        let det_info = tokens[i]
            .word_info
            .as_ref()
            .or_else(|| dictionary.get(&tokens[i].text.to_lowercase()));
        let Some(det_info) = det_info else {
            continue;
        };
        if det_info.category != WordCategory::Determinante {
            continue;
        }

        let mut noun_info = None;
        for j in (i + 1)..tokens.len() {
            if tokens[j].is_sentence_boundary() {
                break;
            }
            if tokens[j].token_type != TokenType::Word {
                continue;
            }
            let info = tokens[j]
                .word_info
                .as_ref()
                .or_else(|| dictionary.get(&tokens[j].text.to_lowercase()));
            let Some(info) = info else {
                break;
            };
            match info.category {
                WordCategory::Sustantivo => {
                    noun_info = Some(info);
                    break;
                }
                WordCategory::Adjetivo | WordCategory::Determinante | WordCategory::Articulo => {
                    continue;
                }
                _ => break,
            }
        }

        let Some(noun_info) = noun_info else {
            continue;
        };
        if det_info.gender == Gender::None || noun_info.gender == Gender::None {
            continue;
        }
        if det_info.number == Number::None || noun_info.number == Number::None {
            continue;
        }

        if det_info.gender == noun_info.gender && det_info.number == noun_info.number {
            tokens[i].corrected_grammar = None;
        }
    }
}

fn apply_cuyo_agreement(tokens: &mut [Token], dictionary: &Trie) {
    let word_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, t)| (t.token_type == TokenType::Word).then_some(idx))
        .collect();

    for pos in 0..word_positions.len().saturating_sub(1) {
        let idx = word_positions[pos];
        let next_idx = word_positions[pos + 1];
        if has_sentence_boundary(tokens, idx, next_idx)
            || has_non_whitespace_between(tokens, idx, next_idx)
            || tokens[idx].corrected_grammar.is_some()
        {
            continue;
        }

        let rel = normalize_simple(tokens[idx].effective_text());
        if !matches!(rel.as_str(), "cuyo" | "cuya" | "cuyos" | "cuyas") {
            continue;
        }

        let noun_info = tokens[next_idx]
            .word_info
            .as_ref()
            .or_else(|| dictionary.get(&normalize_simple(tokens[next_idx].effective_text())));
        let Some(noun_info) = noun_info else {
            continue;
        };
        if noun_info.category != WordCategory::Sustantivo
            || noun_info.gender == Gender::None
            || noun_info.number == Number::None
        {
            continue;
        }

        let expected = match (noun_info.gender, noun_info.number) {
            (Gender::Masculine, Number::Singular) => "cuyo",
            (Gender::Feminine, Number::Singular) => "cuya",
            (Gender::Masculine, Number::Plural) => "cuyos",
            (Gender::Feminine, Number::Plural) => "cuyas",
            _ => continue,
        };

        if rel != expected {
            tokens[idx].corrected_grammar =
                Some(preserve_initial_case(tokens[idx].text.as_str(), expected));
        }
    }
}

fn apply_apocope_before_singular_noun(tokens: &mut [Token], dictionary: &Trie) {
    let word_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, t)| (t.token_type == TokenType::Word).then_some(idx))
        .collect();

    for pos in 0..word_positions.len().saturating_sub(2) {
        let det_idx = word_positions[pos];
        let adj_idx = word_positions[pos + 1];
        let noun_idx = word_positions[pos + 2];

        if has_sentence_boundary(tokens, det_idx, adj_idx)
            || has_sentence_boundary(tokens, adj_idx, noun_idx)
            || has_non_whitespace_between(tokens, det_idx, adj_idx)
            || has_non_whitespace_between(tokens, adj_idx, noun_idx)
            || tokens[adj_idx].corrected_grammar.is_some()
        {
            continue;
        }

        let det = normalize_simple(tokens[det_idx].effective_text());
        if !matches!(det.as_str(), "un" | "el" | "una" | "la") {
            continue;
        }

        let adj = normalize_simple(tokens[adj_idx].effective_text());
        let noun_info = tokens[noun_idx]
            .word_info
            .as_ref()
            .or_else(|| dictionary.get(&normalize_simple(tokens[noun_idx].effective_text())));
        let Some(noun_info) = noun_info else {
            continue;
        };
        if noun_info.category != WordCategory::Sustantivo || noun_info.number != Number::Singular {
            continue;
        }

        let expected = if adj == "grande" {
            Some("gran")
        } else if noun_info.gender == Gender::Masculine {
            match adj.as_str() {
                "bueno" => Some("buen"),
                "malo" => Some("mal"),
                "primero" => Some("primer"),
                "tercero" => Some("tercer"),
                _ => None,
            }
        } else {
            None
        };

        if let Some(expected) = expected {
            if adj != expected {
                tokens[adj_idx].corrected_grammar = Some(preserve_initial_case(
                    tokens[adj_idx].text.as_str(),
                    expected,
                ));
            }
        }
    }
}

fn apply_apocope_alguno_ninguno_before_noun(tokens: &mut [Token], dictionary: &Trie) {
    let word_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, t)| (t.token_type == TokenType::Word).then_some(idx))
        .collect();

    for pos in 0..word_positions.len().saturating_sub(1) {
        let det_idx = word_positions[pos];
        let noun_idx = word_positions[pos + 1];

        if has_sentence_boundary(tokens, det_idx, noun_idx)
            || has_non_whitespace_between(tokens, det_idx, noun_idx)
            || tokens[det_idx].corrected_grammar.is_some()
        {
            continue;
        }

        let det = normalize_simple(tokens[det_idx].effective_text());
        let expected = match det.as_str() {
            "alguno" => Some("alg\u{00FA}n"),
            "ninguno" => Some("ning\u{00FA}n"),
            _ => None,
        };
        let Some(expected) = expected else {
            continue;
        };

        let noun_info = tokens[noun_idx]
            .word_info
            .as_ref()
            .or_else(|| dictionary.get(&normalize_simple(tokens[noun_idx].effective_text())));
        let Some(noun_info) = noun_info else {
            continue;
        };
        if noun_info.category != WordCategory::Sustantivo
            || noun_info.gender != Gender::Masculine
            || noun_info.number != Number::Singular
        {
            continue;
        }

        tokens[det_idx].corrected_grammar = Some(preserve_initial_case(
            tokens[det_idx].text.as_str(),
            expected,
        ));
    }
}

fn apply_second_person_preterite_extra_s(
    tokens: &mut [Token],
    verb_recognizer: Option<&dyn VerbFormRecognizer>,
) {
    let word_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, t)| (t.token_type == TokenType::Word).then_some(idx))
        .collect();

    for pos in 0..word_positions.len() {
        let idx = word_positions[pos];
        if tokens[idx].corrected_grammar.is_some() {
            continue;
        }

        let (prev_norm, prev_raw_lower) = if pos > 0 {
            let prev_idx = word_positions[pos - 1];
            let prev_token = &tokens[prev_idx];
            if has_sentence_boundary(tokens, prev_idx, idx) {
                (None, None)
            } else {
                let raw = prev_token.effective_text().to_lowercase();
                (Some(normalize_simple(raw.as_str())), Some(raw))
            }
        } else {
            (None, None)
        };

        // Evitar leer como verbo formas nominales reales:
        // "los trastes", "unos contrastes", etc.
        let prev_is_tonic_tu = prev_raw_lower.as_deref() == Some("tú");
        let prev_is_plain_tu = prev_norm.as_deref() == Some("tu");
        let prev_is_nominal_determiner = prev_norm.as_deref().is_some_and(is_nominal_determiner)
            && !prev_is_tonic_tu;
        let prev_is_article_or_det = if pos > 0 {
            let prev_idx = word_positions[pos - 1];
            let prev_token = &tokens[prev_idx];
            prev_token.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Articulo | WordCategory::Determinante
                )
            })
        } else {
            false
        };

        // Usar texto original del token (no effective_text), porque en esta fase
        // `effective_text` puede contener ya la lista de sugerencias ortográficas.
        let word_text = tokens[idx].text.trim();
        if word_text.is_empty() {
            continue;
        }
        let normalized = normalize_simple(word_text);
        if !matches_suffix_extra_s_preterite(normalized.as_str()) {
            continue;
        }

        // Si viene de determinante/artículo, solo permitir en el caso ambiguo
        // "tu + verbo vulgar en -stes" cuando no hay lectura nominal en diccionario.
        if prev_is_nominal_determiner || prev_is_article_or_det {
            let current_is_nominal = tokens[idx].word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Sustantivo
                        | WordCategory::Adjetivo
                        | WordCategory::Articulo
                        | WordCategory::Determinante
                )
            });
            if !(prev_is_plain_tu && !current_is_nominal) {
                continue;
            }
        }

        let Some(candidate) = remove_last_char(word_text) else {
            continue;
        };
        let candidate_lower = candidate.to_lowercase();
        let candidate_norm = normalize_simple(candidate_lower.as_str());

        let has_second_person_preterite_shape =
            candidate_norm.ends_with("aste") || candidate_norm.ends_with("iste");
        let is_valid_candidate = if let Some(recognizer) = verb_recognizer {
            recognizer.is_valid_verb_form(candidate_lower.as_str())
                || recognizer.is_valid_verb_form(candidate_norm.as_str())
                || has_second_person_preterite_shape
        } else {
            has_second_person_preterite_shape
        };
        if !is_valid_candidate {
            continue;
        }

        tokens[idx].corrected_grammar = Some(preserve_initial_case(
            tokens[idx].text.as_str(),
            candidate.as_str(),
        ));
        // Evitar ruido doble "|...| [..]" cuando la regla gramatical ya resolvió la forma.
        tokens[idx].corrected_spelling = None;

        // En "tu + verbo vulgar en -stes", "tu" funciona como pronombre
        // personal de 2.ª persona y debe llevar tilde: "Tú dijiste...".
        if prev_is_plain_tu && !prev_is_tonic_tu && pos > 0 {
            let prev_idx = word_positions[pos - 1];
            if !has_sentence_boundary(tokens, prev_idx, idx)
                && tokens[prev_idx].corrected_grammar.is_none()
            {
                tokens[prev_idx].corrected_grammar = Some(preserve_initial_case(
                    tokens[prev_idx].text.as_str(),
                    "tú",
                ));
                tokens[prev_idx].corrected_spelling = None;
            }
        }
    }
}

fn promote_enclitic_missing_accent_from_spelling(
    tokens: &mut [Token],
    dictionary: &Trie,
    verb_recognizer: Option<&dyn VerbFormRecognizer>,
) {
    let Some(recognizer) = verb_recognizer else {
        return;
    };

    for token in tokens.iter_mut() {
        if !token.is_word() || token.corrected_grammar.is_some() {
            continue;
        }
        let Some(spelling_list) = token.corrected_spelling.clone() else {
            continue;
        };
        let word_lower = normalize_simple(&token.text.to_lowercase());
        if has_written_accent(&word_lower) || !looks_like_enclitic_surface(&word_lower) {
            continue;
        }

        let folded_input = fold_spanish_diacritics(&word_lower);
        let mut chosen: Option<String> = None;
        for raw in spelling_list.split(',') {
            let candidate = raw.trim();
            if candidate.is_empty() {
                continue;
            }
            let candidate_lower = candidate.to_lowercase();
            if !has_written_accent(&candidate_lower) {
                continue;
            }
            if !looks_like_enclitic_surface(&candidate_lower) {
                continue;
            }
            let Some(info) = dictionary.get(&candidate_lower) else {
                continue;
            };
            if info.category != WordCategory::Verbo || info.frequency < 300 {
                continue;
            }
            if fold_spanish_diacritics(&candidate_lower) != folded_input {
                continue;
            }
            if !recognizer.is_valid_verb_form(&candidate_lower) {
                continue;
            }
            chosen = Some(preserve_initial_case(&token.text, candidate));
            break;
        }

        if let Some(suggestion) = chosen {
            token.corrected_grammar = Some(suggestion);
            token.corrected_spelling = None;
        }
    }
}

fn matches_suffix_extra_s_preterite(word: &str) -> bool {
    (word.ends_with("astes") || word.ends_with("istes"))
        && !word.ends_with("asteis")
        && !word.ends_with("isteis")
}

fn remove_last_char(s: &str) -> Option<String> {
    let mut chars = s.chars();
    chars.next_back()?;
    Some(chars.collect())
}

fn is_nominal_determiner(word: &str) -> bool {
    matches!(
        word,
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
            | "algun"
            | "alguna"
            | "algunos"
            | "algunas"
            | "ningun"
            | "ninguna"
            | "ningunos"
            | "ningunas"
            | "mucho"
            | "mucha"
            | "muchos"
            | "muchas"
            | "poco"
            | "poca"
            | "pocos"
            | "pocas"
            | "varios"
            | "varias"
            | "todos"
            | "todas"
            | "cada"
            | "cualquier"
            | "cualesquiera"
    )
}

fn has_written_accent(word: &str) -> bool {
    word.chars().any(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ú'))
}

fn fold_spanish_diacritics(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' | 'Á' | 'À' | 'Ä' | 'Â' => 'a',
            'é' | 'è' | 'ë' | 'ê' | 'É' | 'È' | 'Ë' | 'Ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' | 'Í' | 'Ì' | 'Ï' | 'Î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'Ó' | 'Ò' | 'Ö' | 'Ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' | 'Ú' | 'Ù' | 'Ü' | 'Û' => 'u',
            _ => c.to_ascii_lowercase(),
        })
        .collect()
}

fn looks_like_enclitic_surface(word: &str) -> bool {
    const CLITICS: [&str; 11] = [
        "melo", "mela", "melos", "melas", "telo", "tela", "telos", "telas", "me", "te", "se",
    ];
    if CLITICS
        .iter()
        .any(|c| word.len() > c.len() + 1 && word.ends_with(c))
    {
        return true;
    }
    const SIMPLE: [&str; 9] = ["nos", "os", "lo", "la", "los", "las", "le", "les", "mela"];
    SIMPLE
        .iter()
        .any(|c| word.len() > c.len() + 1 && word.ends_with(c))
}
fn preserve_initial_case(original: &str, replacement: &str) -> String {
    if original
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        capitalize_if_needed(replacement)
    } else {
        replacement.to_string()
    }
}

fn has_non_whitespace_between(tokens: &[Token], left: usize, right: usize) -> bool {
    if right <= left + 1 {
        return false;
    }
    for token in tokens.iter().take(right).skip(left + 1) {
        if token.token_type != TokenType::Whitespace {
            return true;
        }
    }
    false
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
