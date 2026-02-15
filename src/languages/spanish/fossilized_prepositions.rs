//! Corrección de locuciones preposicionales fosilizadas de alta precisión.
//!
//! Patrones cubiertos:
//! - en base a -> con base en
//! - de acuerdo a -> de acuerdo con
//! - bajo mi/tu/... punto de vista -> desde ... punto de vista
//! - a nivel de (uso no técnico) -> en cuanto a

use crate::dictionary::WordCategory;
use crate::grammar::tokenizer::TokenType;
use crate::grammar::{has_sentence_boundary, Token};

#[derive(Debug, Clone)]
pub struct FossilizedPrepositionCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

pub struct FossilizedPrepositionAnalyzer;

impl FossilizedPrepositionAnalyzer {
    pub fn analyze(tokens: &[Token]) -> Vec<FossilizedPrepositionCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        for pos in 0..word_tokens.len() {
            if let Some(cs) = Self::check_a_grosso_modo(tokens, &word_tokens, pos) {
                corrections.push(cs);
                continue;
            }
            if let Some(mut cs) = Self::check_a_parte_de(tokens, &word_tokens, pos) {
                corrections.append(&mut cs);
                continue;
            }
            if let Some(mut cs) = Self::check_en_base_a(tokens, &word_tokens, pos) {
                corrections.append(&mut cs);
                continue;
            }
            if let Some(mut cs) = Self::check_de_acuerdo_a(tokens, &word_tokens, pos) {
                corrections.append(&mut cs);
                continue;
            }
            if let Some(cs) = Self::check_bajo_punto_de_vista(tokens, &word_tokens, pos) {
                corrections.push(cs);
                continue;
            }
            if let Some(mut cs) = Self::check_a_nivel_de(tokens, &word_tokens, pos) {
                corrections.append(&mut cs);
            }
        }

