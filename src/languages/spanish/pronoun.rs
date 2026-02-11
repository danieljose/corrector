//! Corrección de laísmo, leísmo y loísmo
//!
//! **Laísmo**: usar "la/las" como complemento indirecto
//! - "La dije la verdad" → "Le dije la verdad"
//!
//! **Leísmo**: usar "le/les" como complemento directo
//! - "Le vi en el parque" → "Lo vi en el parque" (aunque el leísmo de persona masculina es aceptado)
//!
//! **Loísmo**: usar "lo/los" como complemento indirecto (muy raro)
//! - "Lo dije que viniera" → "Le dije que viniera"

use crate::grammar::{has_sentence_boundary, Token};

/// Corrección sugerida para laísmo/leísmo/loísmo
#[derive(Debug, Clone)]
pub struct PronounCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub error_type: PronounErrorType,
    pub reason: String,
}

/// Tipo de error
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PronounErrorType {
    /// "la/las" como CI en lugar de "le/les"
    Laismo,
    /// "le/les" como CD en lugar de "lo/la/los/las"
    Leismo,
    /// "lo/los" como CI en lugar de "le/les"
    Loismo,
    /// Inversion coloquial de cliticos: "me/te se" -> "se me/te"
    CliticInversion,
}

/// Verbos que típicamente requieren complemento INDIRECTO (le/les)
/// Estos son verbos de transferencia, comunicación, etc.
const VERBS_INDIRECT_OBJECT: &[&str] = &[
    // Verbos de comunicación
    "decir",
    "digo",
    "dices",
    "dice",
    "decimos",
    "dicen",
    "dije",
    "dijo",
    "dijeron",
    "dicho",
    "diciendo",
    "contar",
    "cuento",
    "cuentas",
    "cuenta",
    "contamos",
    "cuentan",
    "conté",
    "contó",
    "preguntar",
    "pregunto",
    "preguntas",
    "pregunta",
    "preguntamos",
    "preguntan",
    "pregunté",
    "preguntó",
    "responder",
    "respondo",
    "respondes",
    "responde",
    "respondemos",
    "responden",
    "respondí",
    "respondió",
    "contestar",
    "contesto",
    "contestas",
    "contesta",
    "contestamos",
    "contestan",
    "explicar",
    "explico",
    "explicas",
    "explica",
    "explicamos",
    "explican",
    "expliqué",
    "explicó",
    "comunicar",
    "comunico",
    "comunicas",
    "comunica",
    "comunicamos",
    "comunican",
    "informar",
    "informo",
    "informas",
    "informa",
    "informamos",
    "informan",
    "avisar",
    "aviso",
    "avisas",
    "avisa",
    "avisamos",
    "avisan",
    "advertir",
    "advierto",
    "adviertes",
    "advierte",
    "advertimos",
    "advierten",
    "gritar",
    "grito",
    "gritas",
    "grita",
    "gritamos",
    "gritan",
    "susurrar",
    "susurro",
    "susurras",
    "susurra",
    "susurramos",
    "susurran",
    // Verbos de transferencia
    // NOTA: Verbos ditransitivos (dar, regalar, etc.) se incluyen con cautela.
    // "la dieron" puede ser CD (dieron la cosa) o laísmo (le dieron a ella).
    // Solo marcamos los más claros donde el CD es típicamente una cláusula (que...)
    "dar",
    "doy",
    "das",
    "da",
    "damos",
    "dan",
    "di",
    "dio",
    "dieron",
    "dado",
    "dando",
    "regalar",
    "regalo",
    "regalas",
    "regala",
    "regalamos",
    "regalan",
    "regalé",
    "regaló",
    "prestar",
    "presto",
    "prestas",
    "presta",
    "prestamos",
    "prestan",
    "enviar",
    "envío",
    "envías",
    "envía",
    "enviamos",
    "envían",
    "envié",
    "envió",
    "mandar",
    "mando",
    "mandas",
    "manda",
    "mandamos",
    "mandan",
    "ofrecer",
    "ofrezco",
    "ofreces",
    "ofrece",
    "ofrecemos",
    "ofrecen",
    // ELIMINADOS: devolver, traer, entregar, llevar - son ditransitivos y causan
    // falsos positivos cuando "la/las" es CD legítimo ("la devuelven" = devuelven la cosa)
    // Verbos de enseñanza/muestra
    "enseñar",
    "enseño",
    "enseñas",
    "enseña",
    "enseñamos",
    "enseñan",
    "mostrar",
    "muestro",
    "muestras",
    "muestra",
    "mostramos",
    "muestran",
    "demostrar",
    "demuestro",
    "demuestras",
    "demuestra",
    "demostramos",
    // Verbos de petición
    "pedir",
    "pido",
    "pides",
    "pide",
    "pedimos",
    "piden",
    "pedí",
    "pidió",
    "rogar",
    "ruego",
    "ruegas",
    "ruega",
    "rogamos",
    "ruegan",
    "suplicar",
    "suplico",
    "suplicas",
    "suplica",
    "suplicamos",
    "suplican",
    "exigir",
    "exijo",
    "exiges",
    "exige",
    "exigimos",
    "exigen",
    "ordenar",
    "ordeno",
    "ordenas",
    "ordena",
    "ordenamos",
    "ordenan",
    // Verbos de efecto sobre alguien
    "gustar",
    "gusta",
    "gustan",
    "gustó",
    "gustaron",
    "gustaba",
    "gustaban",
    "interesar",
    "interesa",
    "interesan",
    "interesó",
    "interesaban",
    "importar",
    "importa",
    "importan",
    "importó",
    "importaba",
    "molestar",
    "molesta",
    "molestan",
    "molestó",
    "molestaba",
    "doler",
    "duele",
    "duelen",
    "dolió",
    "dolía",
    "parecer",
    "parece",
    "parecen",
    "pareció",
    "parecía",
    "faltar",
    "falta",
    "faltan",
    "faltó",
    "faltaba",
    "sobrar",
    "sobra",
    "sobran",
    "sobró",
    "sobraba",
    "convenir",
    "conviene",
    "convienen",
    "convino",
    "convenía",
    "pertenecer",
    "pertenece",
    "pertenecen",
    "perteneció",
    "corresponder",
    "corresponde",
    "corresponden",
    "correspondió",
    // Otros verbos con CI
    "agradecer",
    "agradezco",
    "agradeces",
    "agradece",
    "agradecemos",
    "perdonar",
    "perdono",
    "perdonas",
    "perdona",
    "perdonamos", // con CI cuando es "perdonar algo A alguien"
    "permitir",
    "permito",
    "permites",
    "permite",
    "permitimos",
    "permiten",
    "prohibir",
    "prohíbo",
    "prohíbes",
    "prohíbe",
    "prohibimos",
    "prohíben",
    "impedir",
    "impido",
    "impides",
    "impide",
    "impedimos",
    "impiden",
    "escribir",
    "escribo",
    "escribes",
    "escribe",
    "escribimos",
    "escriben",
    "escribí",
    "escribió",
];

