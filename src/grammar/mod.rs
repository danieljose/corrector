//! Motor de gramática
//!
//! Proporciona análisis y corrección gramatical basado en reglas.

pub mod analyzer;
pub mod rules;
pub mod tokenizer;

pub use analyzer::GrammarAnalyzer;
pub use rules::{GrammarRule, RuleAction, RuleCondition, TokenPattern};
pub use tokenizer::{has_sentence_boundary, Token, TokenType, Tokenizer};
