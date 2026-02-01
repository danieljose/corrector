//! Soporte para múltiples idiomas
//!
//! Define el trait común para implementaciones de idiomas específicos.

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
}

/// Crea una instancia del idioma especificado
pub fn get_language(code: &str) -> Option<Box<dyn Language>> {
    match code {
        "es" | "spanish" | "español" => Some(Box::new(spanish::Spanish::new())),
        _ => None,
    }
}
