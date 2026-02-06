//! Corrección de dequeísmo y queísmo
//!
//! **Dequeísmo**: uso incorrecto de "de que" donde solo debería ir "que"
//! - "Pienso de que vendrá" → "Pienso que vendrá"
//!
//! **Queísmo**: omisión incorrecta de "de" antes de "que"
//! - "Me alegro que vengas" → "Me alegro de que vengas"

use crate::grammar::{has_sentence_boundary, Token};

/// Correccion sugerida para dequeismo/queismo
#[derive(Debug, Clone)]
pub struct DequeismoCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub error_type: DequeismoErrorType,
    pub reason: String,
}

/// Tipo de error
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DequeismoErrorType {
    /// "de que" donde debería ser solo "que"
    Dequeismo,
    /// "que" donde debería ser "de que"
    Queismo,
}

/// Verbos y expresiones que NO llevan "de" antes de "que" (dequeísmo si lo llevan)
const VERBS_WITHOUT_DE: &[&str] = &[
    // Verbos de pensamiento/opinión
    "pensar", "pienso", "piensas", "piensa", "pensamos", "piensan", "pensé", "pensó",
    "creer", "creo", "crees", "cree", "creemos", "creen", "creí", "creyó",
    "opinar", "opino", "opinas", "opina", "opinamos", "opinan", "opiné", "opinó",
    "considerar", "considero", "consideras", "considera", "consideramos", "consideran",
    "suponer", "supongo", "supones", "supone", "suponemos", "suponen",
    "imaginar", "imagino", "imaginas", "imagina", "imaginamos", "imaginan",
    // Verbos de comunicación
    "decir", "digo", "dices", "dice", "decimos", "dicen", "dije", "dijo",
    "afirmar", "afirmo", "afirmas", "afirma", "afirmamos", "afirman",
    "negar", "niego", "niegas", "niega", "negamos", "niegan",
    "comunicar", "comunico", "comunicas", "comunica", "comunicamos", "comunican",
    "manifestar", "manifiesto", "manifiestas", "manifiesta", "manifestamos",
    "expresar", "expreso", "expresas", "expresa", "expresamos", "expresan",
    "comentar", "comento", "comentas", "comenta", "comentamos", "comentan",
    // Verbos de percepción
    "ver", "veo", "ves", "ve", "vemos", "ven", "vi", "vio",
    "oír", "oigo", "oyes", "oye", "oímos", "oyen", "oí", "oyó",
    "sentir", "siento", "sientes", "siente", "sentimos", "sienten",
    "notar", "noto", "notas", "nota", "notamos", "notan",
    // Verbos de conocimiento
    "saber", "sé", "sabes", "sabe", "sabemos", "saben", "supe", "supo",
    "conocer", "conozco", "conoces", "conoce", "conocemos", "conocen",
    "entender", "entiendo", "entiendes", "entiende", "entendemos", "entienden",
    "comprender", "comprendo", "comprendes", "comprende", "comprendemos",
    // Verbos de voluntad/deseo
    "querer", "quiero", "quieres", "quiere", "queremos", "quieren",
    "desear", "deseo", "deseas", "desea", "deseamos", "desean",
    "esperar", "espero", "esperas", "espera", "esperamos", "esperan",
    "necesitar", "necesito", "necesitas", "necesita", "necesitamos", "necesitan",
    "preferir", "prefiero", "prefieres", "prefiere", "preferimos", "prefieren",
    // Verbos de duda
    "dudar", "dudo", "dudas", "duda", "dudamos", "dudan",
    // Otros
    "parecer", "parece", "parecía",
    "resultar", "resulta", "resultó",
    "suceder", "sucede", "sucedió",
    "ocurrir", "ocurre", "ocurrió",
];

/// Analizador de dequeísmo/queísmo
pub struct DequeismoAnalyzer;

impl DequeismoAnalyzer {
    /// Analiza los tokens y detecta errores de dequeísmo/queísmo
    pub fn analyze(tokens: &[Token]) -> Vec<DequeismoCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .collect();

