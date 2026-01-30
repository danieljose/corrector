//! Análisis de concordancia sujeto-verbo
//!
//! Detecta errores de concordancia entre pronombres personales y verbos conjugados.
//! Ejemplo: "yo cantas" → "yo canto", "tú canto" → "tú cantas"

use crate::grammar::tokenizer::{Token, TokenType};

/// Persona gramatical del sujeto (según la forma verbal que usa)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammaticalPerson {
    First,  // yo, nosotros
    Second, // tú, vosotros
    Third,  // él, ella, ellos, ellas, usted, ustedes (usted/ustedes usan forma de 3ª persona)
}

/// Número gramatical del sujeto
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammaticalNumber {
    Singular,
    Plural,
}

/// Información del pronombre sujeto
#[derive(Debug, Clone)]
pub struct SubjectInfo {
    pub person: GrammaticalPerson,
    pub number: GrammaticalNumber,
}

/// Corrección de concordancia sujeto-verbo
#[derive(Debug, Clone)]
pub struct SubjectVerbCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub message: String,
}

/// Analizador de concordancia sujeto-verbo
pub struct SubjectVerbAnalyzer;

impl SubjectVerbAnalyzer {
    /// Analiza tokens buscando errores de concordancia sujeto-verbo
    pub fn analyze(tokens: &[Token]) -> Vec<SubjectVerbCorrection> {
        let mut corrections = Vec::new();

        // Buscar patrones de pronombre + verbo
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        for i in 0..word_tokens.len().saturating_sub(1) {
            let (idx1, token1) = word_tokens[i];
            let (idx2, token2) = word_tokens[i + 1];

            // Verificar que no haya puntuación de fin de oración entre los dos tokens
            if Self::has_sentence_boundary(tokens, idx1, idx2) {
                continue;
            }

            // Usar effective_text() para ver correcciones de fases anteriores (ej: diacríticas)
            let text1 = token1.effective_text();
            let text2 = token2.effective_text();

            // Verificar si el primer token es un pronombre personal sujeto
            if let Some(subject_info) = Self::get_subject_info(text1) {
                // Detectar sujetos compuestos
                // Caso 1: "X y pronombre" (pronombre precedido de conjunción)
                // El verbo estará en forma plural, no debemos corregirlo
                if i >= 1 {
                    let (_, prev_token) = word_tokens[i - 1];
                    let prev_lower = prev_token.effective_text().to_lowercase();
                    // "y yo", "y tú", "y ella", etc. indica sujeto compuesto → skip
                    if prev_lower == "y" || prev_lower == "e" {
                        continue;
                    }
                }
                // Caso 2: "Pronombre y X" (pronombre seguido de conjunción)
                // El verbo estará en forma plural, no debemos corregirlo
                if i + 1 < word_tokens.len() {
                    let (_, next_token) = word_tokens[i + 1];
                    let next_lower = next_token.effective_text().to_lowercase();
                    if next_lower == "y" || next_lower == "e" {
                        // Es sujeto compuesto, skip
                        continue;
                    }
                }

                // Verificar si el segundo token es un verbo conjugado
                if let Some(correction) = Self::check_verb_agreement(
                    idx2,
                    text2,
                    &subject_info,
                ) {
                    corrections.push(correction);
                }
            }
        }

        corrections
    }

