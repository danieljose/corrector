//! Detección de redundancias pleonásticas
//!
//! Detecta expresiones redundantes donde se repite innecesariamente una idea.
//! Ejemplo: "subir arriba" → "subir", "bajar abajo" → "bajar"

use crate::grammar::tokenizer::TokenType;
use crate::grammar::{has_sentence_boundary, Token};

/// Corrección de pleonasmo
#[derive(Debug, Clone)]
pub struct PleonasmCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub message: String,
}

/// Tipo de pleonasmo
#[derive(Debug, Clone)]
pub enum PleonasmType {
    /// Verbo + adverbio redundante (subir arriba)
    VerbAdverb,
    /// Sustantivo + complemento redundante (lapso de tiempo)
    NounComplement,
    /// Verbo + verbo redundante (volver a repetir)
    VerbVerb,
}

/// Analizador de pleonasmos
pub struct PleonasmAnalyzer;

impl PleonasmAnalyzer {
    /// Analiza tokens buscando redundancias pleonásticas
    pub fn analyze(tokens: &[Token]) -> Vec<PleonasmCorrection> {
        let mut corrections = Vec::new();

        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Buscar patrones de dos palabras (verbo + adverbio)
        for i in 0..word_tokens.len().saturating_sub(1) {
            let (idx1, token1) = word_tokens[i];
            let (idx2, token2) = word_tokens[i + 1];

            // Verificar que no hay limite de oracion entre las palabras
            if has_sentence_boundary(tokens, idx1, idx2) {
                continue;
            }

            let word1 = token1.effective_text().to_lowercase();
            let word2 = token2.effective_text().to_lowercase();

            if let Some(message) = Self::check_redundant_comparative(&word1, &word2) {
                corrections.push(PleonasmCorrection {
                    token_index: idx1,
                    original: token1.text.clone(),
                    suggestion: "sobra".to_string(),
                    message,
                });
                continue;
            }

            if let Some((suggestion, message)) = Self::check_verb_adverb(&word1, &word2) {
                corrections.push(PleonasmCorrection {
                    token_index: idx2,
                    original: token2.text.clone(),
                    suggestion,
                    message,
                });
            }
        }

        // Buscar patrones de tres palabras (sustantivo + de + sustantivo, volver + a + verbo)
        for i in 0..word_tokens.len().saturating_sub(2) {
            let (idx1, token1) = word_tokens[i];
            let (idx2, token2) = word_tokens[i + 1];
            let (idx3, token3) = word_tokens[i + 2];

            // Verificar que no hay limite de oracion entre las palabras
            if has_sentence_boundary(tokens, idx1, idx2)
                || has_sentence_boundary(tokens, idx2, idx3)
            {
                continue;
            }

            let word1 = token1.effective_text().to_lowercase();
            let word2 = token2.effective_text().to_lowercase();
            let word3 = token3.effective_text().to_lowercase();

            // Patron: sustantivo + de + sustantivo redundante
            if word2 == "de" {
                if let Some((suggestion, message)) = Self::check_noun_complement(&word1, &word3) {
                    // Marcar "de" como sobrante
                    corrections.push(PleonasmCorrection {
                        token_index: idx2,
                        original: format!("{} {}", token2.text, token3.text),
                        suggestion,
                        message: message.clone(),
                    });
                    // Tambien marcar el complemento
                    corrections.push(PleonasmCorrection {
                        token_index: idx3,
                        original: token3.text.clone(),
                        suggestion: "sobra".to_string(),
                        message,
                    });
                }
            }

            // Patron: volver + a + verbo redundante
            if word2 == "a" {
                if let Some((suggestion, message)) = Self::check_verb_verb(&word1, &word3) {
                    corrections.push(PleonasmCorrection {
                        token_index: idx3,
                        original: token3.text.clone(),
                        suggestion,
                        message,
                    });
                }
            }
        }

        corrections
    }

