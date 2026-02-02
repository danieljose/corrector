//! Analizador de concordancia de género común con referente explícito
//!
//! Detecta errores de concordancia en sustantivos de género común (periodista, líder, premio)
//! cuando hay un referente explícito (nombre propio) que indica el género.
//!
//! Ejemplo:
//! - "el periodista María García" → "la periodista María García"
//! - "la premio Nobel Juan Pérez" → "el premio Nobel Juan Pérez"
//!
//! También detecta cuando la gramática corrigió incorrectamente:
//! - Original "la premio Nobel María" → Gramática sugirió "el" → Anulamos porque "María" es femenino

use crate::dictionary::{Gender, ProperNames, Trie};
use crate::grammar::{has_sentence_boundary, Token, TokenType};

use super::exceptions::is_common_gender_noun;
use super::names_gender::get_name_gender;

/// Tipo de acción a realizar
#[derive(Debug, Clone, PartialEq)]
pub enum CommonGenderAction {
    /// Corregir el artículo a la sugerencia
    Correct(String),
    /// Anular una corrección gramatical previa (el original era correcto)
    ClearCorrection,
}

/// Corrección de género común
#[derive(Debug, Clone)]
pub struct CommonGenderCorrection {
    pub token_index: usize,
    pub original: String,
    pub action: CommonGenderAction,
    pub message: String,
}

impl CommonGenderCorrection {
    /// Compatibility helper - returns suggestion if it's a Correct action
    pub fn suggestion(&self) -> Option<&str> {
        match &self.action {
            CommonGenderAction::Correct(s) => Some(s),
            CommonGenderAction::ClearCorrection => None,
        }
    }
}

/// Analizador de género común con referente
pub struct CommonGenderAnalyzer;

