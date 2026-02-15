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
    "pensar",
    "pienso",
    "piensas",
    "piensa",
    "pensamos",
    "piensan",
    "pensé",
    "pensó",
    "creer",
    "creo",
    "crees",
    "cree",
    "creemos",
    "creen",
    "creí",
    "creyó",
    "opinar",
    "opino",
    "opinas",
    "opina",
    "opinamos",
    "opinan",
    "opiné",
    "opinó",
    "considerar",
    "considero",
    "consideras",
    "considera",
    "consideramos",
    "consideran",
    "suponer",
    "supongo",
    "supones",
    "supone",
    "suponemos",
    "suponen",
    "imaginar",
    "imagino",
    "imaginas",
    "imagina",
    "imaginamos",
    "imaginan",
    // Verbos de comunicación
    "decir",
    "digo",
    "dices",
    "dice",
    "decimos",
    "dicen",
    "dije",
    "dijo",
    "afirmar",
    "afirmo",
    "afirmas",
    "afirma",
    "afirmamos",
    "afirman",
    "negar",
    "niego",
    "niegas",
    "niega",
    "negamos",
    "niegan",
    "comunicar",
    "comunico",
    "comunicas",
    "comunica",
    "comunicamos",
    "comunican",
    "manifestar",
    "manifiesto",
    "manifiestas",
    "manifiesta",
    "manifestamos",
    "expresar",
    "expreso",
    "expresas",
    "expresa",
    "expresamos",
    "expresan",
    "comentar",
    "comento",
    "comentas",
    "comenta",
    "comentamos",
    "comentan",
    // Verbos de percepción
    "ver",
    "veo",
    "ves",
    "ve",
    "vemos",
    "ven",
    "vi",
    "vio",
    "oír",
    "oigo",
    "oyes",
    "oye",
    "oímos",
    "oyen",
    "oí",
    "oyó",
    "sentir",
    "siento",
    "sientes",
    "siente",
    "sentimos",
    "sienten",
    "notar",
    "noto",
    "notas",
    "nota",
    "notamos",
    "notan",
    // Verbos de conocimiento
    "saber",
    "sé",
    "sabes",
    "sabe",
    "sabemos",
    "saben",
    "supe",
    "supo",
    "conocer",
    "conozco",
    "conoces",
    "conoce",
    "conocemos",
    "conocen",
    "entender",
    "entiendo",
    "entiendes",
    "entiende",
    "entendemos",
    "entienden",
    "comprender",
    "comprendo",
    "comprendes",
    "comprende",
    "comprendemos",
    // Verbos de voluntad/deseo
    "querer",
    "quiero",
    "quieres",
    "quiere",
    "queremos",
    "quieren",
    "desear",
    "deseo",
    "deseas",
    "desea",
    "deseamos",
    "desean",
    "esperar",
    "espero",
    "esperas",
    "espera",
    "esperamos",
    "esperan",
    "necesitar",
    "necesito",
    "necesitas",
    "necesita",
    "necesitamos",
    "necesitan",
    "preferir",
    "prefiero",
    "prefieres",
    "prefiere",
    "preferimos",
    "prefieren",
    // Verbos de duda
    "dudar",
    "dudo",
    "dudas",
    "duda",
    "dudamos",
    "dudan",
    // Otros
    "parecer",
    "parece",
    "parecía",
    "resultar",
    "resulta",
    "resultó",
    "suceder",
    "sucede",
    "sucedió",
    "ocurrir",
    "ocurre",
    "ocurrió",
];

