//! Correccion de condicional irreal tras "si".
//!
//! Regla principal:
//! - "si + condicional" -> "si + subjuntivo imperfecto"
//!   Ej.: "Si tendria dinero, compraria..." -> "Si tuviera dinero, compraria..."
//!
//! Se aplica de forma conservadora:
//! - Solo en "si" de inicio de clausula condicional.
//! - Solo cuando la forma tras "si" se reconoce realmente como condicional.
//! - Se evita en "si" interrogativo indirecto ("no se si...").

use crate::grammar::tokenizer::TokenType;
use crate::grammar::Token;
use crate::languages::spanish::conjugation::stem_changing::{
    get_stem_changing_verbs, StemChangeType,
};
use crate::languages::VerbFormRecognizer;

#[derive(Debug, Clone)]
pub struct IrrealisConditionalCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
}

pub struct IrrealisConditionalAnalyzer;

impl IrrealisConditionalAnalyzer {
    pub fn analyze(
        tokens: &[Token],
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) -> Vec<IrrealisConditionalCorrection> {
        let Some(recognizer) = verb_recognizer else {
            return Vec::new();
        };

        let mut corrections = Vec::new();

        for i in 0..tokens.len() {
            if tokens[i].token_type != TokenType::Word {
                continue;
            }

            let word_lower = Self::token_text_for_analysis(&tokens[i]).to_lowercase();
            if word_lower != "si" {
                continue;
            }

            let is_como_si_intro = Self::is_como_si_intro(tokens, i);
            if !is_como_si_intro && !Self::is_conditional_si_intro(tokens, i) {
                continue;
            }

            if let Some((verb_idx, suggestion)) =
                Self::find_conditional_verb_in_clause(tokens, i, recognizer)
            {
                corrections.push(IrrealisConditionalCorrection {
                    token_index: verb_idx,
                    original: tokens[verb_idx].text.clone(),
                    suggestion: Self::preserve_case(&tokens[verb_idx].text, &suggestion),
                });
            }
        }

        corrections
    }

    fn is_como_si_intro(tokens: &[Token], si_idx: usize) -> bool {
        if si_idx == 0 {
            return false;
        }

        for i in (0..si_idx).rev() {
            let token = &tokens[i];
            if token.token_type == TokenType::Whitespace {
                continue;
            }

            if token.token_type == TokenType::Punctuation {
                if Self::is_clause_boundary(token.text.as_str()) {
                    return false;
                }
                continue;
            }

            if token.token_type != TokenType::Word {
                return false;
            }

            let prev_norm = Self::normalize_spanish(Self::token_text_for_analysis(token));
            return prev_norm == "como";
        }

        false
    }

    fn find_conditional_verb_in_clause(
        tokens: &[Token],
        si_idx: usize,
        recognizer: &dyn VerbFormRecognizer,
    ) -> Option<(usize, String)> {
        for j in (si_idx + 1)..tokens.len() {
            let token = &tokens[j];

            if token.token_type == TokenType::Whitespace {
                continue;
            }

            if token.token_type == TokenType::Punctuation {
                if Self::is_clause_boundary(token.text.as_str()) {
                    break;
                }
                continue;
            }

            if token.token_type != TokenType::Word {
                continue;
            }

            let observed = Self::token_text_for_analysis(token).to_lowercase();
            let Some((person_slot, infinitive_norm)) =
                Self::parse_conditional_form(&observed, recognizer)
            else {
                // Solo debe mirarse la prótasis: tras "si", el primer verbo conjugado
                // delimita el final útil para esta regla. Si ese primer verbo no es
                // condicional, no se debe saltar a la apódosis.
                if Self::is_finite_non_conditional_verb(tokens, si_idx, j, recognizer) {
                    break;
                }
                continue;
            };

            let Some(suggestion) = Self::build_subj_imperfect_ra(&infinitive_norm, person_slot)
            else {
                continue;
            };

            return Some((j, suggestion));
        }

        None
    }

