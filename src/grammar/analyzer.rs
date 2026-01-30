//! Analizador gramatical

use crate::dictionary::{Trie, WordCategory};
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
        for token in tokens.iter_mut() {
            if token.token_type == TokenType::Word {
                if let Some(info) = dictionary.get(&token.text.to_lowercase()) {
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
            if self.pattern_matches(&rule.pattern, window) {
                if let Some(correction) =
                    self.check_condition_and_correct(rule, window, &word_tokens, window_pos, dictionary, language)
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

                    // Skip if the noun is in a prepositional phrase "de + noun"
                    // In "salsa de tomate casera", "casera" agrees with "salsa", not "tomate"
                    if window_pos >= 1 {
                        let prev_word = &word_tokens[window_pos - 1].1.text.to_lowercase();
                        if prev_word == "de" || prev_word == "del" || prev_word == "con" {
                            // Skip - the adjective likely modifies a previous noun
                            return None;
                        }
                    }

                    // Skip if the previous word is also a noun (compound noun pattern)
                    // In "baliza GPS colocada", "colocada" agrees with "baliza", not "GPS"
                    // In "sistema Windows instalado", "instalado" agrees with "sistema"
                    if window_pos >= 1 {
                        let prev_token = word_tokens[window_pos - 1].1;
                        if let Some(ref info) = prev_token.word_info {
                            if info.category == WordCategory::Sustantivo {
                                // Previous word is also a noun - adjective might agree with it instead
                                // Check if adjective agrees with the previous noun
                                let adj_agrees_with_prev = language.check_gender_agreement(prev_token, token2)
                                    && language.check_number_agreement(prev_token, token2);
                                if adj_agrees_with_prev {
                                    return None; // Skip - adjective agrees with earlier noun
                                }
                            }
                        }
                    }

                    let gender_ok = language.check_gender_agreement(token1, token2);
                    let number_ok = language.check_number_agreement(token1, token2);

                    if !gender_ok || !number_ok {
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
                if let Some(ref info) = token2.word_info {
                    let is_definite = matches!(
                        token1.text.to_lowercase().as_str(),
                        "el" | "la" | "los" | "las"
                    );
                    let correct = language.get_correct_article(info.gender, info.number, is_definite);
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
                ];
                let adj_lower = token2.text.to_lowercase();
                if predicative_adjectives.contains(&adj_lower.as_str()) {
                    // Skip - estos adjetivos frecuentemente no concuerdan con el sustantivo anterior
                    return None;
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
}
