//! Analisis de concordancia de pronombres relativos
//!
//! Detecta errores de concordancia entre el antecedente y el verbo en oraciones de relativo.
//! Ejemplo: "la persona que vinieron" -> "la persona que vino"
//!          "los ninos que llego" -> "los ninos que llegaron"

use crate::dictionary::{Number, WordCategory};
use crate::grammar::tokenizer::{Token, TokenType};

/// Corrección de concordancia de relativos
#[derive(Debug, Clone)]
pub struct RelativeCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub message: String,
}

/// Tiempo verbal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum Tense {
    Present,
    Preterite,
    Imperfect,
    Future,
}

/// Analizador de concordancia de relativos
pub struct RelativeAnalyzer;

impl RelativeAnalyzer {
    /// Analiza tokens buscando errores de concordancia en oraciones de relativo
    pub fn analyze(tokens: &[Token]) -> Vec<RelativeCorrection> {
        let mut corrections = Vec::new();

        // Obtener solo tokens de palabras con sus índices originales
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Buscar patrón: sustantivo + "que" + verbo
        // También maneja: sustantivo1 + "de" + sustantivo2 + "que" + verbo
        for i in 0..word_tokens.len().saturating_sub(2) {
            let (_, potential_antecedent) = word_tokens[i];
            let (_, relative) = word_tokens[i + 1];
            let (verb_idx, verb) = word_tokens[i + 2];

            // Verificar si es un sustantivo + "que" + verbo
            if Self::is_noun(potential_antecedent) && Self::is_relative_pronoun(&relative.text) {
                // Excluir tiempos compuestos: "que han permitido", "que ha hecho"
                // El auxiliar "haber" no debe analizarse para concordancia de relativos
                // porque la concordancia ya está determinada por el sujeto, no el antecedente
                let verb_lower = verb.effective_text().to_lowercase();
                if Self::is_haber_auxiliary(&verb_lower) {
                    continue;
                }
                // Verificar que no haya puntuación de fin de oración entre antecedente y relativo
                let (ant_idx, _) = word_tokens[i];
                let (rel_idx_check, _) = word_tokens[i + 1];
                if Self::has_sentence_boundary(tokens, ant_idx, rel_idx_check) {
                    continue;
                }

                // Verificar si después del verbo hay un sujeto propio (det/poss + noun)
                // Ejemplo: "las necesidades que tiene nuestra población"
                // En este caso, "población" es el sujeto de "tiene", no "necesidades"
                if Self::has_own_subject_after_verb(&word_tokens, i + 2) {
                    continue;
                }

                // Verbos copulativos (ser/estar) con predicativo plural:
                // Ejemplo: "la mortalidad, que son muertes causadas..."
                // La concordancia puede ser con el predicativo, no el antecedente
                if Self::is_copulative_with_plural_predicate(&word_tokens, i + 2) {
                    continue;
                }

                // Verbo + participio que concuerda con el antecedente:
                // Ejemplo: "las misiones que tiene previstas" - "misiones" es objeto directo
                // El sujeto de "tiene" es implícito y diferente del antecedente
                if Self::is_verb_with_agreeing_participle(&word_tokens, i + 2, potential_antecedent) {
                    continue;
                }

                // Buscar si hay un patrón "noun1 de [adj/num]* noun2 que verb"
                // En ese caso, el verdadero antecedente es noun1, no noun2
                // Ejemplos:
                //   "marcos de referencia que" → antecedente = "marcos"
                //   "acelerón de dos décimas que" → antecedente = "acelerón"
                //   "niveles de energía solar que" → antecedente = "niveles"
                let antecedent = Self::find_true_antecedent(&word_tokens, i, potential_antecedent, tokens);

                if let Some(correction) = Self::check_verb_agreement(
                    verb_idx,
                    antecedent,
                    verb,
                ) {
                    corrections.push(correction);
                }
            }
        }

        // Buscar patrón: sustantivo + "quien"/"quienes" (concordancia del relativo)
        for i in 0..word_tokens.len().saturating_sub(1) {
            let (_, antecedent) = word_tokens[i];
            let (rel_idx, relative) = word_tokens[i + 1];

            if Self::is_noun(antecedent) {
                // Excluir locuciones prepositivas como "al final quienes", "por fin quienes"
                // En estos casos, "quienes" es un relativo libre, no refiere al sustantivo anterior
                if i > 0 {
                    let (_, prev_word) = word_tokens[i - 1];
                    let prev_lower = prev_word.effective_text().to_lowercase();
                    // Si está precedido por "al", "por", "en", etc., probablemente es locución
                    if matches!(prev_lower.as_str(), "al" | "del" | "por" | "en" | "con" | "sin") {
                        continue;
                    }
                }

                if let Some(correction) = Self::check_quien_agreement(
                    rel_idx,
                    antecedent,
                    relative,
                ) {
                    corrections.push(correction);
                }
            }
        }

        corrections
    }

    /// Verifica si el token es un sustantivo
    fn is_noun(token: &Token) -> bool {
        if let Some(ref info) = token.word_info {
            info.category == WordCategory::Sustantivo
        } else {
            false
        }
    }

