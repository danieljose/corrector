//! Validación de puntuación
//!
//! Detecta errores de emparejamiento en signos de puntuación:
//! - ¿? (signos de interrogación)
//! - ¡! (signos de exclamación)

use crate::grammar::{Token, TokenType};

/// Error de puntuación detectado
#[derive(Debug, Clone)]
pub struct PunctuationError {
    pub token_index: usize,
    pub original: String,
    pub error_type: PunctuationErrorType,
    pub message: String,
}

/// Tipos de errores de puntuación
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PunctuationErrorType {
    /// Signo de apertura sin cierre (¿ sin ?)
    MissingClosing,
    /// Signo de cierre sin apertura (? sin ¿)
    MissingOpening,
    /// Signos desbalanceados (más de uno sin cerrar)
    Unbalanced,
}

/// Pares de signos de puntuación
#[derive(Debug, Clone, Copy)]
struct PunctuationPair {
    opening: char,
    closing: char,
}

const PUNCTUATION_PAIRS: &[PunctuationPair] = &[
    PunctuationPair {
        opening: '¿',
        closing: '?',
    },
    PunctuationPair {
        opening: '¡',
        closing: '!',
    },
];

/// Analizador de puntuación
pub struct PunctuationAnalyzer;

impl PunctuationAnalyzer {
    /// Analiza los tokens y detecta errores de puntuación
    pub fn analyze(tokens: &[Token]) -> Vec<PunctuationError> {
        let mut errors = Vec::new();

        for pair in PUNCTUATION_PAIRS {
            let pair_errors = Self::check_pair(tokens, pair);
            errors.extend(pair_errors);
        }

        errors
    }

    fn check_pair(tokens: &[Token], pair: &PunctuationPair) -> Vec<PunctuationError> {
        let mut errors = Vec::new();
        let mut opening_stack: Vec<usize> = Vec::new();

        for (idx, token) in tokens.iter().enumerate() {
            if token.token_type != TokenType::Punctuation {
                continue;
            }

            let ch = token.text.chars().next().unwrap_or(' ');

            if ch == pair.opening {
                opening_stack.push(idx);
            } else if ch == pair.closing {
                if opening_stack.pop().is_none() {
                    // Cierre sin apertura
                    errors.push(PunctuationError {
                        token_index: idx,
                        original: token.text.clone(),
                        error_type: PunctuationErrorType::MissingOpening,
                        message: format!(
                            "Signo '{}' sin '{}' de apertura",
                            pair.closing, pair.opening
                        ),
                    });
                }
            }
        }

        // Aperturas sin cierre
        for open_idx in opening_stack {
            errors.push(PunctuationError {
                token_index: open_idx,
                original: tokens[open_idx].text.clone(),
                error_type: PunctuationErrorType::MissingClosing,
                message: format!(
                    "Signo '{}' sin '{}' de cierre",
                    pair.opening, pair.closing
                ),
            });
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<PunctuationError> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        PunctuationAnalyzer::analyze(&tokens)
    }

    #[test]
    fn test_correct_question() {
        let errors = analyze_text("¿Cómo estás?");
        assert!(errors.is_empty(), "No debería haber errores en pregunta correcta");
    }

    #[test]
    fn test_correct_exclamation() {
        let errors = analyze_text("¡Hola mundo!");
        assert!(errors.is_empty(), "No debería haber errores en exclamación correcta");
    }

    #[test]
    fn test_missing_opening_question() {
        let errors = analyze_text("Cómo estás?");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, PunctuationErrorType::MissingOpening);
        assert_eq!(errors[0].original, "?");
    }

    #[test]
    fn test_missing_closing_question() {
        let errors = analyze_text("¿Cómo estás");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, PunctuationErrorType::MissingClosing);
        assert_eq!(errors[0].original, "¿");
    }

    #[test]
    fn test_missing_opening_exclamation() {
        let errors = analyze_text("Hola mundo!");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, PunctuationErrorType::MissingOpening);
        assert_eq!(errors[0].original, "!");
    }

    #[test]
    fn test_missing_closing_exclamation() {
        let errors = analyze_text("¡Hola mundo");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].error_type, PunctuationErrorType::MissingClosing);
        assert_eq!(errors[0].original, "¡");
    }

    #[test]
    fn test_multiple_correct_questions() {
        let errors = analyze_text("¿Cómo te llamas? ¿De dónde eres?");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_nested_exclamation_question() {
        // En español es válido tener interrogación dentro de exclamación
        let errors = analyze_text("¡¿Qué haces?!");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_mixed_errors() {
        let errors = analyze_text("Hola? ¿Qué tal!");
        // "Hola?" -> falta ¿
        // "¿Qué tal!" -> falta ? y sobra ¡ (o falta cierre)
        assert!(errors.len() >= 2);
    }

    #[test]
    fn test_correct_combined() {
        let errors = analyze_text("¿Cómo estás? ¡Muy bien!");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_only_closing_marks() {
        let errors = analyze_text("Hola? Mundo!");
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|e| e.error_type == PunctuationErrorType::MissingOpening));
    }

    #[test]
    fn test_only_opening_marks() {
        let errors = analyze_text("¿Hola ¡Mundo");
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|e| e.error_type == PunctuationErrorType::MissingClosing));
    }
}