/// Analizador de laísmo/leísmo/loísmo
pub struct PronounAnalyzer;

impl PronounAnalyzer {
    fn normalize_spanish(word: &str) -> String {
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

    /// Analiza los tokens y detecta errores de pronombres
    pub fn analyze(tokens: &[Token]) -> Vec<PronounCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.is_word())
            .collect();

        for (pos, (idx, token)) in word_tokens.iter().enumerate() {
            // Usar effective_text() para ver correcciones de fases anteriores
            let word_lower = token.effective_text().to_lowercase();

            if let Some(inversion_corrections) =
                Self::detect_me_te_se_inversion(tokens, &word_tokens, pos)
            {
                corrections.extend(inversion_corrections);
                continue;
            }

            // Buscar el verbo siguiente (pronombre + verbo)
            // Solo si no hay limite de oracion entre ellos
            let next_verb = if pos + 1 < word_tokens.len() {
                let next_idx = word_tokens[pos + 1].0;
                if has_sentence_boundary(tokens, *idx, next_idx) {
                    None
                } else {
                    Some(word_tokens[pos + 1].1.effective_text().to_lowercase())
                }
            } else {
                None
            };

            // Detectar laísmo: "la/las" + verbo de CI
            // NOTA: Si la siguiente palabra es un sustantivo, "la" es artículo, no pronombre
            // NOTA: Si la palabra anterior es "se", entonces "la" es CD válido (se la dio = le dio la cosa)
            if matches!(word_lower.as_str(), "la" | "las") {
                // Verificar si la palabra anterior es "se" (combinación se + la/lo es válida)
                let prev_is_se = if pos > 0 {
                    let prev_word = word_tokens[pos - 1].1.effective_text().to_lowercase();
                    prev_word == "se"
                } else {
                    false
                };

                if !prev_is_se {
                    // Evitar falsos positivos en reduplicación de OD topicalizado:
                    // "La carta la escribió Juan", "Las cartas las envié ayer".
                    if Self::has_topicalized_feminine_object_before_pronoun(
                        tokens,
                        &word_tokens,
                        pos,
                        &word_lower,
                    ) {
                        continue;
                    }

                    if let Some(ref verb) = next_verb {
                        let has_ditransitive_laismo_context =
                            pos + 2 < word_tokens.len()
                                && Self::has_clear_laismo_ditransitive_context(
                                    tokens,
                                    &word_tokens,
                                    verb,
                                    pos + 2,
                                );
                        if Self::is_indirect_object_verb(verb)
                            || has_ditransitive_laismo_context
                        {
                            // Verificar si la siguiente palabra es sustantivo (entonces "la" es artículo)
                            let next_token = &word_tokens[pos + 1].1;
                            let is_noun = next_token
                                .word_info
                                .as_ref()
                                .map(|info| {
                                    info.category == crate::dictionary::WordCategory::Sustantivo
                                })
                                .unwrap_or(false);

                            if !is_noun {
                                let suggestion = if word_lower == "la" { "le" } else { "les" };
                                corrections.push(PronounCorrection {
                                    token_index: *idx,
                                    original: token.text.clone(),
                                    suggestion: Self::preserve_case(&token.text, suggestion),
                                    error_type: PronounErrorType::Laismo,
                                    reason: format!("'{}' requiere complemento indirecto", verb),
                                });
                            }
                        }
                    }
                }
            }

            // Detectar loismo: "lo/los" + verbo de CI (raro pero posible)
            if matches!(word_lower.as_str(), "lo" | "los") {
                // "se lo/los" es secuencia clitica valida (CI=se, CD=lo/los),
                // no debe marcarse como loismo.
                let prev_is_se = if pos > 0 {
                    let prev_word = word_tokens[pos - 1].1.effective_text().to_lowercase();
                    prev_word == "se"
                } else {
                    false
                };
                if prev_is_se {
                    continue;
                }

                if let Some(ref verb) = next_verb {
                    // Solo detectar loismo cuando el contexto es claro:
                    // - patron clasico: "lo dije que...", "lo pregunte a..."
                    // - patron ditransitivo: "lo dieron un premio"
                    if Self::is_clear_indirect_verb(verb) || Self::is_loismo_ditransitive_verb(verb)
                    {
                        // Verificar si hay "que" despues del verbo (patron tipico de loismo)
                        if pos + 2 < word_tokens.len() {
                            let verb_idx = word_tokens[pos + 1].0;
                            let after_verb_idx = word_tokens[pos + 2].0;
                            // Verificar que no hay limite de oracion entre verbo y siguiente palabra
                            if !has_sentence_boundary(tokens, verb_idx, after_verb_idx) {
                                let after_verb =
                                    word_tokens[pos + 2].1.effective_text().to_lowercase();
                                let has_classic_context = after_verb == "que" || after_verb == "a";
                                let has_ditransitive_context =
                                    Self::has_clear_loismo_ditransitive_context(
                                        tokens,
                                        &word_tokens,
                                        verb,
                                        pos + 2,
                                    );
                                if has_classic_context || has_ditransitive_context {
                                    let suggestion = if word_lower == "lo" { "le" } else { "les" };
                                    corrections.push(PronounCorrection {
                                        token_index: *idx,
                                        original: token.text.clone(),
                                        suggestion: Self::preserve_case(&token.text, suggestion),
                                        error_type: PronounErrorType::Loismo,
                                        reason: format!(
                                            "'{}' requiere complemento indirecto",
                                            verb
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            // Detectar leísmo: "le/les" + verbo de CD
            // NOTA: El leísmo de persona masculina singular está aceptado por la RAE
            // Solo marcamos casos claros como "les vi" (plural) o con verbos muy claros
            if matches!(word_lower.as_str(), "le" | "les") {
                if let Some(ref verb) = next_verb {
                    if Self::is_clear_direct_verb(verb) {
                        // Solo marcar plural como error claro, o verbos muy específicos
                        if word_lower == "les" {
                            corrections.push(PronounCorrection {
                                token_index: *idx,
                                original: token.text.clone(),
                                suggestion: Self::preserve_case(&token.text, "los"),
                                error_type: PronounErrorType::Leismo,
                                reason: format!("'{}' requiere complemento directo", verb),
                            });
                        }
                    }
                }
            }
        }

        corrections
    }

    /// Verifica si un verbo requiere complemento indirecto
    fn is_indirect_object_verb(verb: &str) -> bool {
        VERBS_INDIRECT_OBJECT.contains(&verb)
    }

    /// Verbos que claramente requieren CI (para detectar loísmo)
    fn is_clear_indirect_verb(verb: &str) -> bool {
        matches!(
            verb,
            "dije"
                | "dijo"
                | "dijeron"
                | "decir"
                | "digo"
                | "dice"
                | "dicen"
                | "conté"
                | "contó"
                | "contar"
                | "cuento"
                | "cuenta"
                | "pregunté"
                | "preguntó"
                | "preguntar"
                | "pregunto"
                | "pregunta"
                | "pedí"
                | "pidió"
                | "pedir"
                | "pido"
                | "pide"
                | "ordené"
                | "ordenó"
                | "ordenar"
                | "ordeno"
                | "ordena"
        )
    }

    /// Verbos ditransitivos donde "lo + verbo + un/una + sustantivo"
    /// puede señalar loismo con bastante fiabilidad.
    fn is_loismo_ditransitive_verb(verb: &str) -> bool {
        let lower = verb.to_lowercase();
        matches!(
            lower.as_str(),
            "decir"
                | "digo"
                | "dices"
                | "dice"
                | "decimos"
                | "dicen"
                | "dije"
                | "dijiste"
                | "dijo"
                | "dijimos"
                | "dijisteis"
                | "dijeron"
                | "dar"
                | "di"
                | "dio"
                | "dieron"
                | "doy"
                | "da"
                | "dan"
                | "damos"
                | "dais"
                | "pegar"
                | "pegué"
                | "pegue"
                | "pego"
                | "pegó"
                | "pega"
                | "pegaron"
                | "pegan"
                | "pegamos"
                | "pegais"
                | "regalar"
                | "regale"
                | "regalé"
                | "regalo"
                | "regala"
                | "regaló"
                | "regalaron"
                | "regalan"
                | "regalamos"
                | "regalais"
                | "regaláis"
        )
    }

    fn is_pegar_family(verb: &str) -> bool {
        matches!(
            Self::normalize_spanish(verb).as_str(),
            "pegar"
                | "pegue"
                | "pego"
                | "pega"
                | "pegaron"
                | "pegan"
                | "pegamos"
                | "pegais"
        )
    }

    fn is_regalar_family(verb: &str) -> bool {
        let lower = Self::normalize_spanish(verb);
        matches!(
            lower.as_str(),
            "regalar"
                | "regale"
                | "regalo"
                | "regala"
                | "regalaron"
                | "regalan"
                | "regalamos"
                | "regalais"
        )
    }

    /// Persona y numero aproximados para formas cubiertas por is_loismo_ditransitive_verb.
    fn loismo_verb_person_number(verb: &str) -> Option<(u8, bool)> {
        let lower = verb.to_lowercase();
        match lower.as_str() {
            "digo" | "dije" => Some((1, false)),
            "decimos" | "dijimos" => Some((1, true)),
            "dices" | "dijiste" => Some((2, false)),
            "dijisteis" => Some((2, true)),
            "dice" | "dijo" => Some((3, false)),
            "dicen" | "dijeron" => Some((3, true)),
            "di" | "doy" | "pegué" | "pegue" | "pego" | "regalé" | "regale" | "regalo" => {
                Some((1, false))
            }
            "dimos" | "damos" | "pegamos" | "regalamos" => Some((1, true)),
            "das" | "pegas" => Some((2, false)),
            "dais" | "pegais" | "regalais" | "regaláis" => Some((2, true)),
            "dio" | "da" | "pega" | "pegó" | "regala" | "regaló" => Some((3, false)),
            "dieron" | "dan" | "pegaron" | "pegan" | "regalaron" | "regalan" => Some((3, true)),
            _ => None,
        }
    }

    fn object_determiner_number(word: &str) -> Option<bool> {
        match Self::normalize_spanish(word).as_str() {
            "un" | "una" | "el" | "la" => Some(false),
            "unos" | "unas" | "los" | "las" => Some(true),
            _ => None,
        }
    }

    fn is_likely_temporal_noun(word: &str) -> bool {
        matches!(
            Self::normalize_spanish(word).as_str(),
            "dia"
                | "dias"
                | "semana"
                | "semanas"
                | "mes"
                | "meses"
                | "ano"
                | "anos"
                | "noche"
                | "noches"
                | "tarde"
                | "tardes"
                | "manana"
                | "mananas"
                | "lunes"
                | "martes"
                | "miercoles"
                | "jueves"
                | "viernes"
                | "sabado"
                | "sabados"
                | "domingo"
                | "domingos"
                | "vez"
                | "veces"
                | "momento"
                | "momentos"
        )
    }

    fn is_pegar_ditransitive_object_noun(word: &str) -> bool {
        matches!(
            Self::normalize_spanish(word).as_str(),
            "bofetada"
                | "bofetadas"
                | "paliza"
                | "palizas"
                | "golpe"
                | "golpes"
                | "patada"
                | "patadas"
                | "punetazo"
                | "punetazos"
                | "cachetada"
                | "cachetadas"
                | "tortazo"
                | "tortazos"
                | "manotazo"
                | "manotazos"
                | "mamporro"
                | "mamporros"
                | "hostia"
                | "hostias"
        )
    }

    fn is_contar_family(verb: &str) -> bool {
        matches!(
            Self::normalize_spanish(verb).as_str(),
            "contar"
                | "cuento"
                | "cuentas"
                | "cuenta"
                | "contamos"
                | "cuentan"
                | "conte"
                | "conto"
                | "contaron"
        )
    }

    fn is_ensenar_family(verb: &str) -> bool {
        matches!(
            Self::normalize_spanish(verb).as_str(),
            "ensenar"
                | "enseno"
                | "ensenas"
                | "ensena"
                | "ensenamos"
                | "ensenan"
                | "ensene"
                | "ensenaron"
        )
    }

    fn is_feminine_determiner(word: &str, plural: bool) -> bool {
        let lower = Self::normalize_spanish(word);
        if plural {
            matches!(
                lower.as_str(),
                "las"
                    | "unas"
                    | "estas"
                    | "esas"
                    | "aquellas"
                    | "mis"
                    | "tus"
                    | "sus"
                    | "nuestras"
                    | "vuestras"
            )
        } else {
            matches!(
                lower.as_str(),
                "la" | "una"
                    | "esta"
                    | "esa"
                    | "aquella"
                    | "mi"
                    | "tu"
                    | "su"
                    | "nuestra"
                    | "vuestra"
            )
        }
    }

    fn noun_matches_feminine_pronoun(noun_token: &Token, pronoun: &str) -> bool {
        let pronoun_plural = pronoun == "las";

        if let Some(info) = noun_token.word_info.as_ref() {
            if info.category != crate::dictionary::WordCategory::Sustantivo
                || info.gender != crate::dictionary::Gender::Feminine
            {
                return false;
            }
            let noun_plural = if info.number != crate::dictionary::Number::None {
                info.number == crate::dictionary::Number::Plural
            } else {
                Self::normalize_spanish(noun_token.effective_text()).ends_with('s')
            };
            return noun_plural == pronoun_plural;
        }

        // Fallback para tests/unitarios sin word_info: heurística conservadora.
        let noun_lower = Self::normalize_spanish(noun_token.effective_text());
        let noun_plural = noun_lower.ends_with('s');
        if noun_plural != pronoun_plural {
            return false;
        }
        if pronoun_plural {
            noun_lower.ends_with("as")
        } else {
            noun_lower.ends_with('a')
        }
    }

    fn has_topicalized_feminine_object_before_pronoun(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pronoun_pos: usize,
        pronoun: &str,
    ) -> bool {
        if pronoun_pos < 2 {
            return false;
        }

        let (pron_idx, _) = word_tokens[pronoun_pos];
        const MAX_LOOKBACK: usize = 8;
        let min_noun_pos = pronoun_pos.saturating_sub(MAX_LOOKBACK);

        for noun_pos in (min_noun_pos..pronoun_pos).rev() {
            if noun_pos == 0 {
                continue;
            }

            let (noun_idx, noun_token) = word_tokens[noun_pos];
            if has_sentence_boundary(tokens, noun_idx, pron_idx) {
                continue;
            }
            if noun_token
                .effective_text()
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                continue;
            }
            if !Self::noun_matches_feminine_pronoun(noun_token, pronoun) {
                continue;
            }

            let det_pos = noun_pos - 1;
            let (det_idx, det_token) = word_tokens[det_pos];
            if has_sentence_boundary(tokens, det_idx, noun_idx) {
                continue;
            }
            if !Self::is_feminine_determiner(det_token.effective_text(), pronoun == "las") {
                continue;
            }

            if !Self::is_topicalized_np_tail_safe(word_tokens, noun_pos, pronoun_pos) {
                continue;
            }

            // Si aparece "a" justo antes del SN, suele ser CI prepuesto ("A su madre la escribí"),
            // que sí puede ser laísmo.
            if det_pos > 0 {
                let (before_np_idx, before_np_token) = word_tokens[det_pos - 1];
                if !has_sentence_boundary(tokens, before_np_idx, det_idx)
                    && Self::normalize_spanish(before_np_token.effective_text()) == "a"
                {
                    continue;
                }
            }

            return true;
        }

        false
    }

    fn is_topicalized_np_tail_safe(
        word_tokens: &[(usize, &Token)],
        noun_pos: usize,
        pronoun_pos: usize,
    ) -> bool {
        for pos in (noun_pos + 1)..pronoun_pos {
            let token = word_tokens[pos].1;
            let lower = Self::normalize_spanish(token.effective_text());

            if matches!(
                lower.as_str(),
                "de" | "del"
                    | "el"
                    | "la"
                    | "los"
                    | "las"
                    | "un"
                    | "una"
                    | "unos"
                    | "unas"
                    | "y"
                    | "e"
                    | "u"
                    | "o"
            ) {
                continue;
            }

            if token
                .effective_text()
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                continue;
            }

            if let Some(info) = token.word_info.as_ref() {
                if info.category == crate::dictionary::WordCategory::Verbo {
                    return false;
                }
            }
        }

        true
    }

    fn has_clear_loismo_ditransitive_context(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        verb: &str,
        after_verb_pos: usize,
    ) -> bool {
        if !Self::is_loismo_ditransitive_verb(verb) {
            return false;
        }
        let (verb_person, verb_is_plural) = match Self::loismo_verb_person_number(verb) {
            Some(v) => v,
            None => return false,
        };
        let is_pegar = Self::is_pegar_family(verb);
        if after_verb_pos >= word_tokens.len() {
            return false;
        }

        let (after_idx, after_token) = word_tokens[after_verb_pos];
        if has_sentence_boundary(tokens, word_tokens[after_verb_pos - 1].0, after_idx) {
            return false;
        }

        // Caso 1: determinante + sustantivo ("lo dieron un premio", "lo dije la verdad").
        if let Some(object_is_plural) = Self::object_determiner_number(after_token.effective_text())
        {
            if after_verb_pos + 1 >= word_tokens.len() {
                return false;
            }
            let (noun_idx, noun_token) = word_tokens[after_verb_pos + 1];
            if has_sentence_boundary(tokens, after_idx, noun_idx) {
                return false;
            }
            if Self::is_likely_temporal_noun(noun_token.effective_text()) {
                // "Lo dije la semana pasada": "la semana" suele ser CC temporal, no CD.
                return false;
            }
            if is_pegar && !Self::is_pegar_ditransitive_object_noun(noun_token.effective_text()) {
                return false;
            }
            let noun_is_candidate = noun_token
                .word_info
                .as_ref()
                .map(|info| info.category == crate::dictionary::WordCategory::Sustantivo)
                .unwrap_or_else(|| {
                    noun_token
                        .effective_text()
                        .chars()
                        .any(|c| c.is_alphabetic())
                });
            if !noun_is_candidate {
                return false;
            }

            // Si el verbo es 1a/2a persona, el SN posterior no puede ser sujeto.
            if verb_person != 3 {
                return true;
            }

            // En 3a persona, exigir desajuste de numero para reducir ambiguedad
            // con sujetos pospuestos ("Lo dijo un amigo").
            return verb_is_plural != object_is_plural;
        }

        // Caso 2: sustantivo desnudo tras verbo ditransitivo
        // ("lo regalaron flores"), también indicador frecuente de loísmo.
        let noun_is_candidate = after_token
            .word_info
            .as_ref()
            .map(|info| info.category == crate::dictionary::WordCategory::Sustantivo)
            .unwrap_or_else(|| {
                after_token
                    .effective_text()
                    .chars()
                    .any(|c| c.is_alphabetic())
            });
        if !noun_is_candidate {
            return false;
        }
        if is_pegar && !Self::is_pegar_ditransitive_object_noun(after_token.effective_text()) {
            return false;
        }

        // Evitar nombres propios pospuestos ("Lo regaló Juan").
        if after_token
            .effective_text()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return false;
        }

        let noun_is_plural = after_token
            .word_info
            .as_ref()
            .map(|info| info.number == crate::dictionary::Number::Plural)
            .unwrap_or_else(|| {
                Self::normalize_spanish(after_token.effective_text()).ends_with('s')
            });

        if verb_person != 3 {
            return true;
        }

        // En "regalar" permitimos SN desnudo plural como contexto claro de CD.
        // Ej: "Lo regalaron flores" -> "Le regalaron flores".
        if Self::is_regalar_family(verb) && noun_is_plural {
            return true;
        }

        verb_is_plural != noun_is_plural
    }

    fn has_clear_pegar_ditransitive_context(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        after_verb_pos: usize,
    ) -> bool {
        if after_verb_pos >= word_tokens.len() {
            return false;
        }

        let (after_idx, after_token) = word_tokens[after_verb_pos];
        if has_sentence_boundary(tokens, word_tokens[after_verb_pos - 1].0, after_idx) {
            return false;
        }

        if Self::object_determiner_number(after_token.effective_text()).is_some() {
            if after_verb_pos + 1 >= word_tokens.len() {
                return false;
            }
            let (noun_idx, noun_token) = word_tokens[after_verb_pos + 1];
            if has_sentence_boundary(tokens, after_idx, noun_idx) {
                return false;
            }
            return Self::is_pegar_ditransitive_object_noun(noun_token.effective_text());
        }

        Self::is_pegar_ditransitive_object_noun(after_token.effective_text())
    }

    fn has_clear_laismo_ditransitive_context(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        verb: &str,
        after_verb_pos: usize,
    ) -> bool {
        if Self::is_pegar_family(verb) {
            return Self::has_clear_pegar_ditransitive_context(tokens, word_tokens, after_verb_pos);
        }

        if !(Self::is_contar_family(verb)
            || Self::is_ensenar_family(verb)
            || Self::is_regalar_family(verb))
        {
            return false;
        }

        if after_verb_pos >= word_tokens.len() {
            return false;
        }

        let (after_idx, after_token) = word_tokens[after_verb_pos];
        if has_sentence_boundary(tokens, word_tokens[after_verb_pos - 1].0, after_idx) {
            return false;
        }

        if Self::object_determiner_number(after_token.effective_text()).is_some() {
            if after_verb_pos + 1 >= word_tokens.len() {
                return false;
            }
            let (noun_idx, noun_token) = word_tokens[after_verb_pos + 1];
            if has_sentence_boundary(tokens, after_idx, noun_idx) {
                return false;
            }
            if Self::is_likely_temporal_noun(noun_token.effective_text()) {
                return false;
            }
            return noun_token
                .word_info
                .as_ref()
                .map(|info| info.category == crate::dictionary::WordCategory::Sustantivo)
                .unwrap_or_else(|| {
                    noun_token
                        .effective_text()
                        .chars()
                        .any(|c| c.is_alphabetic())
                });
        }

        let noun_is_candidate = after_token
            .word_info
            .as_ref()
            .map(|info| info.category == crate::dictionary::WordCategory::Sustantivo)
            .unwrap_or_else(|| {
                after_token
                    .effective_text()
                    .chars()
                    .any(|c| c.is_alphabetic())
            });
        if !noun_is_candidate {
            return false;
        }
        if after_token
            .effective_text()
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            return false;
        }

        let noun_is_plural = after_token
            .word_info
            .as_ref()
            .map(|info| info.number == crate::dictionary::Number::Plural)
            .unwrap_or_else(|| {
                Self::normalize_spanish(after_token.effective_text()).ends_with('s')
            });
        noun_is_plural
    }

    /// Verbos que claramente requieren CD (para detectar leísmo)
    fn is_clear_direct_verb(verb: &str) -> bool {
        matches!(
            verb,
            "vi" | "vio"
                | "vieron"
                | "ver"
                | "veo"
                | "ves"
                | "ve"
                | "vemos"
                | "ven"
                | "llamé"
                | "llamó"
                | "llamar"
                | "llamo"
                | "llama"
                | "busqué"
                | "buscó"
                | "buscar"
                | "busco"
                | "busca"
                | "encontré"
                | "encontró"
                | "encontrar"
                | "encuentro"
                | "encuentra"
                | "conocí"
                | "conoció"
                | "conocer"
                | "conozco"
                | "conoce"
                | "invité"
                | "invitó"
                | "invitar"
                | "invito"
                | "invita"
        )
    }

    fn detect_me_te_se_inversion(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        pos: usize,
    ) -> Option<Vec<PronounCorrection>> {
        if pos + 2 >= word_tokens.len() {
            return None;
        }

        let (first_idx, first_token) = word_tokens[pos];
        let (second_idx, second_token) = word_tokens[pos + 1];
        let (third_idx, third_token) = word_tokens[pos + 2];

        if has_sentence_boundary(tokens, first_idx, second_idx)
            || has_sentence_boundary(tokens, second_idx, third_idx)
        {
            return None;
        }

        let first = Self::normalize_spanish(first_token.effective_text());
        let second = Self::normalize_spanish(second_token.effective_text());
        if !matches!(first.as_str(), "me" | "te") || second != "se" {
            return None;
        }

        if !Self::looks_like_finite_verb_or_aux(third_token) {
            return None;
        }

        Some(vec![
            PronounCorrection {
                token_index: first_idx,
                original: first_token.text.clone(),
                suggestion: Self::preserve_case(&first_token.text, "se"),
                error_type: PronounErrorType::CliticInversion,
                reason: "Orden clítico: 'se' debe ir antes de 'me/te'".to_string(),
            },
            PronounCorrection {
                token_index: second_idx,
                original: second_token.text.clone(),
                suggestion: Self::preserve_case(&second_token.text, first.as_str()),
                error_type: PronounErrorType::CliticInversion,
                reason: "Orden clítico: 'se' debe ir antes de 'me/te'".to_string(),
            },
        ])
    }

    fn looks_like_finite_verb_or_aux(token: &Token) -> bool {
        if let Some(info) = token.word_info.as_ref() {
            if info.category == crate::dictionary::WordCategory::Verbo {
                return true;
            }
        }

        let word = Self::normalize_spanish(token.effective_text());
        if matches!(
            word.as_str(),
            "es"
                | "era"
                | "eran"
                | "fue"
                | "fueron"
                | "ha"
                | "han"
                | "habia"
                | "hubo"
                | "hay"
                | "cae"
                | "caen"
                | "cayo"
                | "caia"
                | "olvido"
                | "olvida"
                | "olvidaron"
        ) {
            return true;
        }

        let len = word.chars().count();
        len > 3
            && (word.ends_with("o")
                || word.ends_with("a")
                || word.ends_with("e")
                || word.ends_with("an")
                || word.ends_with("en")
                || word.ends_with("aba")
                || word.ends_with("ia")
                || word.ends_with("aron")
                || word.ends_with("ieron"))
    }

    /// Preserva mayusculas del original
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

    fn analyze_text(text: &str) -> Vec<PronounCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        PronounAnalyzer::analyze(&tokens)
    }

    // Tests de laísmo
    #[test]
    fn test_la_dije_laismo() {
        let corrections = analyze_text("la dije la verdad");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_las_di_laismo() {
        let corrections = analyze_text("las di un regalo");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "les");
    }

    #[test]
    fn test_la_pregunte_laismo() {
        let corrections = analyze_text("la pregunté su nombre");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_la_conté_laismo() {
        let corrections = analyze_text("la conté un secreto");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_la_regalé_laismo() {
        let corrections = analyze_text("la regalé flores");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
    }

    #[test]
    fn test_la_escribí_laismo() {
        let corrections = analyze_text("la escribí una carta");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
    }

    #[test]
    fn test_la_pegaron_una_bofetada_laismo() {
        let corrections = analyze_text("la pegaron una bofetada");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_la_contaron_la_verdad_laismo() {
        let corrections = analyze_text("la contaron la verdad");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_la_ensenaron_el_camino_laismo() {
        let corrections = analyze_text("la ensenaron el camino");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_la_regalaron_flores_laismo() {
        let corrections = analyze_text("la regalaron flores");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Laismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_la_pegaron_en_la_pared_not_laismo() {
        let corrections = analyze_text("la pegaron en la pared");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::Laismo),
            "No debe marcar laÃ­smo en uso transitivo de 'pegar': {:?}",
            corrections
        );
    }

    #[test]
    fn test_topicalized_feminine_object_not_laismo() {
        let corrections = analyze_text("la carta la escribió Juan");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::Laismo),
            "No debe marcar laísmo en OD topicalizado reduplicado"
        );

        let corrections = analyze_text("las cartas las envié ayer");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::Laismo),
            "No debe marcar laísmo en OD plural topicalizado reduplicado"
        );

        let corrections = analyze_text("la carta urgente la escribió Juan");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::Laismo),
            "No debe marcar laísmo en OD topicalizado con adjetivo postnominal"
        );

        let corrections = analyze_text("la carta de María la escribió Juan");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::Laismo),
            "No debe marcar laísmo en OD topicalizado con complemento preposicional"
        );
    }

    #[test]
    fn test_topicalized_indirect_object_with_a_still_laismo() {
        let corrections = analyze_text("a su madre la escribí una carta");
        assert!(
            corrections
                .iter()
                .any(|c| c.error_type == PronounErrorType::Laismo),
            "Debe seguir marcando laísmo cuando hay CI prepuesto con 'a'"
        );
    }

    // Tests de loísmo
    #[test]
    fn test_lo_dije_que_loismo() {
        let corrections = analyze_text("lo dije que viniera");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Loismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_lo_pregunté_a_loismo() {
        let corrections = analyze_text("lo pregunté a él");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Loismo);
    }

    #[test]
    fn test_lo_dieron_un_premio_loismo() {
        let corrections = analyze_text("lo dieron un premio");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Loismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_lo_di_un_regalo_loismo() {
        let corrections = analyze_text("lo di un regalo");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Loismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_lo_pegaron_una_paliza_loismo() {
        let corrections = analyze_text("lo pegaron una paliza");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Loismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_lo_pegaron_en_la_pared_not_loismo() {
        let corrections = analyze_text("lo pegaron en la pared");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::Loismo),
            "No debe marcar loísmo en uso transitivo de 'pegar': {:?}",
            corrections
        );
    }

    #[test]
    fn test_lo_regalaron_flores_loismo() {
        let corrections = analyze_text("lo regalaron flores");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Loismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_lo_dije_la_verdad_loismo() {
        let corrections = analyze_text("lo dije la verdad");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Loismo);
        assert_eq!(corrections[0].suggestion, "le");
    }

    #[test]
    fn test_lo_dije_la_semana_pasada_not_loismo() {
        let corrections = analyze_text("lo dije la semana pasada");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::Loismo),
            "No debe marcar loísmo cuando el SN es temporal"
        );
    }

    #[test]
    fn test_lo_regalo_juan_not_loismo() {
        let corrections = analyze_text("lo regaló Juan");
        assert!(
            corrections.is_empty(),
            "No debe marcar loismo cuando hay sujeto propio pospuesto"
        );
    }

    #[test]
    fn test_se_lo_dieron_a_el_not_loismo() {
        let corrections = analyze_text("se lo dieron a él");
        assert!(
            corrections.is_empty(),
            "No debe marcar loismo en secuencia clitica 'se lo'"
        );
    }

    #[test]
    fn test_se_lo_di_a_maria_not_loismo() {
        let corrections = analyze_text("se lo di a María");
        assert!(
            corrections.is_empty(),
            "No debe marcar loismo en secuencia clitica 'se lo'"
        );
    }

    #[test]
    fn test_se_lo_pego_a_ella_not_loismo() {
        let corrections = analyze_text("se lo pegó a ella");
        assert!(
            corrections.is_empty(),
            "No debe marcar loismo en secuencia clitica 'se lo'"
        );
    }

    #[test]
    fn test_se_lo_regalo_a_maria_not_loismo() {
        let corrections = analyze_text("se lo regaló a María");
        assert!(
            corrections.is_empty(),
            "No debe marcar loismo en secuencia clitica 'se lo'"
        );
    }

    #[test]
    fn test_me_se_cayo_clitic_inversion() {
        let corrections = analyze_text("me se cayó");
        assert!(
            corrections.iter().any(|c| {
                c.error_type == PronounErrorType::CliticInversion
                    && c.original.eq_ignore_ascii_case("me")
                    && c.suggestion.eq_ignore_ascii_case("se")
            }),
            "Debe corregir 'me se' a 'se me': {:?}",
            corrections
        );
        assert!(
            corrections.iter().any(|c| {
                c.error_type == PronounErrorType::CliticInversion
                    && c.original.eq_ignore_ascii_case("se")
                    && c.suggestion.eq_ignore_ascii_case("me")
            }),
            "Debe corregir segundo clitico en 'me se': {:?}",
            corrections
        );
    }

    #[test]
    fn test_te_se_olvido_clitic_inversion() {
        let corrections = analyze_text("te se olvidó");
        assert!(
            corrections.iter().any(|c| {
                c.error_type == PronounErrorType::CliticInversion
                    && c.original.eq_ignore_ascii_case("te")
                    && c.suggestion.eq_ignore_ascii_case("se")
            }),
            "Debe corregir 'te se' a 'se te': {:?}",
            corrections
        );
        assert!(
            corrections.iter().any(|c| {
                c.error_type == PronounErrorType::CliticInversion
                    && c.original.eq_ignore_ascii_case("se")
                    && c.suggestion.eq_ignore_ascii_case("te")
            }),
            "Debe corregir segundo clitico en 'te se': {:?}",
            corrections
        );
    }

    #[test]
    fn test_se_me_cayo_no_clitic_inversion() {
        let corrections = analyze_text("se me cayó");
        assert!(
            corrections
                .iter()
                .all(|c| c.error_type != PronounErrorType::CliticInversion),
            "No debe corregir orden clitico ya correcto: {:?}",
            corrections
        );
    }

    #[test]
    fn test_se_lo_dieron_todo_not_loismo() {
        let corrections = analyze_text("se lo dieron todo");
        assert!(
            corrections.is_empty(),
            "No debe marcar loismo en secuencia clitica 'se lo'"
        );
    }

    #[test]
    fn test_lo_dijo_un_amigo_not_loismo() {
        let corrections = analyze_text("lo dijo un amigo");
        assert!(
            corrections.is_empty(),
            "No debe marcar loismo en uso de CD con sujeto pospuesto"
        );
    }

    // Tests de leísmo
    #[test]
    fn test_les_vi_leismo() {
        let corrections = analyze_text("les vi en el parque");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Leismo);
        assert_eq!(corrections[0].suggestion, "los");
    }

    #[test]
    fn test_les_llamé_leismo() {
        let corrections = analyze_text("les llamé ayer");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Leismo);
    }

    #[test]
    fn test_les_busqué_leismo() {
        let corrections = analyze_text("les busqué por todas partes");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, PronounErrorType::Leismo);
    }

    // Tests de casos correctos
    #[test]
    fn test_le_dije_correct() {
        let corrections = analyze_text("le dije la verdad");
        assert!(corrections.is_empty(), "No debería corregir 'le dije'");
    }

    #[test]
    fn test_lo_vi_correct() {
        let corrections = analyze_text("lo vi en el parque");
        assert!(corrections.is_empty(), "No debería corregir 'lo vi'");
    }

    #[test]
    fn test_la_vi_correct() {
        let corrections = analyze_text("la vi ayer");
        assert!(corrections.is_empty(), "No debería corregir 'la vi'");
    }

    #[test]
    fn test_los_llamé_correct() {
        let corrections = analyze_text("los llamé ayer");
        assert!(corrections.is_empty(), "No debería corregir 'los llamé'");
    }

    #[test]
    fn test_les_dije_correct() {
        let corrections = analyze_text("les dije que vinieran");
        assert!(corrections.is_empty(), "No debería corregir 'les dije'");
    }

    // Test de preservacion de mayusculas
    #[test]
    fn test_preserve_case_laismo() {
        let corrections = analyze_text("La dije la verdad");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Le");
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "la" y "dije" estan separados por punto, no debe detectar laismo
        let corrections = analyze_text("Vi a la. Dije que si");
        let laismo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.error_type == PronounErrorType::Laismo)
            .collect();
        assert!(
            laismo_corrections.is_empty(),
            "No debe detectar laismo cuando hay limite de oracion"
        );
    }
}