        // Buscar patrones "verbo + de + que" y "verbo + que"
        for (pos, (idx, token)) in word_tokens.iter().enumerate() {
            // Usar effective_text() para ver correcciones de fases anteriores
            let word_lower = token.effective_text().to_lowercase();

            // Verificar dequeismo: "verbo + de + que" donde no debe haber "de"
            if word_lower == "de" && pos + 1 < word_tokens.len() {
                let next_idx = word_tokens[pos + 1].0;
                // Verificar que no hay limite de oracion entre "de" y "que"
                if !has_sentence_boundary(tokens, *idx, next_idx) {
                    let next_word = word_tokens[pos + 1].1.effective_text().to_lowercase();
                    if next_word == "que" && pos > 0 {
                        let prev_idx = word_tokens[pos - 1].0;
                        // Verificar que no hay limite de oracion entre verbo y "de"
                        if !has_sentence_boundary(tokens, prev_idx, *idx) {
                            let prev_word = word_tokens[pos - 1].1.effective_text().to_lowercase();
                            if Self::is_dequeismo_verb(&prev_word) {
                                if Self::is_nominal_duda_context(&word_tokens, pos, tokens) {
                                    continue;
                                }
                                corrections.push(DequeismoCorrection {
                                    token_index: *idx,
                                    original: token.text.clone(),
                                    suggestion: String::new(), // Eliminar "de"
                                    error_type: DequeismoErrorType::Dequeismo,
                                    reason: format!("'{}' no lleva 'de' antes de 'que'", prev_word),
                                });
                            }
                        }
                    }
                }
            }

            // Verificar queismo: "expresion + que" donde deberia haber "de que"
            if word_lower == "que" && pos > 0 {
                let prev_idx = word_tokens[pos - 1].0;
                // Verificar que no hay limite de oracion entre palabra anterior y "que"
                if !has_sentence_boundary(tokens, prev_idx, *idx) {
                    let prev_word = word_tokens[pos - 1].1.effective_text().to_lowercase();

                    // Verificar que no sea ya "de que"
                    let is_already_de_que = pos >= 2 &&
                        word_tokens[pos - 1].1.effective_text().to_lowercase() == "de";

                    if !is_already_de_que && Self::needs_de_before_que(&prev_word, &word_tokens, pos, tokens) {
                        corrections.push(DequeismoCorrection {
                            token_index: *idx,
                            original: token.text.clone(),
                            suggestion: "de que".to_string(),
                            error_type: DequeismoErrorType::Queismo,
                            reason: format!("'{}' requiere 'de' antes de 'que'", prev_word),
                        });
                    }
                }
            }
        }