    /// Busca el verdadero antecedente en patrones "noun1 de [adj/num]* noun2 que verb"
    /// Retorna noun1 si se encuentra el patrón, o el potential_antecedent original
    ///
    /// IMPORTANTE: Si noun2 tiene un artículo definido (el/la/los/las), probablemente
    /// es el sujeto real del relativo, no noun1. Ejemplo:
    /// - "marcos de referencia que sirven" → antecedente = marcos (sin artículo)
    /// - "actualización de los umbrales que determinan" → antecedente = umbrales (con artículo)
    ///
    /// TAMBIÉN: Si hay una coma antes de noun2, es un apositivo que reinicia la referencia.
    /// - "millones de euros, cifra que llega" → antecedente = cifra (la coma indica apositivo)
    fn find_true_antecedent<'a>(
        word_tokens: &[(usize, &'a Token)],
        noun2_pos: usize,
        potential_antecedent: &'a Token,
        all_tokens: &[Token],
    ) -> &'a Token {
        // Verificar si hay una coma justo antes de noun2 (indica apositivo)
        // En ese caso, noun2 es el verdadero antecedente
        if noun2_pos > 0 {
            let (noun2_idx, _) = word_tokens[noun2_pos];
            // Buscar coma entre la palabra anterior y noun2
            if noun2_idx > 0 {
                for idx in (word_tokens[noun2_pos - 1].0 + 1)..noun2_idx {
                    if let Some(token) = all_tokens.get(idx) {
                        if token.token_type == TokenType::Punctuation && token.text == "," {
                            return potential_antecedent; // Coma indica apositivo, mantener noun2
                        }
                    }
                }
            }
        }

        // Si hay un artículo (definido o indefinido) justo antes de noun2, probablemente noun2 es el sujeto real
        // "de los umbrales que determinan" → umbrales es el sujeto
        // "un escenario que contrarresta" → escenario es el sujeto (no buscar más atrás)
        if noun2_pos > 0 {
            let (_, prev_token) = word_tokens[noun2_pos - 1];
            let prev_lower = prev_token.effective_text().to_lowercase();
            if matches!(prev_lower.as_str(), "el" | "la" | "los" | "las" | "un" | "una" | "unos" | "unas") {
                return potential_antecedent; // Mantener noun2 como antecedente
            }
        }

        // Buscar hacia atrás desde noun2_pos para encontrar "de"
        // El patrón puede ser: noun1 de noun2 que verb
        //                   o: noun1 de adj noun2 que verb
        //                   o: noun1 de adj adj noun2 que verb
        // Máximo 4 posiciones hacia atrás (noun1 de adj adj)
        let max_lookback = 4.min(noun2_pos);

        // Verificar si hay puntuación de separación (: ; --) entre noun2 y posiciones anteriores
        // En "mortalidad: extrínseca, que incluye", no buscar más allá de ":"
        let (noun2_idx, _) = word_tokens[noun2_pos];
        let boundary_limit = if noun2_pos > 0 {
            // Buscar si hay : o ; entre las palabras anteriores
            let mut limit = 0usize;
            for check_offset in 1..=max_lookback.min(noun2_pos) {
                let check_pos = noun2_pos - check_offset;
                let (check_idx, _) = word_tokens[check_pos];
                // Verificar puntuación entre check_idx y la siguiente palabra
                for idx in check_idx..noun2_idx {
                    if let Some(tok) = all_tokens.get(idx) {
                        if tok.token_type == TokenType::Punctuation
                            && (tok.text == ":" || tok.text == ";")
                        {
                            limit = check_offset;
                            break;
                        }
                    }
                }
                if limit > 0 {
                    break;
                }
            }
            if limit > 0 { limit } else { max_lookback }
        } else {
            max_lookback
        };

        // Si hay una conjunción "y/e" inmediatamente antes de noun2, es una frase nominal compuesta
        // "capó y techo que generan" - buscar más atrás
        let mut coord_offset = 0;
        if noun2_pos > 0 {
            let (_, prev_token) = word_tokens[noun2_pos - 1];
            if matches!(prev_token.effective_text().to_lowercase().as_str(), "y" | "e" | "o" | "u") {
                coord_offset = 2; // Saltar conjunción y el sustantivo coordinado
            }
        }

        for offset in 1.max(coord_offset)..=boundary_limit {
            let check_pos = noun2_pos.saturating_sub(offset);
            if check_pos >= noun2_pos {
                continue;
            }
            let (_, token) = word_tokens[check_pos];
            let text_lower = token.effective_text().to_lowercase();

            if text_lower == "de" || text_lower == "del" {
                // Encontramos "de", ahora verificar si hay un sustantivo antes
                if check_pos > 0 {
                    let (_, maybe_noun1) = word_tokens[check_pos - 1];
                    if Self::is_noun(maybe_noun1) {
                        // Excepción: sustantivos colectivos/cuantitativos
                        // En "cantidad de mujeres que acaban", el verbo concuerda con "mujeres"
                        let noun1_lower = maybe_noun1.effective_text().to_lowercase();
                        if Self::is_collective_noun(&noun1_lower) {
                            return potential_antecedent; // Mantener noun2
                        }
                        return maybe_noun1;
                    }
                }
            }

            // Manejar preposiciones locativas: "paneles en el capó y techo que generan"
            // El antecedente real es "paneles", no "techo"
            if matches!(text_lower.as_str(), "en" | "sobre" | "bajo" | "tras" | "ante" | "con" | "sin" |
                                            "entre" | "hacia" | "desde" | "hasta" | "para" | "por") {
                // Verificar si hay un sustantivo antes de la preposición
                if check_pos > 0 {
                    let (_, maybe_noun1) = word_tokens[check_pos - 1];
                    if Self::is_noun(maybe_noun1) {
                        return maybe_noun1;
                    }
                }
            }

            // Si encontramos un sustantivo antes de encontrar "de", detenerse
            // (evita cruzar límites de sintagma)
            // PERO: si estamos en una frase coordinada (coord_offset > 0), no parar
            // en el sustantivo coordinado (que está justo en offset == coord_offset)
            if Self::is_noun(token) && offset > 1 && offset != coord_offset {
                break;
            }
        }

        potential_antecedent
    }

    /// Verifica si la palabra es un sustantivo colectivo o cuantitativo
    /// Estos sustantivos forman grupos pero el verbo concuerda con sus miembros
    fn is_collective_noun(word: &str) -> bool {
        matches!(word,
            "cantidad" | "número" | "mayoría" | "minoría" |
            "parte" | "resto" | "mitad" | "tercio" | "cuarto" |
            "totalidad" | "conjunto" | "grupo" | "serie" |
            "multitud" | "montón" | "infinidad" | "variedad" |
            "porcentaje" | "proporción" | "fracción"
        )
    }

    /// Verifica si la palabra es una forma del auxiliar "haber"
    /// Usado para excluir tiempos compuestos del análisis de relativos
    fn is_haber_auxiliary(word: &str) -> bool {
        matches!(word,
            // Presente indicativo
            "he" | "has" | "ha" | "hemos" | "habéis" | "han" |
            // Imperfecto
            "había" | "habías" | "habíamos" | "habíais" | "habían" |
            // Pretérito indefinido
            "hube" | "hubiste" | "hubo" | "hubimos" | "hubisteis" | "hubieron" |
            // Futuro
            "habré" | "habrás" | "habrá" | "habremos" | "habréis" | "habrán" |
            // Condicional
            "habría" | "habrías" | "habríamos" | "habríais" | "habrían" |
            // Subjuntivo presente
            "haya" | "hayas" | "hayamos" | "hayáis" | "hayan" |
            // Subjuntivo imperfecto
            "hubiera" | "hubieras" | "hubiéramos" | "hubierais" | "hubieran" |
            "hubiese" | "hubieses" | "hubiésemos" | "hubieseis" | "hubiesen"
        )
    }

    /// Verifica si la palabra es un pronombre relativo
    fn is_relative_pronoun(word: &str) -> bool {
        let lower = word.to_lowercase();
        matches!(lower.as_str(), "que" | "quien" | "quienes" | "cual" | "cuales")
    }

