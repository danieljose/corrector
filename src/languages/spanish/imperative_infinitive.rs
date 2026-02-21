//! Deteccion conservadora de infinitivo usado como imperativo.
//!
//! Casos cubiertos:
//! - "¡Callar!" -> "¡Callad!"
//! - "Callar!" -> "Callad!"
//!
//! Regla conservadora:
//! - Debe ser infinitivo reconocido.
//! - Debe estar en contexto exhortativo claro:
//!   - despues de "¡", o
//!   - al inicio de oracion y con "!" de cierre en la misma oracion.

use crate::dictionary::WordCategory;
use crate::grammar::tokenizer::TokenType;
use crate::grammar::Token;
use crate::languages::VerbFormRecognizer;

#[derive(Debug, Clone)]
pub struct ImperativeInfinitiveCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

pub struct ImperativeInfinitiveAnalyzer;

impl ImperativeInfinitiveAnalyzer {
    pub fn analyze(
        tokens: &[Token],
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<ImperativeInfinitiveCorrection> {
        let mut corrections = Vec::new();

        for i in 0..tokens.len() {
            let token = &tokens[i];
            if token.token_type != TokenType::Word {
                continue;
            }

            let word = Self::token_text_for_analysis(token);
            let word_lower = word.to_lowercase();

            if !Self::is_infinitive_candidate(token, &word_lower, verb_recognizer) {
                continue;
            }

            if !Self::is_imperative_context(tokens, i) {
                continue;
            }

            let negative_context = Self::is_negative_infinitive_context(tokens, i);
            let Some(suggestion_base) = (if negative_context {
                Self::infinitive_to_vosotros_negative_imperative(&word_lower)
            } else {
                Self::infinitive_to_vosotros_imperative(&word_lower)
            }) else {
                continue;
            };

            if suggestion_base == word_lower {
                continue;
            }

            corrections.push(ImperativeInfinitiveCorrection {
                token_index: i,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, &suggestion_base),
                reason: if negative_context {
                    "Infinitivo con valor imperativo negativo: mejor forma en subjuntivo"
                        .to_string()
                } else {
                    "Infinitivo con valor imperativo: mejor forma imperativa".to_string()
                },
            });
        }

        corrections
    }

    fn is_imperative_context(tokens: &[Token], word_idx: usize) -> bool {
        let prev = Self::previous_non_whitespace_idx(tokens, word_idx);
        let after_opening_exclamation = prev
            .and_then(|idx| tokens.get(idx))
            .is_some_and(|t| t.token_type == TokenType::Punctuation && t.text == "¡");

        let at_sentence_start = prev.is_none_or(|idx| {
            let t = &tokens[idx];
            t.is_sentence_boundary() || (t.token_type == TokenType::Punctuation && t.text == "¡")
        });

        after_opening_exclamation
            || (at_sentence_start && Self::has_closing_exclamation(tokens, word_idx))
            || Self::is_negative_infinitive_context(tokens, word_idx)
    }

    fn is_negative_infinitive_context(tokens: &[Token], word_idx: usize) -> bool {
        let Some(prev_idx) = Self::previous_non_whitespace_idx(tokens, word_idx) else {
            return false;
        };
        let prev_token = &tokens[prev_idx];
        if prev_token.token_type != TokenType::Word
            || prev_token.effective_text().to_lowercase() != "no"
        {
            return false;
        }

        let prev_prev = Self::previous_non_whitespace_idx(tokens, prev_idx);
        let no_at_sentence_start = prev_prev.is_none_or(|idx| {
            let t = &tokens[idx];
            t.is_sentence_boundary()
                || (t.token_type == TokenType::Punctuation
                    && (t.text == "Â¡" || t.text == "¡" || t.text.contains('¡')))
        });

        no_at_sentence_start && Self::has_closing_exclamation(tokens, word_idx)
    }

    fn has_closing_exclamation(tokens: &[Token], word_idx: usize) -> bool {
        let mut words_checked = 0usize;
        for j in (word_idx + 1)..tokens.len() {
            let token = &tokens[j];
            if token.token_type == TokenType::Whitespace {
                continue;
            }

            if token.token_type == TokenType::Word {
                words_checked += 1;
                if words_checked > 12 {
                    return false;
                }
                continue;
            }

            if token.token_type == TokenType::Punctuation {
                if token.text == "!" {
                    return true;
                }
                if token.is_sentence_boundary() {
                    return false;
                }
            }
        }
        false
    }