    /// Verifica redundancias de verbo + adverbio
    fn check_verb_adverb(verb: &str, adverb: &str) -> Option<(String, String)> {
        // Normalizar formas verbales conjugadas
        let verb_base = Self::get_verb_base(verb);

        let redundant_pairs = [
            // Movimiento vertical
            (
                "subir",
                "arriba",
                "El verbo 'subir' ya implica dirección hacia arriba",
            ),
            (
                "bajar",
                "abajo",
                "El verbo 'bajar' ya implica dirección hacia abajo",
            ),
            (
                "ascender",
                "arriba",
                "El verbo 'ascender' ya implica dirección hacia arriba",
            ),
            (
                "descender",
                "abajo",
                "El verbo 'descender' ya implica dirección hacia abajo",
            ),
            // Movimiento horizontal/direccional
            (
                "salir",
                "afuera",
                "El verbo 'salir' ya implica dirección hacia afuera",
            ),
            (
                "salir",
                "fuera",
                "El verbo 'salir' ya implica dirección hacia afuera",
            ),
            (
                "entrar",
                "adentro",
                "El verbo 'entrar' ya implica dirección hacia adentro",
            ),
            (
                "entrar",
                "dentro",
                "El verbo 'entrar' ya implica dirección hacia adentro",
            ),
            (
                "meter",
                "adentro",
                "El verbo 'meter' ya implica dirección hacia adentro",
            ),
            (
                "meter",
                "dentro",
                "El verbo 'meter' ya implica dirección hacia adentro",
            ),
            (
                "sacar",
                "afuera",
                "El verbo 'sacar' ya implica dirección hacia afuera",
            ),
            (
                "sacar",
                "fuera",
                "El verbo 'sacar' ya implica dirección hacia afuera",
            ),
            // Otros
            (
                "adelantar",
                "adelante",
                "El verbo 'adelantar' ya implica dirección hacia adelante",
            ),
            (
                "retroceder",
                "atrás",
                "El verbo 'retroceder' ya implica dirección hacia atrás",
            ),
            (
                "acular",
                "atrás",
                "El verbo 'acular' ya implica dirección hacia atrás",
            ),
        ];

        for (v, a, msg) in redundant_pairs.iter() {
            if verb_base == *v && adverb == *a {
                return Some(("sobra".to_string(), msg.to_string()));
            }
        }

        None
    }

    fn check_redundant_comparative(first: &str, second: &str) -> Option<String> {
        let is_degree_marker = matches!(first, "mas" | "más");
        if !is_degree_marker {
            return None;
        }

        if matches!(
            second,
            "mejor" | "peor" | "superior" | "inferior" | "mayor" | "menor"
        ) {
            return Some(format!(
                "'{}' ya es comparativo sintético; '{}' es redundante",
                second, first
            ));
        }

        None
    }

    /// Verifica redundancias de sustantivo + de + complemento
    fn check_noun_complement(noun: &str, complement: &str) -> Option<(String, String)> {
        let redundant_pairs = [
            ("lapso", "tiempo", "Un 'lapso' ya es un período de tiempo"),
            ("erario", "público", "El 'erario' es por definición público"),
            ("mendrugo", "pan", "Un 'mendrugo' ya es un trozo de pan"),
            ("cardumen", "peces", "Un 'cardumen' ya es un grupo de peces"),
            ("jauría", "perros", "Una 'jauría' ya es un grupo de perros"),
            ("piara", "cerdos", "Una 'piara' ya es un grupo de cerdos"),
            (
                "hemorragia",
                "sangre",
                "Una 'hemorragia' ya es pérdida de sangre",
            ),
            (
                "tuberculosis",
                "pulmonar",
                "La 'tuberculosis' afecta típicamente los pulmones",
            ),
            (
                "utopía",
                "inalcanzable",
                "Una 'utopía' ya es algo inalcanzable",
            ),
            (
                "panacea",
                "universal",
                "Una 'panacea' ya es un remedio universal",
            ),
            (
                "prerrequisito",
                "previo",
                "Un 'prerrequisito' ya es algo previo",
            ),
            (
                "prever",
                "anticipación",
                "El verbo 'prever' ya implica anticipación",
            ),
        ];

        for (n, c, msg) in redundant_pairs.iter() {
            if noun == *n && complement == *c {
                return Some(("sobra".to_string(), msg.to_string()));
            }
        }

        None
    }