/// Pretéritos irregulares en 3.ª persona plural que no se derivan por sufijo simple.
const IRREGULAR_PRETERITE_PLURAL_INF: &[(&str, &str)] = &[
    ("dijeron", "decir"),
    ("quisieron", "querer"),
    ("supieron", "saber"),
    ("vieron", "ver"),
    ("sintieron", "sentir"),
    ("prefirieron", "preferir"),
    ("creyeron", "creer"),
    ("oyeron", "oír"),
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
                            let is_dequeismo = Self::is_dequeismo_verb(&prev_word)
                                || Self::is_ser_adjective_dequeismo_context(
                                    &word_tokens,
                                    pos,
                                    tokens,
                                );
                            if is_dequeismo {
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
                    let is_already_de_que =
                        pos >= 2 && word_tokens[pos - 1].1.effective_text().to_lowercase() == "de";

                    if !is_already_de_que {
                        let Some(required_prep) = Self::required_preposition_before_que(
                            &prev_word,
                            &word_tokens,
                            pos,
                            tokens,
                        ) else {
                            continue;
                        };
                        corrections.push(DequeismoCorrection {
                            token_index: *idx,
                            original: token.text.clone(),
                            suggestion: format!("{required_prep} que"),
                            error_type: DequeismoErrorType::Queismo,
                            reason: format!(
                                "'{}' requiere '{}' antes de 'que'",
                                prev_word, required_prep
                            ),
                        });
                    }
                }
            }
        }

        corrections
    }

    /// Verifica si un verbo NO debe llevar "de" antes de "que"
    fn is_dequeismo_verb(word: &str) -> bool {
        VERBS_WITHOUT_DE.contains(&word) || Self::is_preterite_plural_without_de(word)
    }

    fn is_preterite_plural_without_de(word: &str) -> bool {
        if let Some(infinitive) = Self::irregular_preterite_plural_infinitive(word) {
            return VERBS_WITHOUT_DE.contains(&infinitive);
        }

        // Pretérito simple 3.ª plural regular: -aron / -ieron
        if let Some(stem) = word.strip_suffix("aron") {
            let infinitive = format!("{stem}ar");
            return VERBS_WITHOUT_DE.contains(&infinitive.as_str());
        }

        if let Some(stem) = word.strip_suffix("ieron") {
            let infinitive_er = format!("{stem}er");
            let infinitive_ir = format!("{stem}ir");
            return VERBS_WITHOUT_DE.contains(&infinitive_er.as_str())
                || VERBS_WITHOUT_DE.contains(&infinitive_ir.as_str());
        }

        // Verbos en -eer/-oír suelen formar -yeron (creyeron, oyeron).
        if let Some(stem) = word.strip_suffix("yeron") {
            let infinitive_er = format!("{stem}er");
            let infinitive_ir = format!("{stem}ir");
            return VERBS_WITHOUT_DE.contains(&infinitive_er.as_str())
                || VERBS_WITHOUT_DE.contains(&infinitive_ir.as_str());
        }

        false
    }

    fn irregular_preterite_plural_infinitive(word: &str) -> Option<&'static str> {
        IRREGULAR_PRETERITE_PLURAL_INF
            .iter()
            .find_map(|(form, infinitive)| (*form == word).then_some(*infinitive))
    }

    fn is_ser_adjective_dequeismo_context(
        word_tokens: &[(usize, &Token)],
        de_pos: usize,
        tokens: &[Token],
    ) -> bool {
        if de_pos < 2 {
            return false;
        }

        let adj_idx = word_tokens[de_pos - 1].0;
        let ser_idx = word_tokens[de_pos - 2].0;
        if has_sentence_boundary(tokens, ser_idx, adj_idx) {
            return false;
        }

        let adj = Self::normalize_spanish(word_tokens[de_pos - 1].1.effective_text());
        let ser = Self::normalize_spanish(word_tokens[de_pos - 2].1.effective_text());

        if !Self::is_ser_form_for_dequeismo(ser.as_str()) {
            return false;
        }

        Self::is_adjective_without_de_after_ser(adj.as_str())
    }

    fn is_ser_form_for_dequeismo(word: &str) -> bool {
        matches!(
            word,
            "es" | "era" | "fue" | "sera" | "seria" | "seran" | "serian"
        )
    }

    fn is_adjective_without_de_after_ser(word: &str) -> bool {
        matches!(
            word,
            "posible"
                | "posibles"
                | "probable"
                | "probables"
                | "necesario"
                | "necesaria"
                | "necesarios"
                | "necesarias"
                | "importante"
                | "importantes"
                | "cierto"
                | "cierta"
                | "ciertos"
                | "ciertas"
                | "verdad"
                | "evidente"
                | "evidentes"
                | "seguro"
                | "segura"
                | "seguros"
                | "seguras"
                | "obvio"
                | "obvia"
                | "obvios"
                | "obvias"
                | "claro"
                | "clara"
                | "claros"
                | "claras"
                | "logico"
                | "lógico"
                | "logica"
                | "lógica"
                | "logicos"
                | "lógicos"
                | "logicas"
                | "lógicas"
                | "natural"
                | "naturales"
                | "normal"
                | "normales"
                | "falso"
                | "falsa"
                | "falsos"
                | "falsas"
        )
    }

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
            "el" | "la"
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
            "me" => matches!(
                verb,
                "alegro"
                    | "alegr\u{00E9}"
                    | "acuerdo"
                    | "acord\u{00E9}"
                    | "arrepiento"
                    | "arrepent\u{00ED}"
                    | "entero"
                    | "enter\u{00E9}"
                    | "olvido"
                    | "olvid\u{00E9}"
                    | "quejo"
                    | "quej\u{00E9}"
                    | "aseguro"
                    | "asegur\u{00E9}"
                    | "preocupo"
                    | "preocupe"
                    | "averguenzo"
                    | "averg\u{00FC}enzo"
                    | "avergonce"
            ),
            "te" => matches!(
                verb,
                "alegras"
                    | "alegraste"
                    | "acuerdas"
                    | "acordaste"
                    | "arrepientes"
                    | "arrepentiste"
                    | "enteras"
                    | "enteraste"
                    | "olvidas"
                    | "olvidaste"
                    | "quejas"
                    | "quejaste"
                    | "aseguras"
                    | "aseguraste"
                    | "preocupas"
                    | "preocupaste"
                    | "averguenzas"
                    | "averg\u{00FC}enzas"
                    | "avergonzaste"
            ),
            "se" => matches!(
                verb,
                "alegra"
                    | "alegran"
                    | "alegr\u{00F3}"
                    | "alegraron"
                    | "acuerda"
                    | "acuerdan"
                    | "acord\u{00F3}"
                    | "acordaron"
                    | "arrepiente"
                    | "arrepienten"
                    | "arrepinti\u{00F3}"
                    | "arrepintieron"
                    | "entera"
                    | "enteran"
                    | "enter\u{00F3}"
                    | "enteraron"
                    | "olvida"
                    | "olvidan"
                    | "olvid\u{00F3}"
                    | "olvidaron"
                    | "queja"
                    | "quejan"
                    | "quej\u{00F3}"
                    | "quejaron"
                    | "asegura"
                    | "aseguran"
                    | "asegur\u{00F3}"
                    | "aseguraron"
                    | "convence"
                    | "convencen"
                    | "convenci\u{00F3}"
                    | "convencieron"
                    | "aprovecha"
                    | "aprovechan"
                    | "aprovech\u{00F3}"
                    | "aprovecharon"
                    | "encarga"
                    | "encargan"
                    | "encarg\u{00F3}"
                    | "encargaron"
                    | "preocupa"
                    | "preocupan"
                    | "preocupo"
                    | "preocuparon"
                    | "averguenza"
                    | "averg\u{00FC}enza"
                    | "averguenzan"
                    | "averg\u{00FC}enzan"
                    | "avergonzo"
                    | "avergonzaron"
            ),
            "nos" => matches!(
                verb,
                "alegramos"
                    | "acordamos"
                    | "arrepentimos"
                    | "enteramos"
                    | "olvidamos"
                    | "quejamos"
                    | "aseguramos"
                    | "preocupamos"
                    | "avergonzamos"
            ),
            "os" => matches!(
                verb,
                "alegr\u{00E1}is"
                    | "alegrasteis"
                    | "acord\u{00E1}is"
                    | "acordasteis"
                    | "arrepent\u{00ED}s"
                    | "arrepentisteis"
                    | "enter\u{00E1}is"
                    | "enterasteis"
                    | "olvid\u{00E1}is"
                    | "olvidasteis"
                    | "quej\u{00E1}is"
                    | "quejasteis"
                    | "asegur\u{00E1}is"
                    | "asegurasteis"
                    | "preocupais"
                    | "preocupasteis"
                    | "avergonzais"
                    | "avergonzasteis"
            ),
            _ => false,
        }
    }

    fn is_optional_clause_prefix_after_que(word: &str) -> bool {
        matches!(
            word,
            "no" | "ya"
                | "nunca"
                | "siempre"
                | "tambien"
                | "también"
                | "aun"
                | "aún"
                | "yo"
                | "tu"
                | "el"
                | "ella"
                | "nosotros"
                | "nosotras"
                | "vosotros"
                | "vosotras"
                | "ellos"
                | "ellas"
                | "usted"
                | "ustedes"
                | "me"
                | "te"
                | "se"
                | "nos"
                | "os"
                | "lo"
                | "la"
                | "los"
                | "las"
                | "le"
                | "les"
        )
    }

    fn looks_like_finite_clause_verb(word: &str) -> bool {
        let len = word.chars().count();
        if len <= 3 {
            return false;
        }
        if matches!(
            word,
            "nada" | "nadie" | "todo" | "todos" | "todas" | "algo" | "esto" | "eso" | "aquello"
        ) {
            return false;
        }
        word.ends_with("ria")
            || word.ends_with("rias")
            || word.ends_with("riamos")
            || word.ends_with("riais")
            || word.ends_with("rian")
            || word.ends_with("ra")
            || word.ends_with("ras")
            || word.ends_with("ramos")
            || word.ends_with("rais")
            || word.ends_with("ran")
            || word.ends_with("se")
            || word.ends_with("ses")
            || word.ends_with("semos")
            || word.ends_with("seis")
            || word.ends_with("sen")
            || word.ends_with("aron")
            || word.ends_with("ieron")
            || word.ends_with("aba")
            || word.ends_with("aban")
            || word.ends_with("ia")
            || word.ends_with("ian")
            || (len > 4 && word.ends_with('o'))
            || (len > 4 && word.ends_with("es"))
            || (len > 4 && word.ends_with("en"))
            || (len > 4 && word.ends_with("as"))
            || (len > 4 && word.ends_with("an"))
            || (len > 5 && word.ends_with('e'))
    }

    fn is_likely_infinitive_form(word: &str) -> bool {
        if word.len() > 3 && (word.ends_with("ar") || word.ends_with("er") || word.ends_with("ir"))
        {
            return true;
        }

        for clitic in [
            "me", "te", "se", "nos", "os", "lo", "la", "los", "las", "le", "les",
        ] {
            if let Some(base) = word.strip_suffix(clitic) {
                if base.len() > 3
                    && (base.ends_with("ar") || base.ends_with("er") || base.ends_with("ir"))
                {
                    return true;
                }
            }
        }

        false
    }

    fn is_temporal_measure_noun(word: &str) -> bool {
        matches!(
            word,
            "hora"
                | "horas"
                | "minuto"
                | "minutos"
                | "segundo"
                | "segundos"
                | "dia"
                | "dias"
                | "semana"
                | "semanas"
                | "mes"
                | "meses"
                | "ano"
                | "anos"
                | "momento"
                | "momentos"
                | "instante"
                | "instantes"
        )
    }

    /// Detecta patrón "verbo + [medida temporal] + después + que ...",
    /// donde "que" suele depender del verbo principal (completiva),
    /// no de "después".
    fn is_embedded_temporal_phrase_before_completive_que(
        word_tokens: &[(usize, &Token)],
        pos: usize,
        tokens: &[Token],
    ) -> bool {
        if pos < 3 {
            return false;
        }

        let temporal_norm = Self::normalize_spanish(word_tokens[pos - 2].1.effective_text());
        if !Self::is_temporal_measure_noun(temporal_norm.as_str()) {
            return false;
        }

        let verb_idx = word_tokens[pos - 3].0;
        let temporal_idx = word_tokens[pos - 2].0;
        let marker_idx = word_tokens[pos - 1].0;
        if has_sentence_boundary(tokens, verb_idx, temporal_idx)
            || has_sentence_boundary(tokens, temporal_idx, marker_idx)
        {
            return false;
        }

        let verb_token = word_tokens[pos - 3].1;
        let verb_norm = Self::normalize_spanish(verb_token.effective_text());
        let dict_verb = verb_token
            .word_info
            .as_ref()
            .is_some_and(|info| info.category == crate::dictionary::WordCategory::Verbo)
            && !Self::is_likely_infinitive_form(verb_norm.as_str());

        dict_verb || Self::looks_like_finite_clause_verb(verb_norm.as_str())
    }

    fn has_likely_verb_after_que(
        word_tokens: &[(usize, &Token)],
        pos: usize,
        tokens: &[Token],
    ) -> bool {
        if pos + 1 >= word_tokens.len() {
            return false;
        }
        let que_idx = word_tokens[pos].0;
        let mut scanned = 0usize;
        for j in (pos + 1)..word_tokens.len() {
            let (idx, token) = word_tokens[j];
            if has_sentence_boundary(tokens, que_idx, idx) {
                break;
            }
            let norm = Self::normalize_spanish(token.effective_text());
            if Self::is_optional_clause_prefix_after_que(norm.as_str()) {
                scanned += 1;
                if scanned > 3 {
                    break;
                }
                continue;
            }
            if matches!(
                norm.as_str(),
                "nada"
                    | "nadie"
                    | "algo"
                    | "todo"
                    | "todos"
                    | "todas"
                    | "esto"
                    | "eso"
                    | "aquello"
                    | "mas"
                    | "menos"
                    | "mejor"
                    | "peor"
                    | "antes"
                    | "despues"
            ) {
                return false;
            }
            if let Some(info) = token.word_info.as_ref() {
                if info.category == crate::dictionary::WordCategory::Verbo {
                    if Self::is_likely_infinitive_form(norm.as_str())
                        || norm.ends_with("ando")
                        || norm.ends_with("iendo")
                        || norm.ends_with("yendo")
                    {
                        return false;
                    }
                    return true;
                }
                if info.category != crate::dictionary::WordCategory::Otro {
                    return false;
                }
            }
            if Self::is_likely_infinitive_form(norm.as_str()) {
                return false;
            }
            return Self::looks_like_finite_clause_verb(norm.as_str());
        }
        false
    }

    /// Verifica si una expresion necesita "de" antes de "que"
    fn required_preposition_before_que(
        prev_word: &str,
        word_tokens: &[(usize, &Token)],
        pos: usize,
        tokens: &[Token],
    ) -> Option<&'static str> {
        let prev_norm = Self::normalize_spanish(prev_word);

        // "depender que" -> "depender de que"
        if matches!(
            prev_norm.as_str(),
            "depende" | "dependen" | "dependia" | "dependian" | "dependio" | "dependieron"
        ) {
            return Some("de");
        }

        // "confiar en que"
        if matches!(
            prev_norm.as_str(),
            "confio"
                | "confias"
                | "confia"
                | "confiamos"
                | "confian"
                | "confiaba"
                | "confiaban"
                | "confie"
                | "confiaron"
        ) {
            return Some("en");
        }

        // "aspirar a que"
        if matches!(
            prev_norm.as_str(),
            "aspiro"
                | "aspiras"
                | "aspira"
                | "aspiramos"
                | "aspiran"
                | "aspiraba"
                | "aspiraban"
                | "aspire"
                | "aspiraron"
        ) {
            return Some("a");
        }

        // "consistir/insistir/fijarse en que"
        if matches!(
            prev_norm.as_str(),
            "consiste"
                | "consisten"
                | "consistia"
                | "consistian"
                | "consistio"
                | "consistieron"
                | "insisto"
                | "insistes"
                | "insiste"
                | "insistimos"
                | "insisten"
                | "insistia"
                | "insistian"
                | "insisti"
                | "insistio"
                | "insistieron"
        ) {
            return Some("en");
        }

        // "se trata que" -> "se trata de que"
        if matches!(
            prev_norm.as_str(),
            "trata" | "tratan" | "trataba" | "trataban" | "trato" | "trataron"
        ) && pos >= 2
        {
            let prev_prev = Self::normalize_spanish(word_tokens[pos - 2].1.effective_text());
            if prev_prev == "se" {
                return Some("de");
            }
        }

        // "me fijé que" -> "me fijé en que"
        if matches!(
            prev_norm.as_str(),
            "fijo" | "fijas" | "fija" | "fijamos" | "fijan" | "fijaba" | "fijaban" | "fije"
        ) && pos >= 2
        {
            let prev_prev = Self::normalize_spanish(word_tokens[pos - 2].1.effective_text());
            if matches!(prev_prev.as_str(), "me" | "te" | "se" | "nos" | "os") {
                return Some("en");
            }
        }

        // Caso especial: "me/te/se + verbo + que" donde el verbo es pronominal
        // Por ejemplo: "me alegro que" -> "me alegro de que"
        // PERO: "me alegra que" es correcto (algo me alegra, no es reflexivo)
        if pos >= 2 {
            // Verificar que no hay limite de oracion entre los tokens
            let prev_idx = word_tokens[pos - 1].0;
            let prev_prev_idx = word_tokens[pos - 2].0;
            if has_sentence_boundary(tokens, prev_prev_idx, prev_idx) {
                return None;
            }
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();

            // Verbos pronominales que requieren "de" cuando el pronombre concuerda.
            if Self::is_reflexive_queismo_form(prev_prev.as_str(), prev_word) {
                return Some("de");
            }

            // "percatarse de que"
            if Self::is_reflexive_percatarse_form(prev_prev.as_str(), prev_word) {
                return Some("de");
            }

            // "darse cuenta que" → "darse cuenta de que"
            if prev_word == "cuenta"
                && matches!(
                    prev_prev.as_str(),
                    "di" | "diste"
                        | "dio"
                        | "dimos"
                        | "dieron"
                        | "doy"
                        | "das"
                        | "da"
                        | "damos"
                        | "dan"
                )
            {
                return Some("de");
            }

            // "estar seguro que" → "estar seguro de que"
            if matches!(prev_word, "seguro" | "segura" | "seguros" | "seguras") {
                if matches!(
                    prev_prev.as_str(),
                    "estoy"
                        | "estás"
                        | "está"
                        | "estamos"
                        | "están"
                        | "estaba"
                        | "estabas"
                        | "estábamos"
                        | "estaban"
                        | "es"
                        | "soy"
                        | "eres"
                        | "somos"
                        | "son"
                ) {
                    return Some("de");
                }
            }

            // "estar convencido que" → "estar convencido de que"
            if matches!(
                prev_word,
                "convencido" | "convencida" | "convencidos" | "convencidas"
            ) {
                if matches!(
                    prev_prev.as_str(),
                    "estoy" | "estás" | "está" | "estamos" | "están"
                ) {
                    return Some("de");
                }
            }
        }

        // "tener miedo/ganas que" → "tener miedo/ganas de que"
        if pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if matches!(
                prev_prev.as_str(),
                "tengo"
                    | "tienes"
                    | "tiene"
                    | "tenemos"
                    | "tienen"
                    | "tenía"
                    | "tenías"
                    | "teníamos"
                    | "tenían"
                    | "tuve"
                    | "tuvo"
            ) {
                if matches!(
                    prev_word,
                    "miedo"
                        | "ganas"
                        | "culpa"
                        | "idea"
                        | "duda"
                        | "derecho"
                        | "necesidad"
                        | "obligación"
                        | "intención"
                        | "esperanza"
                        | "certeza"
                ) {
                    return Some("de");
                }
            }
        }

        // "no cabe duda que" / "no hay duda que" -> "de que"
        if pos >= 2 && matches!(prev_word, "duda" | "dudas") {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
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
                return Some("de");
            }
        }

        // "es hora que" -> "es hora de que"
        if prev_word == "hora" && pos >= 2 {
            let prev_prev = Self::normalize_spanish(word_tokens[pos - 2].1.effective_text());
            if Self::is_ser_form_for_dequeismo(prev_prev.as_str()) {
                return Some("de");
            }
        }

        // "antes/después que + verbo" -> "antes/después de que + verbo"
        if matches!(prev_norm.as_str(), "antes" | "despues")
            && Self::has_likely_verb_after_que(word_tokens, pos, tokens)
            && !Self::is_embedded_temporal_phrase_before_completive_que(word_tokens, pos, tokens)
        {
            return Some("de");
        }

        // "a pesar que" → "a pesar de que"
        if prev_word == "pesar" && pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if prev_prev == "a" {
                return Some("de");
            }
        }

        // "en vez que" → "en vez de que" (raro, pero posible)
        if matches!(prev_word, "vez" | "lugar" | "caso") && pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if prev_prev == "en" {
                return Some("de");
            }
        }

        // "el hecho que" → "el hecho de que"
        if prev_word == "hecho" && pos >= 2 {
            let prev_prev = word_tokens[pos - 2].1.effective_text().to_lowercase();
            if prev_prev == "el" {
                return Some("de");
            }
        }

        None
    }

    fn is_reflexive_percatarse_form(pronoun: &str, verb: &str) -> bool {
        let verb_norm = Self::normalize_spanish(verb);
        match pronoun {
            "me" => matches!(verb_norm.as_str(), "percato" | "percate"),
            "te" => matches!(verb_norm.as_str(), "percatas" | "percataste"),
            "se" => matches!(
                verb_norm.as_str(),
                "percata" | "percatan" | "percato" | "percataron"
            ),
            "nos" => matches!(verb_norm.as_str(), "percatamos"),
            "os" => matches!(verb_norm.as_str(), "percatais" | "percatasteis"),
            _ => false,
        }
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
    fn test_es_posible_de_que_dequeismo() {
        let corrections = analyze_text("es posible de que llueva");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_es_probable_de_que_dequeismo() {
        let corrections = analyze_text("es probable de que venga");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_es_necesario_de_que_dequeismo() {
        let corrections = analyze_text("es necesario de que estudies");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_es_adjetivo_de_que_extended_coverage() {
        let samples = [
            "es importante de que vengas",
            "es cierto de que funcione",
            "es verdad de que llueva",
            "es evidente de que ocurre",
            "es seguro de que viene",
            "es obvio de que falta",
            "es claro de que conviene",
            "es logico de que pase",
            "es natural de que duela",
            "es normal de que suceda",
            "es falso de que exista",
        ];

        for text in samples {
            let corrections = analyze_text(text);
            assert_eq!(
                corrections.len(),
                1,
                "Debe detectar dequeismo en '{}': {:?}",
                text,
                corrections
            );
            assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
        }
    }

    #[test]
    fn test_pensaron_de_que_dequeismo() {
        let corrections = analyze_text("pensaron de que era fácil");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_dijeron_de_que_dequeismo() {
        let corrections = analyze_text("dijeron de que vendría");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Dequeismo);
    }

    #[test]
    fn test_creyeron_de_que_dequeismo() {
        let corrections = analyze_text("creyeron de que era posible");
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
    fn test_no_cabe_duda_que_queismo() {
        let corrections = analyze_text("no cabe duda que vendrá");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
        assert_eq!(corrections[0].suggestion, "de que");
    }

    #[test]
    fn test_es_hora_que_queismo() {
        let corrections = analyze_text("es hora que te vayas");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
        assert_eq!(corrections[0].suggestion, "de que");
    }

    #[test]
    fn test_antes_que_temporal_queismo() {
        let corrections = analyze_text("Antes que llegues avisa");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].error_type, DequeismoErrorType::Queismo);
        assert_eq!(corrections[0].suggestion, "de que");
    }

    #[test]
    fn test_antes_que_adversative_infinitive_no_correction() {
        let cases = [
            "Prefiero callar antes que mentir",
            "Es mejor esperar antes que actuar",
            "Quiero irme antes que quedarme",
        ];

        for text in cases {
            let corrections = analyze_text(text);
            assert!(
                corrections.is_empty(),
                "No debe corregir 'antes que + infinitivo' adversativo: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_mejor_antes_que_despues_no_correction() {
        let corrections = analyze_text("Mejor antes que despues");
        assert!(
            corrections.is_empty(),
            "No debe corregir comparativo adversativo 'antes que despues'"
        );
    }

    #[test]
    fn test_despues_temporal_inserted_before_completive_que_no_correction() {
        let cases = [
            "Renato Flores reconocio horas despues que hubo un fallo",
            "Hablo minutos despues que habia un problema",
            "Reconocio dias despues que se equivoco",
        ];

        for text in cases {
            let corrections = analyze_text(text);
            let wrong = corrections
                .iter()
                .find(|c| c.error_type == DequeismoErrorType::Queismo);
            assert!(
                wrong.is_none(),
                "No debe forzar 'despues de que' cuando 'que' es completiva: {text} -> {corrections:?}"
            );
        }
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
        assert!(
            corrections.is_empty(),
            "No debería corregir 'me alegro de que'"
        );
    }

    #[test]
    fn test_creo_que_correct() {
        let corrections = analyze_text("creo que es verdad");
        assert!(corrections.is_empty(), "No debería corregir 'creo que'");
    }

    #[test]
    fn test_estoy_seguro_de_que_correct() {
        let corrections = analyze_text("estoy seguro de que vendrá");
        assert!(
            corrections.is_empty(),
            "No debería corregir 'estoy seguro de que'"
        );
    }

    #[test]
    fn test_es_posible_que_correct() {
        let corrections = analyze_text("es posible que llueva");
        assert!(
            corrections.is_empty(),
            "No deberÃ­a corregir 'es posible que'"
        );
    }

    #[test]
    fn test_me_alegra_que_correct() {
        // "Me alegra que" es correcto: algo me alegra (impersonal, no reflexivo)
        // vs "Me alegro de que" (reflexivo)
        let corrections = analyze_text("me alegra que hayas venido");
        assert!(
            corrections.is_empty(),
            "No debería corregir 'me alegra que' (impersonal)"
        );
    }

    #[test]
    fn test_te_alegra_que_correct() {
        let corrections = analyze_text("te alegra que sea así");
        assert!(
            corrections.is_empty(),
            "No debería corregir 'te alegra que' (impersonal)"
        );
    }

    #[test]
    fn test_nos_alegra_que_correct() {
        let corrections = analyze_text("nos alegra que vengas");
        assert!(
            corrections.is_empty(),
            "No deberia corregir 'nos alegra que' (impersonal)"
        );
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "pienso" y "de que" estan separados por punto, no debe detectar dequeismo
        let corrections = analyze_text("Yo pienso. De que vengas depende todo");
        let dequeismo_corrections: Vec<_> = corrections
            .iter()
            .filter(|c| c.error_type == DequeismoErrorType::Dequeismo)
            .collect();
        assert!(
            dequeismo_corrections.is_empty(),
            "No debe detectar dequeismo cuando hay limite de oracion"
        );
    }
}