    /// Verifica si hay un límite de oración entre dos índices de tokens
    /// (punto, signos de interrogación/exclamación, comillas de cierre)
    fn has_sentence_boundary(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        for idx in (start_idx + 1)..end_idx {
            if idx < tokens.len() {
                let token = &tokens[idx];
                if token.token_type == TokenType::Punctuation {
                    let text = token.text.as_str();
                    // Puntuacion que indica fin de oracion
                    if matches!(text, "." | "!" | "?" | ";" | "\"" | "\u{201D}" | "\u{BB}") {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Verifica si después del verbo hay un sujeto propio (determinante/posesivo + sustantivo)
    /// Ejemplo: "que tiene nuestra población" → "nuestra población" es el sujeto
    /// También detecta nombres propios: "que negocia SoftBank" → SoftBank es el sujeto
    fn has_own_subject_after_verb(word_tokens: &[(usize, &Token)], verb_pos: usize) -> bool {
        // Necesitamos al menos 1 palabra después del verbo
        if verb_pos + 1 >= word_tokens.len() {
            return false;
        }

        let (_, word_after_verb) = word_tokens[verb_pos + 1];
        let original_text = word_after_verb.effective_text();

        // Verificar si es un nombre propio (mayúscula inicial, no al inicio de oración)
        // Ejemplo: "que negocia SoftBank" → SoftBank es el sujeto
        if original_text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            return true;
        }

        // Para determiner + noun patterns, necesitamos 2 palabras después del verbo
        if verb_pos + 2 >= word_tokens.len() {
            return false;
        }

        let word_lower = original_text.to_lowercase();

        // Verificar si es un determinante posesivo o artículo
        let subject_introducers = [
            // Posesivos
            "mi", "tu", "su", "nuestra", "nuestro", "vuestra", "vuestro",
            "mis", "tus", "sus", "nuestras", "nuestros", "vuestras", "vuestros",
            // Artículos
            "el", "la", "los", "las", "un", "una", "unos", "unas",
            // Demostrativos
            "este", "esta", "estos", "estas", "ese", "esa", "esos", "esas",
            "aquel", "aquella", "aquellos", "aquellas",
            // Distributivos e indefinidos
            "cada", "cualquier", "algún", "ningún", "otro", "otra",
        ];

        if subject_introducers.contains(&word_lower.as_str()) {
            // Verificar si la siguiente palabra es un sustantivo
            let (_, potential_subject) = word_tokens[verb_pos + 2];
            if Self::is_noun(potential_subject) {
                return true;
            }
        }

        false
    }

    /// Verifica si el verbo es copulativo (ser/estar) y va seguido de un predicativo plural
    /// En construcciones como "X que son Y", donde Y es plural, la concordancia puede ser
    /// con el predicativo en lugar del antecedente.
    /// Ejemplo: "la causa, que son las lluvias" - válido aunque "causa" es singular
    fn is_copulative_with_plural_predicate(word_tokens: &[(usize, &Token)], verb_pos: usize) -> bool {
        if verb_pos >= word_tokens.len() {
            return false;
        }

        let (_, verb) = word_tokens[verb_pos];
        let verb_lower = verb.effective_text().to_lowercase();

        // Formas del verbo "ser" en plural (3ª persona)
        let copulative_plural = ["son", "eran", "fueron", "serán", "serían", "sean", "fueran", "fuesen"];

        if !copulative_plural.contains(&verb_lower.as_str()) {
            return false;
        }

        // Verificar si después del verbo hay un sustantivo (el predicativo)
        if verb_pos + 1 < word_tokens.len() {
            let (_, next_word) = word_tokens[verb_pos + 1];
            if Self::is_noun(next_word) {
                // Verificar si el sustantivo es plural
                if let Some(ref info) = next_word.word_info {
                    if info.number == Number::Plural {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Verifica si después del verbo hay un participio que concuerda con el antecedente
    /// En construcciones como "las misiones que tiene previstas", el antecedente es el
    /// objeto directo del verbo, no el sujeto. El sujeto es implícito.
    fn is_verb_with_agreeing_participle(
        word_tokens: &[(usize, &Token)],
        verb_pos: usize,
        antecedent: &Token,
    ) -> bool {
        if verb_pos + 1 >= word_tokens.len() {
            return false;
        }

        let (_, word_after_verb) = word_tokens[verb_pos + 1];
        let word_lower = word_after_verb.effective_text().to_lowercase();

        // Verificar si parece un participio (termina en -ado, -ido, -ada, -ida, etc.)
        let is_participle = word_lower.ends_with("ado")
            || word_lower.ends_with("ado")
            || word_lower.ends_with("ados")
            || word_lower.ends_with("ada")
            || word_lower.ends_with("adas")
            || word_lower.ends_with("ido")
            || word_lower.ends_with("idos")
            || word_lower.ends_with("ida")
            || word_lower.ends_with("idas")
            // Participios irregulares comunes
            || matches!(word_lower.as_str(),
                "previsto" | "previstos" | "prevista" | "previstas" |
                "hecho" | "hechos" | "hecha" | "hechas" |
                "dicho" | "dichos" | "dicha" | "dichas" |
                "escrito" | "escritos" | "escrita" | "escritas" |
                "visto" | "vistos" | "vista" | "vistas" |
                "puesto" | "puestos" | "puesta" | "puestas" |
                "abierto" | "abiertos" | "abierta" | "abiertas" |
                "cubierto" | "cubiertos" | "cubierta" | "cubiertas" |
                "muerto" | "muertos" | "muerta" | "muertas" |
                "vuelto" | "vueltos" | "vuelta" | "vueltas" |
                "roto" | "rotos" | "rota" | "rotas"
            );

        if !is_participle {
            return false;
        }

        // Verificar si el participio concuerda en número con el antecedente
        if let (Some(ant_info), Some(part_info)) = (&antecedent.word_info, &word_after_verb.word_info) {
            // Si ambos tienen el mismo número, probablemente el antecedente es el OD
            if ant_info.number == part_info.number && ant_info.number != Number::None {
                return true;
            }
        }

        // Si no tenemos info del diccionario, inferir del participio
        if let Some(ant_info) = &antecedent.word_info {
            let participle_plural = word_lower.ends_with('s');
            let antecedent_plural = ant_info.number == Number::Plural;
            if participle_plural == antecedent_plural {
                return true;
            }
        }

        false
    }

    /// Obtiene el número gramatical del antecedente
    fn get_antecedent_number(token: &Token) -> Option<Number> {
        token.word_info.as_ref().map(|info| info.number)
    }

    /// Verifica concordancia del verbo con el antecedente
    fn check_verb_agreement(
        verb_index: usize,
        antecedent: &Token,
        verb: &Token,
    ) -> Option<RelativeCorrection> {
        let antecedent_number = Self::get_antecedent_number(antecedent)?;

        // Solo procesar si el antecedente tiene número definido
        if antecedent_number == Number::None {
            return None;
        }

        // Excluir artículos y otras palabras que no son verbos
        // "las decisiones que una IA toma" - "una" no es verbo
        let non_verbs = [
            "un", "una", "unos", "unas", "el", "la", "los", "las",
            "mi", "tu", "su", "mis", "tus", "sus",
            "este", "esta", "estos", "estas", "ese", "esa", "esos", "esas",
            "aquel", "aquella", "aquellos", "aquellas",
            "nuestro", "nuestra", "nuestros", "nuestras",
            "vuestro", "vuestra", "vuestros", "vuestras",
            "cada", "todo", "toda", "todos", "todas",
            "otro", "otra", "otros", "otras",
            "mucho", "mucha", "muchos", "muchas",
            "poco", "poca", "pocos", "pocas",
            "algún", "alguno", "alguna", "algunos", "algunas",
            "ningún", "ninguno", "ninguna", "ningunos", "ningunas",
            "cualquier", "cualquiera", "cualesquiera",
        ];
        let verb_lower = verb.effective_text().to_lowercase();
        if non_verbs.contains(&verb_lower.as_str()) {
            return None;
        }

        // Excluir palabras que no son verbos según el diccionario
        // (adverbios, preposiciones, conjunciones, sustantivos, adjetivos)
        if let Some(ref info) = verb.word_info {
            if !matches!(info.category, WordCategory::Verbo) {
                return None;
            }
        }

        // Excluir palabras que típicamente forman relativos de objeto (no sujeto)
        // En "los ratos que estaba", "ratos" es objeto, no sujeto del verbo
        // Estos sustantivos de tiempo/frecuencia típicamente no son sujeto del verbo subordinado
        let object_relative_nouns = [
            "ratos", "rato", "momento", "momentos", "tiempo", "tiempos",
            "día", "días", "vez", "veces", "hora", "horas",
            "minuto", "minutos", "segundo", "segundos",
            "año", "años", "mes", "meses", "semana", "semanas",
            "ocasión", "ocasiones", "instante", "instantes",
        ];

        let antecedent_lower = antecedent.effective_text().to_lowercase();
        if object_relative_nouns.contains(&antecedent_lower.as_str()) {
            return None;
        }

        // También excluir sustantivos que típicamente son objetos del verbo subordinado
        // "los agravios que pensaba deshacer" - "agravios" es objeto de "deshacer", no sujeto de "pensaba"
        // NOTA: No incluir "problema/problemas" etc. porque SÍ pueden ser sujetos legítimos
        let object_nouns = [
            "agravio", "agravios", "tuerto", "tuertos",
            "favor", "favores", "daño", "daños",
        ];

        if object_nouns.contains(&antecedent_lower.as_str()) {
            return None;
        }

        let verb_lower = verb.effective_text().to_lowercase();

        // Obtener información del verbo incluyendo tiempo
        let (verb_number, infinitive, tense) = Self::get_verb_info_with_tense(&verb_lower)?;

        // Para verbos transitivos comunes, el antecedente puede ser objeto (no sujeto)
        // "la película que estrenaron" - "ellos estrenaron la película" (correcto)
        // En estos casos, no corregir si el antecedente es singular y el verbo plural
        let transitive_verbs = [
            "estrenar", "comprar", "vender", "hacer", "escribir", "leer", "ver",
            "publicar", "producir", "crear", "diseñar", "construir", "fabricar",
            "enviar", "recibir", "entregar", "preparar", "cocinar", "servir",
            "pintar", "dibujar", "grabar", "filmar", "editar", "cortar",
            "abrir", "cerrar", "romper", "arreglar", "reparar",
        ];

        if transitive_verbs.contains(&infinitive.as_str()) {
            // En oraciones de relativo con verbo transitivo, el antecedente puede ser objeto:
            // "la película que estrenaron" - antecedente singular, verbo plural (sujeto: ellos)
            // "los libros que leíste" - antecedente plural, verbo singular (sujeto: tú)
            // En ambos casos, el antecedente no es el sujeto del verbo, así que no corregir
            if antecedent_number != verb_number {
                return None;
            }
        }

        // Verificar si hay discordancia
        if antecedent_number != verb_number {
            // Generar la forma correcta del verbo en el mismo tiempo
            let correct_form = Self::get_correct_verb_form_with_tense(&infinitive, antecedent_number, tense)?;

            if correct_form.to_lowercase() != verb_lower {
                return Some(RelativeCorrection {
                    token_index: verb_index,
                    original: verb.text.clone(),
                    suggestion: correct_form,
                    message: format!(
                        "Concordancia relativo: el verbo '{}' debe concordar con '{}' ({})",
                        verb.text,
                        antecedent.text,
                        if antecedent_number == Number::Singular { "singular" } else { "plural" }
                    ),
                });
            }
        }

        None
    }

    /// Verifica concordancia de "quien/quienes" con el antecedente
    fn check_quien_agreement(
        rel_index: usize,
        antecedent: &Token,
        relative: &Token,
    ) -> Option<RelativeCorrection> {
        let rel_lower = relative.effective_text().to_lowercase();

        // Solo verificar "quien" y "quienes"
        if rel_lower != "quien" && rel_lower != "quienes" {
            return None;
        }

        let antecedent_number = Self::get_antecedent_number(antecedent)?;

        if antecedent_number == Number::None {
            return None;
        }

        let rel_is_singular = rel_lower == "quien";
        let antecedent_is_singular = antecedent_number == Number::Singular;

        if rel_is_singular != antecedent_is_singular {
            let correct = if antecedent_is_singular { "quien" } else { "quienes" };

            // Preservar mayúsculas
            let suggestion = if relative.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                let mut chars = correct.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => correct.to_string(),
                }
            } else {
                correct.to_string()
            };

            return Some(RelativeCorrection {
                token_index: rel_index,
                original: relative.text.clone(),
                suggestion,
                message: format!(
                    "Concordancia: '{}' debe ser '{}' para concordar con '{}'",
                    relative.text, correct, antecedent.text
                ),
            });
        }

        None
    }

    /// Obtiene información del verbo (número, infinitivo, tiempo)
    /// Retorna (número, infinitivo, tiempo)
    fn get_verb_info_with_tense(verb: &str) -> Option<(Number, String, Tense)> {
        // Verbos irregulares comunes - formas de tercera persona

        // ser - presente
        match verb {
            "es" => return Some((Number::Singular, "ser".to_string(), Tense::Present)),
            "son" => return Some((Number::Plural, "ser".to_string(), Tense::Present)),
            _ => {}
        }
        // ser - pretérito
        match verb {
            "fue" => return Some((Number::Singular, "ser".to_string(), Tense::Preterite)),
            "fueron" => return Some((Number::Plural, "ser".to_string(), Tense::Preterite)),
            _ => {}
        }
        // ser - imperfecto
        match verb {
            "era" => return Some((Number::Singular, "ser".to_string(), Tense::Imperfect)),
            "eran" => return Some((Number::Plural, "ser".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // estar - presente
        match verb {
            "está" | "esta" => return Some((Number::Singular, "estar".to_string(), Tense::Present)),
            "están" | "estan" => return Some((Number::Plural, "estar".to_string(), Tense::Present)),
            _ => {}
        }
        // estar - pretérito
        match verb {
            "estuvo" => return Some((Number::Singular, "estar".to_string(), Tense::Preterite)),
            "estuvieron" => return Some((Number::Plural, "estar".to_string(), Tense::Preterite)),
            _ => {}
        }
        // estar - imperfecto
        match verb {
            "estaba" => return Some((Number::Singular, "estar".to_string(), Tense::Imperfect)),
            "estaban" => return Some((Number::Plural, "estar".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // tener - presente
        match verb {
            "tiene" => return Some((Number::Singular, "tener".to_string(), Tense::Present)),
            "tienen" => return Some((Number::Plural, "tener".to_string(), Tense::Present)),
            _ => {}
        }
        // tener - pretérito
        match verb {
            "tuvo" => return Some((Number::Singular, "tener".to_string(), Tense::Preterite)),
            "tuvieron" => return Some((Number::Plural, "tener".to_string(), Tense::Preterite)),
            _ => {}
        }
        // tener - imperfecto
        match verb {
            "tenía" | "tenia" => return Some((Number::Singular, "tener".to_string(), Tense::Imperfect)),
            "tenían" | "tenian" => return Some((Number::Plural, "tener".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // ir - presente
        match verb {
            "va" => return Some((Number::Singular, "ir".to_string(), Tense::Present)),
            "van" => return Some((Number::Plural, "ir".to_string(), Tense::Present)),
            _ => {}
        }
        // ir - pretérito (mismo que ser)
        match verb {
            "fue" => return Some((Number::Singular, "ir".to_string(), Tense::Preterite)),
            "fueron" => return Some((Number::Plural, "ir".to_string(), Tense::Preterite)),
            _ => {}
        }
        // ir - imperfecto
        match verb {
            "iba" => return Some((Number::Singular, "ir".to_string(), Tense::Imperfect)),
            "iban" => return Some((Number::Plural, "ir".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // hacer - presente
        match verb {
            "hace" => return Some((Number::Singular, "hacer".to_string(), Tense::Present)),
            "hacen" => return Some((Number::Plural, "hacer".to_string(), Tense::Present)),
            _ => {}
        }
        // hacer - pretérito
        match verb {
            "hizo" => return Some((Number::Singular, "hacer".to_string(), Tense::Preterite)),
            "hicieron" => return Some((Number::Plural, "hacer".to_string(), Tense::Preterite)),
            _ => {}
        }
        // hacer - imperfecto
        match verb {
            "hacía" | "hacia" => return Some((Number::Singular, "hacer".to_string(), Tense::Imperfect)),
            "hacían" | "hacian" => return Some((Number::Plural, "hacer".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // venir - presente
        match verb {
            "viene" => return Some((Number::Singular, "venir".to_string(), Tense::Present)),
            "vienen" => return Some((Number::Plural, "venir".to_string(), Tense::Present)),
            _ => {}
        }
        // venir - pretérito
        match verb {
            "vino" => return Some((Number::Singular, "venir".to_string(), Tense::Preterite)),
            "vinieron" => return Some((Number::Plural, "venir".to_string(), Tense::Preterite)),
            _ => {}
        }
        // venir - imperfecto
        match verb {
            "venía" | "venia" => return Some((Number::Singular, "venir".to_string(), Tense::Imperfect)),
            "venían" | "venian" => return Some((Number::Plural, "venir".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // poder - presente
        match verb {
            "puede" => return Some((Number::Singular, "poder".to_string(), Tense::Present)),
            "pueden" => return Some((Number::Plural, "poder".to_string(), Tense::Present)),
            _ => {}
        }
        // poder - pretérito
        match verb {
            "pudo" => return Some((Number::Singular, "poder".to_string(), Tense::Preterite)),
            "pudieron" => return Some((Number::Plural, "poder".to_string(), Tense::Preterite)),
            _ => {}
        }
        // poder - imperfecto
        match verb {
            "podía" | "podia" => return Some((Number::Singular, "poder".to_string(), Tense::Imperfect)),
            "podían" | "podian" => return Some((Number::Plural, "poder".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // querer - presente
        match verb {
            "quiere" => return Some((Number::Singular, "querer".to_string(), Tense::Present)),
            "quieren" => return Some((Number::Plural, "querer".to_string(), Tense::Present)),
            _ => {}
        }
        // querer - pretérito
        match verb {
            "quiso" => return Some((Number::Singular, "querer".to_string(), Tense::Preterite)),
            "quisieron" => return Some((Number::Plural, "querer".to_string(), Tense::Preterite)),
            _ => {}
        }

        // decir - presente
        match verb {
            "dice" => return Some((Number::Singular, "decir".to_string(), Tense::Present)),
            "dicen" => return Some((Number::Plural, "decir".to_string(), Tense::Present)),
            _ => {}
        }
        // decir - pretérito
        match verb {
            "dijo" => return Some((Number::Singular, "decir".to_string(), Tense::Preterite)),
            "dijeron" => return Some((Number::Plural, "decir".to_string(), Tense::Preterite)),
            _ => {}
        }

        // saber - presente
        match verb {
            "sabe" => return Some((Number::Singular, "saber".to_string(), Tense::Present)),
            "saben" => return Some((Number::Plural, "saber".to_string(), Tense::Present)),
            _ => {}
        }
        // saber - pretérito
        match verb {
            "supo" => return Some((Number::Singular, "saber".to_string(), Tense::Preterite)),
            "supieron" => return Some((Number::Plural, "saber".to_string(), Tense::Preterite)),
            _ => {}
        }

        // ver - presente
        match verb {
            "ve" => return Some((Number::Singular, "ver".to_string(), Tense::Present)),
            "ven" => return Some((Number::Plural, "ver".to_string(), Tense::Present)),
            _ => {}
        }
        // ver - pretérito
        match verb {
            "vio" => return Some((Number::Singular, "ver".to_string(), Tense::Preterite)),
            "vieron" => return Some((Number::Plural, "ver".to_string(), Tense::Preterite)),
            _ => {}
        }
        // ver - imperfecto
        match verb {
            "veía" | "veia" => return Some((Number::Singular, "ver".to_string(), Tense::Imperfect)),
            "veían" | "veian" => return Some((Number::Plural, "ver".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // dar - presente
        match verb {
            "da" => return Some((Number::Singular, "dar".to_string(), Tense::Present)),
            "dan" => return Some((Number::Plural, "dar".to_string(), Tense::Present)),
            _ => {}
        }
        // dar - pretérito
        match verb {
            "dio" => return Some((Number::Singular, "dar".to_string(), Tense::Preterite)),
            "dieron" => return Some((Number::Plural, "dar".to_string(), Tense::Preterite)),
            _ => {}
        }

        // llegar - presente
        match verb {
            "llega" => return Some((Number::Singular, "llegar".to_string(), Tense::Present)),
            "llegan" => return Some((Number::Plural, "llegar".to_string(), Tense::Present)),
            _ => {}
        }
        // llegar - pretérito
        match verb {
            "llegó" | "llego" => return Some((Number::Singular, "llegar".to_string(), Tense::Preterite)),
            "llegaron" => return Some((Number::Plural, "llegar".to_string(), Tense::Preterite)),
            _ => {}
        }
        // llegar - imperfecto
        match verb {
            "llegaba" => return Some((Number::Singular, "llegar".to_string(), Tense::Imperfect)),
            "llegaban" => return Some((Number::Plural, "llegar".to_string(), Tense::Imperfect)),
            _ => {}
        }

        // Verbos regulares - detectar tiempo y número

        // Pretérito perfecto simple -ar (cantó/cantaron)
        if verb.ends_with("aron") {
            let stem = &verb[..verb.len() - 4];
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}ar", stem), Tense::Preterite));
            }
        }
        if verb.ends_with("ó") && verb.len() > 2 {
            let stem = &verb[..verb.len() - 2]; // quitar la ó
            if !stem.is_empty() && !verb.ends_with("ió") {
                return Some((Number::Singular, format!("{}ar", stem), Tense::Preterite));
            }
        }

        // Pretérito perfecto simple -er/-ir (comió/comieron, vivió/vivieron)
        if verb.ends_with("ieron") {
            let stem = &verb[..verb.len() - 5];
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}ir", stem), Tense::Preterite));
            }
        }
        if verb.ends_with("ió") {
            let stem = &verb[..verb.len() - 2];
            if !stem.is_empty() {
                return Some((Number::Singular, format!("{}ir", stem), Tense::Preterite));
            }
        }

        // Segunda persona singular pretérito -er/-ir (comiste, leíste, viviste)
        // NOTA: Estas formas NO deben analizarse para concordancia de relativos
        // porque el sujeto es "tú", no el antecedente
        if verb.ends_with("iste") || verb.ends_with("íste") {
            // Retornar None para evitar análisis incorrecto
            return None;
        }
        // Segunda persona singular pretérito -ar (cantaste)
        if verb.ends_with("aste") {
            return None;
        }

        // Imperfecto -ar (cantaba/cantaban)
        if verb.ends_with("aban") {
            let stem = &verb[..verb.len() - 4];
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}ar", stem), Tense::Imperfect));
            }
        }
        if verb.ends_with("aba") {
            let stem = &verb[..verb.len() - 3];
            if !stem.is_empty() {
                return Some((Number::Singular, format!("{}ar", stem), Tense::Imperfect));
            }
        }

        // Imperfecto -er/-ir (comía/comían, vivía/vivían)
        if verb.ends_with("ían") || verb.ends_with("ian") {
            let stem = verb.trim_end_matches("ían").trim_end_matches("ian");
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}er", stem), Tense::Imperfect));
            }
        }
        if verb.ends_with("ía") || verb.ends_with("ia") {
            let stem = verb.trim_end_matches("ía").trim_end_matches("ia");
            if !stem.is_empty() {
                return Some((Number::Singular, format!("{}er", stem), Tense::Imperfect));
            }
        }

        // Presente indicativo -ar (canta/cantan)
        if verb.ends_with("an") && !verb.ends_with("ían") && !verb.ends_with("aban") && !verb.ends_with("aron") {
            let stem = &verb[..verb.len() - 2];
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}ar", stem), Tense::Present));
            }
        }
        if verb.ends_with("a") && !verb.ends_with("ía") && !verb.ends_with("aba") && verb.len() > 2 {
            let stem = &verb[..verb.len() - 1];
            if !stem.is_empty() {
                return Some((Number::Singular, format!("{}ar", stem), Tense::Present));
            }
        }

        // Presente indicativo -er/-ir (come/comen, vive/viven)
        if verb.ends_with("en") && !verb.ends_with("ían") && !verb.ends_with("ien") && !verb.ends_with("ieron") {
            let stem = &verb[..verb.len() - 2];
            if !stem.is_empty() {
                return Some((Number::Plural, format!("{}er", stem), Tense::Present));
            }
        }
        // Excluir adverbios terminados en -mente (probablemente, seguramente, etc.)
        if verb.ends_with("mente") {
            return None;
        }
        if verb.ends_with("e") && !verb.ends_with("ía") && verb.len() > 2 {
            let stem = &verb[..verb.len() - 1];
            if !stem.is_empty() {
                return Some((Number::Singular, format!("{}er", stem), Tense::Present));
            }
        }

        None
    }

    /// Genera la forma correcta del verbo para el número y tiempo dados
    fn get_correct_verb_form_with_tense(infinitive: &str, number: Number, tense: Tense) -> Option<String> {
        match tense {
            Tense::Present => Self::get_correct_verb_form(infinitive, number),
            Tense::Preterite => Self::get_correct_verb_form_preterite(infinitive, number),
            Tense::Imperfect => Self::get_correct_verb_form_imperfect(infinitive, number),
            Tense::Future => Self::get_correct_verb_form_future(infinitive, number),
        }
    }

    /// Genera la forma correcta del verbo en pretérito
    fn get_correct_verb_form_preterite(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares en pretérito
        match infinitive {
            "ser" | "ir" => return Some(if number == Number::Singular { "fue" } else { "fueron" }.to_string()),
            "estar" => return Some(if number == Number::Singular { "estuvo" } else { "estuvieron" }.to_string()),
            "tener" => return Some(if number == Number::Singular { "tuvo" } else { "tuvieron" }.to_string()),
            "hacer" => return Some(if number == Number::Singular { "hizo" } else { "hicieron" }.to_string()),
            "venir" => return Some(if number == Number::Singular { "vino" } else { "vinieron" }.to_string()),
            "poder" => return Some(if number == Number::Singular { "pudo" } else { "pudieron" }.to_string()),
            "querer" => return Some(if number == Number::Singular { "quiso" } else { "quisieron" }.to_string()),
            "decir" => return Some(if number == Number::Singular { "dijo" } else { "dijeron" }.to_string()),
            "saber" => return Some(if number == Number::Singular { "supo" } else { "supieron" }.to_string()),
            "ver" => return Some(if number == Number::Singular { "vio" } else { "vieron" }.to_string()),
            "dar" => return Some(if number == Number::Singular { "dio" } else { "dieron" }.to_string()),
            _ => {}
        }

        // Verbos regulares -ar
        if let Some(stem) = infinitive.strip_suffix("ar") {
            return Some(if number == Number::Singular {
                format!("{}ó", stem)
            } else {
                format!("{}aron", stem)
            });
        }

        // Verbos regulares -er/-ir
        if let Some(stem) = infinitive.strip_suffix("er") {
            return Some(if number == Number::Singular {
                format!("{}ió", stem)
            } else {
                format!("{}ieron", stem)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("ir") {
            return Some(if number == Number::Singular {
                format!("{}ió", stem)
            } else {
                format!("{}ieron", stem)
            });
        }

        None
    }

    /// Genera la forma correcta del verbo en imperfecto
    fn get_correct_verb_form_imperfect(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares en imperfecto
        match infinitive {
            "ser" => return Some(if number == Number::Singular { "era" } else { "eran" }.to_string()),
            "ir" => return Some(if number == Number::Singular { "iba" } else { "iban" }.to_string()),
            "ver" => return Some(if number == Number::Singular { "veía" } else { "veían" }.to_string()),
            _ => {}
        }

        // Verbos regulares -ar
        if let Some(stem) = infinitive.strip_suffix("ar") {
            return Some(if number == Number::Singular {
                format!("{}aba", stem)
            } else {
                format!("{}aban", stem)
            });
        }

        // Verbos regulares -er/-ir
        if let Some(stem) = infinitive.strip_suffix("er") {
            return Some(if number == Number::Singular {
                format!("{}ía", stem)
            } else {
                format!("{}ían", stem)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("ir") {
            return Some(if number == Number::Singular {
                format!("{}ía", stem)
            } else {
                format!("{}ían", stem)
            });
        }

        None
    }

    /// Genera la forma correcta del verbo en futuro
    fn get_correct_verb_form_future(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares en futuro
        match infinitive {
            "tener" => return Some(if number == Number::Singular { "tendrá" } else { "tendrán" }.to_string()),
            "poder" => return Some(if number == Number::Singular { "podrá" } else { "podrán" }.to_string()),
            "querer" => return Some(if number == Number::Singular { "querrá" } else { "querrán" }.to_string()),
            "hacer" => return Some(if number == Number::Singular { "hará" } else { "harán" }.to_string()),
            "decir" => return Some(if number == Number::Singular { "dirá" } else { "dirán" }.to_string()),
            "venir" => return Some(if number == Number::Singular { "vendrá" } else { "vendrán" }.to_string()),
            "saber" => return Some(if number == Number::Singular { "sabrá" } else { "sabrán" }.to_string()),
            _ => {}
        }

        // Verbos regulares - futuro usa el infinitivo completo
        Some(if number == Number::Singular {
            format!("{}á", infinitive)
        } else {
            format!("{}án", infinitive)
        })
    }

    /// Genera la forma correcta del verbo para el número dado (presente indicativo)
    fn get_correct_verb_form(infinitive: &str, number: Number) -> Option<String> {
        // Verbos irregulares
        match infinitive {
            "ser" => return Some(if number == Number::Singular { "es" } else { "son" }.to_string()),
            "estar" => return Some(if number == Number::Singular { "está" } else { "están" }.to_string()),
            "tener" => return Some(if number == Number::Singular { "tiene" } else { "tienen" }.to_string()),
            "ir" => return Some(if number == Number::Singular { "va" } else { "van" }.to_string()),
            "hacer" => return Some(if number == Number::Singular { "hace" } else { "hacen" }.to_string()),
            "venir" => return Some(if number == Number::Singular { "viene" } else { "vienen" }.to_string()),
            "poder" => return Some(if number == Number::Singular { "puede" } else { "pueden" }.to_string()),
            "querer" => return Some(if number == Number::Singular { "quiere" } else { "quieren" }.to_string()),
            "decir" => return Some(if number == Number::Singular { "dice" } else { "dicen" }.to_string()),
            "saber" => return Some(if number == Number::Singular { "sabe" } else { "saben" }.to_string()),
            "ver" => return Some(if number == Number::Singular { "ve" } else { "ven" }.to_string()),
            "dar" => return Some(if number == Number::Singular { "da" } else { "dan" }.to_string()),
            "llegar" => return Some(if number == Number::Singular { "llega" } else { "llegan" }.to_string()),
            _ => {}
        }

        // Verbos regulares
        if let Some(stem) = infinitive.strip_suffix("ar") {
            return Some(if number == Number::Singular {
                format!("{}a", stem)
            } else {
                format!("{}an", stem)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("er") {
            return Some(if number == Number::Singular {
                format!("{}e", stem)
            } else {
                format!("{}en", stem)
            });
        }

        if let Some(stem) = infinitive.strip_suffix("ir") {
            return Some(if number == Number::Singular {
                format!("{}e", stem)
            } else {
                format!("{}en", stem)
            });
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::tokenizer::Tokenizer;
    use crate::dictionary::{DictionaryLoader, Trie};

    fn setup_tokens(text: &str) -> Vec<Token> {
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };

        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize(text);

        // Enriquecer tokens con información del diccionario
        for token in &mut tokens {
            if token.token_type == TokenType::Word {
                if let Some(info) = dictionary.get(&token.effective_text().to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        tokens
    }

    #[test]
    fn test_persona_que_vinieron() {
        let tokens = setup_tokens("la persona que vinieron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "vinieron");
        assert_eq!(corrections[0].suggestion, "vino");
    }

    #[test]
    fn test_personas_que_vino() {
        // "vino" es pretérito, la corrección debe ser "vinieron" (pretérito plural)
        let tokens = setup_tokens("las personas que vino");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "vino");
        assert_eq!(corrections[0].suggestion, "vinieron");
    }

    #[test]
    fn test_nino_que_cantan() {
        let tokens = setup_tokens("el niño que cantan");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "cantan");
        assert_eq!(corrections[0].suggestion, "canta");
    }

    #[test]
    fn test_ninos_que_canta() {
        let tokens = setup_tokens("los niños que canta");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "canta");
        assert_eq!(corrections[0].suggestion, "cantan");
    }

    #[test]
    fn test_libro_que_fueron() {
        let tokens = setup_tokens("el libro que fueron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "fue");
    }

    #[test]
    fn test_libros_que_fue() {
        let tokens = setup_tokens("los libros que fue");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "fueron");
    }

    #[test]
    fn test_casa_que_tiene_correct() {
        let tokens = setup_tokens("la casa que tiene");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty(), "No debería haber correcciones para concordancia correcta");
    }

    #[test]
    fn test_casas_que_tienen_correct() {
        let tokens = setup_tokens("las casas que tienen");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert!(corrections.is_empty(), "No debería haber correcciones para concordancia correcta");
    }

    #[test]
    fn test_mujer_que_llegaron() {
        let tokens = setup_tokens("la mujer que llegaron");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "llegó");
    }

    #[test]
    fn test_hombres_que_llegó() {
        // "llegó" es pretérito, la corrección debe ser "llegaron" (pretérito plural)
        let tokens = setup_tokens("los hombres que llegó");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "llegaron");
    }

    #[test]
    fn test_personas_quien() {
        let tokens = setup_tokens("las personas quien");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "quien");
        assert_eq!(corrections[0].suggestion, "quienes");
    }

    #[test]
    fn test_persona_quienes() {
        let tokens = setup_tokens("la persona quienes");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "quienes");
        assert_eq!(corrections[0].suggestion, "quien");
    }

    #[test]
    fn test_persona_quien_correct() {
        let tokens = setup_tokens("la persona quien");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        // Filtrar solo correcciones de quien/quienes
        let quien_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "quien" || c.original == "quienes")
            .collect();
        assert!(quien_corrections.is_empty(), "No debería corregir 'persona quien' que es correcto");
    }

    #[test]
    fn test_problema_que_tienen() {
        let tokens = setup_tokens("el problema que tienen");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tiene");
    }

    #[test]
    fn test_problemas_que_tiene() {
        let tokens = setup_tokens("los problemas que tiene");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tienen");
    }

    #[test]
    fn test_verb_info_irregulars() {
        let info = RelativeAnalyzer::get_verb_info_with_tense("es");
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Singular);
        assert_eq!(inf, "ser");

        let info = RelativeAnalyzer::get_verb_info_with_tense("son");
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Plural);
        assert_eq!(inf, "ser");

        let info = RelativeAnalyzer::get_verb_info_with_tense("tiene");
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Singular);
        assert_eq!(inf, "tener");

        let info = RelativeAnalyzer::get_verb_info_with_tense("tienen");
        assert!(info.is_some());
        let (num, inf, _) = info.unwrap();
        assert_eq!(num, Number::Plural);
        assert_eq!(inf, "tener");
    }

    #[test]
    fn test_get_correct_form() {
        assert_eq!(RelativeAnalyzer::get_correct_verb_form("ser", Number::Singular), Some("es".to_string()));
        assert_eq!(RelativeAnalyzer::get_correct_verb_form("ser", Number::Plural), Some("son".to_string()));
        assert_eq!(RelativeAnalyzer::get_correct_verb_form("cantar", Number::Singular), Some("canta".to_string()));
        assert_eq!(RelativeAnalyzer::get_correct_verb_form("cantar", Number::Plural), Some("cantan".to_string()));
    }

    #[test]
    fn test_sentence_boundary_prevents_false_positive() {
        // "Que vengan" es subjuntivo exhortativo, no relativo de "agresion"
        // El punto y comillas de cierre deben impedir que "agresion" sea antecedente
        let tokens = setup_tokens("no a otra agresion\". \"Que vengan todos");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        // No debe haber correccion de "vengan" a "venga"
        let vengan_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "vengan")
            .collect();
        assert!(vengan_corrections.is_empty(), "No debe corregir 'vengan' cuando hay limite de oracion");
    }

    #[test]
    fn test_noun_de_adj_noun_que_verb_pattern() {
        // En "acelerón de dos décimas que elevó", el antecedente es "acelerón" (singular)
        // No debe sugerir "elevaron" porque "décimas" (plural) no es el sujeto
        let tokens = setup_tokens("un acelerón de dos décimas que elevó el avance");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let elevo_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "elevó")
            .collect();
        assert!(elevo_corrections.is_empty(),
            "No debe corregir 'elevó' - el antecedente es 'acelerón' (singular), no 'décimas'");
    }

    #[test]
    fn test_noun_de_noun_que_verb_pattern() {
        // En "marcos de referencia que sirven", el antecedente es "marcos" (plural)
        // No debe sugerir "sirve" porque "referencia" (singular) no es el sujeto
        let tokens = setup_tokens("los marcos de referencia que sirven de guía");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let sirven_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "sirven")
            .collect();
        assert!(sirven_corrections.is_empty(),
            "No debe corregir 'sirven' - el antecedente es 'marcos' (plural), no 'referencia'");
    }

    #[test]
    fn test_noun_de_article_noun_que_verb_pattern() {
        // En "actualización de los umbrales que determinan", el antecedente es "umbrales" (plural)
        // porque tiene artículo definido "los"
        // No debe sugerir "determina" porque "umbrales" es el verdadero sujeto
        let tokens = setup_tokens("la actualización de los umbrales que determinan el tamaño");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let det_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "determinan")
            .collect();
        assert!(det_corrections.is_empty(),
            "No debe corregir 'determinan' - el antecedente es 'umbrales' (con artículo 'los'), no 'actualización'");
    }

    #[test]
    fn test_indefinite_article_noun_que_verb() {
        // En "un escenario que contrarresta", el antecedente es "escenario" (singular)
        // El artículo indefinido "un" indica inicio de nuevo sintagma nominal
        // No debe buscar más atrás para encontrar un antecedente plural
        let tokens = setup_tokens("con modelos innovadores un escenario que contrarresta los efectos");
        let corrections = RelativeAnalyzer::analyze(&tokens);
        let contra_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.original == "contrarresta")
            .collect();
        assert!(contra_corrections.is_empty(),
            "No debe corregir 'contrarresta' - el antecedente es 'escenario' (con artículo 'un'), no 'modelos'");
    }
}
