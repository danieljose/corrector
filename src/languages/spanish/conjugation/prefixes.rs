//! Analizador de prefijos verbales
//!
//! Permite reconocer formas verbales con prefijos comunes como:
//! - deshago → des + hago → deshacer
//! - rehice → re + hice → rehacer
//! - predigo → pre + digo → predecir

/// Prefijos verbales comunes en español, ordenados por longitud descendente
/// para que el matching encuentre primero los más largos
const PREFIXES: [&str; 23] = [
    // Largos primero
    "contra", "entre", "sobre", "super", "trans", "inter", // Medianos
    "ante", "anti", "auto", "semi", "pre", "sub", "com", "con", "dis", "pro", // Cortos
    "des", "re", "co", "ex", "in", "en", "im",
];

/// Analizador de prefijos verbales
pub struct PrefixAnalyzer;

impl PrefixAnalyzer {
    /// Intenta separar un prefijo de una forma verbal
    ///
    /// Devuelve `Some((prefijo, base))` si encuentra un prefijo conocido,
    /// `None` si la palabra no tiene prefijo reconocible.
    ///
    /// # Ejemplo
    /// ```ignore
    /// let result = PrefixAnalyzer::strip_prefix("deshago");
    /// assert_eq!(result, Some(("des", "hago")));
    /// ```
    pub fn strip_prefix(word: &str) -> Option<(&str, &str)> {
        for prefix in PREFIXES.iter() {
            if word.starts_with(prefix) {
                let base = &word[prefix.len()..];
                // La base debe tener al menos 2 caracteres para ser válida
                if base.len() >= 2 {
                    return Some((prefix, base));
                }
            }
        }
        None
    }

    /// Reconstruye el infinitivo con prefijo
    ///
    /// # Ejemplo
    /// ```ignore
    /// let inf = PrefixAnalyzer::reconstruct_infinitive("des", "hacer");
    /// assert_eq!(inf, "deshacer");
    /// ```
    pub fn reconstruct_infinitive(prefix: &str, base_infinitive: &str) -> String {
        format!("{}{}", prefix, base_infinitive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_prefix_des() {
        assert_eq!(
            PrefixAnalyzer::strip_prefix("deshago"),
            Some(("des", "hago"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("deshice"),
            Some(("des", "hice"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("deshecho"),
            Some(("des", "hecho"))
        );
    }

    #[test]
    fn test_strip_prefix_re() {
        assert_eq!(PrefixAnalyzer::strip_prefix("rehago"), Some(("re", "hago")));
        assert_eq!(PrefixAnalyzer::strip_prefix("rehice"), Some(("re", "hice")));
        assert_eq!(
            PrefixAnalyzer::strip_prefix("rehecho"),
            Some(("re", "hecho"))
        );
    }

    #[test]
    fn test_strip_prefix_pre() {
        assert_eq!(
            PrefixAnalyzer::strip_prefix("predigo"),
            Some(("pre", "digo"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("predije"),
            Some(("pre", "dije"))
        );
    }

    #[test]
    fn test_strip_prefix_contra() {
        assert_eq!(
            PrefixAnalyzer::strip_prefix("contradigo"),
            Some(("contra", "digo"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("contradije"),
            Some(("contra", "dije"))
        );
    }

    #[test]
    fn test_strip_prefix_productive_variants() {
        assert_eq!(
            PrefixAnalyzer::strip_prefix("compusieron"),
            Some(("com", "pusieron"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("convienen"),
            Some(("con", "vienen"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("dispusieron"),
            Some(("dis", "pusieron"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("propusieron"),
            Some(("pro", "pusieron"))
        );
        assert_eq!(
            PrefixAnalyzer::strip_prefix("impusieron"),
            Some(("im", "pusieron"))
        );
    }

    #[test]
    fn test_strip_prefix_no_match() {
        assert_eq!(PrefixAnalyzer::strip_prefix("canto"), None);
        assert_eq!(PrefixAnalyzer::strip_prefix("hago"), None);
        // Muy corto después del prefijo
        assert_eq!(PrefixAnalyzer::strip_prefix("des"), None);
    }

    #[test]
    fn test_reconstruct_infinitive() {
        assert_eq!(
            PrefixAnalyzer::reconstruct_infinitive("des", "hacer"),
            "deshacer"
        );
        assert_eq!(
            PrefixAnalyzer::reconstruct_infinitive("re", "hacer"),
            "rehacer"
        );
        assert_eq!(
            PrefixAnalyzer::reconstruct_infinitive("pre", "decir"),
            "predecir"
        );
    }
}
