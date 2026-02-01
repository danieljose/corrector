//! Analizador gramatical

use crate::dictionary::{Gender, Number, Trie, WordCategory};
use crate::languages::Language;

use super::rules::{GrammarRule, RuleAction, RuleCondition, RuleEngine, TokenPattern};
use super::tokenizer::{Token, TokenType};

/// Corrección gramatical sugerida
#[derive(Debug, Clone)]
pub struct GrammarCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub rule_id: String,
    pub message: String,
}

/// Analizador gramatical
pub struct GrammarAnalyzer {
    rule_engine: RuleEngine,
}

impl GrammarAnalyzer {
    pub fn new() -> Self {
        Self {
            rule_engine: RuleEngine::new(),
        }
    }

    pub fn with_rules(rules: Vec<GrammarRule>) -> Self {
        let mut analyzer = Self::new();
        analyzer.rule_engine.add_rules(rules);
        analyzer
    }

    pub fn add_rule(&mut self, rule: GrammarRule) {
        self.rule_engine.add_rule(rule);
    }

    /// Analiza tokens y retorna correcciones gramaticales
    pub fn analyze(
        &self,
        tokens: &mut [Token],
        dictionary: &Trie,
        language: &dyn Language,
    ) -> Vec<GrammarCorrection> {
        // Primero, enriquecer tokens con información del diccionario
        // Usar effective_text() para que las correcciones ortográficas se propaguen
        // Ejemplo: "este cassa" → spelling corrige "cassa"→"casa", grammar debe ver "casa"
        for token in tokens.iter_mut() {
            if token.token_type == TokenType::Word {
                if let Some(info) = dictionary.get(&token.effective_text().to_lowercase()) {
                    token.word_info = Some(info.clone());
                }
            }
        }

        let mut corrections = Vec::new();

        // Analizar reglas habilitadas
        for rule in self.rule_engine.get_enabled_rules() {
            let rule_corrections = self.apply_rule(rule, tokens, dictionary, language);
            corrections.extend(rule_corrections);
        }

        corrections
    }

    fn apply_rule(
        &self,
        rule: &GrammarRule,
        tokens: &[Token],
        dictionary: &Trie,
        language: &dyn Language,
    ) -> Vec<GrammarCorrection> {
        let mut corrections = Vec::new();
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Buscar patrones en secuencias de palabras
        for (window_pos, window) in word_tokens.windows(rule.pattern.len()).enumerate() {
            // Skip if there's sentence-ending punctuation between tokens
            if self.has_sentence_boundary_between(tokens, window) {
                continue;
            }
            // Skip article-noun agreement checks if there's a number between them
            // Example: "los 10 MB" - the article agrees with the quantity, not the singular noun
            if self.has_number_between(tokens, window) {
                continue;
            }
            if self.pattern_matches(&rule.pattern, window) {
                if let Some(correction) =
                    self.check_condition_and_correct(rule, window, &word_tokens, window_pos, tokens, dictionary, language)
                {
                    corrections.push(correction);
                }
            }
        }

        corrections
    }

    /// Checks if there's a sentence/phrase boundary between tokens in a window
    /// Includes sentence-ending punctuation AND commas (which separate list items)
    fn has_sentence_boundary_between(&self, all_tokens: &[Token], window: &[(usize, &Token)]) -> bool {
        if window.len() < 2 {
            return false;
        }
        // Check tokens between the first and last word in the window
        let first_idx = window[0].0;
        let last_idx = window[window.len() - 1].0;

        for i in first_idx..last_idx {
            if all_tokens[i].token_type == TokenType::Punctuation {
                let punct = &all_tokens[i].text;
                // Include comma as it separates list items ("A, B" are separate elements)
                if punct == "." || punct == "!" || punct == "?" || punct == "..." || punct == "," {
                    return true;
                }
            }
        }
        false
    }

