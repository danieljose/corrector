//! Implementación del idioma español

pub mod capitalization;
pub mod compound;
pub mod conjugation;
pub mod dequeismo;
pub mod diacritics;
pub mod exceptions;
pub mod homophone;
pub mod pleonasm;
pub mod pronoun;
pub mod punctuation;
pub mod relative;
pub mod rules;
pub mod subject_verb;
pub mod vocative;

pub use capitalization::CapitalizationAnalyzer;
pub use compound::CompoundVerbAnalyzer;
pub use conjugation::VerbRecognizer;
pub use dequeismo::DequeismoAnalyzer;
pub use diacritics::DiacriticAnalyzer;
pub use homophone::HomophoneAnalyzer;
pub use pleonasm::PleonasmAnalyzer;
pub use pronoun::PronounAnalyzer;
pub use punctuation::PunctuationAnalyzer;
pub use relative::RelativeAnalyzer;
pub use subject_verb::SubjectVerbAnalyzer;
pub use vocative::VocativeAnalyzer;

use crate::dictionary::{Gender, Number};
use crate::grammar::{GrammarRule, Token};
use crate::languages::Language;

pub struct Spanish {
    exceptions: std::collections::HashSet<String>,
}

impl Spanish {
    pub fn new() -> Self {
        Self {
            exceptions: exceptions::get_exceptions(),
        }
    }
}

impl Language for Spanish {
    fn code(&self) -> &str {
        "es"
    }

    fn name(&self) -> &str {
        "Español"
    }

    fn grammar_rules(&self) -> Vec<GrammarRule> {
        rules::get_spanish_rules()
    }

    fn check_gender_agreement(&self, token1: &Token, token2: &Token) -> bool {
        match (&token1.word_info, &token2.word_info) {
            (Some(info1), Some(info2)) => {
                // Si alguno no tiene género definido, asumimos concordancia
                if info1.gender == Gender::None || info2.gender == Gender::None {
                    return true;
                }

                // Excepción especial: sustantivos femeninos que usan "el" (agua, alma, etc.)
                if exceptions::uses_el_with_feminine(&token2.text) {
                    let article_lower = token1.text.to_lowercase();
                    // "el/un" + agua = correcto
                    if article_lower == "el" || article_lower == "un" {
                        return true;
                    }
                    // "la/una" + agua = incorrecto (debe ser "el/un")
                    if article_lower == "la" || article_lower == "una" {
                        return false;
                    }
                }

                info1.gender == info2.gender
            }
            _ => true, // Sin información, asumimos correcto
        }
    }

    fn check_number_agreement(&self, token1: &Token, token2: &Token) -> bool {
        // Sustantivos invariables (virus, crisis, análisis, etc.) no generan error de número
        // En concordancia art-sust: token1=artículo, token2=sustantivo
        // En concordancia sust-adj: token1=sustantivo, token2=adjetivo
        // Verificamos ambos por si acaso
        if exceptions::is_invariable_noun(&token1.text) || exceptions::is_invariable_noun(&token2.text) {
            return true;
        }

        match (&token1.word_info, &token2.word_info) {
            (Some(info1), Some(info2)) => {
                if info1.number == Number::None || info2.number == Number::None {
                    return true;
                }
                info1.number == info2.number
            }
            _ => true,
        }
    }

    fn get_correct_article(&self, gender: Gender, number: Number, definite: bool) -> &str {
        match (definite, gender, number) {
            (true, Gender::Masculine, Number::Singular) => "el",
            (true, Gender::Masculine, Number::Plural) => "los",
            (true, Gender::Feminine, Number::Singular) => "la",
            (true, Gender::Feminine, Number::Plural) => "las",
            (false, Gender::Masculine, Number::Singular) => "un",
            (false, Gender::Masculine, Number::Plural) => "unos",
            (false, Gender::Feminine, Number::Singular) => "una",
            (false, Gender::Feminine, Number::Plural) => "unas",
            _ => "",
        }
    }

