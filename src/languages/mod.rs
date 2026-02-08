//! Soporte para múltiples idiomas
//!
//! Define el trait común para implementaciones de idiomas específicos.

pub mod catalan;
pub mod spanish;

use crate::dictionary::{Gender, Number};
use crate::grammar::{GrammarRule, Token};

/// Trait que define las capacidades requeridas para un idioma
pub trait Language {
    /// Código del idioma (ej: "es", "en")
    fn code(&self) -> &str;

    /// Nombre del idioma
    fn name(&self) -> &str;

    /// Obtiene las reglas gramaticales del idioma
    fn grammar_rules(&self) -> Vec<GrammarRule>;

    /// Verifica concordancia de género entre dos tokens
    fn check_gender_agreement(&self, token1: &Token, token2: &Token) -> bool;

    /// Verifica concordancia de número entre dos tokens
    fn check_number_agreement(&self, token1: &Token, token2: &Token) -> bool;

    /// Obtiene el artículo correcto para un sustantivo
    fn get_correct_article(&self, gender: Gender, number: Number, definite: bool) -> &str;

    /// Obtiene el artículo correcto considerando el sustantivo específico
    /// Esto permite manejar excepciones como "el agua" (femenino con artículo masculino)
    fn get_correct_article_for_noun(&self, _noun: &str, gender: Gender, number: Number, definite: bool) -> String {
        // Implementación por defecto: usar el método básico
        self.get_correct_article(gender, number, definite).to_string()
    }

    /// Obtiene la forma correcta de un adjetivo
    fn get_adjective_form(&self, adjective: &str, gender: Gender, number: Number) -> Option<String>;

    /// Obtiene la forma correcta de un determinante
    fn get_correct_determiner(&self, determiner: &str, gender: Gender, number: Number) -> Option<String>;

    /// Verifica si una palabra es una excepción conocida
    fn is_exception(&self, word: &str) -> bool;

    /// Heurística: ¿parece forma verbal aunque no esté en el diccionario?
    /// Se usa como fallback en la fase de ortografía para evitar marcar formas
    /// verbales legítimas de verbos ausentes del diccionario.
    fn is_likely_verb_form_in_context(&self, _word: &str, _tokens: &[Token], _index: usize) -> bool {
        false
    }

    /// ¿Es una abreviatura convencional del idioma? (ej: "n.º" en español)
    fn is_known_abbreviation(&self, _word: &str) -> bool {
        false
    }

    /// Analiza un artículo y devuelve (definido, número, género)
    fn article_features(&self, _article: &str) -> Option<(bool, Number, Gender)> {
        None
    }

    /// Analiza un determinante y devuelve (familia, número, género)
    fn determiner_features(&self, _determiner: &str) -> Option<(&str, Number, Gender)> {
        None
    }

    /// ¿Es esta palabra una preposición que introduce complementos nominales?
    fn is_preposition(&self, _word: &str) -> bool {
        false
    }

    /// ¿Es esta palabra una forma de participio?
    fn is_participle_form(&self, _word: &str) -> bool {
        false
    }

    /// ¿Es este sustantivo de género común (incluye formas plurales)?
    fn is_common_gender_noun_form(&self, _noun: &str) -> bool {
        false
    }

    /// ¿Admite este sustantivo artículos de ambos géneros por ambigüedad semántica?
    fn allows_both_gender_articles(&self, _word: &str) -> bool {
        false
    }

    /// ¿Es esta palabra una conjunción coordinante?
    fn is_conjunction(&self, _word: &str) -> bool {
        false
    }

    /// ¿Es esta palabra un sustantivo temporal?
    fn is_time_noun(&self, _word: &str) -> bool {
        false
    }

    /// ¿Es este adjetivo predicativo (no debe corregirse en concordancia con sustantivo anterior)?
    fn is_predicative_adjective(&self, _word: &str) -> bool {
        false
    }
}

/// Crea una instancia del idioma especificado
pub fn get_language(code: &str) -> Option<Box<dyn Language>> {
    match code {
        "es" | "spanish" | "español" => Some(Box::new(spanish::Spanish::new())),
        "ca" | "catalan" | "català" => Some(Box::new(catalan::Catalan::new())),
        _ => None,
    }
}