impl CommonGenderAnalyzer {
    /// Analiza tokens buscando sustantivos de género común con referente explícito
    pub fn analyze(
        tokens: &[Token],
        dictionary: &Trie,
        proper_names: &ProperNames,
    ) -> Vec<CommonGenderCorrection> {
        let mut corrections = Vec::new();

        // Recopilar índices de tokens de tipo Word
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Buscar patrón: artículo + sustantivo_género_común + [adjetivos]* + nombre_propio
        for (word_idx, (token_idx, token)) in word_tokens.iter().enumerate() {
            // Verificar si es un artículo usando el texto POST-ortografía pero PRE-gramática
            // Esto es: corrected_spelling si existe, sino el texto original
            // NO usamos effective_text() porque incluiría corrected_grammar
            let post_spelling_article = token.corrected_spelling
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(&token.text);
            let post_spelling_article_lower = post_spelling_article.to_lowercase();

            let post_spelling_gender = match post_spelling_article_lower.as_str() {
                "el" | "un" => Some(Gender::Masculine),
                "la" | "una" => Some(Gender::Feminine),
                _ => None,
            };

            let Some(post_spelling_gender) = post_spelling_gender else {
                continue;
            };

            // Verificar si el siguiente token es un sustantivo de género común
            if word_idx + 1 >= word_tokens.len() {
                continue;
            }

            let (noun_token_idx, noun_token) = word_tokens[word_idx + 1];
            let noun_lower = noun_token.effective_text().to_lowercase();

            if !is_common_gender_noun(&noun_lower) {
                continue;
            }

            // Verificar que no hay límite de oración entre artículo y sustantivo
            if has_sentence_boundary(tokens, *token_idx, noun_token_idx) {
                continue;
            }

            // Buscar un nombre propio en los siguientes 4 tokens
            // Saltar adjetivos intermedios
            let referent_result = Self::find_referent_gender(
                tokens,
                &word_tokens,
                word_idx + 2,
                *token_idx,
                dictionary,
                proper_names,
            );

            let Some(referent_gender) = referent_result else {
                // Sin referente explícito, no podemos determinar el género correcto
                continue;
            };

            // Comparar el artículo POST-ORTOGRAFÍA con el género del referente
            if post_spelling_gender == referent_gender {
                // El artículo (tras ortografía) es correcto para el referente
                // Si hay una corrección gramatical previa, debemos anularla
                if token.corrected_grammar.is_some() {
                    corrections.push(CommonGenderCorrection {
                        token_index: *token_idx,
                        original: token.text.clone(),
                        action: CommonGenderAction::ClearCorrection,
                        message: format!(
                            "Artículo '{}' correcto para referente femenino/masculino (anulando corrección previa)",
                            post_spelling_article
                        ),
                    });
                }
                // Si no hay corrección previa, no hacer nada (ya está correcto)
            } else {
                // El artículo post-ortografía NO concuerda con el referente → corregir
                let correct_article = match (&post_spelling_article_lower.as_str(), &referent_gender) {
                    (&"el", &Gender::Feminine) => "la",
                    (&"la", &Gender::Masculine) => "el",
                    (&"un", &Gender::Feminine) => "una",
                    (&"una", &Gender::Masculine) => "un",
                    _ => continue,
                };

                // Preservar mayúsculas
                let suggestion = if token.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    let mut chars = correct_article.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                        None => correct_article.to_string(),
                    }
                } else {
                    correct_article.to_string()
                };

                corrections.push(CommonGenderCorrection {
                    token_index: *token_idx,
                    original: token.text.clone(),
                    action: CommonGenderAction::Correct(suggestion),
                    message: format!(
                        "Concordancia con referente: '{}' debería ser '{}' (referente femenino/masculino)",
                        token.text, correct_article
                    ),
                });
            }
        }

        corrections
    }

    /// Busca el género del referente (nombre propio) en una ventana de tokens
    /// Verifica límites de oración durante la búsqueda
    fn find_referent_gender(
        all_tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        start_idx: usize,
        article_token_idx: usize,
        dictionary: &Trie,
        proper_names: &ProperNames,
    ) -> Option<Gender> {
        // Buscar en una ventana de 4 tokens
        let window_size = 4.min(word_tokens.len().saturating_sub(start_idx));

        for offset in 0..window_size {
            let idx = start_idx + offset;
            if idx >= word_tokens.len() {
                break;
            }

            let (current_token_idx, token) = &word_tokens[idx];

            // Verificar que no hay límite de oración entre el artículo y este token
            if has_sentence_boundary(all_tokens, article_token_idx, *current_token_idx) {
                break;
            }

            let text = token.effective_text();

            // Saltar adjetivos (verificar con diccionario)
            if Self::is_likely_adjective(text, dictionary) {
                continue;
            }

            // Saltar palabras especiales como "Nobel", "Pulitzer" (títulos de premios)
            let lower = text.to_lowercase();
            if matches!(lower.as_str(), "nobel" | "pulitzer" | "cervantes" | "goya" |
                        "príncipe" | "princesa" | "nacional" | "internacional") {
                continue;
            }

            // Verificar si es un nombre propio
            // Debe estar capitalizado (no al inicio de oración) y estar en la lista de nombres
            if Self::is_proper_name_candidate(text, proper_names) {
                if let Some(gender) = get_name_gender(text) {
                    return Some(gender);
                }
            }

            // Si encontramos una palabra que no es adjetivo ni nombre conocido, parar
            // (probablemente es un verbo u otra categoría)
            if !Self::is_likely_adjective(text, dictionary) {
                // Verificar si parece verbo u otra palabra
                if let Some(info) = dictionary.get(&text.to_lowercase()) {
                    use crate::dictionary::WordCategory;
                    if matches!(info.category, WordCategory::Verbo | WordCategory::Preposicion |
                               WordCategory::Conjuncion | WordCategory::Adverbio) {
                        break;
                    }
                }
            }
        }

        None
    }

    /// Verifica si una palabra es probablemente un adjetivo
    fn is_likely_adjective(word: &str, dictionary: &Trie) -> bool {
        use crate::dictionary::WordCategory;

        if let Some(info) = dictionary.get(&word.to_lowercase()) {
            return info.category == WordCategory::Adjetivo;
        }

        // Heurística para adjetivos no en diccionario
        let lower = word.to_lowercase();
        lower.ends_with("oso") || lower.ends_with("osa") ||
        lower.ends_with("ivo") || lower.ends_with("iva") ||
        lower.ends_with("ico") || lower.ends_with("ica") ||
        lower.ends_with("ble") || lower.ends_with("al") ||
        lower.ends_with("nte")
    }

    /// Verifica si una palabra es candidata a nombre propio
    fn is_proper_name_candidate(word: &str, proper_names: &ProperNames) -> bool {
        // Debe estar capitalizado
        let first_char = word.chars().next();
        if !first_char.map(|c| c.is_uppercase()).unwrap_or(false) {
            return false;
        }

        // Debe estar en la lista de nombres propios
        proper_names.is_proper_name(word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::DictionaryLoader;
    use crate::grammar::Tokenizer;
    use std::path::Path;

    fn setup() -> (Trie, ProperNames) {
        let dict_path = Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };

        let names_path = Path::new("data/names.txt");
        let proper_names = if names_path.exists() {
            ProperNames::load_from_file(names_path).unwrap_or_default()
        } else {
            ProperNames::default()
        };

        (dictionary, proper_names)
    }

    #[test]
    fn test_el_periodista_maria_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista María García informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "el");
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_la_periodista_maria_no_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la periodista María García informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "No debería haber corrección para 'la periodista María'");
    }

    #[test]
    fn test_la_periodista_juan_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la periodista Juan López informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "la");
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("el".to_string()));
    }

    #[test]
    fn test_el_periodista_juan_no_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista Juan López informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "No debería haber corrección para 'el periodista Juan'");
    }

    #[test]
    fn test_common_gender_with_adjective() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista española María García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "María" es femenino y sugerir "la"
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_common_gender_without_referent() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista informó sobre el evento");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Sin referente explícito, no debería haber corrección
        assert!(corrections.is_empty(), "Sin referente no debe haber corrección");
    }

    #[test]
    fn test_el_premio_nobel_maria_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el premio Nobel María Curie");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "premio" es de género común en contextos como "la premio Nobel"
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_la_lider_ana_no_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la líder Ana García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "No debería haber corrección para 'la líder Ana'");
    }

    #[test]
    fn test_un_artista_carmen_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("un artista Carmen López");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "un");
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("una".to_string()));
    }

    #[test]
    fn test_preserves_uppercase() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("El periodista María García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("La".to_string()));
    }

    // ==========================================================================
    // Tests de límites de oración
    // ==========================================================================

    #[test]
    fn test_sentence_boundary_blocks_referent() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // El punto separa "periodista" de "María", no debería encontrar referente
        let tokens = tokenizer.tokenize("El periodista. María llegó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debería haber corrección porque "María" está en otra oración
        assert!(corrections.is_empty(), "No debería cruzar límites de oración: {:?}", corrections);
    }

    #[test]
    fn test_sentence_boundary_with_question_mark() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("¿Quién es el periodista? María lo sabe");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debería haber corrección porque "María" está en otra oración
        assert!(corrections.is_empty(), "No debería cruzar límites de oración con ?: {:?}", corrections);
    }

    // ==========================================================================
    // Test de anulación de corrección gramatical
    // ==========================================================================

    #[test]
    fn test_clear_grammar_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize("la premio Nobel María Curie");

        // Simular que la gramática corrigió "la" a "el" (porque "premio" es masculino en dict)
        tokens[0].corrected_grammar = Some("el".to_string());

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "la" era correcto y pedir anular la corrección
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::ClearCorrection);
    }

    #[test]
    fn test_no_clear_when_grammar_was_right() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el premio Nobel Juan Pérez");

        // Simular que la gramática dejó "el" sin cambiar (correcto)
        // No hay corrected_grammar

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debería haber corrección porque "el" ya es correcto para "Juan"
        assert!(corrections.is_empty(), "No debería haber corrección cuando ya está correcto");
    }

    // ==========================================================================
    // Test de corrección ortográfica previa
    // ==========================================================================

    #[test]
    fn test_spelling_corrected_article_detected() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize("laa premio Nobel María Curie");

        // Simular que ortografía corrigió "laa" → "la"
        tokens[0].corrected_spelling = Some("la".to_string());
        // Y gramática después cambió a "el" (porque premio es masc en dict)
        tokens[0].corrected_grammar = Some("el".to_string());

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "la" (effective) es correcto para "María" y anular
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::ClearCorrection);
    }

    #[test]
    fn test_spelling_corrected_article_wrong_for_referent() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize("ell periodista María García");

        // Simular que ortografía corrigió "ell" → "el"
        tokens[0].corrected_spelling = Some("el".to_string());

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "el" (effective) no concuerda con "María" y sugerir "la"
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }
}