    fn get_adjective_form(
        &self,
        adjective: &str,
        gender: Gender,
        number: Number,
    ) -> Option<String> {
        let adj_lower = adjective.to_lowercase();

        // Detectar tipo de adjetivo por su terminación
        let last_char = adj_lower.chars().last()?;

        // Adjetivos invariables en género (terminan en -e, -es, o consonante)
        // Solo cambian en número: interesante/interesantes, amable/amables, fácil/fáciles
        // También: adicional/adicionales, especial/especiales
        let is_invariable_gender = last_char == 'e'
            || !matches!(last_char, 'a' | 'o' | 's')
            || (adj_lower.ends_with("es") && !adj_lower.ends_with("os") && !adj_lower.ends_with("as")
                && !adj_lower.ends_with("eses")); // Excluir casos como "intereses"

        if is_invariable_gender {
            // Obtener base singular del adjetivo
            // "interesantes" -> "interesante" (quitar solo 's')
            // "adicionales" -> "adicional" (quitar 'es')
            // "fáciles" -> "fácil" (quitar 'es')
            // "capaces" -> "capaz" (cambio ortográfico c->z)
            let base = if adj_lower.ends_with("ces") {
                // Cambio ortográfico: capaces -> capaz, felices -> feliz
                let without_ces = &adj_lower[..adj_lower.len() - 3];
                format!("{}z", without_ces)
            } else if adj_lower.ends_with("es") {
                let without_es = &adj_lower[..adj_lower.len() - 2];
                // Si la raíz sin "es" termina en vocal, el singular debería terminar en 'e'
                // Ejemplo: "interesant" + "e" = "interesante"
                let last = without_es.chars().last();
                if last.map(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')).unwrap_or(false) {
                    // La raíz termina en vocal, añadir 'e' para el singular
                    format!("{}e", without_es)
                } else {
                    // La raíz termina en consonante, usar tal cual
                    without_es.to_string()
                }
            } else if adj_lower.ends_with('s') {
                adj_lower.trim_end_matches('s').to_string()
            } else {
                adj_lower.clone()
            };

            // Estos adjetivos no cambian de género, solo de número
            return match number {
                Number::Singular => Some(base),
                Number::Plural => {
                    // Añadir 's' si termina en vocal, 'es' si termina en consonante
                    // Cambio ortográfico z->c antes de 'es': capaz -> capaces
                    if base.ends_with('z') {
                        let without_z = &base[..base.len() - 1];
                        Some(format!("{}ces", without_z))
                    } else {
                        let last = base.chars().last();
                        if last.map(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')).unwrap_or(false) {
                            Some(format!("{}s", base))
                        } else {
                            Some(format!("{}es", base))
                        }
                    }
                }
                _ => None,
            };
        }

        // Adjetivos regulares que cambian en género y número (bueno/buena/buenos/buenas)
        // Quitar terminación o/a/os/as para obtener la base
        let base = if adj_lower.ends_with("os") || adj_lower.ends_with("as") {
            &adj_lower[..adj_lower.len() - 2]
        } else if adj_lower.ends_with('o') || adj_lower.ends_with('a') {
            &adj_lower[..adj_lower.len() - 1]
        } else {
            &adj_lower
        };

        let suffix = match (gender, number) {
            (Gender::Masculine, Number::Singular) => "o",
            (Gender::Masculine, Number::Plural) => "os",
            (Gender::Feminine, Number::Singular) => "a",
            (Gender::Feminine, Number::Plural) => "as",
            _ => return None,
        };

        Some(format!("{}{}", base, suffix))
    }

