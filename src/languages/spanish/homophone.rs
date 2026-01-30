//! Corrección de homófonos (ortonimia)
//!
//! Detecta y corrige confusiones entre palabras que suenan igual pero se escriben diferente:
//! - hay/ahí/ay
//! - haya/halla/aya
//! - vaya/valla/baya
//! - hecho/echo
//! - tuvo/tubo
//! - a ver/haber
//! - iba (no "iva")
//! - hierba/hierva
//! - bello/vello
//! - botar/votar

use crate::grammar::tokenizer::TokenType;
use crate::grammar::Token;

/// Correccion sugerida para homofonos
#[derive(Debug, Clone)]
pub struct HomophoneCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

/// Analizador de homofonos
pub struct HomophoneAnalyzer;

impl HomophoneAnalyzer {
    /// Analiza los tokens y detecta errores de homofonos
    pub fn analyze(tokens: &[Token]) -> Vec<HomophoneCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .collect();

        for (pos, (idx, token)) in word_tokens.iter().enumerate() {
            // Usar effective_text() para ver correcciones de fases anteriores
            let word_lower = token.effective_text().to_lowercase();

            // Obtener contexto (tambien con effective_text)
            // Solo considerar palabra anterior si no hay limite de oracion entre ellas
            let prev_word = if pos > 0 {
                let prev_idx = word_tokens[pos - 1].0;
                if Self::has_sentence_boundary(tokens, prev_idx, *idx) {
                    None
                } else {
                    Some(word_tokens[pos - 1].1.effective_text().to_lowercase())
                }
            } else {
                None
            };

            // Solo considerar palabra siguiente si no hay limite de oracion entre ellas
            let next_word = if pos + 1 < word_tokens.len() {
                let next_idx = word_tokens[pos + 1].0;
                if Self::has_sentence_boundary(tokens, *idx, next_idx) {
                    None
                } else {
                    Some(word_tokens[pos + 1].1.effective_text().to_lowercase())
                }
            } else {
                None
            };

            // Verificar cada grupo de homófonos
            if let Some(correction) = Self::check_hay_ahi_ay(&word_lower, *idx, token, prev_word.as_deref(), next_word.as_deref()) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_haya_halla(&word_lower, *idx, token, prev_word.as_deref(), next_word.as_deref()) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_vaya_valla(&word_lower, *idx, token, prev_word.as_deref(), next_word.as_deref()) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_hecho_echo(&word_lower, *idx, token, prev_word.as_deref(), next_word.as_deref()) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_tuvo_tubo(&word_lower, *idx, token, prev_word.as_deref(), next_word.as_deref()) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_iba(&word_lower, *idx, token) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_hierba_hierva(&word_lower, *idx, token, prev_word.as_deref(), next_word.as_deref()) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_bello_vello(&word_lower, *idx, token, prev_word.as_deref()) {
                corrections.push(correction);
            } else if let Some(correction) = Self::check_botar_votar(&word_lower, *idx, token, prev_word.as_deref(), next_word.as_deref()) {
                corrections.push(correction);
            }
        }

