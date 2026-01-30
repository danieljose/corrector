//! Sistema de reglas gramaticales

use crate::dictionary::WordCategory;

/// Identificador de regla
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuleId(pub String);

/// Patrón de tokens que debe coincidir
#[derive(Debug, Clone)]
pub enum TokenPattern {
    Category(WordCategory),
    Word(String),
    AnyWord,
}

/// Condición que debe cumplirse
#[derive(Debug, Clone)]
pub enum RuleCondition {
    GenderMismatch,
    NumberMismatch,
    GenderAndNumberMismatch,
    Custom(String),
}

/// Acción a tomar cuando se detecta un error
#[derive(Debug, Clone)]
pub enum RuleAction {
    CorrectArticle,
    CorrectAdjective,
    CorrectDeterminer,
    CorrectVerb,
    SuggestAlternative(String),
}

/// Regla gramatical
#[derive(Debug, Clone)]
pub struct GrammarRule {
    pub id: RuleId,
    pub name: String,
    pub description: String,
    pub pattern: Vec<TokenPattern>,
    pub condition: RuleCondition,
    pub action: RuleAction,
    pub enabled: bool,
}

impl GrammarRule {
    pub fn new(
        id: &str,
        name: &str,
        pattern: Vec<TokenPattern>,
        condition: RuleCondition,
        action: RuleAction,
    ) -> Self {
        Self {
            id: RuleId(id.to_string()),
            name: name.to_string(),
            description: String::new(),
            pattern,
            condition,
            action,
            enabled: true,
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }
}

/// Motor de reglas
pub struct RuleEngine {
    rules: Vec<GrammarRule>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: GrammarRule) {
        self.rules.push(rule);
    }

    pub fn add_rules(&mut self, rules: Vec<GrammarRule>) {
        self.rules.extend(rules);
    }

    pub fn get_rules(&self) -> &[GrammarRule] {
        &self.rules
    }

    pub fn get_enabled_rules(&self) -> Vec<&GrammarRule> {
        self.rules.iter().filter(|r| r.enabled).collect()
    }

    pub fn enable_rule(&mut self, id: &str) {
        for rule in &mut self.rules {
            if rule.id.0 == id {
                rule.enabled = true;
            }
        }
    }

    pub fn disable_rule(&mut self, id: &str) {
        for rule in &mut self.rules {
            if rule.id.0 == id {
                rule.enabled = false;
            }
        }
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