    /// Verifica si hay puntuación de fin de oración entre dos índices de tokens
    fn has_sentence_boundary(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        // Signos que marcan fin de oración o separación fuerte
        const SENTENCE_BOUNDARIES: &[&str] = &[
            ".", "?", "!", ";", ":", "¿", "¡", "...", "—", "–",
        ];

        for idx in (start_idx + 1)..end_idx {
            if let Some(token) = tokens.get(idx) {
                if token.token_type == TokenType::Punctuation {
                    let text = token.text.as_str();
                    if SENTENCE_BOUNDARIES.contains(&text) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Obtiene información gramatical de un pronombre personal sujeto
    fn get_subject_info(word: &str) -> Option<SubjectInfo> {
        let lower = word.to_lowercase();
        match lower.as_str() {
            "yo" => Some(SubjectInfo {
                person: GrammaticalPerson::First,
                number: GrammaticalNumber::Singular,
            }),
            // NOTA: Solo "tú" con tilde es pronombre personal; "tu" sin tilde es posesivo
            "tú" => Some(SubjectInfo {
                person: GrammaticalPerson::Second,
                number: GrammaticalNumber::Singular,
            }),
            // "usted" usa forma de tercera persona singular
            // NOTA: Solo "él" con tilde es pronombre personal; "el" sin tilde es artículo
            "él" | "ella" | "usted" => Some(SubjectInfo {
                person: GrammaticalPerson::Third,
                number: GrammaticalNumber::Singular,
            }),
            "nosotros" | "nosotras" => Some(SubjectInfo {
                person: GrammaticalPerson::First,
                number: GrammaticalNumber::Plural,
            }),
            "vosotros" | "vosotras" => Some(SubjectInfo {
                person: GrammaticalPerson::Second,
                number: GrammaticalNumber::Plural,
            }),
            // "ustedes" usa forma de tercera persona plural
            "ellos" | "ellas" | "ustedes" => Some(SubjectInfo {
                person: GrammaticalPerson::Third,
                number: GrammaticalNumber::Plural,
            }),
            _ => None,
        }
    }

    /// Verifica si el verbo concuerda con el sujeto y devuelve corrección si no
    fn check_verb_agreement(
        verb_index: usize,
        verb: &str,
        subject: &SubjectInfo,
    ) -> Option<SubjectVerbCorrection> {
        let verb_lower = verb.to_lowercase();

        // Obtener información de la conjugación del verbo
        if let Some((verb_person, verb_number, infinitive)) = Self::get_verb_info(&verb_lower) {
            // Verificar concordancia
            if verb_person != subject.person || verb_number != subject.number {
                // Generar la forma correcta
                if let Some(correct_form) = Self::get_correct_form(
                    &infinitive,
                    subject.person,
                    subject.number,
                ) {
                    // Solo corregir si tenemos una forma diferente
                    if correct_form.to_lowercase() != verb_lower {
                        return Some(SubjectVerbCorrection {
                            token_index: verb_index,
                            original: verb.to_string(),
                            suggestion: correct_form.clone(),
                            message: format!(
                                "Concordancia sujeto-verbo: '{}' debería ser '{}'",
                                verb, correct_form
                            ),
                        });
                    }
                }
            }
        }

        None
    }

    /// Obtiene información de persona/número del verbo conjugado
    /// Devuelve (persona, número, infinitivo)
    fn get_verb_info(verb: &str) -> Option<(GrammaticalPerson, GrammaticalNumber, String)> {
        // Excluir preposiciones y otras palabras que no son verbos pero podrían parecer formas verbales
        let non_verbs = [
            // Adverbios cortos que terminan en -o/-a/-e
            "no", "ya", "nunca", "ahora", "luego", "antes", "después",
            // Artículos y pronombres átonos
            "el", "la", "los", "las", "lo",
            "un", "una", "unos", "unas",
            "me", "te", "se", "nos", "os", "le", "les",
            // Participios de presente usados como sustantivos/adjetivos (-ante, -ente)
            "pensante", "amante", "estudiante", "cantante", "brillante",
            "importante", "constante", "instante", "distante", "elegante",
            "gigante", "ambulante", "abundante", "dominante", "fascinante",
            "ente", "paciente", "pendiente", "presente", "ausente",
            "consciente", "inconsciente", "evidente", "diferente", "excelente",
            // Preposiciones
            "de", "a", "en", "con", "por", "para", "sin",
            "sobre", "ante", "entre", "desde", "durante", "mediante", "según",
            "contra", "hacia", "hasta", "mediante", "tras",
            // Conjunciones y relativos
            "que", "porque", "aunque", "mientras", "donde", "como", "cuando",
            "sino", "pero", "mas", "pues", "luego",
            // Demostrativos y adjetivos que terminan en -o/-a
            "este", "ese", "aquel", "grande",
            "mismo", "misma", "mismos", "mismas",
            "otro", "otra", "otros", "otras",
            "poco", "poca", "pocos", "pocas",
            "mucho", "mucha", "muchos", "muchas",
            "tanto", "tanta", "tantos", "tantas",
            "cuanto", "cuanta", "cuantos", "cuantas",
            "todo", "toda", "todos", "todas",
            "alguno", "alguna", "algunos", "algunas",
            "ninguno", "ninguna", "ningunos", "ningunas",
            "cierto", "cierta", "ciertos", "ciertas",
            "propio", "propia", "propios", "propias",
            "solo", "sola", "solos", "solas",
            "medio", "media", "medios", "medias",
            "doble", "triple",
            // Subjuntivo imperfecto - formas comunes que se confunden con verbos -ar
            // ser/ir
            "fuera", "fueras", "fuéramos", "fuerais", "fueran",
            "fuese", "fueses", "fuésemos", "fueseis", "fuesen",
            // tener
            "tuviera", "tuvieras", "tuviéramos", "tuvierais", "tuvieran",
            "tuviese", "tuvieses", "tuviésemos", "tuvieseis", "tuviesen",
            // estar
            "estuviera", "estuvieras", "estuviéramos", "estuvierais", "estuvieran",
            "estuviese", "estuvieses", "estuviésemos", "estuvieseis", "estuviesen",
            // hacer
            "hiciera", "hicieras", "hiciéramos", "hicierais", "hicieran",
            "hiciese", "hicieses", "hiciésemos", "hicieseis", "hiciesen",
            // poder
            "pudiera", "pudieras", "pudiéramos", "pudierais", "pudieran",
            "pudiese", "pudieses", "pudiésemos", "pudieseis", "pudiesen",
            // poner
            "pusiera", "pusieras", "pusiéramos", "pusierais", "pusieran",
            "pusiese", "pusieses", "pusiésemos", "pusieseis", "pusiesen",
            // saber
            "supiera", "supieras", "supiéramos", "supierais", "supieran",
            "supiese", "supieses", "supiésemos", "supieseis", "supiesen",
            // querer
            "quisiera", "quisieras", "quisiéramos", "quisierais", "quisieran",
            "quisiese", "quisieses", "quisiésemos", "quisieseis", "quisiesen",
            // venir
            "viniera", "vinieras", "viniéramos", "vinierais", "vinieran",
            "viniese", "vinieses", "viniésemos", "vinieseis", "viniesen",
            // decir
            "dijera", "dijeras", "dijéramos", "dijerais", "dijeran",
            "dijese", "dijeses", "dijésemos", "dijeseis", "dijesen",
            // Pretérito de decir
            "dijo", "dijeron",
            // Imperfecto de ser (se confunde con verbos -ar)
            "era", "eras", "éramos", "erais", "eran",
            // Imperfecto de ir
            "iba", "ibas", "íbamos", "ibais", "iban",
            // Pretérito de hacer
            "hizo", "hicieron",
            // Pretérito de poner
            "puso", "pusieron",
            // Pretérito de tener
            "tuvo", "tuvieron",
            // Pretérito de traer
            "trajo", "trajeron",
            // Pretérito de venir
            "vino", "vinieron",
            // Pretérito de saber
            "supo", "supieron",
            // Pretérito de querer
            "quiso", "quisieron",
        ];
        if non_verbs.contains(&verb) {
            return None;
        }

        // Verbos irregulares comunes - ser
        match verb {
            "soy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "ser".to_string())),
            "eres" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "ser".to_string())),
            "es" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "ser".to_string())),
            "somos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "ser".to_string())),
            "sois" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "ser".to_string())),
            "son" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "ser".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - estar
        match verb {
            "estoy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "estar".to_string())),
            "estás" | "estas" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "estar".to_string())),
            "está" | "esta" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "estar".to_string())),
            "estamos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "estar".to_string())),
            "estáis" | "estais" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "estar".to_string())),
            "están" | "estan" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "estar".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - tener
        match verb {
            "tengo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "tener".to_string())),
            "tienes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "tener".to_string())),
            "tiene" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "tener".to_string())),
            "tenemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "tener".to_string())),
            "tenéis" | "teneis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "tener".to_string())),
            "tienen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "tener".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - ir
        match verb {
            "voy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "ir".to_string())),
            "vas" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "ir".to_string())),
            "va" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "ir".to_string())),
            "vamos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "ir".to_string())),
            "vais" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "ir".to_string())),
            "van" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "ir".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - hacer
        match verb {
            "hago" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "hacer".to_string())),
            "haces" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "hacer".to_string())),
            "hace" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "hacer".to_string())),
            "hacemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "hacer".to_string())),
            "hacéis" | "haceis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "hacer".to_string())),
            "hacen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "hacer".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - poder
        match verb {
            "puedo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "poder".to_string())),
            "puedes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "poder".to_string())),
            "puede" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "poder".to_string())),
            "podemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "poder".to_string())),
            "podéis" | "podeis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "poder".to_string())),
            "pueden" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "poder".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - querer
        match verb {
            "quiero" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "querer".to_string())),
            "quieres" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "querer".to_string())),
            "quiere" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "querer".to_string())),
            "queremos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "querer".to_string())),
            "queréis" | "quereis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "querer".to_string())),
            "quieren" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "querer".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - decir
        match verb {
            "digo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "decir".to_string())),
            "dices" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "decir".to_string())),
            "dice" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "decir".to_string())),
            "decimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "decir".to_string())),
            "decís" | "decis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "decir".to_string())),
            "dicen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "decir".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - saber
        // NOTA: Solo "sé" con tilde es verbo; "se" sin tilde es pronombre reflexivo
        match verb {
            "sé" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "saber".to_string())),
            "sabes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "saber".to_string())),
            "sabe" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "saber".to_string())),
            "sabemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "saber".to_string())),
            "sabéis" | "sabeis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "saber".to_string())),
            "saben" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "saber".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - venir
        match verb {
            "vengo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "venir".to_string())),
            "vienes" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "venir".to_string())),
            "viene" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "venir".to_string())),
            "venimos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "venir".to_string())),
            "venís" | "venis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "venir".to_string())),
            "vienen" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "venir".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - dar
        match verb {
            "doy" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "dar".to_string())),
            "das" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "dar".to_string())),
            "da" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "dar".to_string())),
            "damos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "dar".to_string())),
            "dais" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "dar".to_string())),
            "dan" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "dar".to_string())),
            _ => {}
        }

        // Verbos irregulares comunes - ver
        match verb {
            "veo" => return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, "ver".to_string())),
            "ves" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, "ver".to_string())),
            "ve" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, "ver".to_string())),
            "vemos" => return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, "ver".to_string())),
            "veis" => return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, "ver".to_string())),
            "ven" => return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, "ver".to_string())),
            _ => {}
        }

        // Verbos regulares -ar (presente indicativo)
        if let Some(stem) = verb.strip_suffix("o") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Singular, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("as") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("a") {
            if !stem.is_empty() && !verb.ends_with("ía") {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("amos") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("áis") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("ais") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, format!("{}ar", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("an") {
            if !stem.is_empty() && !verb.ends_with("ían") {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, format!("{}ar", stem)));
            }
        }

        // Verbos regulares -er (presente indicativo)
        // NOTA: Excluimos stems que terminan en "c" porque probablemente son
        // subjuntivo de verbos -zar (ej: "garantice" es subjuntivo de "garantizar",
        // no indicativo de hipotético "garanticer")
        if let Some(stem) = verb.strip_suffix("es") {
            if !stem.is_empty() && !verb.ends_with("as") && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Singular, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("e") {
            if !stem.is_empty() && !verb.ends_with("a") && !verb.ends_with("ie") && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Singular, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("emos") {
            if !stem.is_empty() && !stem.ends_with('c') {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("éis") {
            if !stem.is_empty() && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("eis") {
            if !stem.is_empty() && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, format!("{}er", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("en") {
            if !stem.is_empty() && !verb.ends_with("an") && !verb.ends_with("ien") && !stem.ends_with('c') {
                return Some((GrammaticalPerson::Third, GrammaticalNumber::Plural, format!("{}er", stem)));
            }
        }

        // Verbos regulares -ir (presente indicativo)
        if let Some(stem) = verb.strip_suffix("imos") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::First, GrammaticalNumber::Plural, format!("{}ir", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("ís") {
            if !stem.is_empty() {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, format!("{}ir", stem)));
            }
        }
        if let Some(stem) = verb.strip_suffix("is") {
            if !stem.is_empty() && !verb.ends_with("ais") && !verb.ends_with("eis") {
                return Some((GrammaticalPerson::Second, GrammaticalNumber::Plural, format!("{}ir", stem)));
            }
        }

        None
    }

    /// Obtiene la forma correcta del verbo para la persona y número dados
    fn get_correct_form(
        infinitive: &str,
        person: GrammaticalPerson,
        number: GrammaticalNumber,
    ) -> Option<String> {
        // Verbos irregulares - ser
        if infinitive == "ser" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "soy",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "eres",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "es",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "somos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "sois",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "son",
            }.to_string());
        }

        // Verbos irregulares - estar
        if infinitive == "estar" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "estoy",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "estás",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "está",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "estamos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "estáis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "están",
            }.to_string());
        }

        // Verbos irregulares - tener
        if infinitive == "tener" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "tengo",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "tienes",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "tiene",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "tenemos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "tenéis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "tienen",
            }.to_string());
        }

        // Verbos irregulares - ir
        if infinitive == "ir" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "voy",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "vas",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "va",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "vamos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "vais",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "van",
            }.to_string());
        }

        // Verbos irregulares - hacer
        if infinitive == "hacer" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "hago",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "haces",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "hace",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "hacemos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "hacéis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "hacen",
            }.to_string());
        }

        // Verbos irregulares - poder
        if infinitive == "poder" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "puedo",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "puedes",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "puede",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "podemos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "podéis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "pueden",
            }.to_string());
        }

        // Verbos irregulares - querer
        if infinitive == "querer" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "quiero",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "quieres",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "quiere",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "queremos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "queréis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "quieren",
            }.to_string());
        }

        // Verbos irregulares - decir
        if infinitive == "decir" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "digo",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "dices",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "dice",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "decimos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "decís",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "dicen",
            }.to_string());
        }

        // Verbos irregulares - saber
        if infinitive == "saber" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "sé",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "sabes",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "sabe",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "sabemos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "sabéis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "saben",
            }.to_string());
        }

        // Verbos irregulares - venir
        if infinitive == "venir" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "vengo",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "vienes",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "viene",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "venimos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "venís",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "vienen",
            }.to_string());
        }

        // Verbos irregulares - dar
        if infinitive == "dar" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "doy",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "das",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "da",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "damos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "dais",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "dan",
            }.to_string());
        }

        // Verbos irregulares - ver
        if infinitive == "ver" {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => "veo",
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => "ves",
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => "ve",
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => "vemos",
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => "veis",
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => "ven",
            }.to_string());
        }

        // Verbos regulares -ar
        if let Some(stem) = infinitive.strip_suffix("ar") {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}o", stem),
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}as", stem),
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}a", stem),
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}amos", stem),
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}áis", stem),
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}an", stem),
            });
        }

        // Verbos regulares -er
        if let Some(stem) = infinitive.strip_suffix("er") {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}o", stem),
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}es", stem),
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}e", stem),
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}emos", stem),
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}éis", stem),
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}en", stem),
            });
        }

        // Verbos regulares -ir
        if let Some(stem) = infinitive.strip_suffix("ir") {
            return Some(match (person, number) {
                (GrammaticalPerson::First, GrammaticalNumber::Singular) => format!("{}o", stem),
                (GrammaticalPerson::Second, GrammaticalNumber::Singular) => format!("{}es", stem),
                (GrammaticalPerson::Third, GrammaticalNumber::Singular) => format!("{}e", stem),
                (GrammaticalPerson::First, GrammaticalNumber::Plural) => format!("{}imos", stem),
                (GrammaticalPerson::Second, GrammaticalNumber::Plural) => format!("{}ís", stem),
                (GrammaticalPerson::Third, GrammaticalNumber::Plural) => format!("{}en", stem),
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::tokenizer::Tokenizer;

    fn tokenize(text: &str) -> Vec<Token> {
        Tokenizer::new().tokenize(text)
    }

    #[test]
    fn test_yo_with_wrong_verb() {
        // "yo cantas" debería sugerir "canto"
        let tokens = tokenize("yo cantas");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canto");
    }

    #[test]
    fn test_tu_with_wrong_verb() {
        // "tú canto" debería sugerir "cantas"
        let tokens = tokenize("tú canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantas");
    }

    #[test]
    fn test_el_with_wrong_verb() {
        // "él cantamos" debería sugerir "canta"
        let tokens = tokenize("él cantamos");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canta");
    }

    #[test]
    fn test_nosotros_with_wrong_verb() {
        // "nosotros canta" debería sugerir "cantamos"
        let tokens = tokenize("nosotros canta");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantamos");
    }

    #[test]
    fn test_ellos_with_wrong_verb() {
        // "ellos canto" debería sugerir "cantan"
        let tokens = tokenize("ellos canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantan");
    }

    #[test]
    fn test_correct_agreement() {
        // "yo canto" es correcto, no debería haber correcciones
        let tokens = tokenize("yo canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_irregular_ser_yo() {
        // "yo eres" debería sugerir "soy"
        let tokens = tokenize("yo eres");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "soy");
    }

    #[test]
    fn test_irregular_ser_tu() {
        // "tú soy" debería sugerir "eres"
        let tokens = tokenize("tú soy");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "eres");
    }

    #[test]
    fn test_irregular_estar() {
        // "yo estás" debería sugerir "estoy"
        let tokens = tokenize("yo estás");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "estoy");
    }

    #[test]
    fn test_irregular_tener() {
        // "yo tienes" debería sugerir "tengo"
        let tokens = tokenize("yo tienes");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tengo");
    }

    #[test]
    fn test_irregular_ir() {
        // "yo vas" debería sugerir "voy"
        let tokens = tokenize("yo vas");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "voy");
    }

    #[test]
    fn test_irregular_hacer() {
        // "yo haces" debería sugerir "hago"
        let tokens = tokenize("yo haces");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hago");
    }

    #[test]
    fn test_correct_irregular() {
        // "yo soy" es correcto
        let tokens = tokenize("yo soy");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_er_verb() {
        // "yo comes" debería sugerir "como"
        let tokens = tokenize("yo comes");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "como");
    }

    #[test]
    fn test_ir_verb() {
        // "yo vives" debería sugerir "vivo"
        let tokens = tokenize("yo vives");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "vivo");
    }

    #[test]
    fn test_vosotros() {
        // "vosotros canto" debería sugerir "cantáis"
        let tokens = tokenize("vosotros canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "cantáis");
    }

    #[test]
    fn test_ella_with_nosotros_form() {
        // "ella cantamos" debería sugerir "canta"
        let tokens = tokenize("ella cantamos");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canta");
    }

    #[test]
    fn test_multiple_errors_in_sentence() {
        // Una oración con múltiples errores potenciales
        // Solo detectamos el patrón pronombre + verbo inmediato
        let tokens = tokenize("yo cantas bien");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canto");
    }

    // =========================================================================
    // Tests de límites de oración - no debe cruzar puntuación
    // =========================================================================

    #[test]
    fn test_no_cross_period() {
        // "él. canto" - el punto separa oraciones, no debe emparejar "él" con "canto"
        let tokens = tokenize("él. canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar punto: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_question_mark() {
        // "¿él? canto" - el signo de interrogación separa
        let tokens = tokenize("¿él? canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar signo de interrogación: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_exclamation() {
        // "¡él! canto" - el signo de exclamación separa
        let tokens = tokenize("¡él! canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar signo de exclamación: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_semicolon() {
        // "él; canto" - el punto y coma separa
        let tokens = tokenize("él; canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar punto y coma: {:?}",
            corrections
        );
    }

    #[test]
    fn test_no_cross_colon() {
        // "él: canto" - los dos puntos separan
        let tokens = tokenize("él: canto");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería cruzar dos puntos: {:?}",
            corrections
        );
    }

    #[test]
    fn test_still_detects_within_sentence() {
        // "Dijo que yo cantas mal" - error dentro de la misma oración
        let tokens = tokenize("Dijo que yo cantas mal");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canto");
    }

    #[test]
    fn test_comma_does_not_block() {
        // "yo, sinceramente, cantas" - las comas no bloquean (no son fin de oración)
        // pero tampoco debería detectar error porque "cantas" no es inmediato a "yo"
        // Nota: el analizador solo mira word tokens consecutivos
        let tokens = tokenize("yo cantas, bien");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1, "Coma después no debería afectar detección");
    }

    #[test]
    fn test_multiple_sentences_independent() {
        // "Yo canto. Él cantas." - cada oración es independiente
        // Primera oración correcta, segunda tiene error
        let tokens = tokenize("Yo canto. Él cantas.");
        let corrections = SubjectVerbAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "canta");
    }
}