        corrections
    }

    /// hay (verbo haber) / ahí (adverbio lugar) / ay (interjección)
    fn check_hay_ahi_ay(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "hay" => {
                // "hay" es correcto cuando es verbo impersonal: "hay mucha gente"
                // Error común: usar "hay" en lugar de "ahí" (lugar)
                // Contexto: después de preposición de lugar suele ser "ahí"
                if let Some(p) = prev {
                    if matches!(p, "por" | "de" | "desde" | "hasta" | "hacia") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "ahí"),
                            reason: "Adverbio de lugar (no verbo haber)".to_string(),
                        });
                    }
                }
                None
            }
            "ahi" => {
                // "ahi" sin tilde es incorrecto, siempre es "ahí"
                Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "ahí"),
                    reason: "Adverbio de lugar (requiere tilde)".to_string(),
                })
            }
            "ahí" => {
                // "ahí" es correcto como adverbio de lugar
                // Error: usar "ahí" en lugar de "hay" (verbo)
                // Contexto: si va seguido de sustantivo/artículo, puede ser "hay"
                if let Some(n) = next {
                    if matches!(n, "un" | "una" | "unos" | "unas" | "mucho" | "mucha" | "muchos" | "muchas" | "poco" | "poca") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hay"),
                            reason: "Verbo haber impersonal".to_string(),
                        });
                    }
                }
                None
            }
            "ai" => {
                // "ai" es incorrecto, probablemente quiso decir "ahí" o "ay"
                // Si está solo o con signos de exclamación, es "ay"
                Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "ay"),
                    reason: "Interjección de dolor/sorpresa".to_string(),
                })
            }
            _ => None,
        }
    }

    /// haya (verbo haber/árbol) / halla (verbo hallar) / aya (niñera)
    fn check_haya_halla(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "halla" => {
                // "halla" es verbo hallar (encontrar)
                // Error: usar "halla" en lugar de "haya" (subjuntivo de haber)
                // Contexto: después de "que", "aunque", "ojalá" suele ser "haya"
                if let Some(p) = prev {
                    if matches!(p, "que" | "aunque" | "ojalá" | "quizá" | "quizás" | "cuando" | "si") {
                        // Verificar si va seguido de participio (entonces es "haya")
                        if let Some(n) = next {
                            if n.ends_with("ado") || n.ends_with("ido") || n.ends_with("to") || n.ends_with("cho") {
                                return Some(HomophoneCorrection {
                                    token_index: idx,
                                    original: token.text.clone(),
                                    suggestion: Self::preserve_case(&token.text, "haya"),
                                    reason: "Subjuntivo de haber + participio".to_string(),
                                });
                            }
                        }
                    }
                }
                None
            }
            "haya" => {
                // "haya" puede ser subjuntivo de haber o el árbol
                // Error: usar "haya" en lugar de "halla" (encontrar)
                // Contexto: si va seguido de complemento directo sin participio
                if let Some(p) = prev {
                    // "se haya" + no participio = probablemente "se halla"
                    if p == "se" {
                        if let Some(n) = next {
                            if !n.ends_with("ado") && !n.ends_with("ido") && !n.ends_with("to") && !n.ends_with("cho") {
                                // Probablemente es "se halla" (se encuentra)
                                return Some(HomophoneCorrection {
                                    token_index: idx,
                                    original: token.text.clone(),
                                    suggestion: Self::preserve_case(&token.text, "halla"),
                                    reason: "Verbo hallar (encontrar)".to_string(),
                                });
                            }
                        }
                    }
                }
                None
            }
            "aya" => {
                // "aya" es niñera (arcaico), muy raro
                // Probablemente quiso decir "haya"
                if let Some(p) = prev {
                    if matches!(p, "que" | "aunque" | "ojalá" | "quizá" | "quizás") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "haya"),
                            reason: "Subjuntivo de haber".to_string(),
                        });
                    }
                }
                None
            }
            "haiga" => {
                // "haiga" es incorrecto, siempre es "haya"
                Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: Self::preserve_case(&token.text, "haya"),
                    reason: "Forma correcta del subjuntivo de haber".to_string(),
                })
            }
            _ => None,
        }
    }

    /// vaya (verbo ir) / valla (cerca) / baya (fruto)
    fn check_vaya_valla(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "valla" => {
                // "valla" es cerca/obstáculo
                // Error: usar "valla" en lugar de "vaya" (verbo ir)
                if let Some(p) = prev {
                    // "que valla" = "que vaya"
                    if matches!(p, "que" | "ojalá" | "quizá" | "aunque") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                // "valla a" = "vaya a"
                if let Some(n) = next {
                    if n == "a" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                None
            }
            "baya" => {
                // "baya" es fruto pequeño
                // Error: usar "baya" en lugar de "vaya"
                if let Some(p) = prev {
                    if matches!(p, "que" | "ojalá" | "quizá" | "aunque") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                if let Some(n) = next {
                    if n == "a" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "vaya"),
                            reason: "Subjuntivo de ir".to_string(),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// hecho (participio hacer) / echo (verbo echar)
    fn check_hecho_echo(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        _next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "echo" => {
                // "echo" es verbo echar (yo echo, él echa)
                // Error: usar "echo" en lugar de "hecho" (participio)
                if let Some(p) = prev {
                    // "he echo" = "he hecho"
                    if matches!(p, "he" | "has" | "ha" | "hemos" | "habéis" | "han" | "había" | "habías") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hecho"),
                            reason: "Participio de hacer".to_string(),
                        });
                    }
                    // "de echo" = "de hecho"
                    if p == "de" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hecho"),
                            reason: "Locución 'de hecho'".to_string(),
                        });
                    }
                }
                None
            }
            "hecho" => {
                // "hecho" es participio de hacer o sustantivo
                // Error: usar "hecho" en lugar de "echo" (echar)
                if let Some(p) = prev {
                    // "lo hecho" cuando debería ser "lo echo" (yo lo echo)
                    // Difícil de detectar sin más contexto
                    // "te hecho de menos" = "te echo de menos"
                    if matches!(p, "te" | "lo" | "la" | "le" | "los" | "las" | "les" | "me" | "nos") {
                        // Podría ser "te echo" pero también "lo hecho está hecho"
                        // Solo corregir casos claros como "te hecho de menos"
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// tuvo (verbo tener) / tubo (sustantivo)
    fn check_tuvo_tubo(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "tubo" => {
                // "tubo" es sustantivo (cilindro)
                // Error: usar "tubo" en lugar de "tuvo" (verbo tener)
                if let Some(n) = next {
                    // "tubo que" = "tuvo que"
                    if n == "que" {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "tuvo"),
                            reason: "Pretérito de tener".to_string(),
                        });
                    }
                }
                // Después de pronombre personal suele ser verbo
                if let Some(p) = prev {
                    if matches!(p, "él" | "ella" | "usted" | "quien" | "que" | "no" | "lo" | "la" | "le") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "tuvo"),
                            reason: "Pretérito de tener".to_string(),
                        });
                    }
                }
                None
            }
            "tuvo" => {
                // "tuvo" es verbo tener
                // Raro confundir en esta dirección, pero verificar contexto de sustantivo
                if let Some(p) = prev {
                    // "el tuvo" cuando es sustantivo = "el tubo"
                    if matches!(p, "el" | "un" | "este" | "ese" | "aquel") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "tubo"),
                            reason: "Sustantivo (cilindro)".to_string(),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// iba (verbo ir) - "iva" no existe
    fn check_iba(word: &str, idx: usize, token: &Token) -> Option<HomophoneCorrection> {
        if word == "iva" {
            // "iva" no existe como palabra (excepto siglas IVA)
            // Si está en minúsculas, probablemente es "iba"
            if token.text == "iva" {
                return Some(HomophoneCorrection {
                    token_index: idx,
                    original: token.text.clone(),
                    suggestion: "iba".to_string(),
                    reason: "Imperfecto de ir (con b)".to_string(),
                });
            }
        }
        None
    }

    /// hierba (planta) / hierva (verbo hervir)
    fn check_hierba_hierva(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        _next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "hierva" => {
                // "hierva" es subjuntivo de hervir
                // Error: usar "hierva" en lugar de "hierba" (planta)
                if let Some(p) = prev {
                    // "la hierva" = "la hierba"
                    if matches!(p, "la" | "una" | "esta" | "esa" | "aquella" | "mala" | "buena") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hierba"),
                            reason: "Sustantivo (planta)".to_string(),
                        });
                    }
                }
                None
            }
            "hierba" => {
                // "hierba" es sustantivo
                // Error: usar "hierba" en lugar de "hierva" (verbo)
                if let Some(p) = prev {
                    // "que hierba" = "que hierva"
                    if matches!(p, "que" | "ojalá" | "cuando") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "hierva"),
                            reason: "Subjuntivo de hervir".to_string(),
                        });
                    }
                }
                None
            }
            "yerba" | "yerva" => {
                // Variantes, "yerba" es aceptado, "yerva" no
                if word == "yerva" {
                    return Some(HomophoneCorrection {
                        token_index: idx,
                        original: token.text.clone(),
                        suggestion: Self::preserve_case(&token.text, "yerba"),
                        reason: "Variante de hierba".to_string(),
                    });
                }
                None
            }
            _ => None,
        }
    }

    /// bello (hermoso) / vello (pelo)
    fn check_bello_vello(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "vello" => {
                // "vello" es pelo fino
                // Error: usar "vello" en lugar de "bello" (hermoso)
                if let Some(p) = prev {
                    // "muy vello" = "muy bello"
                    if matches!(p, "muy" | "tan" | "qué" | "más" | "menos" | "lo") {
                        return Some(HomophoneCorrection {
                            token_index: idx,
                            original: token.text.clone(),
                            suggestion: Self::preserve_case(&token.text, "bello"),
                            reason: "Adjetivo (hermoso)".to_string(),
                        });
                    }
                }
                None
            }
            "bello" => {
                // "bello" es adjetivo
                // Error: usar "bello" en lugar de "vello" (pelo)
                if let Some(p) = prev {
                    // "el bello corporal" = "el vello corporal"
                    if matches!(p, "el" | "del" | "con" | "sin") {
                        // Solo si parece contexto de pelo
                        // Difícil de detectar, mejor no corregir
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// botar (saltar/tirar) / votar (elecciones)
    fn check_botar_votar(
        word: &str,
        idx: usize,
        token: &Token,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Option<HomophoneCorrection> {
        match word {
            "botar" | "boto" | "bota" | "botas" | "botan" | "botó" | "botaron" => {
                // Verbos de botar (saltar/tirar basura)
                // Error: usar "botar" en lugar de "votar"
                if let Some(n) = next {
                    // "botar por" = "votar por"
                    if matches!(n, "por" | "a" | "en") {
                        if let Some(p) = prev {
                            if matches!(p, "voy" | "vamos" | "vas" | "van" | "ir" | "para" | "quiero" | "puedo" | "debo") {
                                let suggestion = word.replacen('b', "v", 1);
                                return Some(HomophoneCorrection {
                                    token_index: idx,
                                    original: token.text.clone(),
                                    suggestion: Self::preserve_case(&token.text, &suggestion),
                                    reason: "Verbo votar (elecciones)".to_string(),
                                });
                            }
                        }
                    }
                }
                None
            }
            "votar" | "voto" | "vota" | "votas" | "votan" | "votó" | "votaron" => {
                // Verbos de votar
                // Error: usar "votar" en lugar de "botar" (tirar)
                if let Some(n) = next {
                    // "votar la basura" = "botar la basura"
                    if matches!(n, "la" | "el" | "eso" | "esto" | "aquello") {
                        // Verificar si el contexto sugiere "tirar"
                        // Difícil sin más contexto
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Preserva mayusculas del original
    fn preserve_case(original: &str, replacement: &str) -> String {
        if original.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            let mut chars = replacement.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => replacement.to_string(),
            }
        } else {
            replacement.to_string()
        }
    }

    /// Verifica si hay un limite de oracion entre dos indices de tokens
    fn has_sentence_boundary(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        for idx in (start_idx + 1)..end_idx {
            if idx < tokens.len() {
                let token = &tokens[idx];
                if token.token_type == TokenType::Punctuation {
                    let text = token.text.as_str();
                    if matches!(text, "." | "!" | "?" | ";" | ":" | "\"" | "\u{201D}" | "\u{BB}") {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<HomophoneCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        HomophoneAnalyzer::analyze(&tokens)
    }

    // Tests para hay/ahí/ay
    #[test]
    fn test_hay_correct() {
        let corrections = analyze_text("hay mucha gente");
        assert!(corrections.is_empty(), "No debería corregir 'hay' como verbo");
    }

    #[test]
    fn test_hay_should_be_ahi() {
        let corrections = analyze_text("por hay está");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ahí");
    }

    #[test]
    fn test_ahi_without_accent() {
        let corrections = analyze_text("esta ahi");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ahí");
    }

    #[test]
    fn test_ahi_should_be_hay() {
        let corrections = analyze_text("ahí mucha gente");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hay");
    }

    // Tests para haya/halla
    #[test]
    fn test_halla_should_be_haya() {
        let corrections = analyze_text("que halla llegado");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    #[test]
    fn test_haya_should_be_halla() {
        let corrections = analyze_text("se haya aquí");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "halla");
    }

    #[test]
    fn test_haiga_incorrect() {
        let corrections = analyze_text("que haiga venido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "haya");
    }

    // Tests para vaya/valla
    #[test]
    fn test_valla_should_be_vaya() {
        let corrections = analyze_text("que valla bien");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vaya");
    }

    #[test]
    fn test_valla_a_should_be_vaya() {
        let corrections = analyze_text("valla a casa");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vaya");
    }

    #[test]
    fn test_baya_should_be_vaya() {
        let corrections = analyze_text("que baya rápido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vaya");
    }

    // Tests para hecho/echo
    #[test]
    fn test_echo_should_be_hecho() {
        let corrections = analyze_text("he echo la tarea");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hecho");
    }

    #[test]
    fn test_de_echo_should_be_de_hecho() {
        let corrections = analyze_text("de echo es así");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hecho");
    }

    // Tests para tuvo/tubo
    #[test]
    fn test_tubo_should_be_tuvo() {
        let corrections = analyze_text("tubo que salir");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tuvo");
    }

    #[test]
    fn test_tuvo_should_be_tubo() {
        let corrections = analyze_text("el tuvo de metal");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tubo");
    }

    // Tests para iba/iva
    #[test]
    fn test_iva_should_be_iba() {
        let corrections = analyze_text("iva caminando");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "iba");
    }

    // Tests para hierba/hierva
    #[test]
    fn test_hierva_should_be_hierba() {
        let corrections = analyze_text("la hierva verde");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hierba");
    }

    #[test]
    fn test_hierba_should_be_hierva() {
        let corrections = analyze_text("que hierba el agua");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hierva");
    }

    #[test]
    fn test_yerva_incorrect() {
        let corrections = analyze_text("la yerva");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "yerba");
    }

    // Tests para bello/vello
    #[test]
    fn test_vello_should_be_bello() {
        let corrections = analyze_text("muy vello");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "bello");
    }

    // Test de preservacion de mayusculas
    #[test]
    fn test_preserve_case() {
        let corrections = analyze_text("Haiga venido");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Haya");
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "por" y "hay" estan separados por punto, no debe sugerir "ahi"
        let corrections = analyze_text("Vino por. Hay mucha gente");
        let ahi_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.suggestion == "ahi" || c.suggestion == "ahí")
            .collect();
        assert!(ahi_corrections.is_empty(), "No debe corregir 'hay' cuando hay limite de oracion");
    }
}
