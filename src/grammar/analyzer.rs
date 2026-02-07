//! Analizador gramatical

use crate::dictionary::{Gender, Number, Trie, WordCategory};
use crate::languages::spanish::exceptions::{allows_both_gender_articles, is_common_gender_noun};
use crate::languages::spanish::VerbRecognizer;
use crate::languages::Language;
use crate::spelling::levenshtein::damerau_levenshtein_distance;
use crate::units;

use super::rules::{GrammarRule, RuleAction, RuleCondition, RuleEngine, TokenPattern};
use super::tokenizer::{has_sentence_boundary, Token, TokenType};

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
        verb_recognizer: Option<&VerbRecognizer>,
    ) -> Vec<GrammarCorrection> {
        // Primero, enriquecer tokens con información del diccionario
        // Usar effective_text() para que las correcciones ortográficas se propaguen
        // Ejemplo: "este cassa" → spelling corrige "cassa"→"casa", grammar debe ver "casa"
        for token in tokens.iter_mut() {
            if token.token_type == TokenType::Word {
                let lower = token.effective_text().to_lowercase();
                if let Some(info) = dictionary.get(&lower) {
                    token.word_info = Some(info.clone());
                } else if let Some(info) = dictionary.derive_plural_info(&lower) {
                    token.word_info = Some(info);
                }
            }
        }

        let mut corrections = Vec::new();

        // Analizar reglas habilitadas
        for rule in self.rule_engine.get_enabled_rules() {
            let rule_corrections = self.apply_rule(rule, tokens, dictionary, language, verb_recognizer);
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
        verb_recognizer: Option<&VerbRecognizer>,
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
                    self.check_condition_and_correct(rule, window, &word_tokens, window_pos, tokens, dictionary, language, verb_recognizer)
                {
                    corrections.push(correction);
                }
            }
        }

        corrections
    }

    /// Checks if there's a sentence/phrase boundary between tokens in a window
    /// Uses the unified has_sentence_boundary() plus comma (which separates list items)
    fn has_sentence_boundary_between(&self, all_tokens: &[Token], window: &[(usize, &Token)]) -> bool {
        if window.len() < 2 {
            return false;
        }
        let first_idx = window[0].0;
        let last_idx = window[window.len() - 1].0;

        // Use unified sentence boundary detection
        if has_sentence_boundary(all_tokens, first_idx, last_idx) {
            return true;
        }

        // Also check for comma (separates list items: "A, B" are separate elements)
        for i in (first_idx + 1)..last_idx {
            if all_tokens[i].token_type == TokenType::Punctuation && all_tokens[i].text == "," {
                return true;
            }
        }
        false
    }

    /// Checks if there's a sentence boundary between tokens, ignoring quote marks.
    fn has_sentence_boundary_except_quotes(all_tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        let (start, end) = if start_idx < end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };

        for i in (start + 1)..end {
            if all_tokens[i].is_sentence_boundary() {
                let text = all_tokens[i].text.as_str();
                if matches!(text, "\"" | "\u{201C}" | "\u{201D}" | "\u{00BB}" | "\u{00AB}") {
                    continue;
                }
                return true;
            }
        }
        false
    }

    /// Checks if there's a number between tokens followed by a unit/abbreviation
    /// Used to skip article-noun agreement only when the noun is a unit
    /// Example: "los 10 MB" - skip (MB is unit, article agrees with quantity)
    /// Example: "los 3 casas" - don't skip (casas is regular noun, should correct to "las")
    fn has_number_between(&self, all_tokens: &[Token], window: &[(usize, &Token)]) -> bool {
        if window.len() < 2 {
            return false;
        }
        let first_idx = window[0].0;
        let last_idx = window[window.len() - 1].0;

        // Check if there's a number between the tokens
        let mut has_number = false;
        for i in (first_idx + 1)..last_idx {
            if all_tokens[i].token_type == TokenType::Number {
                has_number = true;
                break;
            }
        }

        if !has_number {
            return false;
        }

        // Only skip if the noun (last token in window) looks like a unit/abbreviation
        let noun = &window[window.len() - 1].1;
        let noun_text = noun.effective_text();

        // Units are typically:
        // 1. All uppercase abbreviations: MB, GB, KB, TB, Hz, MHz, GHz, etc.
        // 2. Short lowercase units: km, m, cm, mm, kg, g, mg, ml, l, etc.
        // 3. Currency and measurement words
        Self::is_unit_or_abbreviation(noun_text)
    }

    /// Checks if a word is a unit, abbreviation, or measurement (delegates to centralized module)
    fn is_unit_or_abbreviation(word: &str) -> bool {
        units::is_unit_like(word)
    }

    /// Devuelve true si el sustantivo es de género común en singular o plural.
    fn is_common_gender_noun_form(noun: &str) -> bool {
        let noun_lower = noun.to_lowercase();
        if is_common_gender_noun(&noun_lower) {
            return true;
        }

        if let Some(stem) = noun_lower.strip_suffix("es") {
            if is_common_gender_noun(stem) {
                return true;
            }
        }

        if let Some(stem) = noun_lower.strip_suffix('s') {
            if is_common_gender_noun(stem) {
                return true;
            }
        }

        false
    }

    fn article_features(article: &str) -> Option<(bool, Number, Gender)> {
        match article {
            "el" => Some((true, Number::Singular, Gender::Masculine)),
            "la" => Some((true, Number::Singular, Gender::Feminine)),
            "los" => Some((true, Number::Plural, Gender::Masculine)),
            "las" => Some((true, Number::Plural, Gender::Feminine)),
            "un" => Some((false, Number::Singular, Gender::Masculine)),
            "una" => Some((false, Number::Singular, Gender::Feminine)),
            "unos" => Some((false, Number::Plural, Gender::Masculine)),
            "unas" => Some((false, Number::Plural, Gender::Feminine)),
            _ => None,
        }
    }

    /// Devuelve true cuando el cambio de artículo altera solo género (mismo número y definitud).
    fn is_pure_gender_article_swap(current: &str, suggested: &str) -> bool {
        let Some((curr_def, curr_num, curr_gender)) = Self::article_features(current) else {
            return false;
        };
        let Some((sugg_def, sugg_num, sugg_gender)) = Self::article_features(suggested) else {
            return false;
        };

        curr_def == sugg_def && curr_num == sugg_num && curr_gender != sugg_gender
    }

    fn determiner_features(determiner: &str) -> Option<(&'static str, Number, Gender)> {
        match determiner {
            "el" => Some(("art_def", Number::Singular, Gender::Masculine)),
            "la" => Some(("art_def", Number::Singular, Gender::Feminine)),
            "los" => Some(("art_def", Number::Plural, Gender::Masculine)),
            "las" => Some(("art_def", Number::Plural, Gender::Feminine)),
            "un" => Some(("art_indef", Number::Singular, Gender::Masculine)),
            "una" => Some(("art_indef", Number::Singular, Gender::Feminine)),
            "unos" => Some(("art_indef", Number::Plural, Gender::Masculine)),
            "unas" => Some(("art_indef", Number::Plural, Gender::Feminine)),
            "este" => Some(("dem_este", Number::Singular, Gender::Masculine)),
            "esta" => Some(("dem_este", Number::Singular, Gender::Feminine)),
            "estos" => Some(("dem_este", Number::Plural, Gender::Masculine)),
            "estas" => Some(("dem_este", Number::Plural, Gender::Feminine)),
            "ese" => Some(("dem_ese", Number::Singular, Gender::Masculine)),
            "esa" => Some(("dem_ese", Number::Singular, Gender::Feminine)),
            "esos" => Some(("dem_ese", Number::Plural, Gender::Masculine)),
            "esas" => Some(("dem_ese", Number::Plural, Gender::Feminine)),
            "aquel" => Some(("dem_aquel", Number::Singular, Gender::Masculine)),
            "aquella" => Some(("dem_aquel", Number::Singular, Gender::Feminine)),
            "aquellos" => Some(("dem_aquel", Number::Plural, Gender::Masculine)),
            "aquellas" => Some(("dem_aquel", Number::Plural, Gender::Feminine)),
            "nuestro" => Some(("pos_nuestro", Number::Singular, Gender::Masculine)),
            "nuestra" => Some(("pos_nuestro", Number::Singular, Gender::Feminine)),
            "nuestros" => Some(("pos_nuestro", Number::Plural, Gender::Masculine)),
            "nuestras" => Some(("pos_nuestro", Number::Plural, Gender::Feminine)),
            "vuestro" => Some(("pos_vuestro", Number::Singular, Gender::Masculine)),
            "vuestra" => Some(("pos_vuestro", Number::Singular, Gender::Feminine)),
            "vuestros" => Some(("pos_vuestro", Number::Plural, Gender::Masculine)),
            "vuestras" => Some(("pos_vuestro", Number::Plural, Gender::Feminine)),
            _ => None,
        }
    }

    /// Devuelve true cuando el cambio de determinante altera solo género
    /// (misma familia y mismo número).
    fn is_pure_gender_determiner_swap(current: &str, suggested: &str) -> bool {
        let Some((curr_family, curr_num, curr_gender)) = Self::determiner_features(current) else {
            return false;
        };
        let Some((sugg_family, sugg_num, sugg_gender)) = Self::determiner_features(suggested) else {
            return false;
        };

        curr_family == sugg_family && curr_num == sugg_num && curr_gender != sugg_gender
    }

    fn adjective_oa_features(adjective: &str) -> Option<(Number, Gender, String)> {
        if adjective.ends_with("os") {
            return Some((
                Number::Plural,
                Gender::Masculine,
                adjective.trim_end_matches("os").to_string(),
            ));
        }
        if adjective.ends_with("as") {
            return Some((
                Number::Plural,
                Gender::Feminine,
                adjective.trim_end_matches("as").to_string(),
            ));
        }
        if adjective.ends_with('o') {
            return Some((
                Number::Singular,
                Gender::Masculine,
                adjective.trim_end_matches('o').to_string(),
            ));
        }
        if adjective.ends_with('a') {
            return Some((
                Number::Singular,
                Gender::Feminine,
                adjective.trim_end_matches('a').to_string(),
            ));
        }
        None
    }

    /// Devuelve true cuando el cambio de adjetivo altera solo género
    /// manteniendo número y raíz (patrón regular -o/-a, -os/-as).
    fn is_pure_gender_adjective_swap(current: &str, suggested: &str) -> bool {
        let Some((curr_num, curr_gender, curr_stem)) = Self::adjective_oa_features(current) else {
            return false;
        };
        let Some((sugg_num, sugg_gender, sugg_stem)) = Self::adjective_oa_features(suggested) else {
            return false;
        };

        curr_num == sugg_num && curr_gender != sugg_gender && curr_stem == sugg_stem
    }

    /// Checks whether the sentence containing token_idx is ALL-CAPS.
    /// Used to avoid skipping corrections in fully uppercased text (headlines, posters, etc.).
    fn is_all_caps_sentence(tokens: &[Token], token_idx: usize) -> bool {
        let mut start = 0;
        if token_idx < tokens.len() {
            for i in (0..=token_idx).rev() {
                if tokens[i].is_sentence_boundary() {
                    start = i + 1;
                    break;
                }
            }
        }

        let mut end = tokens.len();
        for i in (token_idx + 1)..tokens.len() {
            if tokens[i].is_sentence_boundary() {
                end = i;
                break;
            }
        }

        let mut saw_word = false;
        for token in tokens[start..end].iter() {
            if token.token_type != TokenType::Word {
                continue;
            }
            let text = token.effective_text();
            if text.chars().any(|c| c.is_alphabetic()) {
                saw_word = true;
                if !text.chars().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
                    return false;
                }
            }
        }

        saw_word
    }

    /// Busca el siguiente sustantivo tras un determinante, saltando adjetivos/artículos/determinantes.
    /// True when spelling provides multiple candidates ("a,b,c"), so the
    /// propagated effective word is only a heuristic first option.
    fn has_ambiguous_spelling_suggestions(token: &Token) -> bool {
        token
            .corrected_spelling
            .as_ref()
            .map(|s| s.contains(','))
            .unwrap_or(false)
    }

    /// True cuando el "effective_text" viene de una proyección ortográfica ambigua
    /// cuya mejor candidata está lejos de la palabra original (baja confianza).
    fn has_low_confidence_spelling_projection(token: &Token) -> bool {
        let Some(suggestions) = token.corrected_spelling.as_ref() else {
            return false;
        };
        if !suggestions.contains(',') {
            return false;
        }

        let Some(first_candidate) = suggestions.split(',').next() else {
            return true;
        };
        if first_candidate.is_empty() {
            return true;
        }

        let original = token.text.to_lowercase();
        let candidate = first_candidate.to_lowercase();
        damerau_levenshtein_distance(&original, &candidate) > 1
    }

    fn infer_noun_features_from_left_determiner(
        tokens: &[Token],
        noun_idx: usize,
    ) -> Option<(Gender, Number)> {
        if noun_idx == 0 {
            return None;
        }

        let mut i = noun_idx;
        while i > 0 {
            i -= 1;
            let token = &tokens[i];
            if token.is_sentence_boundary() {
                break;
            }
            if token.token_type != TokenType::Word {
                continue;
            }

            let lower = token.effective_text().to_lowercase();
            if let Some((_family, number, gender)) = Self::determiner_features(&lower) {
                return Some((gender, number));
            }

            if let Some(ref info) = token.word_info {
                match info.category {
                    WordCategory::Adjetivo | WordCategory::Determinante | WordCategory::Articulo => {
                        continue;
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }

        None
    }

    fn find_next_noun_after<'a>(tokens: &'a [Token], start_idx: usize) -> Option<&'a Token> {
        for i in (start_idx + 1)..tokens.len() {
            if tokens[i].is_sentence_boundary() {
                break;
            }
            if tokens[i].token_type != TokenType::Word {
                continue;
            }
            if let Some(ref info) = tokens[i].word_info {
                match info.category {
                    WordCategory::Sustantivo => return Some(&tokens[i]),
                    WordCategory::Adjetivo | WordCategory::Determinante | WordCategory::Articulo => continue,
                    _ => break,
                }
            } else {
                break;
            }
        }
        None
    }

    /// Detecta coordinación nominal previa al sustantivo actual:
    /// [sustantivo] [conj] [det/art opcional] [sustantivo_actual]
    /// Útil para aceptar adjetivo plural en "una medicina y una nutrición personalizadas".
    fn has_coordinated_noun_before(word_tokens: &[(usize, &Token)], noun_pos: usize) -> bool {
        if noun_pos == 0 {
            return false;
        }

        let mut pos = noun_pos as isize - 1;
        while pos >= 0 {
            let token = word_tokens[pos as usize].1;

            if token.token_type == TokenType::Number {
                pos -= 1;
                continue;
            }

            if let Some(ref info) = token.word_info {
                if info.category == WordCategory::Determinante
                    || info.category == WordCategory::Articulo
                    || info.category == WordCategory::Adjetivo
                {
                    pos -= 1;
                    continue;
                }
            }

            let lower = token.text.to_lowercase();
            if lower == "y" || lower == "e" || lower == "o" || lower == "u" || lower == "ni" {
                pos -= 1;
                while pos >= 0 {
                    let left = word_tokens[pos as usize].1;

                    if left.token_type == TokenType::Number {
                        pos -= 1;
                        continue;
                    }

                    if let Some(ref left_info) = left.word_info {
                        if left_info.category == WordCategory::Sustantivo {
                            return true;
                        }
                        if left_info.category == WordCategory::Determinante
                            || left_info.category == WordCategory::Articulo
                            || left_info.category == WordCategory::Adjetivo
                        {
                            pos -= 1;
                            continue;
                        }
                    }
                    break;
                }
            }
            break;
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

    /// Checks if a word is a participle form (used as adjective, needs agreement correction)
    /// Participles: -ado/-ada/-ados/-adas, -ido/-ida/-idos/-idas
    /// Irregular: -to/-ta, -cho/-cha, -so/-sa (escrito, hecho, impreso, etc.)
    fn is_participle_form(word: &str) -> bool {
        // Regular participles
        if word.ends_with("ado") || word.ends_with("ada")
            || word.ends_with("ados") || word.ends_with("adas")
            || word.ends_with("ido") || word.ends_with("ida")
            || word.ends_with("idos") || word.ends_with("idas") {
            return true;
        }

        // Participios con tilde (verbos en -aer, -eer, -oír, -eír)
        // oír -> oído, caer -> caído, leer -> leído, creer -> creído,
        // traer -> traído, reír -> reído, freír -> freído, etc.
        if word.ends_with("ído") || word.ends_with("ída")
            || word.ends_with("ídos") || word.ends_with("ídas") {
            return true;
        }

        // Irregular participles (with gender/number variations)
        // -to: escrito, abierto, roto, muerto, puesto, visto, vuelto, cubierto, etc.
        // -cho: hecho, dicho, satisfecho, etc.
        // -so: impreso, etc.
        if word.ends_with("to") || word.ends_with("ta")
            || word.ends_with("tos") || word.ends_with("tas")
            || word.ends_with("cho") || word.ends_with("cha")
            || word.ends_with("chos") || word.ends_with("chas")
            || word.ends_with("so") || word.ends_with("sa")
            || word.ends_with("sos") || word.ends_with("sas") {
            // Be more restrictive for -to/-so endings - only match known patterns
            // to avoid false positives with words like "gato", "caso"
            let irregular_participle_stems = [
                "escrit", "abiert", "rot", "muert", "puest", "vist", "vuelt",
                "cubiert", "descubiert", "devuelt", "envuelt", "resuelv", "resuelt",
                "disuelv", "disuelt", "revuelt", "compuest", "dispuest", "expuest",
                "impuest", "opuest", "propuest", "supuest", "frit", "inscrit",
                "proscrit", "suscrit", "descript", "prescrit",
                "hech", "dich", "satisfech", "contradicho", "maldich", "bendich",
                "impres", "confes", "expres", "compres", "supres",
            ];

            for stem in irregular_participle_stems {
                if word.starts_with(stem) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a word is a gerund (invariable verb form) using VerbRecognizer when available
    fn is_gerund(word: &str, verb_recognizer: Option<&VerbRecognizer>) -> bool {
        if let Some(vr) = verb_recognizer {
            return vr.is_gerund(word);
        }
        false
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
        verb_recognizer: Option<&VerbRecognizer>,
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
                            tokens,
                            language,
                            verb_recognizer,
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
                            tokens,
                            language,
                            verb_recognizer,
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
                                // Search for noun before "de", skipping adjectives/articles/determiners/numbers
                                let mut noun_pos = search_pos - 1;
                                let mut found_noun = false;

                                while noun_pos >= 0 {
                                    let noun_candidate = word_tokens[noun_pos as usize].1;

                                    if let Some(ref info) = noun_candidate.word_info {
                                        if info.category == WordCategory::Sustantivo {
                                            found_noun = true;
                                            // Check if adjective agrees with this earlier noun
                                            let adj_agrees = language.check_gender_agreement(noun_candidate, token2)
                                                && language.check_number_agreement(noun_candidate, token2);
                                            if adj_agrees {
                                                return None; // Skip - adjective agrees with noun before "de"
                                            }
                                            // Adjective doesn't agree with this noun - keep searching backward
                                            // through nested prepositional phrases
                                            search_pos = noun_pos - 1;
                                            break;
                                        }

                                        if info.category == WordCategory::Adjetivo
                                            || info.category == WordCategory::Articulo
                                            || info.category == WordCategory::Determinante
                                        {
                                            noun_pos -= 1;
                                            continue;
                                        }
                                    }

                                    if noun_candidate.token_type == TokenType::Number {
                                        noun_pos -= 1;
                                        continue;
                                    }

                                    // Stop at other word types (verbs, etc.)
                                    break;
                                }

                                if found_noun {
                                    continue;
                                }

                                // No noun found before this preposition; keep searching further left
                                search_pos -= 1;
                                continue;
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

                    // Skip coordinated noun phrases when adjective is plural:
                    // "alienación y soledad modernas", "una medicina y una nutrición personalizadas".
                    if let Some(ref adj_info) = token2.word_info {
                        if adj_info.number == Number::Plural
                            && Self::has_coordinated_noun_before(word_tokens, window_pos)
                        {
                            return None;
                        }
                    }

                    // Skip distributive coordinated adjectives:
                    // "los sectores público y privado" = "el sector público y el sector privado"
                    // "los sectores público, privado y mixto"
                    // "los sectores público ni privado"
                    if let (Some(ref noun_info), Some(ref adj_info)) = (&token1.word_info, &token2.word_info) {
                        if noun_info.number == Number::Plural
                            && (adj_info.number == Number::Singular || adj_info.number == Number::None)
                        {
                            let current_pos = window_pos + rule.pattern.len() - 1;
                            let mut pos = current_pos + 1;
                            let mut saw_following_adj = false;
                            let mut saw_conjunction = false;
                            while pos < word_tokens.len() {
                                let (tok_idx, tok) = word_tokens[pos];
                                if Self::has_sentence_boundary_except_quotes(tokens, *idx2, tok_idx) {
                                    break;
                                }
                                let tok_lower = tok.text.to_lowercase();
                                if tok_lower == "y"
                                    || tok_lower == "e"
                                    || tok_lower == "o"
                                    || tok_lower == "u"
                                    || tok_lower == "ni"
                                {
                                    saw_conjunction = true;
                                    if pos + 1 >= word_tokens.len() {
                                        break;
                                    }
                                    let (next_idx, next_adj) = word_tokens[pos + 1];
                                    if Self::has_sentence_boundary_except_quotes(tokens, *idx2, next_idx) {
                                        break;
                                    }
                                    if let Some(ref next_info) = next_adj.word_info {
                                        if next_info.category == WordCategory::Adjetivo
                                            && (next_info.number == Number::Singular
                                                || next_info.number == Number::None)
                                        {
                                            let gender_matches = adj_info.gender == noun_info.gender
                                                || adj_info.gender == Gender::None
                                                || noun_info.gender == Gender::None;
                                            let next_gender_matches = next_info.gender == noun_info.gender
                                                || next_info.gender == Gender::None
                                                || noun_info.gender == Gender::None;
                                            if gender_matches && next_gender_matches {
                                                return None;
                                            }
                                        }
                                    }
                                    break;
                                }

                                if let Some(ref tok_info) = tok.word_info {
                                    if tok_info.category == WordCategory::Adjetivo {
                                        if !(tok_info.number == Number::Singular
                                            || tok_info.number == Number::None)
                                        {
                                            break;
                                        }
                                        let gender_matches = tok_info.gender == noun_info.gender
                                            || tok_info.gender == Gender::None
                                            || noun_info.gender == Gender::None;
                                        if !gender_matches {
                                            break;
                                        }
                                        saw_following_adj = true;
                                        pos += 1;
                                        continue;
                                    }
                                }
                                break;
                            }
                            if saw_following_adj && !saw_conjunction {
                                return None;
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
                        // Absolute participle clause: "una vez obtenidas las credenciales".
                        // Here the participle agrees with the following noun phrase, not with "vez".
                        let is_una_vez_absolute = matches!(noun_lower.as_str(), "vez" | "veces") && {
                            let noun_idx = *idx1;
                            let mut prev_word: Option<String> = None;
                            for i in (0..noun_idx).rev() {
                                let t = &tokens[i];
                                if t.is_sentence_boundary() {
                                    break;
                                }
                                if t.token_type != TokenType::Word {
                                    continue;
                                }
                                prev_word = Some(t.effective_text().to_lowercase());
                                break;
                            }
                            matches!(prev_word.as_deref(), Some("una" | "unas"))
                        };
                        let is_participle = Self::is_participle_form(&adj_lower);
                        if (is_time_noun || is_una_vez_absolute) && is_participle {
                            return None; // Skip - participle agrees with implicit subject
                        }
                    }

                    // Skip partitive expressions: "uno de los", "una de las", etc.
                    // In "días uno de los accidentes", "uno" is not an adjective for "días"
                    if window_pos + 2 < word_tokens.len() {
                        let next_word = &word_tokens[window_pos + 2].1.text.to_lowercase();
                        let second_word = token2.text.to_lowercase();
                        let partitive_words = ["uno", "una", "unos", "unas",
                                              "alguno", "alguna", "algunos", "algunas",
                                              "ninguno", "ninguna", "ningunos", "ningunas",
                                              "cualquiera", "cualesquiera", "cada"];
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
                                        if next_info.category == WordCategory::Sustantivo
                                            && !Self::has_ambiguous_spelling_suggestions(next_token)
                                        {
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
                            tokens,
                            language,
                            verb_recognizer,
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
        tokens: &[Token],
        language: &dyn Language,
        verb_recognizer: Option<&VerbRecognizer>,
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
                if Self::has_low_confidence_spelling_projection(token2) {
                    return None;
                }
                if let Some(ref info) = token2.word_info {
                    if info.category != WordCategory::Sustantivo {
                        return None;
                    }
                    let is_definite = matches!(
                        token1.text.to_lowercase().as_str(),
                        "el" | "la" | "los" | "las"
                    );
                    // Usar el sustantivo para manejar excepciones como "el agua"
                    let noun = token2.effective_text();
                    let correct = language.get_correct_article_for_noun(noun, info.gender, info.number, is_definite);
                    if !correct.is_empty() && correct != token1.text.to_lowercase() {
                        // Para sustantivos de género común (periodista, artista, etc.),
                        // no forzar cambios de artículo que solo alteran género sin referente explícito.
                        let current_article_lower = token1.effective_text().to_lowercase();
                        if Self::is_common_gender_noun_form(noun)
                            && Self::is_pure_gender_article_swap(&current_article_lower, &correct)
                        {
                            return None;
                        }

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

                // Skip gerunds - they are invariable verb forms that never agree in gender/number
                // Example: "abandonando" should NOT become "abandonanda"
                if Self::is_gerund(&adj_lower, verb_recognizer) {
                    return None;
                }

                // Skip if the word is recognized as a FINITE verb form (not participle)
                // Example: "El Ministerio del Interior intensifica" - "intensifica" is a verb, not adjective
                // Words like "intensifica", "modifica", "unifica" are in dictionary as adjectives (f.s. forms)
                // but when used after noun phrases they're typically the main verb
                // IMPORTANT: Participles (-ado/-ido/-to/-cho) function as adjectives and SHOULD be corrected
                // Example: "la puerta cerrado" → "cerrada" - participle used as adjective needs agreement
                if let Some(vr) = verb_recognizer {
                    if vr.is_valid_verb_form(&adj_lower) && !Self::is_participle_form(&adj_lower) {
                        return None;
                    }
                }

                // Skip if the adjective is capitalized mid-sentence (likely a proper name),
                // unless the whole sentence is ALL-CAPS (headlines should still be corrected).
                // Example: "Conferencia Severo Ochoa" - "Severo" is a proper name, not an adjective
                if token2.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    // Check if it's not at the start of text (where capitalization is normal)
                    if idx2 > 0 && !Self::is_all_caps_sentence(tokens, idx2) {
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
                    if token1.text.is_ascii() && Self::has_low_confidence_spelling_projection(token1) {
                        if let Some((det_gender, det_number)) =
                            Self::infer_noun_features_from_left_determiner(tokens, idx1)
                        {
                            if let Some(correct) =
                                language.get_adjective_form(&token2.text, det_gender, det_number)
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
                        return None;
                    }

                    if let Some(correct) =
                        language.get_adjective_form(&token2.text, noun_info.gender, noun_info.number)
                    {
                        if correct.to_lowercase() != token2.text.to_lowercase() {
                            let noun_text = token1.effective_text().to_lowercase();
                            let current_adj_lower = token2.effective_text().to_lowercase();
                            let correct_adj_lower = correct.to_lowercase();

                            // Para sustantivos ambiguos por significado (p. ej. "el cólera"),
                            // no forzar cambios que alteren solo género en adjetivos.
                            if allows_both_gender_articles(&noun_text)
                                && Self::is_pure_gender_adjective_swap(
                                    &current_adj_lower,
                                    &correct_adj_lower,
                                )
                            {
                                return None;
                            }

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
                // Salvaguarda: si el determinante ya concuerda con el sustantivo siguiente,
                // no corregirlo aunque haya un sustantivo previo en una frase con preposición.
                if let Some(next_noun) = Self::find_next_noun_after(tokens, idx1) {
                    let gender_ok = language.check_gender_agreement(token1, next_noun);
                    let number_ok = language.check_number_agreement(token1, next_noun);
                    if gender_ok && number_ok {
                        return None;
                    }
                }
                if let Some(ref noun_info) = token2.word_info {
                    if Self::has_low_confidence_spelling_projection(token2) {
                        return None;
                    }
                    if noun_info.category != WordCategory::Sustantivo {
                        return None;
                    }
                    if let Some(correct) =
                        language.get_correct_determiner(&token1.text, noun_info.gender, noun_info.number)
                    {
                        let noun_text = token2.effective_text().to_lowercase();
                        let current_det_lower = token1.effective_text().to_lowercase();

                        // Para sustantivos con género ambiguo por significado (p. ej. "cólera"),
                        // no forzar swaps que cambien solo género en determinantes.
                        if allows_both_gender_articles(&noun_text)
                            && Self::is_pure_gender_determiner_swap(&current_det_lower, &correct)
                        {
                            return None;
                        }

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        // Debe sugerir "estas" en lugar de "estos" porque "casas" es femenino plural
        let det_correction = corrections.iter().find(|c| c.original == "estos");
        assert!(det_correction.is_some(), "Debería encontrar corrección para 'estos'");
        assert_eq!(det_correction.unwrap().suggestion, "estas");
    }

    #[test]
    fn test_determiner_after_preposition_uses_following_noun() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la casa de nuestro Gobierno");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let det_correction = corrections.iter().find(|c| c.original == "nuestro");
        assert!(
            det_correction.is_none(),
            "No debe corregir 'nuestro' cuando concuerda con el sustantivo siguiente: {:?}",
            det_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_y_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público y privado");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos coordinados: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_commas_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público, privado y mixto");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con comas: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_ni_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público ni privado");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con 'ni': {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_asyndetic_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público, privado, mixto");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos sin conjunción: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_o_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público o privado");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con 'o': {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_u_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público u oficial");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con 'u': {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_ni_twice_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público ni privado ni mixto");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con 'ni... ni...': {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_parentheses_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público (y privado)");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con paréntesis: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_parenthetical_list_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público (privado y mixto)");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con lista parentética: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_quotes_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público \"privado\" y mixto");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con comillas: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_angle_quotes_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público «privado» y mixto");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con comillas angulares: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_distributive_adjectives_with_em_dash_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sectores público — privado — y mixto");
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "público");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivos distributivos con guiones largos: {:?}",
            adj_correction
        );
    }

    #[test]
    fn test_coordinated_nouns_plural_adjective_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("Una medicina y una nutrición personalizadas");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
        let adj_correction = corrections.iter().find(|c| c.original.to_lowercase() == "personalizadas");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivo plural con sustantivos coordinados: {:?}",
            corrections
        );

        let mut tokens = tokenizer.tokenize("Un hombre y una mujer cansados");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
        let adj_correction = corrections.iter().find(|c| c.original.to_lowercase() == "cansados");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivo plural con coordinación mixta: {:?}",
            corrections
        );
    }

    #[test]
    fn test_non_coordinated_plural_adjective_still_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("Una nutrición personalizadas");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
        let adj_correction = corrections.iter().find(|c| c.original.to_lowercase() == "personalizadas");
        assert!(
            adj_correction.is_some(),
            "Debe seguir corrigiendo discordancia sin coordinación: {:?}",
            corrections
        );
        assert_eq!(adj_correction.unwrap().suggestion.to_lowercase(), "personalizada");
    }

    #[test]
    fn test_pronoun_adjective_no_correction() {
        // "él mismo" no debe corregirse porque "él" es pronombre, no sustantivo
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("él mismo");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

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
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "un");
        assert!(art_correction.is_none(), "No debería corregir 'un hacha' que es correcto");
    }

    #[test]
    fn test_colera_masculine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("el c\u{00f3}lera es una enfermedad");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "el");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'el cólera': {:?}",
            corrections
        );
    }

    #[test]
    fn test_colera_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la c\u{00f3}lera de Aquiles fue legendaria");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'la cólera': {:?}",
            corrections
        );
    }

    #[test]
    fn test_cometa_masculine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("el cometa");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "el");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'el cometa': {:?}",
            corrections
        );
    }

    #[test]
    fn test_cometa_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la cometa");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'la cometa': {:?}",
            corrections
        );
    }

    #[test]
    fn test_capitales_masculine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los capitales");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "los");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'los capitales': {:?}",
            corrections
        );
    }

    #[test]
    fn test_capitales_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("las capitales");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "las");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'las capitales': {:?}",
            corrections
        );
    }

    #[test]
    fn test_radio_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la radio llegó");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'la radio': {:?}",
            corrections
        );
    }

    #[test]
    fn test_internet_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la internet llegó");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'la internet': {:?}",
            corrections
        );
    }

    #[test]
    fn test_sarten_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la sartén llegó");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'la sartén': {:?}",
            corrections
        );
    }

    #[test]
    fn test_azucar_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la azúcar llegó");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'la azúcar': {:?}",
            corrections
        );
    }

    #[test]
    fn test_calor_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la calor aprieta");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No deber\u{00ed}a corregir 'la calor': {:?}",
            corrections
        );
    }

    #[test]
    fn test_maraton_feminine_article_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la marat\u{00f3}n fue dura");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_none(),
            "No deber\u{00ed}a corregir 'la marat\u{00f3}n': {:?}",
            corrections
        );
    }

    #[test]
    fn test_colera_masculine_demonstrative_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("este c\u{00f3}lera es una enfermedad");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let det_correction = corrections.iter().find(|c| c.original == "este");
        assert!(
            det_correction.is_none(),
            "No deber\u{00ed}a corregir 'este c\u{00f3}lera': {:?}",
            corrections
        );
    }

    #[test]
    fn test_colera_masculine_possessive_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("nuestro c\u{00f3}lera fue intenso");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let det_correction = corrections.iter().find(|c| c.original == "nuestro");
        assert!(
            det_correction.is_none(),
            "No deber\u{00ed}a corregir 'nuestro c\u{00f3}lera': {:?}",
            corrections
        );
    }

    #[test]
    fn test_ambiguous_gender_determiner_number_mismatch_still_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("estos c\u{00f3}lera");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let det_correction = corrections.iter().find(|c| c.original == "estos");
        assert!(
            det_correction.is_some(),
            "Deber\u{00ed}a corregir desajuste de n\u{00fa}mero en determinante ambiguo: {:?}",
            corrections
        );
        assert_eq!(det_correction.unwrap().suggestion, "esta");
    }

    #[test]
    fn test_colera_masculine_adjective_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("el c\u{00f3}lera asi\u{00e1}tico mata r\u{00e1}pido");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let adj_correction = corrections.iter().find(|c| c.original == "asi\u{00e1}tico");
        assert!(
            adj_correction.is_none(),
            "No deber\u{00ed}a corregir adjetivo masculino en 'el c\u{00f3}lera ...': {:?}",
            corrections
        );
    }

    #[test]
    fn test_ambiguous_gender_adjective_number_mismatch_still_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("el c\u{00f3}lera asi\u{00e1}ticos");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let adj_correction = corrections.iter().find(|c| c.original == "asi\u{00e1}ticos");
        assert!(
            adj_correction.is_some(),
            "Deber\u{00ed}a corregir desajuste de n\u{00fa}mero en adjetivo ambiguo: {:?}",
            corrections
        );
    }

    // ==========================================================================
    // Tests para número entre artículo y sustantivo
    // ==========================================================================

    #[test]
    fn test_number_between_with_unit_no_correction() {
        // "los 10 MB" es correcto - MB es unidad invariable
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los 10 MB");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "los");
        assert!(art_correction.is_none(), "No debería corregir 'los 10 MB' - MB es unidad invariable");
    }

    #[test]
    fn test_number_between_with_currency_corrects() {
        // "la 10 euros" debe corregirse a "los 10 euros" - euros tiene género/número
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la 10 euros");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(art_correction.is_some(), "Debería corregir 'la 10 euros' a 'los 10 euros'");
        assert_eq!(art_correction.unwrap().suggestion, "los");
    }

    #[test]
    fn test_number_between_with_regular_noun_corrects() {
        // "los 3 casas" debe corregirse a "las 3 casas"
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los 3 casas");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "los");
        assert!(art_correction.is_some(), "Debería corregir 'los 3 casas' a 'las 3 casas'");
        assert_eq!(art_correction.unwrap().suggestion, "las");
    }

    #[test]
    fn test_common_gender_plural_article_not_forced_without_referent() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los periodistas publicaron la noticia");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "los");
        assert!(
            art_correction.is_none(),
            "No debería forzar 'los'→'las' en sustantivo de género común sin referente: {:?}",
            corrections
        );
    }

    #[test]
    fn test_non_common_gender_plural_article_still_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los puertas se cerraron");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "los");
        assert!(
            art_correction.is_some(),
            "Debe seguir corrigiendo sustantivos no comunes ('los puertas'): {:?}",
            corrections
        );
        assert_eq!(art_correction.unwrap().suggestion, "las");
    }

    #[test]
    fn test_gerund_not_corrected_for_gender() {
        // Gerunds are invariable - "abandonando" should NOT become "abandonanda"
        // Real case: "la conciliación... como un derecho, abandonando su consideración"
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenizer.tokenize("la conciliación, abandonando su consideración");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&verb_recognizer));

        let gerund_correction = corrections.iter().find(|c| c.original == "abandonando");
        assert!(
            gerund_correction.is_none(),
            "No debería corregir el gerundio 'abandonando' - los gerundios son invariables"
        );
    }

    #[test]
    fn test_gerund_comiendo_not_corrected() {
        // "la ensalada, comiendo despacio" - "comiendo" should not become "comida"
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();
        let verb_recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenizer.tokenize("la ensalada, comiendo despacio");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&verb_recognizer));

        let gerund_correction = corrections.iter().find(|c| c.original == "comiendo");
        assert!(
            gerund_correction.is_none(),
            "No debería corregir el gerundio 'comiendo'"
        );
    }

    #[test]
    fn test_acronym_not_corrected_for_agreement() {
        // Acronyms are invariable - "SATSE" should NOT become "satsen" or similar
        // Real case: "los sindicatos SATSE, CCOO"
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sindicatos SATSE");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let acronym_correction = corrections.iter().find(|c| c.original == "SATSE");
        assert!(
            acronym_correction.is_none(),
            "No debería corregir el acrónimo 'SATSE' - los acrónimos son invariables"
        );
    }

    #[test]
    fn test_multiple_acronyms_not_corrected() {
        // Multiple acronyms after plural noun
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("los sindicatos CCOO y UGT");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let ccoo_correction = corrections.iter().find(|c| c.original == "CCOO");
        let ugt_correction = corrections.iter().find(|c| c.original == "UGT");
        assert!(
            ccoo_correction.is_none(),
            "No debería corregir el acrónimo 'CCOO'"
        );
        assert!(
            ugt_correction.is_none(),
            "No debería corregir el acrónimo 'UGT'"
        );
    }

    #[test]
    fn test_all_caps_noun_adj_correction() {
        // All-caps headlines should still be corrected
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("LA CASA BLANCO");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let adj_correction = corrections.iter().find(|c| c.original == "BLANCO");
        assert!(adj_correction.is_some(), "Should correct adjective in all-caps text");
        assert_eq!(adj_correction.unwrap().suggestion.to_lowercase(), "blanca");
    }


    #[test]
    fn test_participle_after_long_prep_phrase_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("las pensiones de clases pasivas del estado causadas en 2026");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let adj_correction = corrections.iter().find(|c| c.original == "causadas");
        assert!(adj_correction.is_none(), "No debe corregir 'causadas' en este contexto");
    }

    #[test]
    fn test_una_vez_absolute_participle_not_forced_to_singular() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let samples = [
            "Una vez obtenidas las credenciales",
            "Una vez firmados los contratos",
            "Una vez revisadas las cuentas",
            "Una vez cumplidas las condiciones",
            "Una vez resueltos los problemas",
        ];

        for sample in samples {
            let mut tokens = tokenizer.tokenize(sample);
            let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
            let participle_correction = corrections.iter().find(|c| {
                c.original.eq_ignore_ascii_case("obtenidas")
                    || c.original.eq_ignore_ascii_case("firmados")
                    || c.original.eq_ignore_ascii_case("revisadas")
                    || c.original.eq_ignore_ascii_case("cumplidas")
                    || c.original.eq_ignore_ascii_case("resueltos")
            });
            assert!(
                participle_correction.is_none(),
                "No debe forzar concordancia del participio con 'vez' en '{}': {:?}",
                sample,
                corrections
            );
        }
    }

    #[test]
    fn test_adjective_not_blocked_by_ambiguous_next_spelling_noun() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenizer.tokenize("La niña bonito salio");
        tokens[3].corrected_spelling = Some("salmo,palio,salix,savio,salir".to_string());

        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));
        let adj_correction = corrections.iter().find(|c| c.original == "bonito");
        assert!(
            adj_correction.is_some(),
            "Debe corregir 'bonito' aunque la palabra siguiente tenga spelling ambiguo: {:?}",
            corrections
        );
        assert_eq!(adj_correction.unwrap().suggestion, "bonita");
    }

}