    fn get_correct_determiner(&self, determiner: &str, gender: Gender, number: Number) -> Option<String> {
        let det_lower = determiner.to_lowercase();

        // Identificar el tipo de determinante y devolver la forma correcta

        // Determinantes demostrativos - este/esta/estos/estas
        if det_lower == "este" || det_lower == "esta" || det_lower == "estos" || det_lower == "estas" {
            return Some(match (gender, number) {
                (Gender::Masculine, Number::Singular) => "este",
                (Gender::Feminine, Number::Singular) => "esta",
                (Gender::Masculine, Number::Plural) => "estos",
                (Gender::Feminine, Number::Plural) => "estas",
                _ => return None,
            }.to_string());
        }

        // Determinantes demostrativos - ese/esa/esos/esas
        if det_lower == "ese" || det_lower == "esa" || det_lower == "esos" || det_lower == "esas" {
            return Some(match (gender, number) {
                (Gender::Masculine, Number::Singular) => "ese",
                (Gender::Feminine, Number::Singular) => "esa",
                (Gender::Masculine, Number::Plural) => "esos",
                (Gender::Feminine, Number::Plural) => "esas",
                _ => return None,
            }.to_string());
        }

        // Determinantes demostrativos - aquel/aquella/aquellos/aquellas
        if det_lower == "aquel" || det_lower == "aquella" || det_lower == "aquellos" || det_lower == "aquellas" {
            return Some(match (gender, number) {
                (Gender::Masculine, Number::Singular) => "aquel",
                (Gender::Feminine, Number::Singular) => "aquella",
                (Gender::Masculine, Number::Plural) => "aquellos",
                (Gender::Feminine, Number::Plural) => "aquellas",
                _ => return None,
            }.to_string());
        }

        // Determinantes posesivos - nuestro/nuestra/nuestros/nuestras
        if det_lower == "nuestro" || det_lower == "nuestra" || det_lower == "nuestros" || det_lower == "nuestras" {
            return Some(match (gender, number) {
                (Gender::Masculine, Number::Singular) => "nuestro",
                (Gender::Feminine, Number::Singular) => "nuestra",
                (Gender::Masculine, Number::Plural) => "nuestros",
                (Gender::Feminine, Number::Plural) => "nuestras",
                _ => return None,
            }.to_string());
        }

        // Determinantes posesivos - vuestro/vuestra/vuestros/vuestras
        if det_lower == "vuestro" || det_lower == "vuestra" || det_lower == "vuestros" || det_lower == "vuestras" {
            return Some(match (gender, number) {
                (Gender::Masculine, Number::Singular) => "vuestro",
                (Gender::Feminine, Number::Singular) => "vuestra",
                (Gender::Masculine, Number::Plural) => "vuestros",
                (Gender::Feminine, Number::Plural) => "vuestras",
                _ => return None,
            }.to_string());
        }

        // Determinantes invariables en género (mi, tu, su, etc.) - no se corrigen aquí
        None
    }

    fn is_exception(&self, word: &str) -> bool {
        self.exceptions.contains(&word.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_correct_determiner_este_to_esta() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("este", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("esta".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_esta_to_este() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("esta", Gender::Masculine, Number::Singular);
        assert_eq!(result, Some("este".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_este_to_estos() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("este", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("estos".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_este_to_estas() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("este", Gender::Feminine, Number::Plural);
        assert_eq!(result, Some("estas".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_ese_to_esa() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("ese", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("esa".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_esa_to_esos() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("esa", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("esos".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_aquel_to_aquella() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("aquel", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("aquella".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_aquella_to_aquellos() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("aquella", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("aquellos".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_nuestro_to_nuestra() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("nuestro", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("nuestra".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_nuestra_to_nuestros() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("nuestra", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("nuestros".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_vuestro_to_vuestra() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("vuestro", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("vuestra".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_invariable_returns_none() {
        let spanish = Spanish::new();
        // Determinantes invariables como "mi", "tu", "su" no se corrigen
        let result = spanish.get_correct_determiner("mi", Gender::Feminine, Number::Singular);
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_correct_determiner_preserves_when_correct() {
        let spanish = Spanish::new();
        // "esta" con género femenino singular debería devolver "esta"
        let result = spanish.get_correct_determiner("esta", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("esta".to_string()));
    }
}
