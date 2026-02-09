//! Pipeline de fases específicas del español.

use crate::dictionary::{Gender, Number, ProperNames, Trie, WordCategory};
use crate::grammar::tokenizer::TokenType;
use crate::grammar::Token;
use crate::languages::spanish::common_gender::CommonGenderAction;
use crate::languages::spanish::dequeismo::DequeismoErrorType;
use crate::languages::spanish::punctuation::PunctuationErrorType;
use crate::languages::spanish::{
    CapitalizationAnalyzer, CommonGenderAnalyzer, CompoundVerbAnalyzer, DequeismoAnalyzer,
    DiacriticAnalyzer, FossilizedPrepositionAnalyzer, HomophoneAnalyzer, ImpersonalAnalyzer,
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
            } else if tokens[correction.token_index].corrected_grammar.is_none() {
                tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
            }
        }
    }

    // Fase 6: Locuciones preposicionales fosilizadas
    let fossilized_preposition_corrections = FossilizedPrepositionAnalyzer::analyze(tokens);
    for correction in fossilized_preposition_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
        }
    }

    // Fase 7: Dequeísmo/queísmo
    let deq_corrections = DequeismoAnalyzer::analyze(tokens);
    for correction in deq_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            let suggestion = match correction.error_type {
                DequeismoErrorType::Dequeismo => "sobra".to_string(),
                DequeismoErrorType::Queismo => correction.suggestion.clone(),
            };
            tokens[correction.token_index].corrected_grammar = Some(suggestion);
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

    // Fase 12: Concordancia sujeto-verbo
    let subject_verb_corrections =
        SubjectVerbAnalyzer::analyze_with_recognizer(tokens, verb_recognizer);
    for correction in subject_verb_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 13: Concordancia de relativos
    let relative_corrections = RelativeAnalyzer::analyze_with_recognizer(tokens, verb_recognizer);
    for correction in relative_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion.clone());
        }
    }

    // Fase 14: Pleonasmos
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

    // Fase 15: Mayúsculas
    let cap_corrections = CapitalizationAnalyzer::analyze(tokens);
    for correction in cap_corrections {
        if correction.token_index < tokens.len() {
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

    // Fase 16: Puntuación
    let punct_errors = PunctuationAnalyzer::analyze(tokens);
    for error in punct_errors {
        if error.token_index < tokens.len() && tokens[error.token_index].corrected_grammar.is_none()
        {
            let suggestion = match error.error_type {
                PunctuationErrorType::MissingOpening => {
                    format!("falta {}", get_opening_sign(&error.original))
                }
                PunctuationErrorType::MissingClosing => {
                    format!("falta {}", get_closing_sign(&error.original))
                }
                PunctuationErrorType::Unbalanced => "desbalanceado".to_string(),
            };
            tokens[error.token_index].corrected_grammar = Some(suggestion);
        }
    }

    // Fase 17: Comas vocativas
    let vocative_corrections = VocativeAnalyzer::analyze(tokens);
    for correction in vocative_corrections {
        if correction.token_index < tokens.len()
            && tokens[correction.token_index].corrected_grammar.is_none()
        {
            tokens[correction.token_index].corrected_grammar = Some(correction.suggestion);
        }
    }

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
    if common_tlds.contains(&word_lower.as_str()) {
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