        corrections
    }

    /// Verifica si un verbo NO debe llevar "de" antes de "que"
    fn is_dequeismo_verb(word: &str) -> bool {
        VERBS_WITHOUT_DE.contains(&word)
    }

    fn is_nominal_duda_context(
        word_tokens: &[(usize, &Token)],
        de_pos: usize,
        tokens: &[Token],
    ) -> bool {
        if de_pos == 0 {
            return false;
        }

        let prev_word = word_tokens[de_pos - 1].1.effective_text().to_lowercase();
        if !matches!(prev_word.as_str(), "duda" | "dudas") {
            return false;
        }

        if let Some(ref info) = word_tokens[de_pos - 1].1.word_info {
            if info.category == crate::dictionary::WordCategory::Sustantivo {
                return true;
            }
        }

        if de_pos >= 2 {
            let prev_idx = word_tokens[de_pos - 1].0;
            let prev_prev_idx = word_tokens[de_pos - 2].0;
            if has_sentence_boundary(tokens, prev_prev_idx, prev_idx) {
                return false;
            }

            let prev_prev = word_tokens[de_pos - 2].1.effective_text().to_lowercase();
            if Self::is_determiner_or_possessive(&prev_prev) {
                return true;
            }

            if matches!(
                prev_prev.as_str(),
                "cabe"
                    | "caben"
                    | "cabía"
                    | "cabia"
                    | "hay"
                    | "había"
                    | "habia"
                    | "hubo"
                    | "habrá"
                    | "habra"
                    | "habrían"
                    | "habrian"
            ) {
                return true;
            }
        }

        false
    }

    fn is_determiner_or_possessive(word: &str) -> bool {
        matches!(
            word,
            "el"
                | "la"
                | "los"
                | "las"
                | "un"
                | "una"
                | "unos"
                | "unas"
                | "mi"
                | "mis"
                | "tu"
                | "tus"
                | "su"
                | "sus"
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
        )
    }

    /// Verifica patrones pronominales de queísmo: "me/te/se/nos/os + verbo + que".
    /// Incluye presente y pretérito para mejorar cobertura sin perder concordancia de persona.
    fn is_reflexive_queismo_form(pronoun: &str, verb: &str) -> bool {
        match pronoun {
            "me" => matches!(verb,
                "alegro" | "alegr\u{00E9}" |
                "acuerdo" | "acord\u{00E9}" |
                "arrepiento" | "arrepent\u{00ED}" |
                "entero" | "enter\u{00E9}" |
                "olvido" | "olvid\u{00E9}" |
                "quejo" | "quej\u{00E9}" |
                "aseguro" | "asegur\u{00E9}"
            ),
            "te" => matches!(verb,
                "alegras" | "alegraste" |
                "acuerdas" | "acordaste" |
                "arrepientes" | "arrepentiste" |
                "enteras" | "enteraste" |
                "olvidas" | "olvidaste" |
                "quejas" | "quejaste" |
                "aseguras" | "aseguraste"
            ),
            "se" => matches!(verb,
                "alegra" | "alegran" | "alegr\u{00F3}" | "alegraron" |
                "acuerda" | "acuerdan" | "acord\u{00F3}" | "acordaron" |
                "arrepiente" | "arrepienten" | "arrepinti\u{00F3}" | "arrepintieron" |
                "entera" | "enteran" | "enter\u{00F3}" | "enteraron" |
                "olvida" | "olvidan" | "olvid\u{00F3}" | "olvidaron" |
                "queja" | "quejan" | "quej\u{00F3}" | "quejaron" |
                "asegura" | "aseguran" | "asegur\u{00F3}" | "aseguraron"
            ),
            "nos" => matches!(verb,
                "alegramos" | "acordamos" | "arrepentimos" | "enteramos" |
                "olvidamos" | "quejamos" | "aseguramos"
            ),
            "os" => matches!(verb,
                "alegr\u{00E1}is" | "alegrasteis" |
                "acord\u{00E1}is" | "acordasteis" |
                "arrepent\u{00ED}s" | "arrepentisteis" |
                "enter\u{00E1}is" | "enterasteis" |
                "olvid\u{00E1}is" | "olvidasteis" |
                "quej\u{00E1}is" | "quejasteis" |
                "asegur\u{00E1}is" | "asegurasteis"
            ),
            _ => false,
        }
    }

    /// Verifica si una expresion necesita "de" antes de "que"
    fn needs_de_before_que(prev_word: &str, word_tokens: &[(usize, &Token)], pos: usize, tokens: &[Token]) -> bool {
        // Caso especial: "me/te/se + verbo + que" donde el verbo es pronominal
        // Por ejemplo: "me alegro que" -> "me alegro de que"
        // PERO: "me alegra que" es correcto (algo me alegra, no es reflexivo)
        if pos >= 2 {
            // Verificar que no hay limite de oracion entre los tokens
            let prev_idx = word_tokens[pos - 1].0;
            let prev_prev_idx = word_tokens[pos - 2].0;
            if has_sentence_boundary(tokens, prev_prev_idx, prev_idx) {
                return false;
            }
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();

            // Verbos pronominales que requieren "de" cuando el pronombre concuerda.
            if Self::is_reflexive_queismo_form(prev_prev.as_str(), prev_word) {
                return true;
            }

            // "darse cuenta que" → "darse cuenta de que"
            if prev_word == "cuenta" && matches!(prev_prev.as_str(), "di" | "diste" | "dio" | "dimos" | "dieron" | "doy" | "das" | "da" | "damos" | "dan") {
                return true;
            }

            // "estar seguro que" → "estar seguro de que"
            if matches!(prev_word, "seguro" | "segura" | "seguros" | "seguras") {
                if matches!(prev_prev.as_str(), "estoy" | "estás" | "está" | "estamos" | "están" |
                    "estaba" | "estabas" | "estábamos" | "estaban" |
                    "es" | "soy" | "eres" | "somos" | "son") {
                    return true;
                }
            }

            // "estar convencido que" → "estar convencido de que"
            if matches!(prev_word, "convencido" | "convencida" | "convencidos" | "convencidas") {
                if matches!(prev_prev.as_str(), "estoy" | "estás" | "está" | "estamos" | "están") {
                    return true;
                }
            }
        }

        // "tener miedo/ganas que" → "tener miedo/ganas de que"
        if pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if matches!(prev_prev.as_str(), "tengo" | "tienes" | "tiene" | "tenemos" | "tienen" |
                "tenía" | "tenías" | "teníamos" | "tenían" | "tuve" | "tuvo") {
                if matches!(prev_word, "miedo" | "ganas" | "culpa" | "idea" | "duda" |
                    "derecho" | "necesidad" | "obligación" | "intención" | "esperanza" | "certeza") {
                    return true;
                }
            }
        }

        // "a pesar que" → "a pesar de que"
        if prev_word == "pesar" && pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if prev_prev == "a" {
                return true;
            }
        }

        // "en vez que" → "en vez de que" (raro, pero posible)
        if matches!(prev_word, "vez" | "lugar" | "caso") && pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if prev_prev == "en" {
                return true;
            }
        }

        // "el hecho que" → "el hecho de que"
        if prev_word == "hecho" && pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if prev_prev == "el" {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<DequeismoCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        DequeismoAnalyzer::analyze(&tokens)
    }

    // Tests de dequeísmo
    #[test]
    fn test_pienso_de_que_dequeismo() {
        let corrections = analyze_text("pienso de que vendrá");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_creo_de_que_dequeismo() {
        let corrections = analyze_text("creo de que es verdad");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_dijo_de_que_dequeismo() {
        let corrections = analyze_text("dijo de que vendría");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_opino_de_que_dequeismo() {
        let corrections = analyze_text("opino de que deberías");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_supongo_de_que_dequeismo() {
        let corrections = analyze_text("supongo de que sí");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_parece_de_que_dequeismo() {
        let corrections = analyze_text("parece de que llueve");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_duda_nominal_de_que_no_dequeismo_false_positive() {
        let corrections = analyze_text("No cabe duda de que es verdad");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'no cabe duda de que': {corrections:?}"
        );

        let corrections = analyze_text("No hay duda de que vendrá");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'no hay duda de que': {corrections:?}"
        );

        let corrections = analyze_text("Tengo la duda de que sea correcto");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'la duda de que': {corrections:?}"
        );
    }

    #[test]
    fn test_duda_verb_de_que_still_detected_as_dequeismo() {
        let corrections = analyze_text("Él duda de que sea cierto");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    // Tests de queísmo
    #[test]
    fn test_me_alegro_que_queismo() {
        let corrections = analyze_text("me alegro que vengas");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
        assert_eq!(corrections[0].suggestion, "de que");
    }

    #[test]
    fn test_me_acuerdo_que_queismo() {
        let corrections = analyze_text("me acuerdo que era as\u{00ED}");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_me_acorde_que_queismo() {
        let corrections = analyze_text("me acord\u{00E9} que era as\u{00ED}");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_se_alegraron_que_queismo() {
        let corrections = analyze_text("se alegraron que ganaron");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_se_entero_que_queismo() {
        let corrections = analyze_text("se enter\u{00F3} que era tarde");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_se_olvido_que_queismo() {
        let corrections = analyze_text("se olvid\u{00F3} que ten\u{00ED}a cita");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_estoy_seguro_que_queismo() {
        let corrections = analyze_text("estoy seguro que vendrá");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_me_di_cuenta_que_queismo() {
        let corrections = analyze_text("me di cuenta que era tarde");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_tengo_miedo_que_queismo() {
        let corrections = analyze_text("tengo miedo que pase");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_a_pesar_que_queismo() {
        let corrections = analyze_text("a pesar que llovía");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    #[test]
    fn test_el_hecho_que_queismo() {
        let corrections = analyze_text("el hecho que vino");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
    }

    // Tests de casos correctos (no deben generar correcciones)
    #[test]
    fn test_pienso_que_correct() {
        let corrections = analyze_text("pienso que vendrá");
        assert!(corrections.is_empty(), "No debería corregir 'pienso que'");
    }

    #[test]
    fn test_me_alegro_de_que_correct() {
        let corrections = analyze_text("me alegro de que vengas");
        assert!(corrections.is_empty(), "No debería corregir 'me alegro de que'");
    }

    #[test]
    fn test_creo_que_correct() {
        let corrections = analyze_text("creo que es verdad");
        assert!(corrections.is_empty(), "No debería corregir 'creo que'");
    }

    #[test]
    fn test_estoy_seguro_de_que_correct() {
        let corrections = analyze_text("estoy seguro de que vendrá");
        assert!(corrections.is_empty(), "No debería corregir 'estoy seguro de que'");
    }

    #[test]
    fn test_me_alegra_que_correct() {
        // "Me alegra que" es correcto: algo me alegra (impersonal, no reflexivo)
        // vs "Me alegro de que" (reflexivo)
        let corrections = analyze_text("me alegra que hayas venido");
        assert!(corrections.is_empty(), "No debería corregir 'me alegra que' (impersonal)");
    }

    #[test]
    fn test_te_alegra_que_correct() {
        let corrections = analyze_text("te alegra que sea así");
        assert!(corrections.is_empty(), "No debería corregir 'te alegra que' (impersonal)");
    }

    #[test]
    fn test_nos_alegra_que_correct() {
        let corrections = analyze_text("nos alegra que vengas");
        assert!(corrections.is_empty(), "No deberia corregir 'nos alegra que' (impersonal)");
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "pienso" y "de que" estan separados por punto, no debe detectar dequeismo
        let corrections = analyze_text("Yo pienso. De que vengas depende todo");
        let dequeismo_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.error_type == DequeismoErrorType::Dequeismo)
            .collect();
        assert!(dequeismo_corrections.is_empty(), "No debe detectar dequeismo cuando hay limite de oracion");
    }
}