    /// Checks if there's a number between tokens in a window
    /// Used to skip article-noun agreement when there's a number in between
    /// Example: "los 10 MB" - "los" agrees with the quantity, not the singular "MB"
    fn has_number_between(&self, all_tokens: &[Token], window: &[(usize, &Token)]) -> bool {
        if window.len() < 2 {
            return false;
        }
        let first_idx = window[0].0;
        let last_idx = window[window.len() - 1].0;

        for i in (first_idx + 1)..last_idx {
            if all_tokens[i].token_type == TokenType::Number {
                return true;
            }
        }
        false
    }

    fn pattern_matches(&self, pattern: &[TokenPattern], window: &[(usize, &Token)]) -> bool {
        if pattern.len() != window.len() {
            return false;
        }

        for (pat, (_, token)) in pattern.iter().zip(window.iter()) {
            let matches = match pat {
                TokenPattern::Category(cat) => {
                    token
                        .word_info
                        .as_ref()
                        .map(|info| info.category == *cat)
                        .unwrap_or(false)
                }
                TokenPattern::Word(word) => token.text.to_lowercase() == word.to_lowercase(),
                TokenPattern::AnyWord => true,
            };

            if !matches {
                return false;
            }
        }

        true
    }

    fn check_condition_and_correct(
        &self,
        rule: &GrammarRule,
        window: &[(usize, &Token)],
        word_tokens: &[(usize, &Token)],
        window_pos: usize,
        tokens: &[Token],
        _dictionary: &Trie,
        language: &dyn Language,
    ) -> Option<GrammarCorrection> {
        match &rule.condition {
            RuleCondition::GenderMismatch => {
                if window.len() >= 2 {
                    let (idx1, token1) = &window[0];
                    let (idx2, token2) = &window[1];

                    if !language.check_gender_agreement(token1, token2) {
                        return self.generate_correction(
                            rule,
                            *idx1,
                            *idx2,
                            token1,
                            token2,
                            language,
                        );
                    }
                }
            }
            RuleCondition::NumberMismatch => {
                if window.len() >= 2 {
                    let (idx1, token1) = &window[0];
                    let (idx2, token2) = &window[1];

                    if !language.check_number_agreement(token1, token2) {
                        return self.generate_correction(
                            rule,
                            *idx1,
                            *idx2,
                            token1,
                            token2,
                            language,
                        );
                    }
                }
            }
            RuleCondition::GenderAndNumberMismatch => {
                if window.len() >= 2 {
                    let (idx1, token1) = &window[0];
                    let (idx2, token2) = &window[1];

                    // Skip if the noun is in a prepositional phrase "de + [adj]* noun"
                    // In "salsa de tomate casera", "casera" agrees with "salsa", not "tomate"
                    // In "cohetes de nueva generación capaces", "capaces" agrees with "cohetes"
                    // In "campus de millones de dólares exclusivo", "exclusivo" agrees with "campus"
                    if window_pos >= 1 {
                        // Search backwards for "de" before the noun, skipping adjectives/articles
                        // Also traverse through nested prepositional phrases
                        let mut search_pos = window_pos as isize - 1;
                        while search_pos >= 0 {
                            let search_token = word_tokens[search_pos as usize].1;
                            let word_lower = search_token.text.to_lowercase();

                            // Found "de/del/con" - check if adjective agrees with noun before preposition
                            if word_lower == "de" || word_lower == "del" || word_lower == "con" {
                                // Search for noun before "de"
                                if search_pos >= 1 {
                                    let noun_before_de = word_tokens[(search_pos - 1) as usize].1;
                                    if let Some(ref info) = noun_before_de.word_info {
                                        if info.category == WordCategory::Sustantivo {
                                            // Check if adjective agrees with this earlier noun
                                            let adj_agrees = language.check_gender_agreement(noun_before_de, token2)
                                                && language.check_number_agreement(noun_before_de, token2);
                                            if adj_agrees {
                                                return None; // Skip - adjective agrees with noun before "de"
                                            }
                                            // Adjective doesn't agree with this noun - continue searching
                                            // backward through nested prepositional phrases
                                            // "campus de millones de dólares" - if adj doesn't match "millones",
                                            // keep looking to find "campus"
                                            search_pos -= 2; // Skip noun and continue
                                            continue;
                                        }
                                    }
                                }
                                break;
                            }

                            // Continue searching if we find adjectives/articles between noun and "de"
                            if let Some(ref info) = search_token.word_info {
                                if info.category == WordCategory::Adjetivo
                                   || info.category == WordCategory::Articulo
                                   || info.category == WordCategory::Determinante {
                                    search_pos -= 1;
                                    continue;
                                }
                            }
                            // Also continue if we find numbers (e.g., "11.000 millones")
                            if search_token.token_type == TokenType::Number {
                                search_pos -= 1;
                                continue;
                            }
                            // Stop at other word types (verbs, etc.)
                            break;
                        }
                    }

                    // Skip compound subjects: "noun1 y noun2 adjective"
                    // In "alienación y soledad modernas", adjective is plural to match compound subject
                    if window_pos >= 2 {
                        let prev_word = &word_tokens[window_pos - 1].1.text.to_lowercase();
                        if prev_word == "y" || prev_word == "e" {
                            // Check if there's a noun before "y"
                            let before_y = word_tokens[window_pos - 2].1;
                            if let Some(ref info) = before_y.word_info {
                                if info.category == WordCategory::Sustantivo {
                                    // Compound subject - adjective should be plural
                                    if let Some(ref adj_info) = token2.word_info {
                                        if adj_info.number == Number::Plural {
                                            return None; // Skip - plural adjective with compound subject is correct
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Skip adverbial mínimo/máximo: "300 pesetas mínimo", "5 personas máximo"
                    // Here mínimo/máximo is used as an invariable adverb meaning "at minimum/maximum"
                    // Pattern: [number] [noun] [mínimo/máximo]
                    {
                        let adj_lower = token2.text.to_lowercase();
                        if matches!(adj_lower.as_str(), "mínimo" | "máximo" | "mínima" | "máxima" |
                                                        "mínimos" | "máximos" | "mínimas" | "máximas") {
                            // Check if there's a number before the noun in the original tokens array
                            // idx1 is the index of the noun in the original tokens array
                            let noun_idx = *idx1;
                            if noun_idx >= 1 {
                                // Look backwards in original tokens for a number (skipping whitespace)
                                for i in (0..noun_idx).rev() {
                                    let t = &tokens[i];
                                    if t.token_type == TokenType::Number {
                                        return None; // Skip - adverbial mínimo/máximo
                                    }
                                    // Stop if we hit another word (not just whitespace)
                                    if t.token_type == TokenType::Word {
                                        break;
                                    }
                                }
                            }
                        }
                    }

                    // Skip time nouns followed by participles: "una semana aparcado", "tres horas sentado"
                    // The participle agrees with an implicit subject, not the time noun
                    {
                        let noun_lower = token1.text.to_lowercase();
                        let adj_lower = token2.text.to_lowercase();
                        let is_time_noun = matches!(noun_lower.as_str(),
                            "segundo" | "segundos" | "minuto" | "minutos" |
                            "hora" | "horas" | "día" | "días" |
                            "semana" | "semanas" | "mes" | "meses" |
                            "año" | "años" | "rato" | "momento" | "instante");
                        let is_participle = adj_lower.ends_with("ado") || adj_lower.ends_with("ido") ||
                                           adj_lower.ends_with("ados") || adj_lower.ends_with("idos") ||
                                           adj_lower.ends_with("ada") || adj_lower.ends_with("ida") ||
                                           adj_lower.ends_with("adas") || adj_lower.ends_with("idas");
                        if is_time_noun && is_participle {
                            return None; // Skip - participle agrees with implicit subject
                        }
                    }

                    // Skip partitive expressions: "uno de los", "una de las", etc.
                    // In "días uno de los accidentes", "uno" is not an adjective for "días"
                    if window_pos + 2 < word_tokens.len() {
                        let next_word = &word_tokens[window_pos + 2].1.text.to_lowercase();
                        let second_word = token2.text.to_lowercase();
                        let partitive_words = ["uno", "una", "alguno", "alguna", "ninguno", "ninguna",
                                              "cualquiera", "cada"];
                        if partitive_words.contains(&second_word.as_str()) && next_word == "de" {
                            return None;
                        }
                    }

                    // Skip if there's an earlier noun that the adjective agrees with
                    // Traverse backwards through adjectives to find a noun
                    // In "baliza GPS colocada", "colocada" agrees with "baliza", not "GPS"
                    // In "terapia de edición genética CRISPR adaptada", "adaptada" agrees with "terapia"
                    {
                        let mut search_pos = window_pos as isize - 1;
                        while search_pos >= 0 {
                            let search_token = word_tokens[search_pos as usize].1;
                            if let Some(ref info) = search_token.word_info {
                                match info.category {
                                    WordCategory::Sustantivo => {
                                        // Found a noun - check if adjective agrees with it
                                        let adj_agrees = language.check_gender_agreement(search_token, token2)
                                            && language.check_number_agreement(search_token, token2);
                                        if adj_agrees {
                                            return None; // Skip - adjective agrees with earlier noun
                                        }
                                        break; // Stop at first noun whether it agrees or not
                                    }
                                    WordCategory::Adjetivo => {
                                        // Skip adjectives, continue looking
                                        search_pos -= 1;
                                    }
                                    WordCategory::Preposicion => {
                                        // Skip prepositions like "de", continue looking
                                        search_pos -= 1;
                                    }
                                    _ => break, // Stop at other word types
                                }
                            } else {
                                // Unknown word (like CRISPR before dictionary), skip it
                                search_pos -= 1;
                            }
                        }
                    }

                    // Skip number > 1 with invariable unit + plural adjective: "5 kWh necesarios"
                    // When a quantity > 1 precedes a singular unit noun, the adjective should be plural
                    // Examples: "13,6 kWh necesarios", "100 km recorridos", "500W teóricos"
                    {
                        let noun_idx = *idx1;
                        if let Some(ref adj_info) = token2.word_info {
                            // Check if adjective is plural and noun is singular
                            if adj_info.number == Number::Plural {
                                if let Some(ref noun_info) = token1.word_info {
                                    if noun_info.number == Number::Singular {
                                        // Look backwards for a number before the noun
                                        for i in (0..noun_idx).rev() {
                                            let t = &tokens[i];
                                            if t.token_type == TokenType::Number {
                                                // Found a number - check if it's > 1
                                                // Parse the number (handle decimals with comma)
                                                let num_text = t.text.replace(',', ".");
                                                if let Ok(num) = num_text.parse::<f64>() {
                                                    if num > 1.0 {
                                                        return None; // Skip - plural adjective is correct
                                                    }
                                                }
                                                break;
                                            }
                                            // Stop if we hit another word
                                            if t.token_type == TokenType::Word {
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let gender_ok = language.check_gender_agreement(token1, token2);
                    let number_ok = language.check_number_agreement(token1, token2);

                    if !gender_ok || !number_ok {
                        // Antes de corregir, verificar si el adjetivo concuerda con un sustantivo DESPUÉS
                        // En "suspenso futuras expediciones", "futuras" va con "expediciones", no "suspenso"
                        if let Some(ref adj_info) = token2.word_info {
                            if adj_info.category == WordCategory::Adjetivo {
                                let current_pos = window_pos + rule.pattern.len() - 1;
                                if current_pos + 1 < word_tokens.len() {
                                    let (_, next_token) = word_tokens[current_pos + 1];
                                    if let Some(ref next_info) = next_token.word_info {
                                        if next_info.category == WordCategory::Sustantivo {
                                            // Si el adjetivo concuerda con el siguiente sustantivo, no corregir
                                            // Si el género es None (no especificado), solo comparar números
                                            let gender_matches = adj_info.gender == next_info.gender
                                                || adj_info.gender == Gender::None
                                                || next_info.gender == Gender::None;
                                            if gender_matches && adj_info.number == next_info.number {
                                                return None;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        return self.generate_correction(
                            rule,
                            *idx1,
                            *idx2,
                            token1,
                            token2,
                            language,
                        );
                    }
                }
            }
            RuleCondition::Custom(_) => {
                // Condiciones custom se manejan en implementaciones específicas
            }
        }

        None
    }

    fn generate_correction(
        &self,
        rule: &GrammarRule,
        idx1: usize,
        idx2: usize,
        token1: &Token,
        token2: &Token,
        language: &dyn Language,
    ) -> Option<GrammarCorrection> {
        match &rule.action {
            RuleAction::CorrectArticle => {
                // Corregir artículo según el sustantivo
                // Skip if noun is capitalized mid-sentence (likely a title or proper noun)
                // Example: "El Capital" (Marx's book), "La Odisea" (Homer's poem)
                if token2.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    // Check if it's not at the start of text (where capitalization is normal)
                    if idx2 > 0 {
                        return None; // Capitalized noun mid-sentence = likely title/proper noun
                    }
                }
                if let Some(ref info) = token2.word_info {
                    let is_definite = matches!(
                        token1.text.to_lowercase().as_str(),
                        "el" | "la" | "los" | "las"
                    );
                    // Usar el sustantivo para manejar excepciones como "el agua"
                    let noun = token2.effective_text();
                    let correct = language.get_correct_article_for_noun(noun, info.gender, info.number, is_definite);
                    if !correct.is_empty() && correct != token1.text.to_lowercase() {
                        // Preservar mayúsculas si el original las tenía
                        let suggestion = if token1.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                            let mut chars = correct.chars();
                            match chars.next() {
                                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                                None => correct.to_string(),
                            }
                        } else {
                            correct.to_string()
                        };

                        return Some(GrammarCorrection {
                            token_index: idx1,
                            original: token1.text.clone(),
                            suggestion,
                            rule_id: rule.id.0.clone(),
                            message: format!(
                                "Concordancia de género: '{}' debería ser '{}'",
                                token1.text, correct
                            ),
                        });
                    }
                }
            }
            RuleAction::CorrectAdjective => {
                // Corregir adjetivo según el sustantivo
                // token1 = sustantivo, token2 = adjetivo
                // NOTA: Excluir adjetivos predicativos comunes que suelen concordar con el sujeto,
                // no con el sustantivo más cercano (ej: "fueron al parque juntos")
                // Adjetivos y participios que suelen usarse en función predicativa
                // después de verbos como "estar", "quedar", "resultar", "permanecer"
                // y NO deben corregirse para concordar con el sustantivo anterior
                let predicative_adjectives = [
                    // Adjetivos predicativos comunes
                    "juntos", "juntas", "junto", "junta",
                    "solos", "solas", "solo", "sola",
                    "presentes", "presente",
                    "ausentes", "ausente",
                    "contentos", "contentas", "contento", "contenta",
                    "satisfechos", "satisfechas", "satisfecho", "satisfecha",
                    "dispuestos", "dispuestas", "dispuesto", "dispuesta",
                    "seguros", "seguras", "seguro", "segura",
                    "listos", "listas", "listo", "lista",
                    "muertos", "muertas", "muerto", "muerta",
                    "vivos", "vivas", "vivo", "viva",
                    // Participios frecuentes tras "estado/a" (ha estado sometida, etc.)
                    "sometidos", "sometidas", "sometido", "sometida",
                    "expuestos", "expuestas", "expuesto", "expuesta",
                    "obligados", "obligadas", "obligado", "obligada",
                    "destinados", "destinadas", "destinado", "destinada",
                    "condenados", "condenadas", "condenado", "condenada",
                    "llamados", "llamadas", "llamado", "llamada",
                    "considerados", "consideradas", "considerado", "considerada",
                    // Participios que frecuentemente modifican un sustantivo lejano
                    "recogidos", "recogidas", "recogido", "recogida",
                    "publicados", "publicadas", "publicado", "publicada",
                    "citados", "citadas", "citado", "citada",
                    "mencionados", "mencionadas", "mencionado", "mencionada",
                    // Locuciones prepositivas invariables (debido a, gracias a, etc.)
                    "debido", "gracias",
                    // Participios que modifican un sujeto lejano en construcciones absolutas
                    // "La economía creció, apoyada por el turismo" - apoyada concuerda con economía
                    "apoyados", "apoyadas", "apoyado", "apoyada",
                    "impulsados", "impulsadas", "impulsado", "impulsada",
                    "afectados", "afectadas", "afectado", "afectada",
                    "motivados", "motivadas", "motivado", "motivada",
                    "acompañados", "acompañadas", "acompañado", "acompañada",
                    "seguidos", "seguidas", "seguido", "seguida",
                    "precedidos", "precedidas", "precedido", "precedida",
                    "liderados", "lideradas", "liderado", "liderada",
                    "encabezados", "encabezadas", "encabezado", "encabezada",
                    "respaldados", "respaldadas", "respaldado", "respaldada",
                    "marcados", "marcadas", "marcado", "marcada",
                    "caracterizados", "caracterizadas", "caracterizado", "caracterizada",
                    // Participios que pueden concordar con sustantivos coordinados de género mixto
                    // "hábitats y especies cubiertos" - cubiertos concuerda con el grupo, no solo especies
                    "cubiertos", "cubiertas", "cubierto", "cubierta",
                    "incluidos", "incluidas", "incluido", "incluida",
                    "excluidos", "excluidas", "excluido", "excluida",
                    "protegidos", "protegidas", "protegido", "protegida",
                    "relacionados", "relacionadas", "relacionado", "relacionada",
                    "situados", "situadas", "situado", "situada",
                    "ubicados", "ubicadas", "ubicado", "ubicada",
                    // Participios de estado (tras X tiempo ingresado/internado)
                    "ingresados", "ingresadas", "ingresado", "ingresada",
                    "internados", "internadas", "internado", "internada",
                    "hospitalizado", "hospitalizada", "hospitalizados", "hospitalizadas",
                    "conectados", "conectadas", "conectado", "conectada",
                    "dormidos", "dormidas", "dormido", "dormida",
                    "despiertos", "despiertas", "despierto", "despierta",
                    "sentados", "sentadas", "sentado", "sentada",
                    "parados", "paradas", "parado", "parada",
                    "acostados", "acostadas", "acostado", "acostada",
                    // Participios en construcciones absolutas ("una vez reclamados", "una vez absorbidos")
                    "absorbidos", "absorbidas", "absorbido", "absorbida",
                    "reclamados", "reclamadas", "reclamado", "reclamada",
                    "asociados", "asociadas", "asociado", "asociada",
                    "completados", "completadas", "completado", "completada",
                    "terminados", "terminadas", "terminado", "terminada",
                    "finalizados", "finalizadas", "finalizado", "finalizada",
                    "aprobados", "aprobadas", "aprobado", "aprobada",
                    "confirmados", "confirmadas", "confirmado", "confirmada",
                    "verificados", "verificadas", "verificado", "verificada",
                    "validados", "validadas", "validado", "validada",
                    "aceptados", "aceptadas", "aceptado", "aceptada",
                    "rechazados", "rechazadas", "rechazado", "rechazada",
                ];
                let adj_lower = token2.text.to_lowercase();
                if predicative_adjectives.contains(&adj_lower.as_str()) {
                    // Skip - estos adjetivos frecuentemente no concuerdan con el sustantivo anterior
                    return None;
                }

                // Skip if the adjective is capitalized mid-sentence (likely a proper name)
                // Example: "Conferencia Severo Ochoa" - "Severo" is a proper name, not an adjective
                if token2.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    // Check if it's not at the start of text (where capitalization is normal)
                    if idx2 > 0 {
                        return None; // Capitalized word mid-sentence = likely proper name
                    }
                }

                // Skip if adjective has invariable gender (like numerals: cuarenta, treinta, etc.)
                // These never change form regardless of the noun they modify
                if let Some(ref adj_info) = token2.word_info {
                    if adj_info.gender == Gender::None {
                        return None;
                    }
                }

                if let Some(ref noun_info) = token1.word_info {
                    if let Some(correct) =
                        language.get_adjective_form(&token2.text, noun_info.gender, noun_info.number)
                    {
                        if correct.to_lowercase() != token2.text.to_lowercase() {
                            return Some(GrammarCorrection {
                                token_index: idx2,
                                original: token2.text.clone(),
                                suggestion: correct.clone(),
                                rule_id: rule.id.0.clone(),
                                message: format!(
                                    "Concordancia: '{}' debería ser '{}'",
                                    token2.text, correct
                                ),
                            });
                        }
                    }
                }
            }
            RuleAction::CorrectDeterminer => {
                // Corregir determinante según el sustantivo
                // token1 = determinante, token2 = sustantivo
                if let Some(ref noun_info) = token2.word_info {
                    if let Some(correct) =
                        language.get_correct_determiner(&token1.text, noun_info.gender, noun_info.number)
                    {
                        if correct.to_lowercase() != token1.text.to_lowercase() {
                            // Preservar mayúsculas si el original las tenía
                            let suggestion = if token1.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                                let mut chars = correct.chars();
                                match chars.next() {
                                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                                    None => correct.to_string(),
                                }
                            } else {
                                correct.to_string()
                            };

                            return Some(GrammarCorrection {
                                token_index: idx1,
                                original: token1.text.clone(),
                                suggestion,
                                rule_id: rule.id.0.clone(),
                                message: format!(
                                    "Concordancia determinante-sustantivo: '{}' debería ser '{}'",
                                    token1.text, correct
                                ),
                            });
                        }
                    }
                }
            }
            RuleAction::CorrectVerb => {
                // Concordancia sujeto-verbo se maneja en SubjectVerbAnalyzer
            }
            RuleAction::SuggestAlternative(alt) => {
                return Some(GrammarCorrection {
                    token_index: idx1,
                    original: token1.text.clone(),
                    suggestion: alt.clone(),
                    rule_id: rule.id.0.clone(),
                    message: format!("Sugerencia: usar '{}' en lugar de '{}'", alt, token1.text),
                });
            }
        }

        None
    }
}

impl Default for GrammarAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::DictionaryLoader;
    use crate::languages::spanish::Spanish;

    fn setup() -> (Trie, Spanish) {
        let dict_path = std::path::Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };
        let language = Spanish::new();
        (dictionary, language)
    }

    #[test]
    fn test_determiner_este_casa_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("este casa");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // Debe sugerir "esta" en lugar de "este" porque "casa" es femenino
        let det_correction = corrections.iter().find(|c| c.original == "este");
        assert!(det_correction.is_some(), "Debería encontrar corrección para 'este'");
        assert_eq!(det_correction.unwrap().suggestion, "esta");
    }

    #[test]
    fn test_determiner_esta_libro_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("esta libro");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // Debe sugerir "este" en lugar de "esta" porque "libro" es masculino
        let det_correction = corrections.iter().find(|c| c.original == "esta");
        assert!(det_correction.is_some(), "Debería encontrar corrección para 'esta'");
        assert_eq!(det_correction.unwrap().suggestion, "este");
    }

    #[test]
    fn test_determiner_ese_mujer_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("ese mujer");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // Debe sugerir "esa" en lugar de "ese" porque "mujer" es femenino
        let det_correction = corrections.iter().find(|c| c.original == "ese");
        assert!(det_correction.is_some(), "Debería encontrar corrección para 'ese'");
        assert_eq!(det_correction.unwrap().suggestion, "esa");
    }

    #[test]
    fn test_determiner_aquel_ventana_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("aquel ventana");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // Debe sugerir "aquella" en lugar de "aquel" porque "ventana" es femenino
        let det_correction = corrections.iter().find(|c| c.original == "aquel");
        assert!(det_correction.is_some(), "Debería encontrar corrección para 'aquel'");
        assert_eq!(det_correction.unwrap().suggestion, "aquella");
    }

    #[test]
    fn test_determiner_nuestro_familia_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("nuestro familia");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // Debe sugerir "nuestra" en lugar de "nuestro" porque "familia" es femenino
        let det_correction = corrections.iter().find(|c| c.original == "nuestro");
        assert!(det_correction.is_some(), "Debería encontrar corrección para 'nuestro'");
        assert_eq!(det_correction.unwrap().suggestion, "nuestra");
    }

    #[test]
    fn test_determiner_correct_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("esta casa");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // No debería haber correcciones porque "esta casa" es correcto
        let det_correction = corrections.iter().find(|c| c.original == "esta");
        assert!(det_correction.is_none(), "No debería haber corrección para 'esta casa' que es correcto");
    }

    #[test]
    fn test_determiner_plural_estos_casas_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("estos casas");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // Debe sugerir "estas" en lugar de "estos" porque "casas" es femenino plural
        let det_correction = corrections.iter().find(|c| c.original == "estos");
        assert!(det_correction.is_some(), "Debería encontrar corrección para 'estos'");
        assert_eq!(det_correction.unwrap().suggestion, "estas");
    }

    #[test]
    fn test_pronoun_adjective_no_correction() {
        // "él mismo" no debe corregirse porque "él" es pronombre, no sustantivo
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("él mismo");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // No debería haber correcciones porque "él" es pronombre, no sustantivo
        let adj_correction = corrections.iter().find(|c| c.original == "mismo");
        assert!(adj_correction.is_none(), "No debería corregir 'mismo' porque 'él' es pronombre, no sustantivo");
    }

    #[test]
    fn test_pronoun_adjective_uppercase_no_correction() {
        // "Él mismo" (con mayúscula) tampoco debe corregirse
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("Él mismo");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // No debería haber correcciones porque "Él" es pronombre, no sustantivo
        let adj_correction = corrections.iter().find(|c| c.original == "mismo");
        assert!(adj_correction.is_none(), "No debería corregir 'mismo' porque 'Él' es pronombre, no sustantivo");
    }

    #[test]
    fn test_pronoun_adjective_el_alto_no_correction() {
        // "él alto" no debe corregirse porque "él" es pronombre, no sustantivo
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("él alto");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        // No debería haber correcciones porque "él" es pronombre, no sustantivo
        let adj_correction = corrections.iter().find(|c| c.original == "alto");
        assert!(adj_correction.is_none(), "No debería corregir 'alto' porque 'él' es pronombre, no sustantivo. Correcciones: {:?}", corrections);
    }

    // ==========================================================================
    // Tests para sustantivos femeninos con "a" tónica (el agua, un hacha)
    // ==========================================================================

    #[test]
    fn test_feminine_tonic_a_la_agua_correction() {
        // "la agua" es incorrecto, debe ser "el agua"
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la agua");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(art_correction.is_some(), "Debería corregir 'la agua' a 'el agua'");
        assert_eq!(art_correction.unwrap().suggestion, "el");
    }

    #[test]
    fn test_feminine_tonic_a_una_aguila_correction() {
        // "una águila" es incorrecto, debe ser "un águila"
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("una águila");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        let art_correction = corrections.iter().find(|c| c.original == "una");
        assert!(art_correction.is_some(), "Debería corregir 'una águila' a 'un águila'");
        assert_eq!(art_correction.unwrap().suggestion, "un");
    }

    #[test]
    fn test_feminine_tonic_a_el_agua_no_correction() {
        // "el agua" es correcto, NO debe corregirse
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("el agua");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        let art_correction = corrections.iter().find(|c| c.original == "el");
        assert!(art_correction.is_none(), "No debería corregir 'el agua' que es correcto");
    }

    #[test]
    fn test_feminine_tonic_a_un_hacha_no_correction() {
        // "un hacha" es correcto, NO debe corregirse
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("un hacha");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language);

        let art_correction = corrections.iter().find(|c| c.original == "un");
        assert!(art_correction.is_none(), "No debería corregir 'un hacha' que es correcto");
    }
}
