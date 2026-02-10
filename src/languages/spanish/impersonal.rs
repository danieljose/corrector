//! Detección de verbos impersonales pluralizados (haber existencial y hacer temporal).
//!
//! El verbo "haber" en uso existencial/impersonal debe ir siempre en
//! 3.ª persona del singular:
//!
//! - "habían muchas personas" → "había muchas personas"
//! - "hubieron accidentes" → "hubo accidentes"
//! - "habrán problemas" → "habrá problemas"
//! - "han habido quejas" → "ha habido quejas"
//!
//! El verbo "hacer" en expresiones temporales es impersonal y debe ir
//! siempre en 3.ª persona del singular:
//!
//! - "hacen tres años" → "hace tres años"
//! - "hacían muchos días" → "hacía muchos días"
//! - "hicieron dos semanas" → "hizo dos semanas"

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

/// Modales/perífrasis en plural que, delante de "haber" existencial,
/// deben ir en singular: "deben haber" -> "debe haber", etc.
const MODAL_PLURAL_TO_SINGULAR: &[(&str, &str)] = &[
    // deber
    ("deben", "debe"),
    ("debían", "debía"),
    ("debian", "debia"),
    ("deberán", "deberá"),
    ("deberan", "debera"),
    ("deberían", "debería"),
    ("deberian", "deberia"),
    ("deban", "deba"),
    ("debieran", "debiera"),
    ("debiesen", "debiese"),
    // poder
    ("pueden", "puede"),
    ("podían", "podía"),
    ("podian", "podia"),
    ("podrán", "podrá"),
    ("podran", "podra"),
    ("podrían", "podría"),
    ("podrian", "podria"),
    ("puedan", "pueda"),
    ("pudieran", "pudiera"),
    ("pudiesen", "pudiese"),
    // tener + que + haber
    ("tienen", "tiene"),
    ("tenían", "tenía"),
    ("tenian", "tenia"),
    ("tendrán", "tendrá"),
    ("tendran", "tendra"),
    ("tendrían", "tendría"),
    ("tendrian", "tendria"),
    ("tengan", "tenga"),
    ("tuvieran", "tuviera"),
    ("tuviesen", "tuviese"),
    // soler + haber
    ("suelen", "suele"),
    ("solían", "solía"),
    ("solian", "solia"),
    ("solerán", "solerá"),
    ("soleran", "solera"),
    ("solerían", "solería"),
    ("solerian", "soleria"),
    ("suelan", "suela"),
    ("solieran", "soliera"),
    ("soliesen", "soliese"),
    // ir + a + haber
    ("van", "va"),
    ("iban", "iba"),
    ("irán", "irá"),
    ("iran", "ira"),
    ("vayan", "vaya"),
];

