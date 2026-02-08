//! Detección de haber impersonal pluralizado.
//!
//! El verbo "haber" en uso existencial/impersonal debe ir siempre en
//! 3.ª persona del singular:
//!
//! - "habían muchas personas" → "había muchas personas"
//! - "hubieron accidentes" → "hubo accidentes"
//! - "habrán problemas" → "habrá problemas"
//! - "han habido quejas" → "ha habido quejas"

use crate::grammar::tokenizer::TokenType;
use crate::grammar::{has_sentence_boundary, Token};

/// Corrección de haber impersonal
#[derive(Debug, Clone)]
pub struct ImpersonalCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
}

/// Tabla de formas plurales de haber → forma singular correcta (impersonal).
///
/// Solo incluimos formas donde la pluralización es inequívocamente un error:
/// la forma singular existe y la forma plural no es auxiliar legítimo sin contexto.
const PLURAL_TO_SINGULAR: &[(&str, &str)] = &[
    // Imperfecto indicativo
    ("habían", "había"),
    // Pretérito indefinido
    ("hubieron", "hubo"),
    // Futuro
    ("habrán", "habrá"),
    // Condicional
    ("habrían", "habría"),
    // Subjuntivo presente
    ("hayan", "haya"),
    // Subjuntivo imperfecto (-ra)
    ("hubieran", "hubiera"),
    // Subjuntivo imperfecto (-se)
    ("hubiesen", "hubiese"),
];

/// Formas plurales que son ambiguas: "han" es correcto como auxiliar
/// ("han comido") pero incorrecto como existencial ("han habido quejas").
/// Solo se corrigen cuando van seguidas de "habido".
const AMBIGUOUS_PLURAL: &[(&str, &str)] = &[("han", "ha")];

pub struct ImpersonalAnalyzer;

impl ImpersonalAnalyzer {
    /// Analiza tokens y detecta haber impersonal pluralizado.
    pub fn analyze(tokens: &[Token]) -> Vec<ImpersonalCorrection> {
        let mut corrections = Vec::new();

        for i in 0..tokens.len() {
            if tokens[i].token_type != TokenType::Word {
                continue;
            }

            let word_lower = tokens[i].effective_text().to_lowercase();

            // Caso 1: formas inequívocamente plurales (habían, hubieron, habrán...)
            if let Some(singular) = Self::get_singular_for(&word_lower) {
                if Self::is_followed_by_nominal(tokens, i) {
                    corrections.push(ImpersonalCorrection {
                        token_index: i,
                        original: tokens[i].text.clone(),
                        suggestion: Self::preserve_case(&tokens[i].text, singular),
                    });
                }
            }

            // Caso 2: "han/habían habido + SN" → compuesto existencial pluralizado
            if let Some(singular) = Self::get_ambiguous_singular(&word_lower) {
                if let Some(habido_idx) = Self::find_habido_after(tokens, i) {
                    if Self::is_followed_by_nominal(tokens, habido_idx) {
                        corrections.push(ImpersonalCorrection {
                            token_index: i,
                            original: tokens[i].text.clone(),
                            suggestion: Self::preserve_case(&tokens[i].text, singular),
                        });
                    }
                }
            }

            // Caso 2b: formas inequívocas + "habido" (e.g. "habían habido quejas")
            if let Some(singular) = Self::get_singular_for(&word_lower) {
                if let Some(habido_idx) = Self::find_habido_after(tokens, i) {
                    if Self::is_followed_by_nominal(tokens, habido_idx) {
                        // Solo añadir si no se añadió ya en caso 1
                        if !corrections.iter().any(|c| c.token_index == i) {
                            corrections.push(ImpersonalCorrection {
                                token_index: i,
                                original: tokens[i].text.clone(),
                                suggestion: Self::preserve_case(&tokens[i].text, singular),
                            });
                        }
                    }
                }
            }
        }

        corrections
    }

