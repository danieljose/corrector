//! Analizador gramatical

use crate::dictionary::{Gender, Number, Trie, WordCategory};
use crate::languages::{Language, VerbFormRecognizer};
use crate::spelling::levenshtein::damerau_levenshtein_distance;
use crate::units;
use std::cell::RefCell;

use super::rules::{GrammarRule, RuleAction, RuleCondition, RuleEngine, TokenPattern};
use super::tokenizer::has_sentence_boundary as has_sentence_boundary_slow;
use super::tokenizer::{SentenceBoundaryIndex, Token, TokenType};

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

struct BoundaryCacheEntry {
    ptr: *const Token,
    len: usize,
    index: SentenceBoundaryIndex,
}

thread_local! {
    static BOUNDARY_CACHE: RefCell<Option<BoundaryCacheEntry>> = const { RefCell::new(None) };
}

struct BoundaryCacheGuard;

impl BoundaryCacheGuard {
    fn new(tokens: &[Token]) -> Self {
        BOUNDARY_CACHE.with(|cache| {
            *cache.borrow_mut() = Some(BoundaryCacheEntry {
                ptr: tokens.as_ptr(),
                len: tokens.len(),
                index: SentenceBoundaryIndex::new(tokens),
            });
        });
        Self
    }
}

impl Drop for BoundaryCacheGuard {
    fn drop(&mut self) {
        BOUNDARY_CACHE.with(|cache| {
            *cache.borrow_mut() = None;
        });
    }
}