    fn is_infinitive_candidate(
        token: &Token,
        word_lower: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if !Self::looks_like_infinitive(word_lower) {
            return false;
        }

        if let Some(vr) = verb_recognizer {
            if vr.knows_infinitive(word_lower) {
                return true;
            }
        }

        token
            .word_info
            .as_ref()
            .is_some_and(|info| info.category == WordCategory::Verbo)
    }

    fn looks_like_infinitive(word_lower: &str) -> bool {
        if word_lower.len() < 2 {
            return false;
        }

        if !word_lower.chars().all(|c| c.is_alphabetic()) {
            return false;
        }

        word_lower.ends_with("ar") || word_lower.ends_with("er") || word_lower.ends_with("ir")
    }

    fn infinitive_to_vosotros_imperative(word_lower: &str) -> Option<String> {
        if let Some(stem) = word_lower.strip_suffix("ar") {
            return Some(format!("{stem}ad"));
        }
        if let Some(stem) = word_lower.strip_suffix("er") {
            return Some(format!("{stem}ed"));
        }
        if let Some(stem) = word_lower.strip_suffix("ir") {
            return Some(format!("{stem}id"));
        }
        None
    }

    fn infinitive_to_vosotros_negative_imperative(word_lower: &str) -> Option<String> {
        if let Some(stem) = word_lower.strip_suffix("ar") {
            return Some(format!("{stem}éis"));
        }
        if let Some(stem) = word_lower.strip_suffix("er") {
            return Some(format!("{stem}áis"));
        }
        if let Some(stem) = word_lower.strip_suffix("ir") {
            return Some(format!("{stem}áis"));
        }
        None
    }

    fn previous_non_whitespace_idx(tokens: &[Token], start: usize) -> Option<usize> {
        if start == 0 {
            return None;
        }
        (0..start)
            .rev()
            .find(|&i| tokens[i].token_type != TokenType::Whitespace)
    }

    fn token_text_for_analysis(token: &Token) -> &str {
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
    use crate::dictionary::{Gender, Number, Trie, WordCategory, WordInfo};
    use crate::grammar::Tokenizer;
    use crate::languages::spanish::VerbRecognizer;

    fn build_recognizer(infinitives: &[&str]) -> VerbRecognizer {
        let mut trie = Trie::new();
        for inf in infinitives {
            trie.insert(
                inf,
                WordInfo {
                    category: WordCategory::Verbo,
                    gender: Gender::None,
                    number: Number::None,
                    extra: String::new(),
                    frequency: 100,
                },
            );
        }
        VerbRecognizer::from_dictionary(&trie)
    }

    fn analyze_text(text: &str, infinitives: &[&str]) -> Vec<ImperativeInfinitiveCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        let recognizer = build_recognizer(infinitives);
        ImperativeInfinitiveAnalyzer::analyze(&tokens, Some(&recognizer))
    }

    #[test]
    fn test_detects_opening_exclamation_infinitive() {
        let corrections = analyze_text("¡Callar!", &["callar"]);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original.to_lowercase(), "callar");
        assert_eq!(corrections[0].suggestion, "Callad");
    }

    #[test]
    fn test_detects_sentence_start_with_closing_exclamation() {
        let corrections = analyze_text("Callar!", &["callar"]);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Callad");
    }

    #[test]
    fn test_no_correction_outside_imperative_context() {
        let corrections = analyze_text("Callar es dificil", &["callar"]);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_no_correction_when_not_sentence_start() {
        let corrections = analyze_text("Debes callar!", &["callar"]);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_no_correction_if_already_imperative() {
        let corrections = analyze_text("¡Callad!", &["callar"]);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_detects_negative_infinitive_imperative() {
        let corrections = analyze_text("¡No gritar!", &["gritar"]);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "gritéis");
    }

    #[test]
    fn test_no_negative_imperative_without_exclamation_context() {
        let corrections = analyze_text("No gritar es la norma", &["gritar"]);
        assert!(corrections.is_empty());
    }
}