    /// Busca la forma singular para una forma plural inequívoca de haber.
    fn get_singular_for(word: &str) -> Option<&'static str> {
        PLURAL_TO_SINGULAR
            .iter()
            .find(|(plural, _)| *plural == word)
            .map(|(_, singular)| *singular)
    }

    /// Busca la forma singular para una forma plural ambigua ("han").
    fn get_ambiguous_singular(word: &str) -> Option<&'static str> {
        AMBIGUOUS_PLURAL
            .iter()
            .find(|(plural, _)| *plural == word)
            .map(|(_, singular)| *singular)
    }

    /// Verifica si tras el token en `idx` hay un sintagma nominal
    /// (determinante/adjetivo/sustantivo), lo que indica uso existencial.
    ///
    /// Salta whitespace. Si encuentra "de" inmediatamente, es perífrasis
    /// "haber de + infinitivo" → no es existencial.
    fn is_followed_by_nominal(tokens: &[Token], idx: usize) -> bool {
        let mut j = idx + 1;

        // Saltar whitespace
        while j < tokens.len() && tokens[j].token_type == TokenType::Whitespace {
            j += 1;
        }

        if j >= tokens.len() {
            return false;
        }

        // Verificar límite de oración
        if has_sentence_boundary(tokens, idx, j) {
            return false;
        }

        let next_lower = tokens[j].effective_text().to_lowercase();

        // "haber de + infinitivo" → no es existencial
        if next_lower == "de" {
            return false;
        }

        // Si lo siguiente es un participio suelto (no "habido"), probablemente
        // es auxiliar: "habían comido" → no corregir.
        // Participio: termina en -ado, -ido, -to, -so, -cho
        if Self::looks_like_participle(&next_lower) && next_lower != "habido" {
            return false;
        }

        // Determinantes y cuantificadores que introducen SN existencial
        if Self::is_existential_introducer(&next_lower) {
            return true;
        }

        // Verificar por categoría gramatical del token
        if let Some(ref info) = tokens[j].word_info {
            use crate::dictionary::WordCategory;
            match info.category {
                WordCategory::Sustantivo => return true,
                WordCategory::Determinante | WordCategory::Articulo => return true,
                WordCategory::Adjetivo => {
                    // Adjetivo + sustantivo: "habían grandes problemas"
                    // Verificar que después hay un sustantivo
                    return Self::has_noun_after(tokens, j);
                }
                _ => {}
            }
        }

        false
    }

    /// Busca "habido" tras el token en `idx` (saltando whitespace).
    fn find_habido_after(tokens: &[Token], idx: usize) -> Option<usize> {
        let mut j = idx + 1;
        while j < tokens.len() && tokens[j].token_type == TokenType::Whitespace {
            j += 1;
        }
        if j < tokens.len()
            && tokens[j].token_type == TokenType::Word
            && tokens[j].effective_text().to_lowercase() == "habido"
        {
            Some(j)
        } else {
            None
        }
    }

    /// Verifica si hay un sustantivo tras `idx` (saltando whitespace, det, adj).
    fn has_noun_after(tokens: &[Token], idx: usize) -> bool {
        for j in (idx + 1)..tokens.len() {
            if tokens[j].token_type == TokenType::Whitespace {
                continue;
            }
            if tokens[j].is_sentence_boundary() {
                return false;
            }
            if tokens[j].token_type != TokenType::Word {
                return false;
            }
            if let Some(ref info) = tokens[j].word_info {
                use crate::dictionary::WordCategory;
                match info.category {
                    WordCategory::Sustantivo => return true,
                    WordCategory::Adjetivo | WordCategory::Determinante | WordCategory::Articulo => {
                        continue
                    }
                    _ => return false,
                }
            }
            return false;
        }
        false
    }

    /// ¿Parece un participio? (terminaciones típicas)
    fn looks_like_participle(word: &str) -> bool {
        word.ends_with("ado")
            || word.ends_with("ido")
            || word.ends_with("to")
            || word.ends_with("so")
            || word.ends_with("cho")
    }

    /// Determinantes/cuantificadores que introducen SN existencial.
    fn is_existential_introducer(word: &str) -> bool {
        matches!(
            word,
            "muchos"
                | "muchas"
                | "pocos"
                | "pocas"
                | "varios"
                | "varias"
                | "algunos"
                | "algunas"
                | "bastantes"
                | "demasiados"
                | "demasiadas"
                | "suficientes"
                | "numerosos"
                | "numerosas"
                | "tantos"
                | "tantas"
                | "más"
                | "menos"
                | "ciertos"
                | "ciertas"
        )
    }

    /// Preserva la capitalización del original al generar la sugerencia.
    fn preserve_case(original: &str, replacement: &str) -> String {
        if original.chars().next().map_or(false, |c| c.is_uppercase()) {
            let mut chars = replacement.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
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
    use crate::grammar::Tokenizer;

    fn tokenize(text: &str) -> Vec<Token> {
        Tokenizer::new().tokenize(text)
    }

    // ==========================================================================
    // Casos básicos: haber impersonal pluralizado
    // ==========================================================================

    #[test]
    fn test_habian_muchas_personas() {
        let tokens = tokenize("habían muchas personas");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "había");
    }

    // Nota: "hubieron accidentes", "habrán consecuencias", etc. requieren
    // tokens enriquecidos con word_info (solo disponible en pipeline completo).
    // Se testean como tests de integración en tests/spanish_corrector.rs.

    // ==========================================================================
    // Caso compuesto: "han habido" + SN
    // ==========================================================================

    #[test]
    fn test_han_habido_quejas() {
        let tokens = tokenize("han habido muchas quejas");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ha");
    }

    // "habían habido problemas" requiere word_info → test de integración

    // ==========================================================================
    // Falsos positivos: NO corregir uso auxiliar correcto
    // ==========================================================================

    #[test]
    fn test_habian_comido_no_correction() {
        // Auxiliar: "habían comido" es correcto
        let tokens = tokenize("habían comido mucho");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty(), "No debería corregir auxiliar: {:?}", corrections);
    }

    #[test]
    fn test_han_llegado_no_correction() {
        // Auxiliar: "han llegado" es correcto
        let tokens = tokenize("han llegado los invitados");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty(), "No debería corregir auxiliar: {:?}", corrections);
    }

    #[test]
    fn test_hubieran_venido_no_correction() {
        let tokens = tokenize("si hubieran venido antes");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty(), "No debería corregir auxiliar: {:?}", corrections);
    }

    #[test]
    fn test_haber_de_perifrasis_no_correction() {
        // "habían de marcharse" → perífrasis, no existencial
        let tokens = tokenize("habían de marcharse");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty(), "No debería corregir perífrasis: {:?}", corrections);
    }

    // ==========================================================================
    // Preservación de mayúsculas
    // ==========================================================================

    #[test]
    fn test_capitalization_preserved() {
        let tokens = tokenize("Habían muchas personas");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Había");
    }

    // ==========================================================================
    // Cuantificadores
    // ==========================================================================

    #[test]
    fn test_habian_varios_casos() {
        let tokens = tokenize("habían varios casos");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "había");
    }

    #[test]
    fn test_habian_demasiados_errores() {
        let tokens = tokenize("habían demasiados errores");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "había");
    }
}
