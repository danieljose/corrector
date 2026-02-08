//! Reglas gramaticales del español

use crate::dictionary::WordCategory;
use crate::grammar::{GrammarRule, RuleAction, RuleCondition, TokenPattern};

/// Obtiene las reglas gramaticales del español
pub fn get_spanish_rules() -> Vec<GrammarRule> {
    vec![
        // Concordancia artículo-sustantivo en género
        GrammarRule::new(
            "es_art_noun_gender",
            "Concordancia artículo-sustantivo (género)",
            vec![
                TokenPattern::Category(WordCategory::Articulo),
                TokenPattern::Category(WordCategory::Sustantivo),
            ],
            RuleCondition::GenderMismatch,
            RuleAction::CorrectArticle,
        )
        .with_description("El artículo debe concordar en género con el sustantivo"),
        // Concordancia artículo-sustantivo en número
        GrammarRule::new(
            "es_art_noun_number",
            "Concordancia artículo-sustantivo (número)",
            vec![
                TokenPattern::Category(WordCategory::Articulo),
                TokenPattern::Category(WordCategory::Sustantivo),
            ],
            RuleCondition::NumberMismatch,
            RuleAction::CorrectArticle,
        )
        .with_description("El artículo debe concordar en número con el sustantivo"),
        // Concordancia sustantivo-adjetivo
        GrammarRule::new(
            "es_noun_adj_agreement",
            "Concordancia sustantivo-adjetivo",
            vec![
                TokenPattern::Category(WordCategory::Sustantivo),
                TokenPattern::Category(WordCategory::Adjetivo),
            ],
            RuleCondition::GenderAndNumberMismatch,
            RuleAction::CorrectAdjective,
        )
        .with_description("El adjetivo debe concordar en género y número con el sustantivo"),
        // Concordancia determinante-sustantivo
        GrammarRule::new(
            "es_det_noun_agreement",
            "Concordancia determinante-sustantivo",
            vec![
                TokenPattern::Category(WordCategory::Determinante),
                TokenPattern::Category(WordCategory::Sustantivo),
            ],
            RuleCondition::GenderAndNumberMismatch,
            RuleAction::CorrectDeterminer,
        )
        .with_description("El determinante debe concordar con el sustantivo"),
    ]
}