    fn is_finite_non_conditional_verb(
        tokens: &[Token],
        si_idx: usize,
        word_idx: usize,
        recognizer: &dyn VerbFormRecognizer,
    ) -> bool {
        let word = Self::token_text_for_analysis(&tokens[word_idx]);
        let word_norm = Self::normalize_spanish(word);
        if !recognizer.is_valid_verb_form(word) && !recognizer.is_valid_verb_form(&word_norm) {
            return false;
        }

        if recognizer.is_gerund(word) || recognizer.is_gerund(&word_norm) {
            return false;
        }

        // Excluir no finitos típicos para no cerrar la prótasis antes de tiempo.
        if word_norm.ends_with("ar") || word_norm.ends_with("er") || word_norm.ends_with("ir") {
            return false;
        }
        if word_norm.ends_with("ado")
            || word_norm.ends_with("ada")
            || word_norm.ends_with("ados")
            || word_norm.ends_with("adas")
            || word_norm.ends_with("ido")
            || word_norm.ends_with("ida")
            || word_norm.ends_with("idos")
            || word_norm.ends_with("idas")
            || matches!(
                word_norm.as_str(),
                "hecho"
                    | "dicho"
                    | "visto"
                    | "puesto"
                    | "abierto"
                    | "escrito"
                    | "roto"
                    | "muerto"
                    | "impreso"
                    | "frito"
                    | "satisfecho"
            )
        {
            return false;
        }

        // Evita que ciertos homografos no verbales ("si este...", "si sobre eso...",
        // "si como alternativa...") cierren la busqueda de la protasis demasiado pronto.
        if Self::is_initial_homograph_nonverbal_use(
            tokens, si_idx, word_idx, &word_norm, recognizer,
        ) {
            return false;
        }

        // Evita cerrar por homografos verbo/sustantivo dentro de sintagmas nominales:
        // "si sobre el tema tendria...", "si en la calle tendria...".
        if Self::is_nominal_homograph_context(tokens, si_idx, word_idx) {
            return false;
        }

        true
    }

    fn is_initial_homograph_nonverbal_use(
        tokens: &[Token],
        si_idx: usize,
        word_idx: usize,
        word_norm: &str,
        recognizer: &dyn VerbFormRecognizer,
    ) -> bool {
        if !Self::is_first_word_after_si(tokens, si_idx, word_idx) {
            return false;
        }

        match word_norm {
            // "este/esta/..." se confunden con formas de "estar" sin tilde.
            "este" | "esta" | "estos" | "estas" | "ese" | "esa" | "esos" | "esas" | "aquel"
            | "aquella" | "aquellos" | "aquellas" => true,
            // Preposiciones homografas frecuentes de "sobrar/bajar" en apertura de protasis.
            "sobre" | "bajo" => Self::next_word_in_clause(tokens, word_idx)
                .as_deref()
                .is_some_and(Self::is_nominal_marker),
            // "como" (comer) vs "como" preposicional/discursivo.
            // Solo relajamos cuando hay una pista nominal clara para mantener precision.
            "como" => {
                let Some(next_word) = Self::next_word_in_clause(tokens, word_idx) else {
                    return false;
                };
                let next_norm = Self::normalize_spanish(&next_word);
                if Self::is_likely_verbal_como_continuation(next_norm.as_str()) {
                    return false;
                }
                Self::is_nominal_marker(next_norm.as_str())
                    || Self::is_likely_nominal_content(next_norm.as_str())
                    || Self::is_fixed_como_nonverbal_noun(next_norm.as_str())
                    || (!recognizer.is_valid_verb_form(next_norm.as_str())
                        && next_norm.len() >= 5
                        && next_norm.ends_with('a'))
            }
            _ => false,
        }
    }

    fn is_first_word_after_si(tokens: &[Token], si_idx: usize, word_idx: usize) -> bool {
        for token in tokens.iter().take(word_idx).skip(si_idx + 1) {
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.token_type == TokenType::Punctuation {
                if Self::is_clause_boundary(token.text.as_str()) {
                    return false;
                }
                continue;
            }
            if token.token_type == TokenType::Word {
                return false;
            }
        }
        true
    }

    fn next_word_in_clause(tokens: &[Token], idx: usize) -> Option<String> {
        for token in tokens.iter().skip(idx + 1) {
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.token_type == TokenType::Punctuation {
                if Self::is_clause_boundary(token.text.as_str()) {
                    return None;
                }
                continue;
            }
            if token.token_type == TokenType::Word {
                return Some(Self::token_text_for_analysis(token).to_string());
            }
        }
        None
    }