#[inline]
fn has_sentence_boundary(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
    BOUNDARY_CACHE.with(|cache| {
        if let Some(entry) = cache.borrow().as_ref() {
            if entry.ptr == tokens.as_ptr() && entry.len == tokens.len() {
                return entry.index.has_between(start_idx, end_idx);
            }
        }
        has_sentence_boundary_slow(tokens, start_idx, end_idx)
    })
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
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<GrammarCorrection> {
        let _boundary_cache_guard = BoundaryCacheGuard::new(tokens);
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
            let rule_corrections =
                self.apply_rule(rule, tokens, dictionary, language, verb_recognizer);
            corrections.extend(rule_corrections);
        }

        // Cuantificador + articulo + sustantivo:
        // "todas los niños" -> "todos los niños".
        for correction in self.detect_quantifier_article_noun_agreement(tokens, language) {
            let duplicated = corrections.iter().any(|existing| {
                existing.token_index == correction.token_index
                    && existing.suggestion.to_lowercase() == correction.suggestion.to_lowercase()
            });
            if !duplicated {
                corrections.push(correction);
            }
        }

        // Numeral + sustantivo en singular: "dos libro" -> "dos libros".
        for correction in self.detect_numeral_noun_number_agreement(tokens, dictionary, language) {
            let duplicated = corrections.iter().any(|existing| {
                existing.token_index == correction.token_index
                    && existing.suggestion.to_lowercase() == correction.suggestion.to_lowercase()
            });
            if !duplicated {
                corrections.push(correction);
            }
        }

        // Concordancia predicativa: sujeto + verbo copulativo + adjetivo atributo.
        // Ej.: "La casa es bonito" -> "bonita".
        for correction in self
            .detect_copulative_predicative_adjective_agreement(tokens, language, verb_recognizer)
        {
            let duplicated = corrections.iter().any(|existing| {
                existing.token_index == correction.token_index
                    && existing.suggestion.to_lowercase() == correction.suggestion.to_lowercase()
            });
            if !duplicated {
                corrections.push(correction);
            }
        }

        // Concordancia atributiva con adverbio intermedio:
        // "una persona muy bueno" -> "una persona muy buena".
        for correction in self.detect_noun_adverb_adjective_agreement(tokens, language, verb_recognizer)
        {
            let duplicated = corrections.iter().any(|existing| {
                existing.token_index == correction.token_index
                    && existing.suggestion.to_lowercase() == correction.suggestion.to_lowercase()
            });
            if !duplicated {
                corrections.push(correction);
            }
        }

        corrections
    }

    fn apply_rule(
        &self,
        rule: &GrammarRule,
        tokens: &[Token],
        dictionary: &Trie,
        language: &dyn Language,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
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
            if self.pattern_matches(&rule.pattern, window, language) {
                if let Some(correction) = self.check_condition_and_correct(
                    rule,
                    window,
                    &word_tokens,
                    window_pos,
                    tokens,
                    dictionary,
                    language,
                    verb_recognizer,
                ) {
                    corrections.push(correction);
                }
            }
        }

        corrections
    }

    fn detect_copulative_predicative_adjective_agreement(
        &self,
        tokens: &[Token],
        language: &dyn Language,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<GrammarCorrection> {
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();
        let mut corrections = Vec::new();

        for i in 0..word_tokens.len() {
            if i + 2 >= word_tokens.len() {
                break;
            }

            let (subject_idx, subject_token) = word_tokens[i];
            let (verb_idx, verb_token) = word_tokens[i + 1];

            if has_sentence_boundary(tokens, subject_idx, verb_idx)
                || Self::has_non_whitespace_between(tokens, subject_idx, verb_idx)
            {
                continue;
            }
            let mut subject_features =
                Self::extract_nominal_subject_features(subject_token, language);
            if let Some((left_gender, _)) =
                Self::coordinated_subject_left_features(tokens, &word_tokens, i, language)
            {
                let right_gender = subject_features
                    .map(|(gender, _)| gender)
                    .unwrap_or(Gender::None);
                let merged_gender =
                    Self::merge_coordinated_subject_gender(left_gender, right_gender);
                subject_features = Some((merged_gender, Number::Plural));
            }
            let Some((subject_gender, subject_number)) = subject_features else {
                continue;
            };
            if Self::is_de_complement_nominal_subject(tokens, &word_tokens, i) {
                continue;
            }
            if Self::is_quantified_temporal_complement_subject(
                tokens,
                &word_tokens,
                i,
                language,
                verb_recognizer,
            ) {
                continue;
            }
            if Self::is_prepositional_phrase_subject(tokens, &word_tokens, i, language) {
                continue;
            }
            if Self::is_postposed_relative_clause_subject(tokens, &word_tokens, i, verb_recognizer)
            {
                continue;
            }

            if !Self::is_copulative_predicative_verb(verb_token, verb_recognizer) {
                continue;
            }

            let mut adj_pos = i + 2;
            let mut has_intermediate_adverb = false;
            if adj_pos < word_tokens.len() {
                let (mid_idx, mid_token) = word_tokens[adj_pos];
                let mid_lower = mid_token.effective_text().to_lowercase();
                let is_mid_adverb = mid_token
                    .word_info
                    .as_ref()
                    .map(|info| info.category == WordCategory::Adverbio)
                    .unwrap_or(false)
                    || mid_lower.ends_with("mente")
                    || matches!(
                        mid_lower.as_str(),
                        "muy" | "mas" | "más" | "tan" | "poco" | "bastante" | "demasiado"
                    );
                if is_mid_adverb {
                    if has_sentence_boundary(tokens, verb_idx, mid_idx)
                        || Self::has_non_whitespace_between(tokens, verb_idx, mid_idx)
                    {
                        continue;
                    }
                    adj_pos += 1;
                    has_intermediate_adverb = true;
                }
            }
            if adj_pos >= word_tokens.len() {
                continue;
            }
            let (adj_idx, adj_token) = word_tokens[adj_pos];
            if has_sentence_boundary(tokens, verb_idx, adj_idx) {
                continue;
            }
            if has_intermediate_adverb {
                let (mid_idx, _) = word_tokens[adj_pos - 1];
                if Self::has_non_whitespace_between(tokens, mid_idx, adj_idx) {
                    continue;
                }
            } else if Self::has_non_whitespace_between(tokens, verb_idx, adj_idx) {
                continue;
            }

            let Some(adj_info) = adj_token.word_info.as_ref() else {
                continue;
            };
            let adj_lower = adj_token.effective_text().to_lowercase();
            let is_participle_verb = adj_info.category == WordCategory::Verbo
                && language.is_participle_form(&adj_lower);
            let is_likely_otro_adjective = adj_info.category == WordCategory::Otro
                && adj_lower.ends_with("és");
            let can_be_predicative_adjective = adj_info.category == WordCategory::Adjetivo
                || is_participle_verb
                || ((adj_info.category == WordCategory::Sustantivo || is_likely_otro_adjective)
                    && language
                        .get_adjective_form(
                            &adj_token.text,
                            subject_gender,
                            subject_number,
                        )
                        .is_some());
            if !can_be_predicative_adjective {
                continue;
            }
            if Self::is_gerund(&adj_lower, verb_recognizer) {
                continue;
            }

            if !is_participle_verb {
                let gender_ok = adj_info.gender == Gender::None || adj_info.gender == subject_gender;
                let number_ok = adj_info.number == Number::None || adj_info.number == subject_number;
                let has_surface_agreement_signal =
                    adj_info.gender != Gender::None || adj_info.number != Number::None;
                if has_surface_agreement_signal && gender_ok && number_ok {
                    continue;
                }
            }

            if let Some(correct) = language.get_adjective_form(
                &adj_token.text,
                subject_gender,
                subject_number,
            ) {
                if correct.to_lowercase() != adj_token.text.to_lowercase() {
                    let suggestion = Self::preserve_initial_case(&adj_token.text, &correct);
                    corrections.push(GrammarCorrection {
                        token_index: adj_idx,
                        original: adj_token.text.clone(),
                        suggestion: suggestion.clone(),
                        rule_id: "es_copulative_predicative_adj_agreement".to_string(),
                        message: format!(
                            "Concordancia predicativa: '{}' deberÃ­a ser '{}'",
                            adj_token.text, suggestion
                        ),
                    });
                }
            }
        }

        corrections
    }

    fn detect_quantifier_article_noun_agreement(
        &self,
        tokens: &[Token],
        language: &dyn Language,
    ) -> Vec<GrammarCorrection> {
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();
        let mut corrections = Vec::new();

        for i in 0..word_tokens.len().saturating_sub(2) {
            let (q_idx, quant_token) = word_tokens[i];
            let (art_idx, article_token) = word_tokens[i + 1];
            let (_, noun_token) = word_tokens[i + 2];

            if has_sentence_boundary(tokens, q_idx, art_idx)
                || has_sentence_boundary(tokens, art_idx, word_tokens[i + 2].0)
                || Self::has_non_whitespace_between(tokens, q_idx, art_idx)
                || Self::has_non_whitespace_between(tokens, art_idx, word_tokens[i + 2].0)
            {
                continue;
            }

            let quant_lower = quant_token.effective_text().to_lowercase();
            let Some((quant_family, _, _)) = language.determiner_features(&quant_lower) else {
                continue;
            };
            if !quant_family.starts_with("quant_") {
                continue;
            }

            let article_lower = article_token.effective_text().to_lowercase();
            let Some((article_family, _, _)) = language.determiner_features(&article_lower) else {
                continue;
            };
            if !article_family.starts_with("art_") {
                continue;
            }

            let Some(noun_info) = noun_token.word_info.as_ref() else {
                continue;
            };
            if noun_info.category != WordCategory::Sustantivo {
                continue;
            }
            if Self::is_likely_adverbial_quantifier_use(tokens, &word_tokens, i) {
                continue;
            }

            if language.check_gender_agreement(quant_token, noun_token)
                && language.check_number_agreement(quant_token, noun_token)
            {
                continue;
            }

            let Some(correct) =
                language.get_correct_determiner(&quant_token.text, noun_info.gender, noun_info.number)
            else {
                continue;
            };

            if correct.to_lowercase() == quant_token.text.to_lowercase() {
                continue;
            }

            let suggestion = if quant_token
                .text
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                let mut chars = correct.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    None => correct.to_string(),
                }
            } else {
                correct.to_string()
            };

            corrections.push(GrammarCorrection {
                token_index: q_idx,
                original: quant_token.text.clone(),
                suggestion,
                rule_id: "quantifier_article_noun_agreement".to_string(),
                message: format!(
                    "Concordancia determinante-sustantivo: '{}' debería ser '{}'",
                    quant_token.text, correct
                ),
            });
        }

        corrections
    }

    fn detect_noun_adverb_adjective_agreement(
        &self,
        tokens: &[Token],
        language: &dyn Language,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<GrammarCorrection> {
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();
        let mut corrections = Vec::new();

        for i in 0..word_tokens.len().saturating_sub(2) {
            let (noun_idx, noun_token) = word_tokens[i];
            let (adv_idx, adv_token) = word_tokens[i + 1];
            let (adj_idx, adj_token) = word_tokens[i + 2];

            if has_sentence_boundary(tokens, noun_idx, adv_idx)
                || has_sentence_boundary(tokens, adv_idx, adj_idx)
                || Self::has_non_whitespace_between(tokens, noun_idx, adv_idx)
                || Self::has_non_whitespace_between(tokens, adv_idx, adj_idx)
            {
                continue;
            }

            let Some(noun_info) = noun_token.word_info.as_ref() else {
                continue;
            };
            if noun_info.category != WordCategory::Sustantivo
                || noun_info.gender == Gender::None
                || noun_info.number == Number::None
            {
                continue;
            }

            let adv_lower = adv_token.effective_text().to_lowercase();
            let is_degree_adverb = adv_token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Adverbio)
                .unwrap_or(false)
                || Self::is_degree_adverb_word(adv_lower.as_str());
            if !is_degree_adverb {
                continue;
            }

            let Some(adj_info) = adj_token.word_info.as_ref() else {
                continue;
            };
            let adj_lower = adj_token.effective_text().to_lowercase();
            if Self::is_gerund(&adj_lower, verb_recognizer) {
                continue;
            }
            if let Some(vr) = verb_recognizer {
                if vr.is_valid_verb_form(&adj_lower) && !language.is_participle_form(&adj_lower) {
                    continue;
                }
            }

            let is_participle_verb = adj_info.category == WordCategory::Verbo
                && language.is_participle_form(&adj_lower);
            let is_likely_otro_adjective = adj_info.category == WordCategory::Otro
                && adj_lower.ends_with("és");
            let can_be_attributive_adjective = adj_info.category == WordCategory::Adjetivo
                || is_participle_verb
                || ((adj_info.category == WordCategory::Sustantivo || is_likely_otro_adjective)
                    && language
                        .get_adjective_form(&adj_token.text, noun_info.gender, noun_info.number)
                        .is_some());
            if !can_be_attributive_adjective {
                continue;
            }

            if !is_participle_verb {
                let gender_ok = adj_info.gender == Gender::None || adj_info.gender == noun_info.gender;
                let number_ok = adj_info.number == Number::None || adj_info.number == noun_info.number;
                let has_surface_agreement_signal =
                    adj_info.gender != Gender::None || adj_info.number != Number::None;
                if has_surface_agreement_signal && gender_ok && number_ok {
                    continue;
                }
            }

            if let Some(correct) =
                language.get_adjective_form(&adj_token.text, noun_info.gender, noun_info.number)
            {
                if correct.to_lowercase() != adj_token.text.to_lowercase() {
                    corrections.push(GrammarCorrection {
                        token_index: adj_idx,
                        original: adj_token.text.clone(),
                        suggestion: Self::preserve_initial_case(&adj_token.text, &correct),
                        rule_id: "noun_adverb_adjective_agreement".to_string(),
                        message: format!(
                            "Concordancia atributiva: '{}' deberia ser '{}'",
                            adj_token.text, correct
                        ),
                    });
                }
            }
        }

        corrections
    }

    fn is_degree_adverb_word(word: &str) -> bool {
        matches!(
            word,
            "muy"
                | "mas"
                | "más"
                | "menos"
                | "tan"
                | "poco"
                | "bastante"
                | "demasiado"
        )
    }

    fn is_likely_adverbial_quantifier_use(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        quant_pos: usize,
    ) -> bool {
        let quant_lower = word_tokens[quant_pos].1.effective_text().to_lowercase();
        if !matches!(
            quant_lower.as_str(),
            "mucho"
                | "mucha"
                | "muchos"
                | "muchas"
                | "poco"
                | "poca"
                | "pocos"
                | "pocas"
                | "bastante"
                | "bastantes"
                | "demasiado"
                | "demasiada"
                | "demasiados"
                | "demasiadas"
        ) {
            return false;
        }
        if quant_pos == 0 {
            return false;
        }

        let (prev_idx, prev_token) = word_tokens[quant_pos - 1];
        let (quant_idx, _) = word_tokens[quant_pos];
        if has_sentence_boundary(tokens, prev_idx, quant_idx)
            || Self::has_non_whitespace_between(tokens, prev_idx, quant_idx)
        {
            return false;
        }

        let prev_lower = prev_token.effective_text().to_lowercase();
        if Self::is_ser_form_for_adverbial_quantifier_guard(prev_lower.as_str()) {
            return false;
        }

        prev_token
            .word_info
            .as_ref()
            .map(|info| info.category == WordCategory::Verbo)
            .unwrap_or_else(|| Self::looks_like_common_finite_verb(prev_lower.as_str()))
    }

    fn is_ser_form_for_adverbial_quantifier_guard(word: &str) -> bool {
        matches!(
            word,
            "es"
                | "son"
                | "era"
                | "eran"
                | "fue"
                | "fueron"
                | "sera"
                | "seran"
                | "seria"
                | "serian"
                | "sea"
                | "sean"
        )
    }

    fn is_demasiado_adverb_before_caras(
        tokens: &[Token],
        det_idx: usize,
        det_token: &Token,
        noun_idx: usize,
        noun_token: &Token,
    ) -> bool {
        let det_lower = Self::normalize_spanish_word(det_token.effective_text());
        if det_lower != "demasiado" {
            return false;
        }
        let noun_lower = Self::normalize_spanish_word(noun_token.effective_text());
        if noun_lower == "caras" {
            let Some(prev_word_idx) = Self::previous_word_in_clause(tokens, det_idx) else {
                return false;
            };
            if has_sentence_boundary(tokens, prev_word_idx, det_idx)
                || Self::has_non_whitespace_between(tokens, prev_word_idx, det_idx)
            {
                return false;
            }
            let prev_lower = Self::normalize_spanish_word(tokens[prev_word_idx].effective_text());
            return matches!(
                prev_lower.as_str(),
                "es"
                    | "son"
                    | "era"
                    | "eran"
                    | "fue"
                    | "fueron"
                    | "sera"
                    | "seran"
                    | "seria"
                    | "serian"
                    | "esta"
                    | "estan"
                    | "estaba"
                    | "estaban"
                    | "estuvo"
                    | "estuvieron"
            );
        }

        if noun_lower == "tarde" {
            if let Some(prev_word_idx) = Self::previous_word_in_clause(tokens, det_idx) {
                if has_sentence_boundary(tokens, prev_word_idx, det_idx)
                    || Self::has_non_whitespace_between(tokens, prev_word_idx, det_idx)
                {
                    return false;
                }
                let prev_token = &tokens[prev_word_idx];
                let prev_lower = Self::normalize_spanish_word(prev_token.effective_text());
                return prev_token
                    .word_info
                    .as_ref()
                    .map(|info| info.category == WordCategory::Verbo)
                    .unwrap_or_else(|| Self::is_likely_finite_verb_for_tarde_guard(prev_lower.as_str()));
            }

            // Inicio de oración: "Demasiado tarde llegaron", "Demasiado tarde para actuar".
            // Aquí "demasiado" es adverbio de grado, no determinante.
            let Some(next_word_idx) = Self::next_word_in_clause(tokens, noun_idx) else {
                return true;
            };
            if has_sentence_boundary(tokens, noun_idx, next_word_idx)
                || Self::has_non_whitespace_between(tokens, noun_idx, next_word_idx)
            {
                return true;
            }
            let next_token = &tokens[next_word_idx];
            let next_lower = Self::normalize_spanish_word(next_token.effective_text());
            let next_is_verb = next_token
                .word_info
                .as_ref()
                .map(|info| info.category == WordCategory::Verbo)
                .unwrap_or(false);
            return next_is_verb
                || Self::is_likely_finite_verb_for_tarde_guard(next_lower.as_str())
                || matches!(
                    next_lower.as_str(),
                    "para"
                        | "por"
                        | "de"
                        | "en"
                        | "sin"
                        | "con"
                        | "a"
                        | "hasta"
                        | "desde"
                        | "hacia"
                );
        }

        false
    }

    fn is_likely_finite_verb_for_tarde_guard(word: &str) -> bool {
        let normalized = Self::normalize_spanish_word(word);
        Self::looks_like_common_finite_verb(normalized.as_str())
            || normalized.ends_with("aron")
            || normalized.ends_with("ieron")
            || normalized.ends_with("aba")
            || normalized.ends_with("ia")
            || normalized.ends_with("io")
    }

    fn looks_like_common_finite_verb(word: &str) -> bool {
        let normalized = Self::normalize_spanish_word(word);
        matches!(
            normalized.as_str(),
            "gusta"
                | "gustan"
                | "trabaja"
                | "trabajan"
                | "habla"
                | "hablan"
                | "llueve"
                | "llueven"
                | "sale"
                | "salen"
                | "vive"
                | "viven"
                | "come"
                | "comen"
        ) || (normalized.len() > 3
            && (normalized.ends_with("a")
                || normalized.ends_with("e")
                || normalized.ends_with("an")
                || normalized.ends_with("en")
                || normalized.ends_with("o")))
    }

    fn detect_numeral_noun_number_agreement(
        &self,
        tokens: &[Token],
        dictionary: &Trie,
        language: &dyn Language,
    ) -> Vec<GrammarCorrection> {
        if language.code() != "es" {
            return Vec::new();
        }

        let mut corrections = Vec::new();

        for i in 0..tokens.len() {
            let Some(quantity) = Self::parse_spanish_numeral_quantity(&tokens[i]) else {
                continue;
            };
            if quantity <= 1 {
                continue;
            }

            let mut noun_idx = i + 1;
            while noun_idx < tokens.len() && tokens[noun_idx].token_type == TokenType::Whitespace {
                noun_idx += 1;
            }
            if noun_idx >= tokens.len() || tokens[noun_idx].token_type != TokenType::Word {
                continue;
            }

            if has_sentence_boundary(tokens, i, noun_idx)
                || Self::has_non_whitespace_between(tokens, i, noun_idx)
            {
                continue;
            }

            let noun_token = &tokens[noun_idx];
            let Some(noun_info) = noun_token.word_info.as_ref() else {
                continue;
            };
            if noun_info.category != WordCategory::Sustantivo {
                continue;
            }
            if noun_info.number == Number::Plural {
                continue;
            }

            let noun_lower = noun_token.effective_text().to_lowercase();
            if noun_info.number == Number::None && Self::looks_like_plural_noun_surface(&noun_lower) {
                continue;
            }

            let Some(suggestion_raw) =
                Self::build_spanish_plural_noun_suggestion(noun_token.effective_text(), dictionary)
            else {
                continue;
            };
            if suggestion_raw.to_lowercase() == noun_lower {
                continue;
            }

            corrections.push(GrammarCorrection {
                token_index: noun_idx,
                original: noun_token.text.clone(),
                suggestion: Self::preserve_initial_case(&noun_token.text, &suggestion_raw),
                rule_id: "numeral_noun_number_agreement".to_string(),
                message: format!(
                    "Concordancia numeral-sustantivo: '{}' deberÃ­a ser '{}'",
                    noun_token.text, suggestion_raw
                ),
            });
        }

        corrections
    }

    fn parse_spanish_numeral_quantity(token: &Token) -> Option<u32> {
        match token.token_type {
            TokenType::Number => {
                let raw = token.effective_text();
                if raw.chars().all(|c| c.is_ascii_digit()) {
                    return raw.parse::<u32>().ok();
                }
                None
            }
            TokenType::Word => {
                let normalized = Self::normalize_spanish_word(token.effective_text());
                let quantity = match normalized.as_str() {
                    "un" | "una" | "uno" => 1,
                    "dos" => 2,
                    "tres" => 3,
                    "cuatro" => 4,
                    "cinco" => 5,
                    "seis" => 6,
                    "siete" => 7,
                    "ocho" => 8,
                    "nueve" => 9,
                    "diez" => 10,
                    "once" => 11,
                    "doce" => 12,
                    "trece" => 13,
                    "catorce" => 14,
                    "quince" => 15,
                    "dieciseis" => 16,
                    "diecisiete" => 17,
                    "dieciocho" => 18,
                    "diecinueve" => 19,
                    "veinte" => 20,
                    "treinta" => 30,
                    "cuarenta" => 40,
                    "cincuenta" => 50,
                    "sesenta" => 60,
                    "setenta" => 70,
                    "ochenta" => 80,
                    "noventa" => 90,
                    "cien" | "ciento" => 100,
                    "mil" => 1000,
                    _ => return None,
                };
                Some(quantity)
            }
            _ => None,
        }
    }

    fn build_spanish_plural_noun_suggestion(noun: &str, dictionary: &Trie) -> Option<String> {
        let lower = noun.to_lowercase();
        let candidates = Self::spanish_plural_noun_candidates(&lower);
        if candidates.is_empty() {
            return None;
        }

        for candidate in &candidates {
            if dictionary.get(candidate).is_some() || dictionary.derive_plural_info(candidate).is_some() {
                return Some(candidate.clone());
            }
        }

        candidates.into_iter().next()
    }

    fn build_spanish_singular_noun_suggestion(noun: &str, dictionary: &Trie) -> Option<String> {
        let lower = noun.to_lowercase();
        let candidates = crate::languages::spanish::plurals::depluralize_candidates(&lower);
        if candidates.is_empty() {
            return None;
        }

        for candidate in &candidates {
            if dictionary.get(candidate).is_some() {
                return Some(candidate.clone());
            }
        }

        candidates.into_iter().next()
    }

    fn is_ningun_quantifier(word_lower: &str) -> bool {
        matches!(
            word_lower,
            "ningun" | "ningún" | "ninguno" | "ninguna" | "ningunos" | "ningunas"
        )
    }

    fn maybe_build_ningun_plural_noun_correction(
        rule: &GrammarRule,
        noun_idx: usize,
        det_token: &Token,
        noun_token: &Token,
        dictionary: &Trie,
    ) -> Option<GrammarCorrection> {
        if !matches!(rule.action, RuleAction::CorrectDeterminer) {
            return None;
        }

        let det_lower = Self::normalize_spanish_word(det_token.effective_text());
        if !Self::is_ningun_quantifier(det_lower.as_str()) {
            return None;
        }

        let noun_info = noun_token.word_info.as_ref()?;
        if noun_info.category != WordCategory::Sustantivo {
            return None;
        }

        let noun_lower = Self::normalize_spanish_word(noun_token.effective_text());
        let noun_is_plural = noun_info.number == Number::Plural
            || (noun_info.number == Number::None && Self::looks_like_plural_noun_surface(&noun_lower));
        if !noun_is_plural {
            return None;
        }

        let suggestion_raw =
            Self::build_spanish_singular_noun_suggestion(noun_token.effective_text(), dictionary)?;
        if suggestion_raw.to_lowercase() == noun_lower {
            return None;
        }

        Some(GrammarCorrection {
            token_index: noun_idx,
            original: noun_token.text.clone(),
            suggestion: Self::preserve_initial_case(&noun_token.text, &suggestion_raw),
            rule_id: rule.id.0.clone(),
            message: format!(
                "Concordancia con 'ningún/ninguna': '{}' debería ser '{}'",
                noun_token.text, suggestion_raw
            ),
        })
    }

    fn spanish_plural_noun_candidates(noun_lower: &str) -> Vec<String> {
        if noun_lower.is_empty() {
            return Vec::new();
        }

        if noun_lower.ends_with('z') {
            let stem = &noun_lower[..noun_lower.len() - 'z'.len_utf8()];
            return vec![format!("{stem}ces")];
        }

        if noun_lower.ends_with('s') || noun_lower.ends_with('x') {
            return Vec::new();
        }

        if noun_lower.ends_with('í') || noun_lower.ends_with('ú') {
            return vec![format!("{noun_lower}es"), format!("{noun_lower}s")];
        }

        if matches!(
            noun_lower.chars().last(),
            Some('a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'ó')
        ) {
            return vec![format!("{noun_lower}s")];
        }

        vec![format!("{noun_lower}es")]
    }

    fn looks_like_plural_noun_surface(word_lower: &str) -> bool {
        word_lower.len() > 2 && word_lower.ends_with('s')
    }

    fn normalize_spanish_word(word: &str) -> String {
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

    fn extract_nominal_subject_features(
        token: &Token,
        language: &dyn Language,
    ) -> Option<(Gender, Number)> {
        let mut features = None;
        if let Some(subject_info) = token.word_info.as_ref() {
            if matches!(
                subject_info.category,
                WordCategory::Sustantivo | WordCategory::Pronombre
            ) && subject_info.gender != Gender::None
                && subject_info.number != Number::None
            {
                features = Some((subject_info.gender, subject_info.number));
            }
        }

        if features.is_none() {
            let lower = Self::normalize_spanish_word(token.effective_text());
            features = match lower.as_str() {
                // Pronombres personales sin marca léxica de género:
                // usamos masculino genérico para evitar forzar femenino
                // en coordinaciones mixtas tipo "yo/tú/usted + ella".
                "yo" | "tu" | "tú" | "usted" | "vos" => {
                    Some((Gender::Masculine, Number::Singular))
                }
                "ustedes" => Some((Gender::Masculine, Number::Plural)),
                _ => None,
            };
        }

        if language.code() == "es"
            && token
                .effective_text()
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
        {
            if let Some(gender) =
                crate::languages::spanish::get_name_gender(token.effective_text())
            {
                features = Some((gender, Number::Singular));
            }
        }
        features
    }

    fn merge_coordinated_subject_gender(left_gender: Gender, right_gender: Gender) -> Gender {
        match (left_gender, right_gender) {
            (Gender::Feminine, Gender::Feminine) => Gender::Feminine,
            (Gender::None, Gender::Feminine) | (Gender::Feminine, Gender::None) => {
                Gender::Feminine
            }
            (Gender::None, Gender::None) => Gender::None,
            _ => Gender::Masculine,
        }
    }

    fn coordinated_subject_left_features(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        subject_pos: usize,
        language: &dyn Language,
    ) -> Option<(Gender, Number)> {
        let left_pos = Self::coordinated_subject_left_pos(tokens, word_tokens, subject_pos)?;
        let (_, left_token) = word_tokens[left_pos];
        Self::extract_nominal_subject_features(left_token, language)
            .or(Some((Gender::None, Number::Singular)))
    }

    fn coordinated_subject_left_pos(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        subject_pos: usize,
    ) -> Option<usize> {
        if subject_pos < 2 {
            return None;
        }

        let (subject_idx, _) = word_tokens[subject_pos];
        let mut coord_pos = None;
        let mut right_idx = subject_idx;

        let mut probe = subject_pos as isize - 1;
        while probe >= 0 {
            let (idx, token) = word_tokens[probe as usize];
            if has_sentence_boundary(tokens, idx, right_idx)
                || Self::has_non_whitespace_between(tokens, idx, right_idx)
            {
                break;
            }

            let lower = token.effective_text().to_lowercase();
            if Self::is_coordination_conjunction(lower.as_str()) {
                coord_pos = Some(probe as usize);
                break;
            }

            if Self::is_nominal_bridge_token(token) {
                right_idx = idx;
                probe -= 1;
                continue;
            }

            break;
        }

        let coord_pos = coord_pos?;
        if coord_pos == 0 {
            return None;
        }

        let (coord_idx, _) = word_tokens[coord_pos];
        let mut left_probe = coord_pos as isize - 1;
        let mut left_right_idx = coord_idx;
        while left_probe >= 0 {
            let (idx, token) = word_tokens[left_probe as usize];
            if has_sentence_boundary(tokens, idx, left_right_idx)
                || Self::has_non_whitespace_between(tokens, idx, left_right_idx)
            {
                break;
            }

            if Self::is_nominal_or_personal_pronoun(token) || Self::is_likely_proper_name(token) {
                return Some(left_probe as usize);
            }

            if Self::is_nominal_bridge_token(token) {
                left_right_idx = idx;
                left_probe -= 1;
                continue;
            }

            break;
        }

        None
    }

    fn is_nominal_or_personal_pronoun(token: &Token) -> bool {
        if token
            .word_info
            .as_ref()
            .map(|info| matches!(info.category, WordCategory::Sustantivo | WordCategory::Pronombre))
            .unwrap_or(false)
        {
            return true;
        }

        let lower = token.effective_text().to_lowercase();
        matches!(
            lower.as_str(),
            "yo"
                | "tu"
                | "tú"
                | "el"
                | "él"
                | "ella"
                | "ello"
                | "usted"
                | "ustedes"
                | "vos"
                | "vosotros"
                | "vosotras"
                | "nosotros"
                | "nosotras"
                | "ellos"
                | "ellas"
        )
    }

    fn is_coordination_conjunction(word: &str) -> bool {
        matches!(word, "y" | "e" | "o" | "u" | "ni" | "como")
    }

    fn is_nominal_bridge_token(token: &Token) -> bool {
        if Self::is_determiner_like(token) {
            return true;
        }

        token
            .word_info
            .as_ref()
            .map(|info| {
                matches!(
                    info.category,
                    WordCategory::Articulo
                        | WordCategory::Determinante
                        | WordCategory::Adjetivo
                        | WordCategory::Pronombre
                )
            })
            .unwrap_or(false)
    }

    fn is_likely_proper_name(token: &Token) -> bool {
        let text = token.effective_text();
        let Some(first) = text.chars().next() else {
            return false;
        };
        if !first.is_uppercase() {
            return false;
        }

        if token.word_info.as_ref().is_some_and(|info| {
            matches!(
                info.category,
                WordCategory::Articulo
                    | WordCategory::Determinante
                    | WordCategory::Conjuncion
                    | WordCategory::Preposicion
            )
        }) {
            return false;
        }

        let lower = text.to_lowercase();
        if matches!(
            lower.as_str(),
            "el" | "la" | "los" | "las" | "un" | "una" | "unos" | "unas"
        ) {
            return false;
        }

        text.chars()
            .all(|c| c.is_alphabetic() || c == '-' || c == '\'')
    }

    fn is_quantified_temporal_complement_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        subject_pos: usize,
        language: &dyn Language,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if subject_pos < 3 {
            return false;
        }

        let (subject_idx, subject_token) = word_tokens[subject_pos];
        let subject_lower = subject_token.effective_text().to_lowercase();
        if !Self::is_common_temporal_noun(&subject_lower) {
            return false;
        }

        let (det_idx, det_token) = word_tokens[subject_pos - 1];
        let (quant_idx, quant_token) = word_tokens[subject_pos - 2];
        let (prev_idx, prev_token) = word_tokens[subject_pos - 3];

        if has_sentence_boundary(tokens, prev_idx, quant_idx)
            || has_sentence_boundary(tokens, quant_idx, det_idx)
            || has_sentence_boundary(tokens, det_idx, subject_idx)
            || Self::has_non_whitespace_between(tokens, prev_idx, quant_idx)
            || Self::has_non_whitespace_between(tokens, quant_idx, det_idx)
            || Self::has_non_whitespace_between(tokens, det_idx, subject_idx)
        {
            return false;
        }

        let det_lower = det_token.effective_text().to_lowercase();
        let quant_lower = quant_token.effective_text().to_lowercase();
        if !matches!(det_lower.as_str(), "el" | "la" | "los" | "las") {
            return false;
        }
        if !matches!(quant_lower.as_str(), "todo" | "toda" | "todos" | "todas") {
            return false;
        }

        let prev_lower = prev_token.effective_text().to_lowercase();
        if language.is_preposition(&prev_lower) {
            return true;
        }

        prev_token
            .word_info
            .as_ref()
            .map(|info| info.category == WordCategory::Verbo)
            .unwrap_or(false)
            || verb_recognizer
                .map(|vr| vr.is_valid_verb_form(prev_token.effective_text()))
            .unwrap_or(false)
            || Self::looks_like_past_finite_verb(&prev_lower)
    }

    fn is_prepositional_phrase_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        subject_pos: usize,
        language: &dyn Language,
    ) -> bool {
        if subject_pos == 0 {
            return false;
        }

        let (subject_idx, _) = word_tokens[subject_pos];
        let mut probe_pos = subject_pos as isize - 1;
        let mut right_idx = subject_idx;

        while probe_pos >= 0 {
            let (left_idx, left_token) = word_tokens[probe_pos as usize];
            if has_sentence_boundary(tokens, left_idx, right_idx)
                || Self::has_non_whitespace_between(tokens, left_idx, right_idx)
            {
                break;
            }

            let left_lower = left_token.effective_text().to_lowercase();
            if language.is_preposition(&left_lower) {
                return true;
            }

            let bridge_ok = Self::is_determiner_like(left_token)
                || left_token
                    .word_info
                    .as_ref()
                    .map(|info| {
                        matches!(
                            info.category,
                            WordCategory::Articulo
                                | WordCategory::Determinante
                                | WordCategory::Adjetivo
                                | WordCategory::Adverbio
                                | WordCategory::Pronombre
                        )
                    })
                    .unwrap_or(false)
                || left_token.token_type == TokenType::Number;

            if !bridge_ok {
                break;
            }

            right_idx = left_idx;
            probe_pos -= 1;
        }

        false
    }

    fn is_de_complement_nominal_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        subject_pos: usize,
    ) -> bool {
        // Patrón: "núcleo de X"
        if subject_pos >= 2 {
            let (subject_idx, _) = word_tokens[subject_pos];
            let (de_idx, de_token) = word_tokens[subject_pos - 1];
            let (head_idx, head_token) = word_tokens[subject_pos - 2];

            if de_token.effective_text().to_lowercase() == "de"
                && !has_sentence_boundary(tokens, head_idx, de_idx)
                && !has_sentence_boundary(tokens, de_idx, subject_idx)
                && !Self::has_non_whitespace_between(tokens, head_idx, de_idx)
                && !Self::has_non_whitespace_between(tokens, de_idx, subject_idx)
                && Self::is_nominal_or_personal_pronoun(head_token)
            {
                return true;
            }
        }

        // Patrón: "núcleo de det X"
        if subject_pos >= 3 {
            let (subject_idx, _) = word_tokens[subject_pos];
            let (det_idx, det_token) = word_tokens[subject_pos - 1];
            let (de_idx, de_token) = word_tokens[subject_pos - 2];
            let (head_idx, head_token) = word_tokens[subject_pos - 3];

            if de_token.effective_text().to_lowercase() == "de"
                && Self::is_determiner_like(det_token)
                && !has_sentence_boundary(tokens, head_idx, de_idx)
                && !has_sentence_boundary(tokens, de_idx, det_idx)
                && !has_sentence_boundary(tokens, det_idx, subject_idx)
                && !Self::has_non_whitespace_between(tokens, head_idx, de_idx)
                && !Self::has_non_whitespace_between(tokens, de_idx, det_idx)
                && !Self::has_non_whitespace_between(tokens, det_idx, subject_idx)
                && Self::is_nominal_or_personal_pronoun(head_token)
            {
                return true;
            }
        }

        false
    }

    fn is_common_temporal_noun(word_lower: &str) -> bool {
        let normalized = word_lower
            .replace('á', "a")
            .replace('é', "e")
            .replace('í', "i")
            .replace('ó', "o")
            .replace('ú', "u")
            .replace('ñ', "n");

        matches!(
            normalized.as_str(),
            "dia"
                | "dias"
                | "noche"
                | "noches"
                | "tarde"
                | "tardes"
                | "manana"
                | "mananas"
                | "semana"
                | "semanas"
                | "mes"
                | "meses"
                | "ano"
                | "anos"
                | "hora"
                | "horas"
                | "minuto"
                | "minutos"
                | "segundo"
                | "segundos"
                | "momento"
                | "momentos"
                | "temporada"
                | "temporadas"
                | "carrera"
                | "carreras"
                | "jornada"
                | "jornadas"
                | "epoca"
                | "epocas"
        )
    }

    fn looks_like_past_finite_verb(word_lower: &str) -> bool {
        let normalized = word_lower
            .replace('á', "a")
            .replace('é', "e")
            .replace('í', "i")
            .replace('ó', "o")
            .replace('ú', "u");

        matches!(
            normalized.as_str(),
            w if w.ends_with("aba")
                || w.ends_with("abas")
                || w.ends_with("aban")
                || w.ends_with("ia")
                || w.ends_with("ias")
                || w.ends_with("ian")
                || w.ends_with("aste")
                || w.ends_with("iste")
                || w.ends_with("amos")
                || w.ends_with("emos")
                || w.ends_with("imos")
                || w.ends_with("aron")
                || w.ends_with("yeron")
                || w.ends_with("ieron")
        )
    }

    fn is_postposed_relative_clause_subject(
        tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        subject_pos: usize,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if subject_pos < 3 {
            return false;
        }

        let (subject_idx, _) = word_tokens[subject_pos];
        let (det_idx, det_token) = word_tokens[subject_pos - 1];
        if !Self::is_determiner_like(det_token) {
            return false;
        }

        let mut rel_verb_pos = subject_pos - 2;
        let mut bridge_adverb_pos: Option<usize> = None;
        if Self::is_relative_clause_bridge_adverb(word_tokens[rel_verb_pos].1) {
            if rel_verb_pos == 0 {
                return false;
            }
            bridge_adverb_pos = Some(rel_verb_pos);
            rel_verb_pos -= 1;
        }
        if rel_verb_pos == 0 {
            return false;
        }

        let (rel_verb_idx, rel_verb_token) = word_tokens[rel_verb_pos];
        let (que_idx, que_token) = word_tokens[rel_verb_pos - 1];
        if que_token.effective_text().to_lowercase() != "que" {
            return false;
        }

        if let Some(adv_pos) = bridge_adverb_pos {
            let (adv_idx, _) = word_tokens[adv_pos];
            if has_sentence_boundary(tokens, que_idx, rel_verb_idx)
                || has_sentence_boundary(tokens, rel_verb_idx, adv_idx)
                || has_sentence_boundary(tokens, adv_idx, det_idx)
                || has_sentence_boundary(tokens, det_idx, subject_idx)
                || Self::has_non_whitespace_between(tokens, que_idx, rel_verb_idx)
                || Self::has_non_whitespace_between(tokens, rel_verb_idx, adv_idx)
                || Self::has_non_whitespace_between(tokens, adv_idx, det_idx)
                || Self::has_non_whitespace_between(tokens, det_idx, subject_idx)
            {
                return false;
            }
        } else if has_sentence_boundary(tokens, que_idx, rel_verb_idx)
            || has_sentence_boundary(tokens, rel_verb_idx, det_idx)
            || has_sentence_boundary(tokens, det_idx, subject_idx)
            || Self::has_non_whitespace_between(tokens, que_idx, rel_verb_idx)
            || Self::has_non_whitespace_between(tokens, rel_verb_idx, det_idx)
            || Self::has_non_whitespace_between(tokens, det_idx, subject_idx)
        {
            return false;
        }

        let rel_verb_lower = rel_verb_token.effective_text().to_lowercase();
        rel_verb_token
            .word_info
            .as_ref()
            .map(|info| info.category == WordCategory::Verbo)
            .unwrap_or(false)
            || verb_recognizer
                .map(|vr| vr.is_valid_verb_form(rel_verb_token.effective_text()))
                .unwrap_or(false)
            || Self::looks_like_past_finite_verb(&rel_verb_lower)
    }

    fn is_determiner_like(token: &Token) -> bool {
        token
            .word_info
            .as_ref()
            .map(|info| info.category == WordCategory::Determinante)
            .unwrap_or(false)
            || matches!(
                token.effective_text().to_lowercase().as_str(),
                "el"
                    | "la"
                    | "los"
                    | "las"
                    | "un"
                    | "una"
                    | "unos"
                    | "unas"
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
                    | "todo"
                    | "toda"
                    | "todos"
                    | "todas"
                    | "mucho"
                    | "mucha"
                    | "muchos"
                    | "muchas"
                    | "poco"
                    | "poca"
                    | "pocos"
                    | "pocas"
                    | "vario"
                    | "varia"
                    | "varios"
                    | "varias"
                    | "demasiado"
                    | "demasiada"
                    | "demasiados"
                    | "demasiadas"
                    | "otro"
                    | "otra"
                    | "otros"
                    | "otras"
                    | "cierto"
                    | "cierta"
                    | "ciertos"
                    | "ciertas"
                    | "algun"
                    | "algún"
                    | "alguno"
                    | "alguna"
                    | "algunos"
                    | "algunas"
                    | "ningun"
                    | "ningún"
                    | "ninguno"
                    | "ninguna"
                    | "ningunos"
                    | "ningunas"
            )
    }

    fn is_relative_clause_bridge_adverb(token: &Token) -> bool {
        let lower = token.effective_text().to_lowercase();
        token
            .word_info
            .as_ref()
            .map(|info| info.category == WordCategory::Adverbio)
            .unwrap_or(false)
            || lower.ends_with("mente")
            || matches!(
                lower.as_str(),
                "ayer"
                    | "hoy"
                    | "manana"
                    | "mañana"
                    | "anoche"
                    | "antes"
                    | "despues"
                    | "después"
                    | "luego"
                    | "entonces"
                    | "ya"
                    | "siempre"
                    | "nunca"
                    | "todavia"
                    | "todavía"
            )
    }

    /// Checks if there's a sentence/phrase boundary between tokens in a window
    /// Uses the unified has_sentence_boundary() plus comma (which separates list items)
    fn has_sentence_boundary_between(
        &self,
        all_tokens: &[Token],
        window: &[(usize, &Token)],
    ) -> bool {
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

    /// Devuelve true si hay algo distinto de espacios entre dos índices de palabra.
    /// Se usa para exigir secuencias limpias tipo "sustantivo verbo adjetivo".
    fn has_non_whitespace_between(all_tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        let (start, end) = if start_idx < end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };
        for token in &all_tokens[(start + 1)..end] {
            if token.token_type != TokenType::Whitespace {
                return true;
            }
        }
        false
    }

    /// Checks if there's a sentence boundary between tokens, ignoring quote marks.
    fn has_sentence_boundary_except_quotes(
        all_tokens: &[Token],
        start_idx: usize,
        end_idx: usize,
    ) -> bool {
        let (start, end) = if start_idx < end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };

        for i in (start + 1)..end {
            if all_tokens[i].is_sentence_boundary() {
                let text = all_tokens[i].text.as_str();
                if matches!(
                    text,
                    "\"" | "\u{201C}" | "\u{201D}" | "\u{00BB}" | "\u{00AB}"
                ) {
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

    /// Devuelve true cuando el cambio de artículo altera solo género (mismo número y definitud).
    fn is_pure_gender_article_swap(
        current: &str,
        suggested: &str,
        language: &dyn Language,
    ) -> bool {
        let Some((curr_def, curr_num, curr_gender)) = language.article_features(current) else {
            return false;
        };
        let Some((sugg_def, sugg_num, sugg_gender)) = language.article_features(suggested) else {
            return false;
        };

        curr_def == sugg_def && curr_num == sugg_num && curr_gender != sugg_gender
    }

    /// Devuelve true cuando el cambio de determinante altera solo género
    /// (misma familia y mismo número).
    fn is_pure_gender_determiner_swap(
        current: &str,
        suggested: &str,
        language: &dyn Language,
    ) -> bool {
        let Some((curr_family, curr_num, curr_gender)) = language.determiner_features(current)
        else {
            return false;
        };
        let Some((sugg_family, sugg_num, sugg_gender)) = language.determiner_features(suggested)
        else {
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
        let Some((sugg_num, sugg_gender, sugg_stem)) = Self::adjective_oa_features(suggested)
        else {
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

    fn preserve_initial_case(original: &str, replacement: &str) -> String {
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

    fn is_copulative_predicative_verb(
        verb_token: &Token,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let verb_lower = verb_token.effective_text().to_lowercase();
        let verb_norm = verb_lower
            .replace('á', "a")
            .replace('é', "e")
            .replace('í', "i")
            .replace('ó', "o")
            .replace('ú', "u");

        if let Some(vr) = verb_recognizer {
            if let Some(infinitive) = vr.get_infinitive(&verb_lower) {
                let inf_lower = infinitive.to_lowercase();
                if matches!(
                    inf_lower.as_str(),
                    "ser" | "estar" | "parecer" | "quedar" | "resultar" | "permanecer" | "seguir"
                ) {
                    return true;
                }
            }
        }

        matches!(
            verb_norm.as_str(),
            // ser
            "es"
                | "son"
                | "era"
                | "eran"
                | "fue"
                | "fueron"
                | "sera"
                | "seran"
                | "seria"
                | "serian"
                | "sea"
                | "sean"
                | "fuera"
                | "fueran"
                | "fuese"
                | "fuesen"
                // estar
                | "esta"
                | "estan"
                | "estaba"
                | "estaban"
                | "estuvo"
                | "estuvieron"
                | "estara"
                | "estaran"
                | "estaria"
                | "estarian"
                | "este"
                | "esten"
                | "estuviera"
                | "estuvieran"
                | "estuviese"
                | "estuviesen"
                // parecer
                | "parece"
                | "parecen"
                | "parecia"
                | "parecian"
                | "parecio"
                | "parecieron"
                | "parecera"
                | "pareceran"
                | "pareceria"
                | "parecerian"
                | "parezca"
                | "parezcan"
                // quedar
                | "queda"
                | "quedan"
                | "quedaba"
                | "quedaban"
                | "quedo"
                | "quedaron"
                | "quedara"
                | "quedaran"
                | "quedaria"
                | "quedarian"
                | "quede"
                | "queden"
                | "quedase"
                | "quedasen"
                // resultar
                | "resulta"
                | "resultan"
                | "resultaba"
                | "resultaban"
                | "resulto"
                | "resultaron"
                | "resultara"
                | "resultaran"
                | "resultaria"
                | "resultarian"
                | "resulte"
                | "resulten"
                | "resultase"
                | "resultasen"
                // permanecer
                | "permanece"
                | "permanecen"
                | "permanecia"
                | "permanecian"
                | "permanecio"
                | "permanecieron"
                | "permanecera"
                | "permaneceran"
                | "permaneceria"
                | "permanecerian"
                | "permanezca"
                | "permanezcan"
                | "permaneciera"
                | "permanecieran"
                | "permaneciese"
                | "permaneciesen"
                // seguir
                | "sigue"
                | "siguen"
                | "seguia"
                | "seguian"
                | "siguio"
                | "siguieron"
                | "seguira"
                | "seguiran"
                | "seguiria"
                | "seguirian"
                | "siga"
                | "sigan"
                | "siguiera"
                | "siguieran"
                | "siguiese"
                | "siguiesen"
        )
    }

    /// Checks if the next word token after `from_idx` is an article or determiner.
    /// Used to detect verbal context (verb + direct object) like "corta el paso".
    fn next_word_is_article_or_det(tokens: &[Token], from_idx: usize) -> bool {
        for i in (from_idx + 1)..tokens.len() {
            let t = &tokens[i];
            if t.is_sentence_boundary() {
                return false;
            }
            if t.token_type != TokenType::Word {
                continue; // skip whitespace
            }
            // First word token: is it an article or determiner?
            return t
                .word_info
                .as_ref()
                .map(|info| {
                    info.category == WordCategory::Articulo
                        || info.category == WordCategory::Determinante
                })
                .unwrap_or(false);
        }
        false
    }

    fn next_word_in_clause(tokens: &[Token], idx: usize) -> Option<usize> {
        if idx + 1 >= tokens.len() {
            return None;
        }
        for i in (idx + 1)..tokens.len() {
            let t = &tokens[i];
            if t.is_sentence_boundary() {
                return None;
            }
            if t.token_type == TokenType::Word {
                return Some(i);
            }
        }
        None
    }

    fn is_likely_finite_verb_after_feminine_clitic(
        word: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if verb_recognizer.is_some_and(|vr| vr.is_valid_verb_form(word)) {
            return true;
        }

        let lower = word.to_lowercase();
        if lower.ends_with('\u{00E9}') || lower.ends_with('\u{00ED}') {
            return true;
        }

        matches!(
            Self::normalize_spanish_word(word).as_str(),
            "traje"
                | "dije"
                | "hice"
                | "puse"
                | "tuve"
                | "vine"
                | "fui"
                | "vi"
                | "di"
        )
    }

    fn is_likely_finite_verb_after_feminine_clitic_with_category(
        word: &str,
        word_category: Option<WordCategory>,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let lower = word.to_lowercase();
        if lower.ends_with('\u{00E9}') || lower.ends_with('\u{00ED}') {
            return true;
        }

        let normalized = Self::normalize_spanish_word(word);
        let is_irregular_strong_verb = matches!(
            normalized.as_str(),
            "traje"
                | "dije"
                | "hice"
                | "puse"
                | "tuve"
                | "vine"
                | "fui"
                | "vi"
                | "di"
        );
        if is_irregular_strong_verb {
            return true;
        }

        if word_category == Some(WordCategory::Sustantivo) {
            return false;
        }

        Self::is_likely_finite_verb_after_feminine_clitic(word, verb_recognizer)
    }

    fn is_sentence_initial_feminine_clitic_context(
        tokens: &[Token],
        idx1: usize,
        idx2: usize,
        clitic_token: &Token,
        verb_like_token: &Token,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let clitic_lower = Self::normalize_spanish_word(clitic_token.effective_text());
        if !matches!(clitic_lower.as_str(), "la" | "las") {
            return false;
        }
        if !Self::is_sentence_start_word(tokens, idx1) {
            return false;
        }
        if !Self::is_likely_finite_verb_after_feminine_clitic_with_category(
            verb_like_token.effective_text(),
            verb_like_token.word_info.as_ref().map(|info| info.category),
            verb_recognizer,
        ) {
            return false;
        }

        let Some(next_idx) = Self::next_word_in_clause(tokens, idx2) else {
            return false;
        };
        if has_sentence_boundary(tokens, idx2, next_idx)
            || Self::has_non_whitespace_between(tokens, idx2, next_idx)
        {
            return false;
        }

        let next_token = &tokens[next_idx];
        let next_lower = Self::normalize_spanish_word(next_token.effective_text());
        if matches!(
            next_lower.as_str(),
            "a" | "de" | "que" | "si" | "no" | "ya" | "nunca" | "siempre"
        ) {
            return true;
        }

        if next_token.word_info.as_ref().is_some_and(|info| {
            matches!(
                info.category,
                WordCategory::Articulo
                    | WordCategory::Determinante
                    | WordCategory::Sustantivo
                    | WordCategory::Preposicion
                    | WordCategory::Pronombre
                    | WordCategory::Adverbio
            )
        }) {
            return true;
        }

        false
    }

    fn is_sentence_initial_el_pronoun_context(
        tokens: &[Token],
        idx1: usize,
        idx2: usize,
        verb_like_token: &Token,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if !Self::is_sentence_start_word(tokens, idx1) {
            return false;
        }

        let verb_lower = verb_like_token.effective_text().to_lowercase();
        if !Self::is_likely_el_predicate_verb_with_category(
            verb_lower.as_str(),
            verb_like_token.word_info.as_ref().map(|info| info.category),
            verb_recognizer,
        ) {
            return false;
        }

        let mut next_word_token: Option<&Token> = None;
        for i in (idx2 + 1)..tokens.len() {
            let t = &tokens[i];
            if t.is_sentence_boundary() {
                break;
            }
            if t.token_type == TokenType::Word {
                next_word_token = Some(t);
                break;
            }
        }

        let Some(next_token) = next_word_token else {
            return verb_like_token
                .word_info
                .as_ref()
                .is_none_or(|info| info.category != WordCategory::Sustantivo);
        };
        let next_lower = next_token.effective_text().to_lowercase();

        if Self::is_el_predicate_right_context_word(next_lower.as_str()) {
            return true;
        }

        if next_token.word_info.as_ref().is_some_and(|info| {
            matches!(
                info.category,
                WordCategory::Articulo
                    | WordCategory::Determinante
                    | WordCategory::Preposicion
                    | WordCategory::Pronombre
                    | WordCategory::Adverbio
            )
        }) {
            return true;
        }

        if Self::is_likely_transitive_el_predicate_verb(verb_lower.as_str())
            && next_token
                .word_info
                .as_ref()
                .is_some_and(|info| info.category == WordCategory::Sustantivo)
        {
            return true;
        }

        false
    }

    fn is_sentence_start_word(tokens: &[Token], idx: usize) -> bool {
        if idx == 0 {
            return true;
        }
        for i in (0..idx).rev() {
            let t = &tokens[i];
            if t.token_type == TokenType::Whitespace {
                continue;
            }
            if t.is_sentence_boundary() {
                return true;
            }
            if t.token_type == TokenType::Word {
                return false;
            }
        }
        true
    }

    fn previous_word_in_clause(tokens: &[Token], idx: usize) -> Option<usize> {
        if idx == 0 {
            return None;
        }
        for i in (0..idx).rev() {
            let t = &tokens[i];
            if t.is_sentence_boundary() {
                return None;
            }
            if t.token_type == TokenType::Word {
                return Some(i);
            }
        }
        None
    }

    fn is_likely_el_predicate_verb(
        word: &str,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        if verb_recognizer.is_some_and(|vr| vr.is_valid_verb_form(word)) {
            return true;
        }

        matches!(
            word,
            "es"
                | "era"
                | "fue"
                | "sabe"
                | "estudia"
                | "camina"
                | "juega"
                | "baila"
                | "nada"
                | "pinta"
                | "llama"
                | "pierde"
                | "duerme"
                | "cocina"
                | "cuenta"
                | "marcha"
                | "corta"
                | "limpia"
                | "busca"
                | "toca"
                | "gana"
                | "rie"
                | "ríe"
                | "llora"
        )
    }

    fn is_likely_el_predicate_verb_with_category(
        word: &str,
        word_category: Option<WordCategory>,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> bool {
        let in_curated_verb_list = matches!(
            word,
            "es"
                | "era"
                | "fue"
                | "sabe"
                | "estudia"
                | "camina"
                | "juega"
                | "baila"
                | "nada"
                | "pinta"
                | "llama"
                | "pierde"
                | "duerme"
                | "cocina"
                | "cuenta"
                | "marcha"
                | "corta"
                | "limpia"
                | "busca"
                | "toca"
                | "gana"
                | "rie"
                | "ríe"
                | "llora"
        );

        if word_category == Some(WordCategory::Sustantivo) && !in_curated_verb_list {
            return false;
        }

        Self::is_likely_el_predicate_verb(word, verb_recognizer)
    }

    fn is_likely_transitive_el_predicate_verb(word: &str) -> bool {
        matches!(
            word,
            "cocina"
                | "cuenta"
                | "corta"
                | "limpia"
                | "busca"
                | "toca"
                | "llama"
                | "pinta"
                | "estudia"
                | "gana"
        )
    }

    fn is_el_predicate_right_context_word(word: &str) -> bool {
        if word.ends_with("mente") {
            return true;
        }
        matches!(
            word,
            "no"
                | "ya"
                | "si"
                | "sí"
                | "que"
                | "quien"
                | "quién"
                | "quienes"
                | "quiénes"
                | "bien"
                | "mal"
                | "siempre"
                | "nunca"
                | "rapido"
                | "rápido"
                | "pronto"
                | "tarde"
                | "hoy"
                | "ayer"
                | "manana"
                | "mañana"
                | "mucho"
                | "poco"
        )
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
        language: &dyn Language,
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
            if let Some((_family, number, gender)) = language.determiner_features(&lower) {
                return Some((gender, number));
            }

            if let Some(ref info) = token.word_info {
                match info.category {
                    WordCategory::Adjetivo
                    | WordCategory::Determinante
                    | WordCategory::Articulo => {
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
                    WordCategory::Adjetivo
                    | WordCategory::Determinante
                    | WordCategory::Articulo => continue,
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
    fn has_coordinated_noun_before(
        word_tokens: &[(usize, &Token)],
        noun_pos: usize,
        language: &dyn Language,
    ) -> bool {
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
            if language.is_conjunction(&lower) {
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

    fn pattern_matches(
        &self,
        pattern: &[TokenPattern],
        window: &[(usize, &Token)],
        language: &dyn Language,
    ) -> bool {
        if pattern.len() != window.len() {
            return false;
        }

        for (pat, (_, token)) in pattern.iter().zip(window.iter()) {
            let matches = match pat {
                TokenPattern::Category(cat) => {
                    let category_matches = token
                        .word_info
                        .as_ref()
                        .map(|info| info.category == *cat)
                        .unwrap_or(false);
                    if category_matches {
                        true
                    } else if *cat == WordCategory::Determinante {
                        let lower = token.effective_text().to_lowercase();
                        language.determiner_features(&lower).is_some()
                    } else {
                        false
                    }
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

    /// Check if a word is a gerund (invariable verb form) using VerbRecognizer when available
    fn is_gerund(word: &str, verb_recognizer: Option<&dyn VerbFormRecognizer>) -> bool {
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
        dictionary: &Trie,
        language: &dyn Language,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
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

                            // Found nominal complement preposition - check if adjective agrees with
                            // noun before the prepositional phrase ("terapia contra ... basada").
                            if language.is_preposition(&word_lower) {
                                // Search for noun before "de", skipping adjectives/articles/determiners/numbers
                                let mut noun_pos = search_pos - 1;
                                let mut found_noun = false;

                                while noun_pos >= 0 {
                                    let noun_candidate = word_tokens[noun_pos as usize].1;

                                    if let Some(ref info) = noun_candidate.word_info {
                                        if info.category == WordCategory::Sustantivo {
                                            found_noun = true;
                                            // Check if adjective agrees with this earlier noun
                                            let adj_agrees = language
                                                .check_gender_agreement(noun_candidate, token2)
                                                && language
                                                    .check_number_agreement(noun_candidate, token2);
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
                                    || info.category == WordCategory::Determinante
                                {
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
                            && Self::has_coordinated_noun_before(word_tokens, window_pos, language)
                        {
                            return None;
                        }
                    }

                    // Skip distributive coordinated adjectives:
                    // "los sectores público y privado" = "el sector público y el sector privado"
                    // "los sectores público, privado y mixto"
                    // "los sectores público ni privado"
                    if let (Some(ref noun_info), Some(ref adj_info)) =
                        (&token1.word_info, &token2.word_info)
                    {
                        if noun_info.number == Number::Plural
                            && (adj_info.number == Number::Singular
                                || adj_info.number == Number::None)
                        {
                            let current_pos = window_pos + rule.pattern.len() - 1;
                            let mut pos = current_pos + 1;
                            let mut saw_following_adj = false;
                            let mut saw_conjunction = false;
                            while pos < word_tokens.len() {
                                let (tok_idx, tok) = word_tokens[pos];
                                if Self::has_sentence_boundary_except_quotes(tokens, *idx2, tok_idx)
                                {
                                    break;
                                }
                                let tok_lower = tok.text.to_lowercase();
                                if language.is_conjunction(&tok_lower) {
                                    saw_conjunction = true;
                                    if pos + 1 >= word_tokens.len() {
                                        break;
                                    }
                                    let (next_idx, next_adj) = word_tokens[pos + 1];
                                    if Self::has_sentence_boundary_except_quotes(
                                        tokens, *idx2, next_idx,
                                    ) {
                                        break;
                                    }
                                    if let Some(ref next_info) = next_adj.word_info {
                                        if next_info.category == WordCategory::Adjetivo
                                            && (next_info.number == Number::Singular
                                                || next_info.number == Number::None)
                                        {
                                            let gender_matches = adj_info.gender
                                                == noun_info.gender
                                                || adj_info.gender == Gender::None
                                                || noun_info.gender == Gender::None;
                                            let next_gender_matches = next_info.gender
                                                == noun_info.gender
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
                        if matches!(
                            adj_lower.as_str(),
                            "mínimo"
                                | "máximo"
                                | "mínima"
                                | "máxima"
                                | "mínimos"
                                | "máximos"
                                | "mínimas"
                                | "máximas"
                        ) {
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
                        let is_time_noun = language.is_time_noun(&noun_lower);
                        // Absolute participle clause: "una vez obtenidas las credenciales".
                        // Here the participle agrees with the following noun phrase, not with "vez".
                        let is_una_vez_absolute = matches!(noun_lower.as_str(), "vez" | "veces")
                            && {
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
                        let is_participle = language.is_participle_form(&adj_lower);
                        if (is_time_noun || is_una_vez_absolute) && is_participle {
                            return None; // Skip - participle agrees with implicit subject
                        }
                    }

                    // Skip partitive expressions: "uno de los", "una de las", etc.
                    // In "días uno de los accidentes", "uno" is not an adjective for "días"
                    if window_pos + 2 < word_tokens.len() {
                        let next_word = &word_tokens[window_pos + 2].1.text.to_lowercase();
                        let second_word = token2.text.to_lowercase();
                        let partitive_words = [
                            "uno",
                            "una",
                            "unos",
                            "unas",
                            "alguno",
                            "alguna",
                            "algunos",
                            "algunas",
                            "ninguno",
                            "ninguna",
                            "ningunos",
                            "ningunas",
                            "cualquiera",
                            "cualesquiera",
                            "cada",
                        ];
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
                                        let adj_agrees = language
                                            .check_gender_agreement(search_token, token2)
                                            && language
                                                .check_number_agreement(search_token, token2);
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
                        if let Some(correction) = Self::maybe_build_ningun_plural_noun_correction(
                            rule,
                            *idx2,
                            token1,
                            token2,
                            dictionary,
                        ) {
                            return Some(correction);
                        }
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
                                            let gender_matches = adj_info.gender
                                                == next_info.gender
                                                || adj_info.gender == Gender::None
                                                || next_info.gender == Gender::None;
                                            if gender_matches && adj_info.number == next_info.number
                                            {
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
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Option<GrammarCorrection> {
        match &rule.action {
            RuleAction::CorrectArticle => {
                // Corregir artículo según el sustantivo
                // Evitar "El + verbo" al inicio de oración (dejar que diacríticas maneje "el -> él").
                if Self::is_sentence_initial_feminine_clitic_context(
                    tokens,
                    idx1,
                    idx2,
                    token1,
                    token2,
                    verb_recognizer,
                ) {
                    return None;
                }
                if token1.effective_text().eq_ignore_ascii_case("el")
                    && Self::is_sentence_initial_el_pronoun_context(
                        tokens,
                        idx1,
                        idx2,
                        token2,
                        verb_recognizer,
                    )
                {
                    return None;
                }

                // Skip if noun is capitalized mid-sentence (likely a title or proper noun)
                // Example: "El Capital" (Marx's book), "La Odisea" (Homer's poem)
                if token2
                    .text
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
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
                    let correct = language.get_correct_article_for_noun(
                        noun,
                        info.gender,
                        info.number,
                        is_definite,
                    );
                    if !correct.is_empty() && correct != token1.text.to_lowercase() {
                        // Para sustantivos de género común (periodista, artista, etc.),
                        // no forzar cambios de artículo que solo alteran género sin referente explícito.
                        let current_article_lower = token1.effective_text().to_lowercase();
                        if language.is_common_gender_noun_form(noun)
                            && Self::is_pure_gender_article_swap(
                                &current_article_lower,
                                &correct,
                                language,
                            )
                        {
                            return None;
                        }

                        // Preservar mayúsculas si el original las tenía
                        let suggestion = if token1
                            .text
                            .chars()
                            .next()
                            .map(|c| c.is_uppercase())
                            .unwrap_or(false)
                        {
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
                // Evitar leer como SN el patrón "El + verbo + adverbio/adjetivo" al inicio:
                // "El marcha rapido", "El cocina bien".
                if let Some(prev_word_idx) = Self::previous_word_in_clause(tokens, idx1) {
                    let prev_token = &tokens[prev_word_idx];
                    if prev_token.effective_text().eq_ignore_ascii_case("el")
                        && Self::is_sentence_initial_el_pronoun_context(
                            tokens,
                            prev_word_idx,
                            idx1,
                            token1,
                            verb_recognizer,
                        )
                    {
                        return None;
                    }
                }

                // NOTA: Excluir adjetivos predicativos comunes que suelen concordar con el sujeto,
                // no con el sustantivo más cercano (ej: "fueron al parque juntos")
                // Adjetivos y participios que suelen usarse en función predicativa
                // después de verbos como "estar", "quedar", "resultar", "permanecer"
                // y NO deben corregirse para concordar con el sustantivo anterior
                let adj_lower = token2.text.to_lowercase();
                if language.is_predicative_adjective(&adj_lower) {
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
                    if vr.is_valid_verb_form(&adj_lower) && !language.is_participle_form(&adj_lower)
                    {
                        // If the noun is plural and the adj/verb is singular,
                        // a singular verb is impossible (subject-verb disagreement),
                        // so the word must be an adjective that needs correction.
                        // EXCEPT when followed by article/determiner (verbal context:
                        // "corta el paso" = verb + direct object).
                        let noun_is_plural = token1
                            .word_info
                            .as_ref()
                            .map(|info| info.number == Number::Plural)
                            .unwrap_or(false);
                        let noun_is_singular = token1
                            .word_info
                            .as_ref()
                            .map(|info| info.number == Number::Singular)
                            .unwrap_or(false);
                        let adj_is_singular_or_undetermined = token2
                            .word_info
                            .as_ref()
                            .map(|info| {
                                info.number == Number::Singular || info.number == Number::None
                            })
                            .unwrap_or(false);
                        let adj_is_plural = token2
                            .word_info
                            .as_ref()
                            .map(|info| info.number == Number::Plural)
                            .unwrap_or(false);

                        if (noun_is_plural && adj_is_singular_or_undetermined)
                            || (noun_is_singular && adj_is_plural)
                        {
                            let followed_by_det =
                                Self::next_word_is_article_or_det(tokens, idx2);
                            if followed_by_det {
                                return None; // Verbal context → don't correct as adjective
                            }
                            // No direct object → treat as adjective, fall through to correction
                        } else {
                            return None; // Noun is singular → keep original protection
                        }
                    }
                }

                // Skip if the adjective is capitalized mid-sentence (likely a proper name),
                // unless the whole sentence is ALL-CAPS (headlines should still be corrected).
                // Example: "Conferencia Severo Ochoa" - "Severo" is a proper name, not an adjective
                if token2
                    .text
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    // Check if it's not at the start of text (where capitalization is normal)
                    if idx2 > 0 && !Self::is_all_caps_sentence(tokens, idx2) {
                        return None; // Capitalized word mid-sentence = likely proper name
                    }
                }

                if let Some(ref noun_info) = token1.word_info {
                    if token1.text.is_ascii()
                        && Self::has_low_confidence_spelling_projection(token1)
                    {
                        if let Some((det_gender, det_number)) =
                            Self::infer_noun_features_from_left_determiner(tokens, idx1, language)
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

                    if let Some(correct) = language.get_adjective_form(
                        &token2.text,
                        noun_info.gender,
                        noun_info.number,
                    ) {
                        if correct.to_lowercase() != token2.text.to_lowercase() {
                            let noun_text = token1.effective_text().to_lowercase();
                            let current_adj_lower = token2.effective_text().to_lowercase();
                            let correct_adj_lower = correct.to_lowercase();

                            // Para sustantivos ambiguos por significado (p. ej. "el cólera"),
                            // no forzar cambios que alteren solo género en adjetivos.
                            if language.allows_both_gender_articles(&noun_text)
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
                if Self::is_demasiado_adverb_before_caras(tokens, idx1, token1, idx2, token2) {
                    return None;
                }
                if Self::is_sentence_initial_feminine_clitic_context(
                    tokens,
                    idx1,
                    idx2,
                    token1,
                    token2,
                    verb_recognizer,
                ) {
                    return None;
                }
                // Salvaguarda: si el determinante ya concuerda con el sustantivo siguiente,
                // no corregirlo aunque haya un sustantivo previo en una frase con preposición.
                if let Some(next_noun) = Self::find_next_noun_after(tokens, idx1) {
                    let gender_ok = language.check_gender_agreement(token1, next_noun);
                    let number_ok = language.check_number_agreement(token1, next_noun);
                    if gender_ok && number_ok {
                        return None;
                    }
                }
                let target_noun = if token2
                    .word_info
                    .as_ref()
                    .is_some_and(|info| info.category == WordCategory::Sustantivo)
                {
                    Some(token2)
                } else {
                    // Soporta cuantificador + artículo + sustantivo:
                    // "todas los niños", "todos las casas".
                    Self::find_next_noun_after(tokens, idx1)
                };

                if let Some(target_noun) = target_noun {
                    if Self::has_low_confidence_spelling_projection(target_noun) {
                        return None;
                    }
                    let Some(noun_info) = target_noun.word_info.as_ref() else {
                        return None;
                    };
                    if noun_info.category != WordCategory::Sustantivo {
                        return None;
                    }

                    if let Some(correct) = language.get_correct_determiner(
                        &token1.text,
                        noun_info.gender,
                        noun_info.number,
                    ) {
                        let noun_text = target_noun.effective_text().to_lowercase();
                        let current_det_lower = token1.effective_text().to_lowercase();

                        // Para sustantivos con género ambiguo por significado (p. ej. "cólera"),
                        // no forzar swaps que cambien solo género en determinantes.
                        if language.allows_both_gender_articles(&noun_text)
                            && Self::is_pure_gender_determiner_swap(
                                &current_det_lower,
                                &correct,
                                language,
                            )
                        {
                            return None;
                        }

                        if correct.to_lowercase() != token1.text.to_lowercase() {
                            // Preservar mayúsculas si el original las tenía
                            let suggestion = if token1
                                .text
                                .chars()
                                .next()
                                .map(|c| c.is_uppercase())
                                .unwrap_or(false)
                            {
                                let mut chars = correct.chars();
                                match chars.next() {
                                    Some(c) => {
                                        c.to_uppercase().collect::<String>() + chars.as_str()
                                    }
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
    use crate::languages::spanish::{Spanish, VerbRecognizer};

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
        assert!(
            det_correction.is_some(),
            "Debería encontrar corrección para 'este'"
        );
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
        assert!(
            det_correction.is_some(),
            "Debería encontrar corrección para 'esta'"
        );
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
        assert!(
            det_correction.is_some(),
            "Debería encontrar corrección para 'ese'"
        );
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
        assert!(
            det_correction.is_some(),
            "Debería encontrar corrección para 'aquel'"
        );
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
        assert!(
            det_correction.is_some(),
            "Debería encontrar corrección para 'nuestro'"
        );
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
        assert!(
            det_correction.is_none(),
            "No debería haber corrección para 'esta casa' que es correcto"
        );
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
        assert!(
            det_correction.is_some(),
            "Debería encontrar corrección para 'estos'"
        );
        assert_eq!(det_correction.unwrap().suggestion, "estas");
    }

    #[test]
    fn test_quantifier_article_noun_todas_los_ninos_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("todas los niños");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let quant_correction = corrections.iter().find(|c| c.original == "todas");
        assert!(
            quant_correction.is_some(),
            "Debería corregir cuantificador en 'todas los niños': {:?}",
            corrections
        );
        assert_eq!(quant_correction.unwrap().suggestion, "todos");
    }

    #[test]
    fn test_quantifier_article_noun_todos_las_casas_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("todos las casas");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let quant_correction = corrections.iter().find(|c| c.original == "todos");
        assert!(
            quant_correction.is_some(),
            "Debería corregir cuantificador en 'todos las casas': {:?}",
            corrections
        );
        assert_eq!(quant_correction.unwrap().suggestion, "todas");
    }

    #[test]
    fn test_quantifier_article_noun_mucho_adverb_not_forced() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("me gustan mucho los libros");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let mucho_correction = corrections
            .iter()
            .find(|c| c.rule_id == "quantifier_article_noun_agreement" && c.original == "mucho");
        assert!(
            mucho_correction.is_none(),
            "No debe corregir 'mucho' adverbial en 'me gustan mucho los libros': {:?}",
            corrections
        );
    }

    #[test]
    fn test_numeral_noun_singular_dos_libro_is_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("compre dos libro");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let noun_correction = corrections.iter().find(|c| c.original == "libro");
        assert!(
            noun_correction.is_some(),
            "DeberÃ­a corregir 'dos libro' -> 'dos libros': {:?}",
            corrections
        );
        assert_eq!(noun_correction.unwrap().suggestion, "libros");
    }

    #[test]
    fn test_numeral_noun_singular_tres_gato_is_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("hay tres gato");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let noun_correction = corrections.iter().find(|c| c.original == "gato");
        assert!(
            noun_correction.is_some(),
            "DeberÃ­a corregir 'tres gato' -> 'tres gatos': {:?}",
            corrections
        );
        assert_eq!(noun_correction.unwrap().suggestion, "gatos");
    }

    #[test]
    fn test_numeral_noun_plural_already_correct_no_change() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("hay tres gatos");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let noun_correction = corrections.iter().find(|c| c.original == "gatos");
        assert!(
            noun_correction.is_none(),
            "No deberÃ­a corregir cuando ya hay plural tras numeral: {:?}",
            corrections
        );
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
        let adj_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "personalizadas");
        assert!(
            adj_correction.is_none(),
            "No debe corregir adjetivo plural con sustantivos coordinados: {:?}",
            corrections
        );

        let mut tokens = tokenizer.tokenize("Un hombre y una mujer cansados");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
        let adj_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "cansados");
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
        let adj_correction = corrections
            .iter()
            .find(|c| c.original.to_lowercase() == "personalizadas");
        assert!(
            adj_correction.is_some(),
            "Debe seguir corrigiendo discordancia sin coordinación: {:?}",
            corrections
        );
        assert_eq!(
            adj_correction.unwrap().suggestion.to_lowercase(),
            "personalizada"
        );
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
        assert!(
            adj_correction.is_none(),
            "No debería corregir 'mismo' porque 'él' es pronombre, no sustantivo"
        );
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
        assert!(
            adj_correction.is_none(),
            "No debería corregir 'mismo' porque 'Él' es pronombre, no sustantivo"
        );
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

    #[test]
    fn test_copulative_predicative_adjective_agreement_corrections() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let cases = [
            ("La casa es bonito", "bonito", "bonita"),
            ("La casa es muy bonito", "bonito", "bonita"),
            ("Las paredes están sucios", "sucios", "sucias"),
            ("Mi madre está contento", "contento", "contenta"),
            ("La situación es complicado", "complicado", "complicada"),
            ("Estas camisas son rojos", "rojos", "rojas"),
            ("Los niños son traviesas", "traviesas", "traviesos"),
            ("El libro es cara", "cara", "caro"),
        ];

        for (text, wrong, expected) in cases {
            let mut tokens = tokenizer.tokenize(text);
            let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
            let adj_correction = corrections
                .iter()
                .find(|c| c.original.to_lowercase() == wrong);
            assert!(
                adj_correction.is_some(),
                "Debería corregir adjetivo predicativo en '{}': {:?}",
                text,
                corrections
            );
            assert_eq!(adj_correction.unwrap().suggestion.to_lowercase(), expected);
        }
    }

    #[test]
    fn test_copulative_predicative_adjective_agreement_correct_cases_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let cases = [
            "La casa es bonita",
            "La casa es muy bonita",
            "Las paredes están sucias",
            "Mi madre está contenta",
            "La situación es complicada",
            "Estas camisas son rojas",
            "En mi opinion es correcto",
            "En la actualidad es necesario",
            "Sin esta herramienta es complicado",
            "La lista de tareas está actualizada",
            "La actualización de los módulos es correcta",
            "Tanto yo como ella son buenos",
            "Los perros que ladraban toda la noche estan dormidos",
            "Los atletas que corrieron toda la carrera estan cansados",
            "Los estudiantes que leyeron toda la jornada estan agotados",
            "El informe que redactaron los técnicos durante toda la jornada parecía confuso",
            "La serie de cambios que propusieron ayer los técnicos es adecuada",
            "El acta que redactaron los técnicos es correcta",
            "La carta que escribieron los técnicos es buena",
        ];

        for text in cases {
            let mut tokens = tokenizer.tokenize(text);
            let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
            let predicative_correction = corrections
                .iter()
                .find(|c| c.rule_id == "es_copulative_predicative_adj_agreement");
            assert!(
                predicative_correction.is_none(),
                "No debería corregir caso predicativo ya correcto en '{}': {:?}",
                text,
                corrections
            );
        }
    }

    #[test]
    fn test_copulative_predicative_agreement_with_coordinated_subject_no_correction() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let cases = [
            "Pedro y Ana son guapos",
            "El nino y la nina estan cansados",
            "Mi padre y mi madre son buenos",
            "El pan y la leche estan caros",
            "Maria y Ana estan contentas",
        ];

        for text in cases {
            let mut tokens = tokenizer.tokenize(text);
            let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
            let predicative_correction = corrections
                .iter()
                .find(|c| c.rule_id == "es_copulative_predicative_adj_agreement");
            assert!(
                predicative_correction.is_none(),
                "No debe corregir concordancia predicativa con sujeto coordinado en '{}': {:?}",
                text,
                corrections
            );
        }
    }

    #[test]
    fn test_copulative_predicative_agreement_with_coordinated_subject_singular_is_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let cases = [
            ("Juan y María son simpático", "simpático", "simpáticos"),
            ("Pedro y Luis estan cansado", "cansado", "cansados"),
            ("Ana y Marta son guapa", "guapa", "guapas"),
            ("El perro y el gato estan contento", "contento", "contentos"),
            (
                "Mi padre y mi madre estan preocupado",
                "preocupado",
                "preocupados",
            ),
        ];

        for (text, wrong, expected) in cases {
            let mut tokens = tokenizer.tokenize(text);
            let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
            let predicative_correction = corrections.iter().find(|c| {
                c.rule_id == "es_copulative_predicative_adj_agreement"
                    && c.original.to_lowercase() == wrong
            });
            assert!(
                predicative_correction.is_some(),
                "Debe corregir adjetivo singular con sujeto coordinado en '{}': {:?}",
                text,
                corrections
            );
            assert_eq!(
                predicative_correction
                    .expect("correccion esperada")
                    .suggestion
                    .to_lowercase(),
                expected
            );
        }
    }

    #[test]
    fn test_sentence_initial_la_tema_not_treated_as_clitic_context() {
        let (dictionary, _language) = setup();
        let tokenizer = super::super::tokenizer::Tokenizer::new();
        let mut tokens = tokenizer.tokenize("La tema es interesante");

        for token in tokens.iter_mut() {
            if token.token_type != super::super::tokenizer::TokenType::Word {
                continue;
            }
            let lower = token.effective_text().to_lowercase();
            if let Some(info) = dictionary.get(&lower) {
                token.word_info = Some(info.clone());
            } else if let Some(info) = dictionary.derive_plural_info(&lower) {
                token.word_info = Some(info);
            }
        }

        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == super::super::tokenizer::TokenType::Word)
            .collect();

        let is_clitic = GrammarAnalyzer::is_sentence_initial_feminine_clitic_context(
            &tokens,
            word_tokens[0].0,
            word_tokens[1].0,
            word_tokens[0].1,
            word_tokens[1].1,
            None,
        );

        assert!(
            !is_clitic,
            "'La tema ...' no debe bloquear correccion articulo-sustantivo"
        );
        assert!(
            word_tokens[1]
                .1
                .word_info
                .as_ref()
                .is_some_and(|info| {
                    info.category == WordCategory::Sustantivo
                        && info.gender == Gender::Masculine
                }),
            "El token 'tema' debe quedar como sustantivo masculino"
        );
    }

    #[test]
    fn test_article_correction_la_tema_to_el_tema() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("La tema es interesante");
        for token in tokens.iter_mut() {
            if token.token_type != super::super::tokenizer::TokenType::Word {
                continue;
            }
            let lower = token.effective_text().to_lowercase();
            if let Some(info) = dictionary.get(&lower) {
                token.word_info = Some(info.clone());
            } else if let Some(info) = dictionary.derive_plural_info(&lower) {
                token.word_info = Some(info);
            }
        }
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == super::super::tokenizer::TokenType::Word)
            .collect();
        let gender_ok = language.check_gender_agreement(word_tokens[0].1, word_tokens[1].1);
        let allows_both = language.allows_both_gender_articles("tema");
        assert!(
            !gender_ok,
            "Debe haber discordancia de genero en 'La tema'. gender_ok={}, allows_both={}, izq={:?}, der={:?}",
            gender_ok,
            allows_both,
            word_tokens[0].1.word_info,
            word_tokens[1].1.word_info
        );

        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
        let article_correction = corrections.iter().find(|c| c.original == "La");

        assert!(
            article_correction.is_some(),
            "Debe corregir 'La tema' -> 'El tema'. Correcciones: {:?}",
            corrections
        );
        assert_eq!(
            article_correction
                .expect("correccion esperada")
                .suggestion
                .to_lowercase(),
            "el"
        );
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
        assert!(
            art_correction.is_some(),
            "Debería corregir 'la agua' a 'el agua'"
        );
        assert_eq!(art_correction.unwrap().suggestion, "el");
    }

    #[test]
    fn test_feminine_tonic_a_la_acta_correction() {
        // "la acta" es incorrecto, debe ser "el acta"
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("la acta");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "la");
        assert!(
            art_correction.is_some(),
            "Debería corregir 'la acta' a 'el acta'"
        );
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
        assert!(
            art_correction.is_some(),
            "Debería corregir 'una águila' a 'un águila'"
        );
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
        assert!(
            art_correction.is_none(),
            "No debería corregir 'el agua' que es correcto"
        );
    }

    #[test]
    fn test_feminine_tonic_a_el_acta_no_correction() {
        // "el acta" es correcto, NO debe corregirse
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens = tokenizer.tokenize("el acta");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let art_correction = corrections.iter().find(|c| c.original == "el");
        assert!(
            art_correction.is_none(),
            "No debería corregir 'el acta' que es correcto"
        );
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
        assert!(
            art_correction.is_none(),
            "No debería corregir 'un hacha' que es correcto"
        );
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

        let adj_correction = corrections
            .iter()
            .find(|c| c.original == "asi\u{00e1}ticos");
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
        assert!(
            art_correction.is_none(),
            "No debería corregir 'los 10 MB' - MB es unidad invariable"
        );
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
        assert!(
            art_correction.is_some(),
            "Debería corregir 'la 10 euros' a 'los 10 euros'"
        );
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
        assert!(
            art_correction.is_some(),
            "Debería corregir 'los 3 casas' a 'las 3 casas'"
        );
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
        let corrections =
            analyzer.analyze(&mut tokens, &dictionary, &language, Some(&verb_recognizer));

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
        let corrections =
            analyzer.analyze(&mut tokens, &dictionary, &language, Some(&verb_recognizer));

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
        assert!(
            adj_correction.is_some(),
            "Should correct adjective in all-caps text"
        );
        assert_eq!(adj_correction.unwrap().suggestion.to_lowercase(), "blanca");
    }

    #[test]
    fn test_participle_after_long_prep_phrase_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens =
            tokenizer.tokenize("las pensiones de clases pasivas del estado causadas en 2026");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);

        let adj_correction = corrections.iter().find(|c| c.original == "causadas");
        assert!(
            adj_correction.is_none(),
            "No debe corregir 'causadas' en este contexto"
        );
    }

    #[test]
    fn test_participle_after_contra_sobre_prep_phrase_not_corrected() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let samples = [
            "Una terapia contra el glioblastoma basada en protonterapia",
            "Una idea sobre el proyecto basada en la experiencia",
            "Una medida contra el terrorismo basada en la cooperación",
        ];

        for sample in samples {
            let mut tokens = tokenizer.tokenize(sample);
            let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
            let based_correction = corrections
                .iter()
                .find(|c| c.original.eq_ignore_ascii_case("basada"));
            assert!(
                based_correction.is_none(),
                "No debe corregir 'basada' cuando concuerda con el núcleo antes de la preposición en '{}': {:?}",
                sample,
                corrections
            );
        }
    }

    #[test]
    fn test_participle_after_prep_phrase_still_corrects_real_mismatch() {
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();

        let mut tokens =
            tokenizer.tokenize("Una terapia contra el glioblastoma basadas en protonterapia");
        let corrections = analyzer.analyze(&mut tokens, &dictionary, &language, None);
        let based_correction = corrections
            .iter()
            .find(|c| c.original.eq_ignore_ascii_case("basadas"));
        assert!(
            based_correction.is_some(),
            "Debe mantener corrección cuando el participio no concuerda con ningún núcleo: {:?}",
            corrections
        );
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
    fn test_noun_adj_number_agreement_with_verb_homograph() {
        // "larga" is a valid verb form (largar) but with a plural noun it must be an adjective
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenizer.tokenize("caminos larga");
        let corrections =
            analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));

        let adj_correction = corrections.iter().find(|c| c.original == "larga");
        assert!(
            adj_correction.is_some(),
            "Debe corregir 'larga' con sustantivo plural 'caminos': {:?}",
            corrections
        );
        assert_eq!(adj_correction.unwrap().suggestion, "largos");
    }

    #[test]
    fn test_noun_adj_verb_homograph_with_object() {
        // "corta el paso" looks like verb + direct object → should NOT correct
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenizer.tokenize("Los caminos corta el paso");
        let corrections =
            analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));

        let adj_correction = corrections.iter().find(|c| c.original == "corta");
        assert!(
            adj_correction.is_none(),
            "No debe corregir 'corta' cuando le sigue artículo (contexto verbal): {:?}",
            corrections
        );
    }

    #[test]
    fn test_noun_adj_plural_number_only_mismatch() {
        // "caminos largo" — same gender, only number mismatch, still a verb homograph (largar)
        let (dictionary, language) = setup();
        let analyzer = GrammarAnalyzer::with_rules(language.grammar_rules());
        let tokenizer = super::super::tokenizer::Tokenizer::new();
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        let mut tokens = tokenizer.tokenize("caminos largo");
        let corrections =
            analyzer.analyze(&mut tokens, &dictionary, &language, Some(&recognizer));

        let adj_correction = corrections.iter().find(|c| c.original == "largo");
        assert!(
            adj_correction.is_some(),
            "Debe corregir 'largo' con sustantivo plural 'caminos': {:?}",
            corrections
        );
        assert_eq!(adj_correction.unwrap().suggestion, "largos");
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