        corrections
    }

    fn check_a_parte_de(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> Option<Vec<FossilizedPrepositionCorrection>> {
        if pos + 2 >= word_tokens.len() {
            return None;
        }

        let (idx0, tok0) = word_tokens[pos];
        let (idx1, tok1) = word_tokens[pos + 1];
        let (idx2, tok2) = word_tokens[pos + 2];

        let w0 = Self::normalize(tok0.effective_text());
        let w1 = Self::normalize(tok1.effective_text());
        let w2 = Self::normalize(tok2.effective_text());
        if w0 != "a" || w1 != "parte" || w2 != "de" {
            return None;
        }

        if has_sentence_boundary(tokens, idx0, idx1)
            || has_sentence_boundary(tokens, idx1, idx2)
            || !Self::words_are_contiguous(tokens, idx0, idx1)
            || !Self::words_are_contiguous(tokens, idx1, idx2)
        {
            return None;
        }

        Some(vec![
            FossilizedPrepositionCorrection {
                token_index: idx0,
                original: tok0.text.clone(),
                suggestion: Self::preserve_case(&tok0.text, "aparte"),
                reason: "Locución recomendada: 'aparte de'".to_string(),
            },
            FossilizedPrepositionCorrection {
                token_index: idx1,
                original: tok1.text.clone(),
                suggestion: "sobra".to_string(),
                reason: "Locución recomendada: 'aparte de'".to_string(),
            },
        ])
    }

    fn check_a_grosso_modo(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> Option<FossilizedPrepositionCorrection> {
        if pos + 2 >= word_tokens.len() {
            return None;
        }

        let (idx0, tok0) = word_tokens[pos];
        let (idx1, tok1) = word_tokens[pos + 1];
        let (idx2, tok2) = word_tokens[pos + 2];

        let w0 = Self::normalize(tok0.effective_text());
        let w1 = Self::normalize(tok1.effective_text());
        let w2 = Self::normalize(tok2.effective_text());

        if w0 != "a" || w1 != "grosso" || w2 != "modo" {
            return None;
        }
        if has_sentence_boundary(tokens, idx0, idx1)
            || has_sentence_boundary(tokens, idx1, idx2)
            || !Self::words_are_contiguous(tokens, idx0, idx1)
            || !Self::words_are_contiguous(tokens, idx1, idx2)
        {
            return None;
        }

        Some(FossilizedPrepositionCorrection {
            token_index: idx0,
            original: tok0.text.clone(),
            suggestion: "sobra".to_string(),
            reason: "Locución recomendada: 'grosso modo'".to_string(),
        })
    }

    fn check_en_base_a(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> Option<Vec<FossilizedPrepositionCorrection>> {
        if pos + 2 >= word_tokens.len() {
            return None;
        }

        let (idx0, tok0) = word_tokens[pos];
        let (idx1, tok1) = word_tokens[pos + 1];
        let (idx2, tok2) = word_tokens[pos + 2];
        let w0 = Self::normalize(tok0.effective_text());
        let w1 = Self::normalize(tok1.effective_text());
        let w2 = Self::normalize(tok2.effective_text());

        if w0 != "en" || w1 != "base" || !matches!(w2.as_str(), "a" | "al") {
            return None;
        }
        if has_sentence_boundary(tokens, idx0, idx1)
            || has_sentence_boundary(tokens, idx1, idx2)
            || !Self::words_are_contiguous(tokens, idx0, idx1)
            || !Self::words_are_contiguous(tokens, idx1, idx2)
        {
            return None;
        }

        let right = if w2 == "al" { "en el" } else { "en" };
        Some(vec![
            FossilizedPrepositionCorrection {
                token_index: idx0,
                original: tok0.text.clone(),
                suggestion: Self::preserve_case(&tok0.text, "con"),
                reason: "Locución preferida: 'con base en'".to_string(),
            },
            FossilizedPrepositionCorrection {
                token_index: idx2,
                original: tok2.text.clone(),
                suggestion: Self::preserve_case(&tok2.text, right),
                reason: "Locución preferida: 'con base en'".to_string(),
            },
        ])
    }

    fn check_de_acuerdo_a(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> Option<Vec<FossilizedPrepositionCorrection>> {
        if pos + 2 >= word_tokens.len() {
            return None;
        }

        let (idx0, tok0) = word_tokens[pos];
        let (idx1, tok1) = word_tokens[pos + 1];
        let (idx2, tok2) = word_tokens[pos + 2];
        let w0 = Self::normalize(tok0.effective_text());
        let w1 = Self::normalize(tok1.effective_text());
        let w2 = Self::normalize(tok2.effective_text());

        if w0 != "de" || w1 != "acuerdo" || !matches!(w2.as_str(), "a" | "al") {
            return None;
        }
        if has_sentence_boundary(tokens, idx0, idx1)
            || has_sentence_boundary(tokens, idx1, idx2)
            || !Self::words_are_contiguous(tokens, idx0, idx1)
            || !Self::words_are_contiguous(tokens, idx1, idx2)
        {
            return None;
        }

        // Excepción: locución verbal "poner de acuerdo a alguien".
        // Aquí "a" introduce complemento de persona y no debe forzarse "con".
        if pos > 0 {
            let (_, prev_tok) = word_tokens[pos - 1];
            // Usar texto original para no perder el verbo por sugerencias ortográficas
            // (ej. "Pondria" -> "podria,pondría,...").
            let prev = Self::normalize(&prev_tok.text);
            if Self::is_poner_form(&prev) {
                return None;
            }
        }

        let right = if w2 == "al" { "con el" } else { "con" };
        Some(vec![FossilizedPrepositionCorrection {
            token_index: idx2,
            original: tok2.text.clone(),
            suggestion: Self::preserve_case(&tok2.text, right),
            reason: "Régimen recomendado: 'de acuerdo con'".to_string(),
        }])
    }

    fn check_bajo_punto_de_vista(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> Option<FossilizedPrepositionCorrection> {
        if pos + 4 >= word_tokens.len() {
            return None;
        }

        let (idx0, tok0) = word_tokens[pos];
        let (idx1, tok1) = word_tokens[pos + 1];
        let (idx2, tok2) = word_tokens[pos + 2];
        let (idx3, tok3) = word_tokens[pos + 3];
        let (idx4, tok4) = word_tokens[pos + 4];

        let w0 = Self::normalize(tok0.effective_text());
        let w1 = Self::normalize(tok1.effective_text());
        let w2 = Self::normalize(tok2.effective_text());
        let w3 = Self::normalize(tok3.effective_text());
        let w4 = Self::normalize(tok4.effective_text());

        if w0 != "bajo"
            || !Self::is_punto_vista_determiner(&w1)
            || w2 != "punto"
            || w3 != "de"
            || w4 != "vista"
        {
            return None;
        }

        if has_sentence_boundary(tokens, idx0, idx1)
            || has_sentence_boundary(tokens, idx1, idx2)
            || has_sentence_boundary(tokens, idx2, idx3)
            || has_sentence_boundary(tokens, idx3, idx4)
            || !Self::words_are_contiguous(tokens, idx0, idx1)
            || !Self::words_are_contiguous(tokens, idx1, idx2)
            || !Self::words_are_contiguous(tokens, idx2, idx3)
            || !Self::words_are_contiguous(tokens, idx3, idx4)
        {
            return None;
        }

        Some(FossilizedPrepositionCorrection {
            token_index: idx0,
            original: tok0.text.clone(),
            suggestion: Self::preserve_case(&tok0.text, "desde"),
            reason: "Locución recomendada: 'desde ... punto de vista'".to_string(),
        })
    }

    fn check_a_nivel_de(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> Option<Vec<FossilizedPrepositionCorrection>> {
        if pos + 2 >= word_tokens.len() {
            return None;
        }

        let (idx0, tok0) = word_tokens[pos];
        let (idx1, tok1) = word_tokens[pos + 1];
        let (idx2, tok2) = word_tokens[pos + 2];
        let w0 = Self::normalize(tok0.effective_text());
        let w1 = Self::normalize(tok1.effective_text());
        let w2 = Self::normalize(tok2.effective_text());

        if w0 != "a" || w1 != "nivel" || w2 != "de" {
            return None;
        }
        if has_sentence_boundary(tokens, idx0, idx1)
            || has_sentence_boundary(tokens, idx1, idx2)
            || !Self::words_are_contiguous(tokens, idx0, idx1)
            || !Self::words_are_contiguous(tokens, idx1, idx2)
        {
            return None;
        }
        if !Self::is_nontechnical_a_nivel_de(tokens, word_tokens, pos) {
            return None;
        }

        Some(vec![
            FossilizedPrepositionCorrection {
                token_index: idx0,
                original: tok0.text.clone(),
                suggestion: Self::preserve_case(&tok0.text, "en"),
                reason: "Uso no técnico: preferible 'en cuanto a'".to_string(),
            },
            FossilizedPrepositionCorrection {
                token_index: idx1,
                original: tok1.text.clone(),
                suggestion: Self::preserve_case(&tok1.text, "cuanto"),
                reason: "Uso no técnico: preferible 'en cuanto a'".to_string(),
            },
            FossilizedPrepositionCorrection {
                token_index: idx2,
                original: tok2.text.clone(),
                suggestion: Self::preserve_case(&tok2.text, "a"),
                reason: "Uso no técnico: preferible 'en cuanto a'".to_string(),
            },
        ])
    }

    fn is_nontechnical_a_nivel_de(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> bool {
        if pos + 2 >= word_tokens.len() {
            return false;
        }

        let de_idx = word_tokens[pos + 2].0;
        let Some((first_idx, first_after_de)) = Self::next_non_whitespace_token(tokens, de_idx)
        else {
            return false;
        };
        if first_after_de.is_sentence_boundary()
            || first_after_de.token_type == TokenType::Punctuation
            || first_after_de.token_type == TokenType::Number
        {
            return false;
        }

        let mut candidate_idx = first_idx;
        let mut candidate_word = Self::normalize(first_after_de.effective_text());
        if Self::is_nivel_determiner(&candidate_word, Some(first_after_de)) {
            let Some((next_idx, next_token)) =
                Self::next_non_whitespace_token(tokens, candidate_idx)
            else {
                return false;
            };
            if next_token.is_sentence_boundary()
                || next_token.token_type == TokenType::Punctuation
                || next_token.token_type == TokenType::Number
            {
                return false;
            }
            candidate_idx = next_idx;
            candidate_word = Self::normalize(next_token.effective_text());
        }

        if has_sentence_boundary(tokens, de_idx, candidate_idx)
            || Self::is_technical_nivel_head(&candidate_word)
        {
            return false;
        }

        true
    }

    fn is_poner_form(word: &str) -> bool {
        matches!(
            word,
            "poner"
                | "pongo"
                | "pones"
                | "pone"
                | "ponemos"
                | "poneis"
                | "ponen"
                | "ponia"
                | "ponias"
                | "poniamos"
                | "poniais"
                | "ponian"
                | "pondre"
                | "pondras"
                | "pondra"
                | "pondremos"
                | "pondreis"
                | "pondran"
                | "pondria"
                | "pondrias"
                | "pondriamos"
                | "pondriais"
                | "pondrian"
                | "puse"
                | "pusiste"
                | "puso"
                | "pusimos"
                | "pusisteis"
                | "pusieron"
                | "puesto"
                | "puesta"
                | "puestos"
                | "puestas"
        )
    }

    fn words_are_contiguous(tokens: &[Token], left_idx: usize, right_idx: usize) -> bool {
        if right_idx <= left_idx + 1 {
            return true;
        }
        tokens
            .iter()
            .take(right_idx)
            .skip(left_idx + 1)
            .all(|t| t.token_type == TokenType::Whitespace)
    }

    fn next_non_whitespace_token(tokens: &[Token], start_idx: usize) -> Option<(usize, &Token)> {
        for (i, t) in tokens.iter().enumerate().skip(start_idx + 1) {
            if t.token_type == TokenType::Whitespace {
                continue;
            }
            return Some((i, t));
        }
        None
    }

    fn is_punto_vista_determiner(word: &str) -> bool {
        matches!(
            word,
            "mi" | "tu"
                | "su"
                | "nuestro"
                | "nuestra"
                | "nuestros"
                | "nuestras"
                | "vuestro"
                | "vuestra"
                | "vuestros"
                | "vuestras"
                | "este"
                | "esta"
                | "estos"
                | "estas"
                | "ese"
                | "esa"
                | "esos"
                | "esas"
                | "aquel"
                | "aquella"
                | "aquellos"
                | "aquellas"
                | "el"
                | "la"
                | "los"
                | "las"
                | "un"
                | "una"
        )
    }

    fn is_nivel_determiner(word: &str, token: Option<&Token>) -> bool {
        if let Some(tok) = token {
            if tok.word_info.as_ref().is_some_and(|info| {
                matches!(
                    info.category,
                    WordCategory::Articulo | WordCategory::Determinante
                )
            }) {
                return true;
            }
        }
        matches!(
            word,
            "el" | "la"
                | "los"
                | "las"
                | "un"
                | "una"
                | "unos"
                | "unas"
                | "mi"
                | "tu"
                | "su"
                | "nuestro"
                | "nuestra"
                | "nuestros"
                | "nuestras"
                | "vuestro"
                | "vuestra"
                | "vuestros"
                | "vuestras"
                | "este"
                | "esta"
                | "estos"
                | "estas"
                | "ese"
                | "esa"
                | "esos"
                | "esas"
                | "aquel"
                | "aquella"
                | "aquellos"
                | "aquellas"
                | "lo"
        )
    }

    fn is_technical_nivel_head(word: &str) -> bool {
        matches!(
            word,
            "mar"
                | "suelo"
                | "subsuelo"
                | "piso"
                | "calle"
                | "techo"
                | "agua"
                | "rio"
                | "lago"
                | "oceano"
                | "superficie"
                | "altitud"
                | "altura"
                | "cota"
                | "metro"
                | "metros"
                | "kilometro"
                | "kilometros"
                | "centimetro"
                | "centimetros"
                | "milimetro"
                | "milimetros"
                | "mm"
                | "cm"
                | "m"
                | "km"
                | "latitud"
                | "longitud"
                | "presion"
                | "temperatura"
        )
    }

    fn normalize(word: &str) -> String {
        word.to_lowercase()
            .chars()
            .map(|c| match c {
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

    fn analyze_text(text: &str) -> Vec<FossilizedPrepositionCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        FossilizedPrepositionAnalyzer::analyze(&tokens)
    }

    #[test]
    fn test_en_base_a_should_be_con_base_en() {
        let corrections = analyze_text("en base a los datos");
        assert!(corrections.iter().any(|c| c.suggestion == "con"));
        assert!(corrections.iter().any(|c| c.suggestion == "en"));
    }

    #[test]
    fn test_de_acuerdo_a_should_be_de_acuerdo_con() {
        let corrections = analyze_text("de acuerdo a la norma");
        assert!(corrections.iter().any(|c| c.suggestion == "con"));
    }

    #[test]
    fn test_poner_de_acuerdo_a_should_not_be_forced_to_con() {
        let corrections = analyze_text("pondria de acuerdo a rusos");
        assert!(!corrections.iter().any(|c| c.suggestion == "con"));
    }

    #[test]
    fn test_bajo_mi_punto_de_vista_should_be_desde() {
        let corrections = analyze_text("bajo mi punto de vista");
        assert!(corrections.iter().any(|c| c.suggestion == "desde"));
    }

    #[test]
    fn test_a_nivel_de_non_technical_should_be_en_cuanto_a() {
        let corrections = analyze_text("a nivel de educacion");
        assert!(corrections.iter().any(|c| c.suggestion == "en"));
        assert!(corrections.iter().any(|c| c.suggestion == "cuanto"));
        assert!(corrections.iter().filter(|c| c.suggestion == "a").count() >= 1);
    }

    #[test]
    fn test_a_nivel_del_mar_should_not_change() {
        let corrections = analyze_text("a nivel del mar");
        assert!(
            corrections.is_empty(),
            "No debe tocar uso técnico 'a nivel del mar': {:?}",
            corrections
        );
    }

    #[test]
    fn test_a_nivel_de_number_should_not_change() {
        let corrections = analyze_text("a nivel de 300 metros");
        assert!(
            corrections.is_empty(),
            "No debe tocar uso técnico con medida: {:?}",
            corrections
        );
    }

    #[test]
    fn test_a_grosso_modo_should_mark_redundant_a() {
        let corrections = analyze_text("a grosso modo");
        assert!(
            corrections.iter().any(|c| c.suggestion == "sobra"),
            "Debe marcar la preposición redundante en 'a grosso modo': {:?}",
            corrections
        );
    }

    #[test]
    fn test_grosso_modo_should_not_change() {
        let corrections = analyze_text("grosso modo");
        assert!(
            corrections.is_empty(),
            "No debe tocar la locución 'grosso modo': {:?}",
            corrections
        );
    }
}