    /// Verifica redundancias de volver + a + verbo
    fn check_verb_verb(verb1: &str, verb2: &str) -> Option<(String, String)> {
        let verb1_base = Self::get_verb_base(verb1);
        let verb2_base = Self::get_verb_base(verb2);

        // volver a repetir → repetir (repetir ya implica volver a hacer)
        if verb1_base == "volver" && verb2_base == "repetir" {
            return Some((
                "sobra 'volver a'".to_string(),
                "'Repetir' ya significa volver a hacer, 'volver a repetir' es redundante"
                    .to_string(),
            ));
        }

        // volver a reiterar → reiterar
        if verb1_base == "volver" && verb2_base == "reiterar" {
            return Some((
                "sobra 'volver a'".to_string(),
                "'Reiterar' ya significa volver a decir, 'volver a reiterar' es redundante"
                    .to_string(),
            ));
        }

        // volver a reiniciar → reiniciar
        if verb1_base == "volver"
            && (verb2_base == "reiniciar" || verb2_base == "recomenzar" || verb2_base == "reanudar")
        {
            return Some((
                "sobra 'volver a'".to_string(),
                format!("'{}' ya implica volver a empezar", verb2_base),
            ));
        }

        None
    }

    /// Obtiene la forma base del verbo (simplificada)
    fn get_verb_base(verb: &str) -> &str {
        // Formas de "subir"
        if verb.starts_with("sub")
            && (verb.ends_with("o")
                || verb.ends_with("es")
                || verb.ends_with("e")
                || verb.ends_with("imos")
                || verb.ends_with("ís")
                || verb.ends_with("en")
                || verb.ends_with("í")
                || verb.ends_with("ió")
                || verb.ends_with("ieron")
                || verb.ends_with("ía")
                || verb.ends_with("ían")
                || verb == "subir")
        {
            return "subir";
        }

        // Formas de "bajar"
        if verb.starts_with("baj")
            && (verb.ends_with("o")
                || verb.ends_with("as")
                || verb.ends_with("a")
                || verb.ends_with("amos")
                || verb.ends_with("áis")
                || verb.ends_with("an")
                || verb.ends_with("é")
                || verb.ends_with("ó")
                || verb.ends_with("aron")
                || verb.ends_with("aba")
                || verb.ends_with("aban")
                || verb == "bajar")
        {
            return "bajar";
        }

        // Formas de "salir" (incluye imperativo/subjuntivo/futuro/condicional)
        if Self::is_form_of_salir(verb) {
            return "salir";
        }

        // Formas de "entrar"
        if verb.starts_with("entr")
            && (verb.ends_with("o")
                || verb.ends_with("as")
                || verb.ends_with("a")
                || verb.ends_with("amos")
                || verb.ends_with("áis")
                || verb.ends_with("an")
                || verb.ends_with("é")
                || verb.ends_with("ó")
                || verb.ends_with("aron")
                || verb.ends_with("aba")
                || verb.ends_with("aban")
                || verb == "entrar")
        {
            return "entrar";
        }

        // Formas de "meter"
        if verb.starts_with("met")
            && (verb.ends_with("o")
                || verb.ends_with("es")
                || verb.ends_with("e")
                || verb.ends_with("emos")
                || verb.ends_with("éis")
                || verb.ends_with("en")
                || verb.ends_with("í")
                || verb.ends_with("ió")
                || verb.ends_with("ieron")
                || verb.ends_with("ía")
                || verb.ends_with("ían")
                || verb == "meter")
        {
            return "meter";
        }

        // Formas de "sacar"
        if verb.starts_with("sac")
            && (verb.ends_with("o")
                || verb.ends_with("as")
                || verb.ends_with("a")
                || verb.ends_with("amos")
                || verb.ends_with("áis")
                || verb.ends_with("an")
                || verb.ends_with("qué")
                || verb.ends_with("ó")
                || verb.ends_with("aron")
                || verb.ends_with("aba")
                || verb.ends_with("aban")
                || verb == "sacar")
        {
            return "sacar";
        }

        // Formas de "volver"
        if verb == "volver"
            || verb == "vuelvo"
            || verb == "vuelves"
            || verb == "vuelve"
            || verb == "volvemos"
            || verb == "volvéis"
            || verb == "vuelven"
            || verb == "volví"
            || verb == "volvió"
            || verb == "volvieron"
            || verb == "volvía"
            || verb == "volvían"
        {
            return "volver";
        }

        // Formas de "repetir"
        if verb == "repetir"
            || verb == "repito"
            || verb == "repites"
            || verb == "repite"
            || verb == "repetimos"
            || verb == "repetís"
            || verb == "repiten"
            || verb == "repetí"
            || verb == "repitió"
            || verb == "repitieron"
        {
            return "repetir";
        }

        // Formas de "reiterar"
        if verb.starts_with("reiter") {
            return "reiterar";
        }

        // Formas de "reiniciar", "recomenzar", "reanudar"
        if verb.starts_with("reinici") {
            return "reiniciar";
        }
        if verb.starts_with("recomenz") || verb.starts_with("recomienz") {
            return "recomenzar";
        }
        if verb.starts_with("reanud") {
            return "reanudar";
        }

        // Formas de "ascender"
        if verb.starts_with("ascend") || verb.starts_with("asciend") {
            return "ascender";
        }

        // Formas de "descender"
        if verb.starts_with("descend") || verb.starts_with("desciend") {
            return "descender";
        }

        // Formas de "adelantar"
        if verb.starts_with("adelant") {
            return "adelantar";
        }

        // Formas de "retroceder"
        if verb.starts_with("retroced") {
            return "retroceder";
        }

        verb
    }