/// Formas plurales de hacer → forma singular correcta (impersonal temporal).
const HACER_PLURAL_TO_SINGULAR: &[(&str, &str)] = &[
    ("hacen", "hace"),       // presente
    ("hacían", "hacía"),     // imperfecto
    ("hicieron", "hizo"),    // pretérito
    ("harán", "hará"),       // futuro
    ("harían", "haría"),     // condicional
    ("hagan", "haga"),       // subjuntivo presente
    ("hicieran", "hiciera"), // subjuntivo imperfecto -ra
    ("hiciesen", "hiciese"), // subjuntivo imperfecto -se
];

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

            // Caso 2c: modal/perífrasis plural + haber existencial:
            // "deben haber razones", "pueden haber problemas", "van a haber cambios".
            if let Some(singular_modal) = Self::get_modal_singular(&word_lower) {
                if let Some(haber_idx) = Self::find_haber_after_modal(tokens, i, &word_lower) {
                    if Self::is_followed_by_nominal(tokens, haber_idx) {
                        corrections.push(ImpersonalCorrection {
                            token_index: i,
                            original: tokens[i].text.clone(),
                            suggestion: Self::preserve_case(&tokens[i].text, singular_modal),
                        });
                    }
                }
            }

            // Caso 3: Hacer temporal pluralizado: "hacen tres años" → "hace tres años"
            if let Some(singular) = Self::get_hacer_singular(&word_lower) {
                if !Self::has_explicit_subject_before(tokens, i)
                    && Self::is_followed_by_temporal_sn(tokens, i)
                {
                    corrections.push(ImpersonalCorrection {
                        token_index: i,
                        original: tokens[i].text.clone(),
                        suggestion: Self::preserve_case(&tokens[i].text, singular),
                    });
                }
            }

            // Caso 4: "haber" existencial + artículo definido + SN -> preferir indefinido.
            // "Hay/Había el problema..." -> "Hay/Había un problema..."
            if Self::is_existential_haber_head(&word_lower) {
                if let Some((article_idx, indefinite)) =
                    Self::find_defined_article_after_existential_haber(tokens, i)
                {
                    corrections.push(ImpersonalCorrection {
                        token_index: article_idx,
                        original: tokens[article_idx].text.clone(),
                        suggestion: Self::preserve_case(&tokens[article_idx].text, indefinite),
                    });
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

    fn get_modal_singular(word: &str) -> Option<&'static str> {
        MODAL_PLURAL_TO_SINGULAR
            .iter()
            .find(|(plural, _)| *plural == word)
            .map(|(_, singular)| *singular)
    }

    /// Busca la cabeza "haber" tras un modal/perífrasis plural.
    /// Acepta:
    /// - modal + haber ("deben haber...")
    /// - deber + de + haber ("deben de haber...")
    /// - tener + que + haber ("tienen que haber...")
    /// - ir + a + haber ("van a haber...")
    fn find_haber_after_modal(
        tokens: &[Token],
        modal_idx: usize,
        modal_word: &str,
    ) -> Option<usize> {
        let next_idx = Self::next_non_whitespace_idx(tokens, modal_idx)?;
        if has_sentence_boundary(tokens, modal_idx, next_idx) {
            return None;
        }
        if tokens[next_idx].token_type != TokenType::Word {
            return None;
        }

        let next_lower = tokens[next_idx].effective_text().to_lowercase();
        if next_lower == "haber" {
            return Some(next_idx);
        }

        let bridge_ok = (next_lower == "a" && Self::is_ir_plural_modal(modal_word))
            || (next_lower == "de" && Self::is_deber_plural_modal(modal_word))
            || (next_lower == "que" && Self::is_tener_plural_modal(modal_word));

        if bridge_ok {
            let haber_idx = Self::next_non_whitespace_idx(tokens, next_idx)?;
            if has_sentence_boundary(tokens, next_idx, haber_idx)
                || has_sentence_boundary(tokens, modal_idx, haber_idx)
            {
                return None;
            }
            if tokens[haber_idx].token_type == TokenType::Word
                && tokens[haber_idx].effective_text().to_lowercase() == "haber"
            {
                return Some(haber_idx);
            }
        }

        None
    }

    fn is_deber_plural_modal(word: &str) -> bool {
        matches!(
            word,
            "deben"
                | "debían"
                | "debian"
                | "deberán"
                | "deberan"
                | "deberían"
                | "deberian"
                | "deban"
                | "debieran"
                | "debiesen"
        )
    }

    fn is_tener_plural_modal(word: &str) -> bool {
        matches!(
            word,
            "tienen"
                | "tenían"
                | "tenian"
                | "tendrán"
                | "tendran"
                | "tendrían"
                | "tendrian"
                | "tengan"
                | "tuvieran"
                | "tuviesen"
        )
    }

    fn is_ir_plural_modal(word: &str) -> bool {
        matches!(word, "van" | "iban" | "irán" | "iran" | "vayan")
    }

    /// En construcciones existenciales con "haber", un artículo definido suele ser
    /// incorrecto: "hay/había el problema..." -> "hay/había un problema...".
    ///
    /// Regla conservadora:
    /// - Debe haber artículo definido inmediatamente después (saltando espacios).
    /// - Debe seguir una palabra nominal.
    /// - Se excluye "hay la de ..." (coloquial cuantificador).
    fn find_defined_article_after_existential_haber(
        tokens: &[Token],
        haber_idx: usize,
    ) -> Option<(usize, &'static str)> {
        let article_idx = Self::next_non_whitespace_idx(tokens, haber_idx)?;
        if has_sentence_boundary(tokens, haber_idx, article_idx) {
            return None;
        }

        let article_token = &tokens[article_idx];
        if article_token.token_type != TokenType::Word {
            return None;
        }

        let article_lower = article_token.effective_text().to_lowercase();
        let indefinite = match article_lower.as_str() {
            "el" => "un",
            "la" => "una",
            "los" => "unos",
            "las" => "unas",
            _ => return None,
        };

        let next_idx = Self::next_non_whitespace_idx(tokens, article_idx)?;
        if has_sentence_boundary(tokens, article_idx, next_idx) {
            return None;
        }

        let next_token = &tokens[next_idx];
        if next_token.token_type != TokenType::Word {
            return None;
        }

        let next_lower = next_token.effective_text().to_lowercase();
        if matches!(next_lower.as_str(), "de" | "que" | "cual" | "cuales") {
            return None;
        }

        // "no hay la menor duda", "no hay el mínimo problema" — superlative patterns
        // "hay el doble/triple de..." — multiplicative quantifiers
        if matches!(
            next_lower.as_str(),
            "menor"
                | "menores"
                | "m\u{00ED}nimo"
                | "m\u{00ED}nima"
                | "m\u{00ED}nimos"
                | "m\u{00ED}nimas"
                | "m\u{00E1}s"
                | "doble"
                | "triple"
                | "cu\u{00E1}druple"
        ) {
            return None;
        }

        Some((article_idx, indefinite))
    }

    fn is_existential_haber_head(word: &str) -> bool {
        matches!(
            word,
            "hay"
                | "había"
                | "habia"
                | "hubo"
                | "habrá"
                | "habra"
                | "habría"
                | "habria"
                | "haya"
                | "hubiera"
                | "hubiese"
        )
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

    fn next_non_whitespace_idx(tokens: &[Token], idx: usize) -> Option<usize> {
        let mut j = idx + 1;
        while j < tokens.len() && tokens[j].token_type == TokenType::Whitespace {
            j += 1;
        }
        (j < tokens.len()).then_some(j)
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

    // ======================================================================
    // Hacer temporal impersonal
    // ======================================================================

    /// Busca la forma singular para una forma plural de hacer temporal.
    fn get_hacer_singular(word: &str) -> Option<&'static str> {
        HACER_PLURAL_TO_SINGULAR
            .iter()
            .find(|(plural, _)| *plural == word)
            .map(|(_, singular)| *singular)
    }

    /// Verifica si hay un sujeto explícito antes del verbo (sustantivo plural
    /// o pronombre sujeto plural), lo que indica uso transitivo, no impersonal.
    /// Ej: "Los niños hacen tres horas de deberes" → sujeto "niños" → no corregir.
    fn has_explicit_subject_before(tokens: &[Token], idx: usize) -> bool {
        // Retroceder saltando whitespace
        let mut j = idx;
        loop {
            if j == 0 {
                return false;
            }
            j -= 1;
            if tokens[j].token_type == TokenType::Whitespace {
                continue;
            }
            break;
        }

        // Si hay frontera de oración, no hay sujeto
        if tokens[j].is_sentence_boundary() {
            return false;
        }

        if tokens[j].token_type != TokenType::Word {
            return false;
        }

        let prev_lower = tokens[j].effective_text().to_lowercase();

        // Pronombres sujeto plural → sujeto explícito
        if matches!(prev_lower.as_str(), "ellos" | "ellas" | "ustedes") {
            return true;
        }

        // Preposiciones, conjunciones, adverbios → no hay sujeto antes del verbo
        if matches!(
            prev_lower.as_str(),
            "que" | "cuando" | "donde" | "como" | "si" | "ya" | "no" | "también"
                | "además" | "aún" | "todavía"
        ) {
            return false;
        }

        // Usar word_info si disponible
        if let Some(ref info) = tokens[j].word_info {
            use crate::dictionary::WordCategory;
            match info.category {
                WordCategory::Sustantivo | WordCategory::Otro => {
                    // Sustantivo plural → probable sujeto
                    if info.number == crate::dictionary::Number::Plural {
                        return true;
                    }
                    return false;
                }
                WordCategory::Preposicion
                | WordCategory::Conjuncion
                | WordCategory::Adverbio => return false,
                _ => return false,
            }
        }

        // Fallback léxico: palabra terminada en -s (probable plural) que no sea
        // preposición/conjunción/adverbio conocido
        if prev_lower.ends_with('s') && prev_lower.len() > 3 {
            return true;
        }

        false
    }

    /// Verifica si tras el token en `idx` hay un sintagma nominal temporal.
    /// Escanea hasta frontera de oración buscando: fillers opcionales + sustantivo temporal.
    /// Guarda post-temporal: si tras el sustantivo viene "de + sustantivo", es objeto
    /// léxico ("tres horas de deberes") → no corregir.
    fn is_followed_by_temporal_sn(tokens: &[Token], idx: usize) -> bool {
        let mut j = idx + 1;
        let mut found_time_noun_at = None;

        while j < tokens.len() {
            // Saltar whitespace
            if tokens[j].token_type == TokenType::Whitespace {
                j += 1;
                continue;
            }

            // Frontera de oración → parar
            if has_sentence_boundary(tokens, idx, j) || tokens[j].is_sentence_boundary() {
                break;
            }

            // Aceptar tokens numéricos (ej. "hacen 3 años")
            if tokens[j].token_type == TokenType::Number {
                j += 1;
                continue;
            }

            if tokens[j].token_type != TokenType::Word {
                break;
            }

            let w = tokens[j].effective_text().to_lowercase();

            // ¿Es sustantivo temporal?
            if Self::is_hacer_time_noun(&w) {
                found_time_noun_at = Some(j);
                break;
            }

            // ¿Es filler permitido entre hacer y el sustantivo temporal?
            if Self::is_temporal_filler(&w) {
                j += 1;
                continue;
            }

            // Otra palabra no esperada → no es temporal
            break;
        }

        // Si encontramos sustantivo temporal, verificar guardia post-temporal
        if let Some(time_idx) = found_time_noun_at {
            return !Self::has_object_complement_after(tokens, time_idx);
        }

        false
    }

    /// Verifica si tras el sustantivo temporal viene "de + sustantivo",
    /// indicando complemento de objeto ("tres horas de deberes") → no impersonal.
    fn has_object_complement_after(tokens: &[Token], time_idx: usize) -> bool {
        let mut j = time_idx + 1;

        // Saltar whitespace
        while j < tokens.len() && tokens[j].token_type == TokenType::Whitespace {
            j += 1;
        }

        if j >= tokens.len() {
            return false;
        }

        // ¿Viene "de"?
        if tokens[j].token_type == TokenType::Word
            && tokens[j].effective_text().to_lowercase() == "de"
        {
            // Saltar whitespace tras "de"
            let mut k = j + 1;
            while k < tokens.len() && tokens[k].token_type == TokenType::Whitespace {
                k += 1;
            }
            if k >= tokens.len() {
                return false;
            }
            // Si tras "de" viene una palabra que NO es sustantivo temporal,
            // es complemento de objeto ("horas de deberes", "horas de cola")
            if tokens[k].token_type == TokenType::Word {
                let after_de = tokens[k].effective_text().to_lowercase();
                // "hace mucho tiempo de eso" → no bloquear; pero
                // "hacen tres horas de deberes" → sí bloquear.
                // Bloquear si lo que sigue a "de" no es un sustantivo temporal
                // ni un pronombre (que, eso, esto).
                if !Self::is_hacer_time_noun(&after_de)
                    && !matches!(after_de.as_str(), "que" | "eso" | "esto" | "ello")
                {
                    return true;
                }
            }
        }

        false
    }

    /// Sustantivos temporales que aparecen con hacer impersonal.
    fn is_hacer_time_noun(word: &str) -> bool {
        matches!(
            word,
            "segundo"
                | "segundos"
                | "minuto"
                | "minutos"
                | "hora"
                | "horas"
                | "día"
                | "días"
                | "semana"
                | "semanas"
                | "mes"
                | "meses"
                | "año"
                | "años"
                | "rato"
                | "momento"
                | "instante"
                | "tiempo"
                | "siglo"
                | "siglos"
                | "década"
                | "décadas"
        )
    }

    /// Palabras que pueden aparecer entre hacer y el sustantivo temporal.
    fn is_temporal_filler(word: &str) -> bool {
        matches!(
            word,
            // Números
            "un" | "uno" | "una" | "dos" | "tres" | "cuatro" | "cinco"
                | "seis" | "siete" | "ocho" | "nueve" | "diez"
                | "once" | "doce" | "trece" | "catorce" | "quince"
                | "dieciséis" | "diecisiete" | "dieciocho" | "diecinueve"
                | "veinte" | "veintiún" | "veintiuno" | "veintiuna"
                | "veintidós" | "veintitrés" | "veinticuatro" | "veinticinco"
                | "veintiséis" | "veintisiete" | "veintiocho" | "veintinueve"
                | "treinta" | "cuarenta" | "cincuenta" | "sesenta"
                | "setenta" | "ochenta" | "noventa" | "cien" | "ciento"
                | "doscientos" | "doscientas" | "trescientos" | "trescientas"
                | "mil"
                // Cuantificadores
                | "mucho" | "mucha" | "muchos" | "muchas"
                | "poco" | "poca" | "pocos" | "pocas"
                | "bastante" | "bastantes"
                | "varios" | "varias"
                | "tantos" | "tantas"
                | "unos" | "unas"
                | "más" | "menos"
                // Adverbios
                | "ya" | "casi" | "apenas" | "aproximadamente"
                // Preposición "de" (para "más de tres años")
                | "de"
        )
    }

    // ======================================================================
    // Utilidades compartidas
    // ======================================================================

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

    #[test]
    fn test_deben_haber_muchas_razones() {
        let tokens = tokenize("deben haber muchas razones");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "debe");
    }

    #[test]
    fn test_pueden_haber_problemas() {
        let tokens = tokenize("pueden haber muchos problemas");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "puede");
    }

    #[test]
    fn test_van_a_haber_cambios() {
        let tokens = tokenize("van a haber muchos cambios");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "va");
    }

    #[test]
    fn test_deben_de_haber_muchas_razones() {
        let tokens = tokenize("deben de haber muchas razones");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "debe");
    }

    #[test]
    fn test_tienen_que_haber_muchas_razones() {
        let tokens = tokenize("tienen que haber muchas razones");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tiene");
    }

    #[test]
    fn test_suelen_haber_muchos_problemas() {
        let tokens = tokenize("suelen haber muchos problemas");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "suele");
    }

    #[test]
    fn test_deben_haber_llegado_no_correction() {
        let tokens = tokenize("deben haber llegado");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debe corregir cuando 'haber' funciona como auxiliar: {:?}",
            corrections
        );
    }

    #[test]
    fn test_van_a_haber_llegado_no_correction() {
        let tokens = tokenize("van a haber llegado");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debe corregir periprasis cuando sigue participio: {:?}",
            corrections
        );
    }

    #[test]
    fn test_deben_de_haber_llegado_no_correction() {
        let tokens = tokenize("deben de haber llegado");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debe corregir cuando 'haber' es auxiliar en 'deben de haber llegado': {:?}",
            corrections
        );
    }

    #[test]
    fn test_tienen_que_haber_llegado_no_correction() {
        let tokens = tokenize("tienen que haber llegado");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debe corregir cuando 'haber' es auxiliar en 'tienen que haber llegado': {:?}",
            corrections
        );
    }

    // ==========================================================================
    // Hacer temporal impersonal: casos positivos
    // ==========================================================================

    #[test]
    fn test_hacen_tres_años() {
        let tokens = tokenize("hacen tres años que no nos vemos");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hace");
    }

    #[test]
    fn test_hacian_muchos_dias() {
        let tokens = tokenize("hacían muchos días que no llovía");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hacía");
    }

    #[test]
    fn test_hacen_mucho_tiempo() {
        let tokens = tokenize("hacen mucho tiempo de eso");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hace");
    }

    #[test]
    fn test_hacen_ya_dos_semanas() {
        let tokens = tokenize("hacen ya dos semanas");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hace");
    }

    #[test]
    fn test_haran_dos_meses() {
        let tokens = tokenize("harán dos meses que se fue");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hará");
    }

    #[test]
    fn test_hicieron_tres_años() {
        let tokens = tokenize("hicieron tres años en mayo");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hizo");
    }

    #[test]
    fn test_hacen_capitalization() {
        let tokens = tokenize("Hacen tres años");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "Hace");
    }

    #[test]
    fn test_hacen_numeric_token() {
        // "hacen 3 años" con token numérico
        let tokens = tokenize("hacen 3 años");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hace");
    }

    #[test]
    fn test_hacen_mas_de_tres_años() {
        let tokens = tokenize("hacen más de tres años");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hace");
    }

    // ==========================================================================
    // Hacer temporal: falsos positivos (NO corregir)
    // ==========================================================================

    #[test]
    fn test_hacen_no_temporal() {
        // "hacen falta" → no es temporal
        let tokens = tokenize("hacen falta recursos");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería corregir 'hacen falta': {:?}",
            corrections
        );
    }

    #[test]
    fn test_hacen_deporte() {
        // "hacen deporte" → transitivo, no impersonal
        let tokens = tokenize("hacen deporte cada día");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería corregir 'hacen deporte': {:?}",
            corrections
        );
    }

    #[test]
    fn test_hacen_tres_horas_de_deberes() {
        // "hacen tres horas de deberes" → objeto léxico, no impersonal
        let tokens = tokenize("hacen tres horas de deberes");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería corregir objeto léxico 'tres horas de deberes': {:?}",
            corrections
        );
    }

    #[test]
    fn test_hicieron_tres_horas_de_cola() {
        // "hicieron tres horas de cola" → objeto léxico
        let tokens = tokenize("hicieron tres horas de cola");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería corregir objeto léxico 'tres horas de cola': {:?}",
            corrections
        );
    }

    #[test]
    fn test_hacen_con_sujeto_plural() {
        // "ellos hacen dos horas" → sujeto explícito plural, no impersonal
        let tokens = tokenize("ellos hacen dos horas");
        let corrections = ImpersonalAnalyzer::analyze(&tokens);
        assert!(
            corrections.is_empty(),
            "No debería corregir con sujeto explícito 'ellos': {:?}",
            corrections
        );
    }
}