    fn previous_words_in_clause(
        tokens: &[Token],
        si_idx: usize,
        idx: usize,
    ) -> (Option<String>, Option<String>) {
        let mut prev: Option<String> = None;
        let mut prev_prev: Option<String> = None;
        for token in tokens[..idx].iter().rev() {
            if token.token_type == TokenType::Whitespace {
                continue;
            }
            if token.token_type == TokenType::Punctuation {
                if Self::is_clause_boundary(token.text.as_str()) {
                    break;
                }
                continue;
            }
            if token.token_type != TokenType::Word {
                continue;
            }
            let norm = Self::normalize_spanish(Self::token_text_for_analysis(token));
            if prev.is_none() {
                prev = Some(norm);
                continue;
            }
            prev_prev = Some(norm);
            break;
        }

        // Nunca cruzar el "si" que inicia la protasis.
        if let Some(prev_word) = prev.as_deref() {
            let si_norm = Self::normalize_spanish(Self::token_text_for_analysis(&tokens[si_idx]));
            if prev_word == si_norm {
                return (None, None);
            }
        }

        (prev, prev_prev)
    }

    fn is_nominal_homograph_context(tokens: &[Token], si_idx: usize, word_idx: usize) -> bool {
        let (prev_norm, prev_prev_norm) = Self::previous_words_in_clause(tokens, si_idx, word_idx);
        let Some(prev_norm) = prev_norm.as_deref() else {
            return false;
        };

        if Self::is_strong_nominal_marker(prev_norm) {
            return true;
        }

        // "la/lo/las/los" pueden ser cliticos o articulos.
        // Solo asumir uso nominal cuando vienen tras preposicion ("en la calle", "sobre lo ...").
        if Self::is_weak_nominal_marker(prev_norm)
            && prev_prev_norm
                .as_deref()
                .is_some_and(Self::is_nominal_prep_bridge)
        {
            return true;
        }

        false
    }

    fn is_strong_nominal_marker(word_norm: &str) -> bool {
        matches!(
            word_norm,
            "el" | "un"
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
                | "esto"
                | "eso"
                | "aquello"
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
                | "otro"
                | "otra"
                | "otros"
                | "otras"
                | "cualquier"
                | "cada"
                | "algun"
                | "alguna"
                | "algunos"
                | "algunas"
                | "ningun"
                | "ninguna"
                | "ningunos"
                | "ningunas"
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
                | "varios"
                | "varias"
        )
    }

    fn is_weak_nominal_marker(word_norm: &str) -> bool {
        matches!(word_norm, "la" | "las" | "lo" | "los")
    }

    fn is_nominal_prep_bridge(word_norm: &str) -> bool {
        matches!(
            word_norm,
            "a" | "ante"
                | "bajo"
                | "con"
                | "contra"
                | "de"
                | "desde"
                | "en"
                | "entre"
                | "hacia"
                | "hasta"
                | "para"
                | "por"
                | "segun"
                | "sin"
                | "sobre"
                | "tras"
                | "al"
                | "del"
        )
    }

    fn is_nominal_marker(word_norm: &str) -> bool {
        matches!(
            word_norm,
            "el" | "la"
                | "los"
                | "las"
                | "lo"
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
                | "esto"
                | "eso"
                | "aquello"
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
                | "otro"
                | "otra"
                | "otros"
                | "otras"
                | "cualquier"
                | "cada"
                | "algun"
                | "alguna"
                | "algunos"
                | "algunas"
                | "ningun"
                | "ninguna"
                | "ningunos"
                | "ningunas"
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
                | "varios"
                | "varias"
        )
    }