    fn is_form_of_salir(verb: &str) -> bool {
        matches!(
            verb,
            "salir"
                | "sal"
                | "salid"
                | "salgo"
                | "sales"
                | "sale"
                | "salimos"
                | "salís"
                | "salen"
                | "salí"
                | "saliste"
                | "salió"
                | "salisteis"
                | "salieron"
                | "salía"
                | "salías"
                | "salíamos"
                | "salíais"
                | "salían"
                | "salga"
                | "salgas"
                | "salgamos"
                | "salgáis"
                | "salgan"
                | "saliera"
                | "salieras"
                | "saliéramos"
                | "salieramos"
                | "salierais"
                | "salieran"
                | "saliese"
                | "salieses"
                | "saliésemos"
                | "saliesemos"
                | "salieseis"
                | "saliesen"
                | "saldré"
                | "saldrás"
                | "saldrá"
                | "saldremos"
                | "saldréis"
                | "saldrán"
                | "saldria"
                | "saldría"
                | "saldrías"
                | "saldríamos"
                | "saldríais"
                | "saldrían"
                | "saliendo"
                | "salido"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::tokenizer::Tokenizer;

    fn tokenize(text: &str) -> Vec<Token> {
        Tokenizer::new().tokenize(text)
    }

    // Tests de verbo + adverbio
    #[test]
    fn test_subir_arriba() {
        let tokens = tokenize("voy a subir arriba");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "arriba");
        assert_eq!(corrections[0].suggestion, "sobra");
    }

    #[test]
    fn test_bajar_abajo() {
        let tokens = tokenize("tengo que bajar abajo");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "abajo");
    }

