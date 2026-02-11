//! Deteccion conservadora de gerundio de posterioridad.
//!
//! Regla (alta precision):
//! - Verbo de salida/desplazamiento en la clausula previa
//! - Coma
//! - Gerundios de resultado frecuentes ("llegando"/"arribando"/"aprobando"/"terminando")
//! - Continuacion de destino/tiempo ("a"/"al"/"hasta"...)
//!
//! Sugerencia segura: reescritura con "al + infinitivo".

use crate::grammar::tokenizer::TokenType;
use crate::grammar::{has_sentence_boundary, Token};
use crate::languages::VerbFormRecognizer;

#[derive(Debug, Clone)]
pub struct GerundPosteriorityCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

pub struct GerundPosteriorityAnalyzer;

impl GerundPosteriorityAnalyzer {
    pub fn analyze(
        tokens: &[Token],
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<GerundPosteriorityCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        for pos in 0..word_tokens.len() {
            let (idx, token) = word_tokens[pos];
            let gerund = Self::normalize(Self::token_text_for_analysis(token));
            let Some(infinitive) = Self::posteriority_gerund_infinitive(&gerund) else {
                continue;
            };

            let Some(comma_idx) = Self::previous_non_whitespace_idx(tokens, idx) else {
                continue;
            };
            if tokens[comma_idx].token_type != TokenType::Punctuation || tokens[comma_idx].text != "," {
                continue;
            }

            if !Self::has_posteriority_tail(pos, &word_tokens, tokens) {
                continue;
            }

            if !Self::has_movement_verb_before_comma(tokens, comma_idx, verb_recognizer) {
                continue;
            }

            corrections.push(GerundPosteriorityCorrection {
                token_index: idx,
                original: token.text.clone(),
                suggestion: Self::preserve_case(&token.text, &format!("al {}", infinitive)),
                reason: "Gerundio de posterioridad: mejor 'al + infinitivo'".to_string(),
            });
        }

        corrections
    }

    fn has_posteriority_tail(pos: usize, word_tokens: &[(usize, &Token)], tokens: &[Token]) -> bool {
        if pos + 1 >= word_tokens.len() {
            return false;
        }

        let gerund_idx = word_tokens[pos].0;
        let (next_idx, next_token) = word_tokens[pos + 1];
        if has_sentence_boundary(tokens, gerund_idx, next_idx) {
            return false;
        }

        let next = Self::normalize(Self::token_text_for_analysis(next_token));
        if matches!(next.as_str(), "a" | "al" | "hasta") {
            return true;
        }

        // Tambien aceptar marcadores temporales claros:
        // "..., llegando luego/finalmente ..."
        matches!(next.as_str(), "luego" | "finalmente" | "despues")
    }

    fn has_movement_verb_before_comma(
        tokens: &[Token],
        comma_idx: usize,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let mut checked_words = 0usize;
        for i in (0..comma_idx).rev() {
            let token = &tokens[i];

            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.is_sentence_boundary() {
                break;
            }
            if token.token_type != TokenType::Word {
                continue;
            }

            checked_words += 1;
            if checked_words > 10 {
                break;
            }

            let word = Self::token_text_for_analysis(token);
            if Self::is_movement_verb_form(word, verb_recognizer) {
                return true;
            }
        }

        false
    }

    fn is_movement_verb_form(word: &str, verb_recognizer: Option<&dyn VerbFormRecognizer>) -> bool {
        let norm = Self::normalize(word);

        // Lista cerrada de formas muy frecuentes y claras.
        if matches!(
            norm.as_str(),
            "salio"
                | "salieron"
                | "sale"
                | "salen"
                | "salia"
                | "salian"
                | "partio"
                | "partieron"
                | "partia"
                | "partian"
                | "sefue"
                | "fue"
                | "fueron"
                | "semarcho"
                | "semarcharon"
                | "marcho"
                | "marcharon"
                | "iba"
                | "iban"
                | "vino"
                | "vinieron"
                | "venia"
                | "venian"
        ) {
            return true;
        }

        if let Some(vr) = verb_recognizer {
            if vr.is_valid_verb_form(word) {
                if let Some(inf) = vr.get_infinitive(word) {
                    let inf_norm = Self::normalize(&inf);
                    return matches!(
                        inf_norm.as_str(),
                        "salir"
                            | "partir"
                            | "marchar"
                            | "irse"
                            | "ir"
                            | "venir"
                            | "desplazarse"
                            | "dirigirse"
                            | "encaminarse"
                    );
                }
            }
        }

        false
    }

    fn posteriority_gerund_infinitive(word: &str) -> Option<&'static str> {
        match word {
            "llegando" => Some("llegar"),
            "arribando" => Some("arribar"),
            "aprobando" => Some("aprobar"),
            "terminando" => Some("terminar"),
            _ => None,
        }
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

    fn normalize(word: &str) -> String {
        word.to_lowercase()
            .chars()
            .map(|c| match c {
                'a'..='z' | '0'..='9' => c,
                'á' | 'à' | 'ä' | 'â' => 'a',
                'é' | 'è' | 'ë' | 'ê' => 'e',
                'í' | 'ì' | 'ï' | 'î' => 'i',
                'ó' | 'ò' | 'ö' | 'ô' => 'o',
                'ú' | 'ù' | 'ü' | 'û' => 'u',
                'ñ' => 'n',
                _ => c,
            })
            .collect()
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
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<GerundPosteriorityCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        GerundPosteriorityAnalyzer::analyze(&tokens, None)
    }

    #[test]
    fn test_detects_clear_posteriority_gerund() {
        let corrections = analyze_text("Salio de casa, llegando al trabajo a las 9");
        assert!(
            corrections.iter().any(|c| c.suggestion == "al llegar"),
            "Debe detectar gerundio de posterioridad claro: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_comma_no_detection() {
        let corrections = analyze_text("Salio de casa llegando al trabajo");
        assert!(
            corrections.is_empty(),
            "Sin coma no debe forzar correccion: {:?}",
            corrections
        );
    }

    #[test]
    fn test_non_arrival_gerund_not_detected() {
        let corrections = analyze_text("Salio de casa, caminando rapido");
        assert!(
            corrections.is_empty(),
            "No debe marcar gerundios no incluidos en lista cerrada: {:?}",
            corrections
        );
    }

    #[test]
    fn test_detects_terminando_with_temporal_tail() {
        let corrections = analyze_text("Salio de clase, terminando luego el informe");
        assert!(
            corrections.iter().any(|c| c.suggestion == "al terminar"),
            "Debe detectar 'terminando' en patron conservador: {:?}",
            corrections
        );
    }

    #[test]
    fn test_detects_aprobando_with_temporal_tail() {
        let corrections = analyze_text("Salio del examen, aprobando finalmente la materia");
        assert!(
            corrections.iter().any(|c| c.suggestion == "al aprobar"),
            "Debe detectar 'aprobando' en patron conservador: {:?}",
            corrections
        );
    }

    #[test]
    fn test_without_movement_verb_not_detected() {
        let corrections = analyze_text("Trabajo todo el dia, llegando al final cansado");
        assert!(
            corrections.is_empty(),
            "Sin verbo previo de desplazamiento no debe corregir: {:?}",
            corrections
        );
    }
}