    fn is_likely_verbal_como_continuation(word_norm: &str) -> bool {
        matches!(
            word_norm,
            "mucho"
                | "poco"
                | "nada"
                | "todo"
                | "algo"
                | "bien"
                | "mal"
                | "mejor"
                | "peor"
                | "mas"
                | "menos"
                | "ya"
                | "hoy"
                | "manana"
                | "aqui"
                | "ahi"
                | "alli"
                | "aca"
                | "alla"
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

    fn is_likely_nominal_content(word_norm: &str) -> bool {
        word_norm.len() >= 6
            && (word_norm.ends_with("cion")
                || word_norm.ends_with("sion")
                || word_norm.ends_with("dad")
                || word_norm.ends_with("tad")
                || word_norm.ends_with("miento")
                || word_norm.ends_with("mente")
                || word_norm.ends_with("ncia")
                || word_norm.ends_with("ncias")
                || word_norm.ends_with("ismo")
                || word_norm.ends_with("ista")
                || word_norm.ends_with("ario")
                || word_norm.ends_with("aria"))
    }

    fn is_fixed_como_nonverbal_noun(word_norm: &str) -> bool {
        matches!(
            word_norm,
            "alternativa" | "opcion" | "ejemplo" | "referencia" | "base"
        )
    }

    fn parse_conditional_form(
        word: &str,
        recognizer: &dyn VerbFormRecognizer,
    ) -> Option<(usize, String)> {
        let word_norm = Self::normalize_spanish(word);
        let (slot, base) = Self::extract_conditional_slot_and_base(&word_norm)?;

        let infinitive = Self::resolve_infinitive_for_conditional(word, slot, recognizer)
            .or_else(|| Self::infer_infinitive_from_conditional_base(&base, recognizer))?;
        let infinitive_core = infinitive.strip_suffix("se").unwrap_or(&infinitive);
        let infinitive_norm = Self::normalize_spanish(infinitive_core);

        if !Self::matches_conditional_base(&base, &infinitive_norm) {
            return None;
        }

        Some((slot, infinitive_norm))
    }

    fn resolve_infinitive_for_conditional(
        word: &str,
        slot: usize,
        recognizer: &dyn VerbFormRecognizer,
    ) -> Option<String> {
        if let Some(inf) = recognizer.get_infinitive(word) {
            return Some(inf);
        }

        let reaccented = Self::accent_conditional_ending(word, slot)?;
        recognizer.get_infinitive(&reaccented)
    }

    fn infer_infinitive_from_conditional_base(
        base: &str,
        recognizer: &dyn VerbFormRecognizer,
    ) -> Option<String> {
        for ending in ["ar", "er", "ir"] {
            let candidate = format!("{base}{ending}");
            if recognizer.knows_infinitive(&candidate) {
                return Some(candidate);
            }
        }

        for candidate in Self::irregular_future_stem_candidates(base) {
            if recognizer.knows_infinitive(&candidate) {
                return Some(candidate);
            }
        }

        None
    }

    fn irregular_future_stem_candidates(base: &str) -> Vec<String> {
        let mut candidates = Vec::new();

        match base {
            "podr" => candidates.push("poder".to_string()),
            "querr" => candidates.push("querer".to_string()),
            "sabr" => candidates.push("saber".to_string()),
            "cabr" => candidates.push("caber".to_string()),
            "habr" => candidates.push("haber".to_string()),
            "har" => candidates.push("hacer".to_string()),
            "dir" => candidates.push("decir".to_string()),
            _ => {}
        }

        if let Some(stem) = base.strip_suffix("dr") {
            candidates.push(format!("{stem}er"));
            candidates.push(format!("{stem}ir"));
        }
        if let Some(stem) = base.strip_suffix("br") {
            candidates.push(format!("{stem}ber"));
        }
        if let Some(stem) = base.strip_suffix("rr") {
            candidates.push(format!("{stem}rer"));
        }
        if let Some(stem) = base.strip_suffix("odr") {
            candidates.push(format!("{stem}oder"));
        }

        candidates
    }

    fn accent_conditional_ending(word: &str, slot: usize) -> Option<String> {
        match slot {
            0 | 2 => word
                .strip_suffix("ia")
                .or_else(|| word.strip_suffix("\u{00ED}a"))
                .map(|stem| format!("{stem}\u{00ED}a")),
            1 => word
                .strip_suffix("ias")
                .or_else(|| word.strip_suffix("\u{00ED}as"))
                .map(|stem| format!("{stem}\u{00ED}as")),
            3 => word
                .strip_suffix("iamos")
                .or_else(|| word.strip_suffix("\u{00ED}amos"))
                .map(|stem| format!("{stem}\u{00ED}amos")),
            4 => word
                .strip_suffix("iais")
                .or_else(|| word.strip_suffix("\u{00ED}ais"))
                .map(|stem| format!("{stem}\u{00ED}ais")),
            5 => word
                .strip_suffix("ian")
                .or_else(|| word.strip_suffix("\u{00ED}an"))
                .map(|stem| format!("{stem}\u{00ED}an")),
            _ => None,
        }
    }

    fn extract_conditional_slot_and_base(word_norm: &str) -> Option<(usize, String)> {
        // Orden de mas largo a mas corto para evitar colisiones.
        let endings: [(&str, usize); 5] = [
            ("iamos", 3),
            ("iais", 4),
            ("ias", 1),
            ("ian", 5),
            ("ia", 0), // 1a/3a singular comparten forma
        ];

        for (ending, slot) in endings {
            if let Some(base) = word_norm.strip_suffix(ending) {
                if !base.is_empty() {
                    return Some((slot, base.to_string()));
                }
            }
        }

        None
    }

    fn matches_conditional_base(base: &str, infinitive_norm: &str) -> bool {
        if base == infinitive_norm {
            return true;
        }

        Self::irregular_future_stem(infinitive_norm)
            .as_deref()
            .is_some_and(|stem| stem == base)
    }

    fn build_subj_imperfect_ra(infinitive_norm: &str, slot: usize) -> Option<String> {
        let root = if let Some(irregular_root) = Self::irregular_subj_root(infinitive_norm) {
            irregular_root
        } else {
            Self::regular_subj_root(infinitive_norm)?
        };

        let suggestion = if slot == 3 {
            format!("{}ramos", Self::accent_last_vowel(&root))
        } else {
            let suffix = match slot {
                0 => "ra",
                1 => "ras",
                2 => "ra",
                4 => "rais",
                5 => "ran",
                _ => return None,
            };
            format!("{root}{suffix}")
        };

        Some(suggestion)
    }

    fn regular_subj_root(infinitive_norm: &str) -> Option<String> {
        if let Some(stem) = infinitive_norm.strip_suffix("ar") {
            return Some(format!("{stem}a"));
        }

        if let Some(stem) = infinitive_norm.strip_suffix("er") {
            if Self::is_y_irregular_infinitive(infinitive_norm) {
                return None;
            }
            return Some(format!("{stem}ie"));
        }

        if let Some(stem) = infinitive_norm.strip_suffix("ir") {
            if infinitive_norm.ends_with("uir") || Self::is_y_irregular_infinitive(infinitive_norm)
            {
                return None;
            }

            let adjusted_stem = if let Some(change) = Self::stem_change_type_for(infinitive_norm) {
                Self::apply_ir_stem_change_for_subj_imperfect(stem, change)?
            } else {
                stem.to_string()
            };

            return Some(format!("{adjusted_stem}ie"));
        }

        None
    }

    fn stem_change_type_for(infinitive_norm: &str) -> Option<StemChangeType> {
        let map = get_stem_changing_verbs();
        if let Some(change) = map.get(infinitive_norm) {
            return Some(*change);
        }

        for (k, change) in map {
            if Self::normalize_spanish(k) == infinitive_norm {
                return Some(*change);
            }
        }

        None
    }

    fn apply_ir_stem_change_for_subj_imperfect(
        stem: &str,
        change: StemChangeType,
    ) -> Option<String> {
        match change {
            StemChangeType::EToI | StemChangeType::EToIe => {
                Self::replace_last_occurrence(stem, "e", "i")
            }
            StemChangeType::OToUe => Self::replace_last_occurrence(stem, "o", "u"),
            StemChangeType::IToIe => Some(stem.to_string()),
            StemChangeType::UToUe | StemChangeType::CToZc => Some(stem.to_string()),
        }
    }

    fn replace_last_occurrence(text: &str, from: &str, to: &str) -> Option<String> {
        let pos = text.rfind(from)?;
        let mut out = String::with_capacity(text.len() + to.len());
        out.push_str(&text[..pos]);
        out.push_str(to);
        out.push_str(&text[pos + from.len()..]);
        Some(out)
    }

    fn is_y_irregular_infinitive(infinitive_norm: &str) -> bool {
        // caer/creer/leer/oir/poseer/proveer/... -> cayera, creyera, leyera, oyera...
        // (no siguen el patron regular "ie").
        (infinitive_norm.ends_with("aer") || infinitive_norm.ends_with("eer"))
            || infinitive_norm.ends_with("oir")
            || infinitive_norm.ends_with("eir")
    }

    fn irregular_subj_root(infinitive_norm: &str) -> Option<String> {
        match infinitive_norm {
            "ser" | "ir" => Some("fue".to_string()),
            "dar" => Some("die".to_string()),
            "estar" => Some("estuvie".to_string()),
            "haber" => Some("hubie".to_string()),
            "poder" => Some("pudie".to_string()),
            "querer" => Some("quisie".to_string()),
            "saber" => Some("supie".to_string()),
            "caber" => Some("cupie".to_string()),
            "hacer" => Some("hicie".to_string()),
            "decir" => Some("dije".to_string()),
            "ver" => Some("vie".to_string()),
            "andar" => Some("anduvie".to_string()),
            _ => {
                if let Some(prefix) = infinitive_norm.strip_suffix("tener") {
                    return Some(format!("{prefix}tuvie"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("venir") {
                    return Some(format!("{prefix}vinie"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("poner") {
                    return Some(format!("{prefix}pusie"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("hacer") {
                    return Some(format!("{prefix}hicie"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("traer") {
                    return Some(format!("{prefix}traje"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("ducir") {
                    return Some(format!("{prefix}duje"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("salir") {
                    return Some(format!("{prefix}salie"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("valer") {
                    return Some(format!("{prefix}valie"));
                }
                None
            }
        }
    }

    fn irregular_future_stem(infinitive_norm: &str) -> Option<String> {
        match infinitive_norm {
            "poder" => Some("podr".to_string()),
            "querer" => Some("querr".to_string()),
            "saber" => Some("sabr".to_string()),
            "caber" => Some("cabr".to_string()),
            "haber" => Some("habr".to_string()),
            "hacer" => Some("har".to_string()),
            "decir" => Some("dir".to_string()),
            _ => {
                if let Some(prefix) = infinitive_norm.strip_suffix("tener") {
                    return Some(format!("{prefix}tendr"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("venir") {
                    return Some(format!("{prefix}vendr"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("poner") {
                    return Some(format!("{prefix}pondr"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("salir") {
                    return Some(format!("{prefix}saldr"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("valer") {
                    return Some(format!("{prefix}valdr"));
                }
                if let Some(prefix) = infinitive_norm.strip_suffix("hacer") {
                    return Some(format!("{prefix}har"));
                }
                None
            }
        }
    }

    fn is_conditional_si_intro(tokens: &[Token], si_idx: usize) -> bool {
        if si_idx == 0 {
            return true;
        }

        for i in (0..si_idx).rev() {
            let token = &tokens[i];
            if token.token_type == TokenType::Whitespace {
                continue;
            }

            if token.token_type == TokenType::Punctuation {
                return Self::is_si_intro_punctuation(token.text.as_str());
            }

            if token.token_type == TokenType::Word {
                let prev_norm = Self::normalize_spanish(Self::token_text_for_analysis(token));
                return matches!(
                    prev_norm.as_str(),
                    "y" | "e" | "o" | "u" | "pero" | "aunque" | "mas" | "ni" | "incluso" | "aun"
                );
            }

            return false;
        }

        true
    }

    fn token_text_for_analysis(token: &Token) -> &str {
        if let Some(ref correction) = token.corrected_grammar {
            if !correction.starts_with("falta")
                && !correction.starts_with("sobra")
                && correction != "desbalanceado"
            {
                return correction;
            }
        }
        token.text.as_str()
    }

    fn is_si_intro_punctuation(punct: &str) -> bool {
        matches!(
            punct,
            "," | ";" | ":" | "." | "?" | "!" | "\u{00BF}" | "\u{00A1}" | "(" | "[" | "{"
        )
    }

    fn is_clause_boundary(punct: &str) -> bool {
        matches!(
            punct,
            "," | ";" | ":" | "." | "?" | "!" | "\u{00BF}" | "\u{00A1}"
        )
    }

    fn accent_last_vowel(text: &str) -> String {
        let mut chars: Vec<char> = text.chars().collect();
        for idx in (0..chars.len()).rev() {
            let accented = match chars[idx] {
                'a' => Some('\u{00E1}'),
                'e' => Some('\u{00E9}'),
                'i' => Some('\u{00ED}'),
                'o' => Some('\u{00F3}'),
                'u' => Some('\u{00FA}'),
                _ => None,
            };
            if let Some(ch) = accented {
                chars[idx] = ch;
                return chars.into_iter().collect();
            }
        }
        text.to_string()
    }

    fn normalize_spanish(word: &str) -> String {
        word.to_lowercase()
            .chars()
            .map(|c| match c {
                '\u{00E1}' | '\u{00E0}' | '\u{00E4}' | '\u{00E2}' => 'a',
                '\u{00E9}' | '\u{00E8}' | '\u{00EB}' | '\u{00EA}' => 'e',
                '\u{00ED}' | '\u{00EC}' | '\u{00EF}' | '\u{00EE}' => 'i',
                '\u{00F3}' | '\u{00F2}' | '\u{00F6}' | '\u{00F4}' => 'o',
                '\u{00FA}' | '\u{00F9}' | '\u{00FC}' | '\u{00FB}' => 'u',
                '\u{00F1}' => 'n',
                _ => c,
            })
            .collect()
    }

    fn preserve_case(original: &str, replacement: &str) -> String {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::{Gender, Number, Trie, WordCategory, WordInfo};
    use crate::grammar::Tokenizer;
    use crate::languages::spanish::VerbRecognizer;

    fn build_recognizer(infinitives: &[&str]) -> VerbRecognizer {
        let mut trie = Trie::new();
        for inf in infinitives {
            trie.insert(
                inf,
                WordInfo {
                    category: WordCategory::Verbo,
                    gender: Gender::None,
                    number: Number::None,
                    extra: String::new(),
                    frequency: 100,
                },
            );
        }
        VerbRecognizer::from_dictionary(&trie)
    }

    fn analyze_text(text: &str, infinitives: &[&str]) -> Vec<IrrealisConditionalCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        let recognizer = build_recognizer(infinitives);
        IrrealisConditionalAnalyzer::analyze(&tokens, Some(&recognizer))
    }

    #[test]
    fn test_si_tendria_to_tuviera() {
        let corrections = analyze_text(
            "si tendria dinero, compraria una casa",
            &["tener", "comprar"],
        );
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tuviera");
    }

    #[test]
    fn test_si_podrias_to_pudieras() {
        let corrections = analyze_text(
            "si podrias venir, te avisaria",
            &["poder", "venir", "avisar"],
        );
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "pudieras");
    }

    #[test]
    fn test_si_tendriamos_to_tuvieramos() {
        let corrections = analyze_text(
            "si tendr\u{00ED}amos tiempo, viajar\u{00ED}amos",
            &["tener", "viajar"],
        );
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "tuvi\u{00E9}ramos");
    }

    #[test]
    fn test_si_hariamos_to_hicieramos() {
        let corrections = analyze_text(
            "si har\u{00ED}amos eso, saldr\u{00ED}a mal",
            &["hacer", "salir"],
        );
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "hici\u{00E9}ramos");
    }

    #[test]
    fn test_si_diriamos_to_dijeramos() {
        let corrections = analyze_text(
            "si dir\u{00ED}amos la verdad, se enojar\u{00ED}an",
            &["decir", "enojar"],
        );
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "dij\u{00E9}ramos");
    }

    #[test]
    fn test_regular_er_conditional_to_subjunctive() {
        let corrections = analyze_text("si comeria mas, engordaria", &["comer", "engordar"]);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "comiera");
    }

    #[test]
    fn test_no_se_si_tendria_not_treated_as_conditional_si_clause() {
        let corrections = analyze_text("no se si tendria tiempo", &["tener"]);
        assert!(
            corrections.is_empty(),
            "No debe corregir 'si' interrogativo indirecto: {:?}",
            corrections
        );
    }

    #[test]
    fn test_que_si_tendria_not_treated_as_conditional_si_clause() {
        let corrections =
            analyze_text("me pregunto que si tendria tiempo", &["tener", "preguntar"]);
        assert!(
            corrections.is_empty(),
            "No debe corregir 'que si' interrogativo indirecto: {:?}",
            corrections
        );
    }

    #[test]
    fn test_si_tuviera_no_change() {
        let corrections = analyze_text(
            "si tuviera dinero, compraria una casa",
            &["tener", "comprar"],
        );
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_si_comia_no_change() {
        let corrections = analyze_text("si comia mas, engordaba", &["comer", "engordar"]);
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_stem_changing_ir_conditional_to_subjunctive() {
        let corrections = analyze_text("si sentiria dolor, iria al medico", &["sentir", "ir"]);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "sintiera");
    }

    #[test]
    fn test_does_not_correct_apodosis_without_comma_after_correct_protasis() {
        let cases = [
            ("si pudiera iria", vec!["poder", "ir"]),
            ("si fuera rico viajaria", vec!["ser", "viajar"]),
            ("si lo supiera no preguntaria", vec!["saber", "preguntar"]),
            (
                "si tuviera dinero compraria una casa",
                vec!["tener", "comprar"],
            ),
        ];

        for (text, infinitives) in cases {
            let corrections = analyze_text(text, &infinitives);
            assert!(
                corrections.is_empty(),
                "No debe corregir la apodosis cuando la prótasis ya es correcta: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_y_irregular_subjunctive_forms_bound_protasis() {
        let cases = [
            ("si lo creyeran no vendria", vec!["creer", "venir"]),
            ("si lo oyeran no diria nada", vec!["o\u{00ED}r", "decir"]),
            ("si huyeran los encontraria", vec!["huir", "encontrar"]),
            ("si lo incluyeran lo aprobarian", vec!["incluir", "aprobar"]),
        ];

        for (text, infinitives) in cases {
            let corrections = analyze_text(text, &infinitives);
            assert!(
                corrections.is_empty(),
                "No debe corregir apodosis cuando la protasis ya esta en subjuntivo: {text} -> {corrections:?}"
            );
        }
    }

    #[test]
    fn test_creyera_in_protasis_does_not_trigger_apodosis_correction() {
        let corrections = analyze_text("si lo creyera no vendria", &["creer", "venir"]);
        assert!(
            corrections.is_empty(),
            "No debe corregir la apodosis cuando la protasis ya usa 'creyera': {:?}",
            corrections
        );
    }

    #[test]
    fn test_homograph_barriers_do_not_stop_protasis_scan() {
        let cases = [
            (
                "si este verano tendria vacaciones",
                vec!["estar", "tener"],
                "tuviera",
            ),
            (
                "si sobre eso tendria algo",
                vec!["sobrar", "tener"],
                "tuviera",
            ),
            (
                "si bajo esa condicion tendria dudas",
                vec!["bajar", "tener"],
                "tuviera",
            ),
            (
                "si como alternativa ofreceria",
                vec!["comer", "ofrecer"],
                "ofreciera",
            ),
        ];

        for (text, infinitives, expected) in cases {
            let corrections = analyze_text(text, &infinitives);
            assert_eq!(
                corrections.len(),
                1,
                "Debe corregir condicional en protasis pese a homografo: {text} -> {corrections:?}"
            );
            assert_eq!(corrections[0].suggestion, expected);
        }
    }

    #[test]
    fn test_como_real_verb_still_bounds_clause() {
        let corrections = analyze_text("si como mucho engordaria", &["comer", "engordar"]);
        assert!(
            corrections.is_empty(),
            "No debe corregir la apodosis cuando 'como' funciona como verbo: {:?}",
            corrections
        );
    }

    #[test]
    fn test_deep_nominal_homographs_do_not_block_protasis_scan() {
        let cases = [
            (
                "si sobre el tema tendria algo",
                vec!["sobrar", "temer", "tener"],
                "tuviera",
            ),
            (
                "si en la calle tendria dudas",
                vec!["callar", "tener"],
                "tuviera",
            ),
            (
                "si el tema tendria solucion",
                vec!["temer", "tener"],
                "tuviera",
            ),
            (
                "si sobre cualquier tema tendria algo",
                vec!["sobrar", "temer", "tener"],
                "tuviera",
            ),
            (
                "si sobre cada tema tendria algo",
                vec!["sobrar", "temer", "tener"],
                "tuviera",
            ),
            (
                "si con algun esfuerzo tendria exito",
                vec!["esforzar", "tener"],
                "tuviera",
            ),
        ];

        for (text, infinitives, expected) in cases {
            let corrections = analyze_text(text, &infinitives);
            assert_eq!(
                corrections.len(),
                1,
                "Debe corregir condicional aunque haya homografo nominal interno: {text} -> {corrections:?}"
            );
            assert_eq!(corrections[0].suggestion, expected);
        }
    }

    #[test]
    fn test_clitic_la_verb_still_bounds_clause() {
        let corrections = analyze_text("si la come engordaria", &["comer", "engordar"]);
        assert!(
            corrections.is_empty(),
            "No debe confundir clitico + verbo con sintagma nominal: {:?}",
            corrections
        );
    }

    #[test]
    fn test_como_si_conditional_to_subjunctive() {
        let corrections = analyze_text("habla como si sabria la verdad", &["hablar", "saber"]);
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "supiera");
    }
}