    #[test]
    fn test_salir_afuera() {
        let tokens = tokenize("vamos a salir afuera");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "afuera");
    }

    #[test]
    fn test_entrar_adentro() {
        let tokens = tokenize("hay que entrar adentro");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "adentro");
    }

    #[test]
    fn test_meter_adentro() {
        let tokens = tokenize("voy a meter adentro la ropa");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "adentro");
    }

    #[test]
    fn test_sacar_afuera() {
        let tokens = tokenize("hay que sacar afuera la basura");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "afuera");
    }

    #[test]
    fn test_conjugated_sube_arriba() {
        let tokens = tokenize("él sube arriba");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "arriba");
    }

    #[test]
    fn test_conjugated_bajaron_abajo() {
        let tokens = tokenize("ellos bajaron abajo");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "abajo");
    }

    #[test]
    fn test_conjugated_sale_afuera() {
        let tokens = tokenize("el perro sale afuera");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "afuera");
    }

    #[test]
    fn test_imperative_sal_afuera() {
        let tokens = tokenize("sal afuera a jugar");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "afuera");
    }

    #[test]
    fn test_subjunctive_salgan_afuera() {
        let tokens = tokenize("salgan afuera todos");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "afuera");
    }

    // Tests de sustantivo + de + complemento
    #[test]
    fn test_lapso_de_tiempo() {
        let tokens = tokenize("en un lapso de tiempo");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(!corrections.is_empty());
        assert!(corrections.iter().any(|c| c.original == "tiempo"));
    }

    #[test]
    fn test_erario_publico() {
        // "erario público" (sin "de") no es detectado por el patrón actual
        // El patrón detecta "sustantivo + de + complemento"
        let tokens = tokenize("el erario público");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        // No hay correcciones porque no hay "de" entre las palabras
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_cardumen_de_peces() {
        let tokens = tokenize("un cardumen de peces");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(!corrections.is_empty());
    }

    // Tests de volver a + verbo
    #[test]
    fn test_volver_a_repetir() {
        let tokens = tokenize("voy a volver a repetir");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(!corrections.is_empty());
        assert!(corrections.iter().any(|c| c.original == "repetir"));
    }

    #[test]
    fn test_vuelve_a_reiterar() {
        let tokens = tokenize("él vuelve a reiterar");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(!corrections.is_empty());
    }

    // Tests de casos correctos (sin redundancia)
    #[test]
    fn test_subir_sin_arriba() {
        let tokens = tokenize("voy a subir la escalera");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_salir_sin_afuera() {
        let tokens = tokenize("vamos a salir de paseo");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_arriba_sin_subir() {
        let tokens = tokenize("esta arriba en el tejado");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty());
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "subir" y "arriba" estan separados por punto, no debe detectar pleonasmo
        let tokens = tokenize("Voy a subir. Arriba hace frio");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debe detectar pleonasmo cuando hay limite de oracion"
        );
    }

    #[test]
    fn test_mas_mejor_redundant_comparative() {
        let tokens = tokenize("esto es mas mejor");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(
            corrections
                .iter()
                .any(|c| c.original.eq_ignore_ascii_case("mas") && c.suggestion == "sobra"),
            "Debe marcar 'más' como redundante en 'más mejor': {:?}",
            corrections
        );
    }

    #[test]
    fn test_mas_superior_redundant_comparative() {
        let tokens = tokenize("nivel mas superior");
        let corrections = PleonasmAnalyzer::analyze(&tokens);
        assert!(
            corrections
                .iter()
                .any(|c| c.original.eq_ignore_ascii_case("mas") && c.suggestion == "sobra"),
            "Debe marcar 'más' como redundante en 'más superior': {:?}",
            corrections
        );
    }

    #[test]
    fn test_mas_mayor_menor_marked_as_pleonasm() {
        for text in ["es mas mayor que yo", "es mas menor de edad"] {
            let tokens = tokenize(text);
            let corrections = PleonasmAnalyzer::analyze(&tokens);
            let has_mas_redundant = corrections
                .iter()
                .any(|c| c.original.eq_ignore_ascii_case("mas") && c.suggestion == "sobra");
            assert!(
                has_mas_redundant,
                "Debe marcar '{}': {:?}",
                text, corrections
            );
        }
    }
}
