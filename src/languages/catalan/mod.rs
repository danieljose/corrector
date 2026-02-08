//! Implementación del idioma catalán (stub ortográfico)

use crate::dictionary::{Gender, Number};
use crate::grammar::{GrammarRule, Token};
use crate::languages::Language;

pub struct Catalan;

impl Catalan {
    pub fn new() -> Self {
        Self
    }
}

impl Language for Catalan {
    fn code(&self) -> &str {
        "ca"
    }

    fn name(&self) -> &str {
        "Català"
    }

    fn grammar_rules(&self) -> Vec<GrammarRule> {
        vec![]
    }

    fn check_gender_agreement(&self, _token1: &Token, _token2: &Token) -> bool {
        true
    }

    fn check_number_agreement(&self, _token1: &Token, _token2: &Token) -> bool {
        true
    }

    fn get_correct_article(&self, _gender: Gender, _number: Number, _definite: bool) -> &str {
        ""
    }

    fn get_adjective_form(&self, _adjective: &str, _gender: Gender, _number: Number) -> Option<String> {
        None
    }

    fn get_correct_determiner(&self, _determiner: &str, _gender: Gender, _number: Number) -> Option<String> {
        None
    }

    fn is_exception(&self, _word: &str) -> bool {
        false
    }
}
