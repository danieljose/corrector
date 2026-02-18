//! Analizador de pronombres enclíticos
//!
//! Permite separar pronombres adjuntos de formas verbales:
//! - dámelo → da + me + lo (imperativo de dar)
//! - diciéndote → diciendo + te (gerundio de decir)
//! - decirle → decir + le (infinitivo)

/// Pronombres simples para separación recursiva
const SIMPLE_ENCLITICS: [&str; 11] = [
    "me", "te", "se", "nos", "os", // reflexivos
    "lo", "la", "le", "los", "las", "les", // objeto
];

/// Vocales acentuadas y sus equivalentes sin acento
const ACCENT_MAP: [(&str, &str); 5] = [("á", "a"), ("é", "e"), ("í", "i"), ("ó", "o"), ("ú", "u")];

/// Analizador de pronombres enclíticos
pub struct EncliticsAnalyzer;

/// Resultado del análisis de enclíticos
#[derive(Debug, Clone, PartialEq)]
pub struct EncliticsResult {
    /// Base verbal sin enclíticos
    pub base: String,
    /// Pronombres encontrados (en orden)
    pub pronouns: Vec<String>,
}

impl EncliticsAnalyzer {
    /// Intenta separar pronombres enclíticos de una forma verbal
    ///
    /// # Ejemplo
    /// ```ignore
    /// let result = EncliticsAnalyzer::strip_enclitics("dámelo");
    /// assert_eq!(result, Some(EncliticsResult {
    ///     base: "da".to_string(),
    ///     pronouns: vec!["me".to_string(), "lo".to_string()],
    /// }));
    /// ```
    pub fn strip_enclitics(word: &str) -> Option<EncliticsResult> {
        // Try stripping enclitics, checking validity at each step
        // We try more enclitics first (3, 2, 1) to prefer longer strips
        for num_enclitics in (1..=3).rev() {
            if let Some(result) = Self::try_strip_n_enclitics(word, num_enclitics) {
                return Some(result);
            }
        }
        None
    }

    /// Try to strip exactly n enclitics from the word
    fn try_strip_n_enclitics(word: &str, n: usize) -> Option<EncliticsResult> {
        Self::strip_enclitics_recursive(word, n, Vec::new())
    }

    /// Recursively try to strip enclitics
    fn strip_enclitics_recursive(
        current: &str,
        remaining: usize,
        pronouns: Vec<String>,
    ) -> Option<EncliticsResult> {
        if remaining == 0 {
            // Check if the base is valid
            if Self::is_valid_verb_base(current) {
                let base = Self::restore_accent(current);
                return Some(EncliticsResult { base, pronouns });
            }
            return None;
        }

        // Try each enclitic
        for enclitic in SIMPLE_ENCLITICS.iter() {
            if current.ends_with(enclitic) {
                let enc_len = enclitic.len();
                let cur_len = current.len();
                if cur_len > enc_len {
                    let base = &current[..cur_len - enc_len];
                    let base_char_count = base.chars().count();
                    if base_char_count >= 2 {
                        let mut new_pronouns = vec![enclitic.to_string()];
                        new_pronouns.extend(pronouns.clone());
                        if let Some(result) =
                            Self::strip_enclitics_recursive(base, remaining - 1, new_pronouns)
                        {
                            return Some(result);
                        }
                    }
                }
            }
        }

        None
    }

    /// Verifica si una base es válida como forma verbal
    fn is_valid_verb_base(base: &str) -> bool {
        // Imperativo exhortativo de 1ª plural con enclíticos:
        // "analicémoslo" -> base "analicémos" (debe aceptarse como "analicemos").
        if base.ends_with("amos")
            || base.ends_with("emos")
            || base.ends_with("imos")
            || base.ends_with("ámos")
            || base.ends_with("émos")
            || base.ends_with("ímos")
        {
            return base.chars().count() >= 4;
        }

        let last_char = base.chars().last();
        match last_char {
            // Infinitivos con acento (-ár, -ér, -ír)
            Some('r') => {
                let chars: Vec<char> = base.chars().collect();
                if chars.len() >= 2 {
                    let second_last = chars[chars.len() - 2];
                    matches!(second_last, 'a' | 'e' | 'i' | 'á' | 'é' | 'í')
                } else {
                    false
                }
            }
            // Gerundios con acento (-ándo, -iéndo, -yéndo)
            Some('o') => {
                base.ends_with("ando")
                    || base.ends_with("ándo")
                    || base.ends_with("iendo")
                    || base.ends_with("iéndo")
                    || base.ends_with("yendo")
                    || base.ends_with("yéndo")
            }
            // Imperativos terminados en -a, -e (con o sin acento)
            // Esto incluye: canta, cánta, come, cóme, vive, víve
            Some('a' | 'á' | 'e' | 'é') => {
                // Mínimo 2 caracteres para ser un imperativo válido
                base.chars().count() >= 2
            }
            // Imperativos monosilábicos especiales: di, pon, sal, ten, ven, haz
            Some('i' | 'í' | 'n' | 'z' | 'l') => {
                Self::is_likely_monosyllabic_imperative(base)
                    || Self::is_likely_monosyllabic_imperative(&Self::remove_accent(base))
            }
            // Imperativos vosotros (-ad, -ed, -id)
            Some('d') => {
                let chars: Vec<char> = base.chars().collect();
                if chars.len() >= 2 {
                    let second_last = chars[chars.len() - 2];
                    matches!(second_last, 'a' | 'e' | 'i')
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Restaura el acento original de la base verbal
    ///
    /// Cuando se añaden enclíticos, se puede añadir acento:
    /// - Infinitivos: dar + me + lo = dármelo → dar
    /// - Gerundios: cantando + se = cantándose → cantando (mantiene estructura)
    fn restore_accent(base: &str) -> String {
        // Formas exhortativas en 1ª plural con acento desplazado.
        if let Some(stem) = base.strip_suffix("ámos") {
            return format!("{stem}amos");
        }
        if let Some(stem) = base.strip_suffix("émos") {
            return format!("{stem}emos");
        }
        if let Some(stem) = base.strip_suffix("ímos") {
            return format!("{stem}imos");
        }

        // Para infinitivos con acento añadido (-ár, -ér, -ír)
        if base.ends_with("ár") || base.ends_with("ér") || base.ends_with("ír") {
            return Self::remove_accent(base);
        }

        // Para imperativos monosilábicos con acento (dá, dí)
        let base_vowels = Self::count_vowels(base);
        if base_vowels == 1 {
            let unaccented = Self::remove_accent(base);
            if Self::is_likely_monosyllabic_imperative(&unaccented) {
                return unaccented;
            }
        }

        // Para gerundios, mantener el acento si está en la posición correcta
        // cantándo → cantando (quitar acento añadido)
        if base.ends_with("ándo") {
            let without_suffix = &base[..base.len() - "ándo".len()];
            return format!("{}ando", without_suffix);
        }
        if base.ends_with("iéndo") {
            let without_suffix = &base[..base.len() - "iéndo".len()];
            return format!("{}iendo", without_suffix);
        }

        base.to_string()
    }

    /// Quita el acento de una palabra
    fn remove_accent(word: &str) -> String {
        let mut result = word.to_string();
        for (accented, unaccented) in ACCENT_MAP.iter() {
            result = result.replace(accented, unaccented);
        }
        result
    }

    /// Cuenta vocales (aproximación de sílabas)
    fn count_vowels(word: &str) -> usize {
        word.chars().filter(|c| "aeiouáéíóúü".contains(*c)).count()
    }

    /// Verifica si parece un imperativo monosilábico
    fn is_likely_monosyllabic_imperative(word: &str) -> bool {
        // Imperativos irregulares monosilábicos comunes
        matches!(
            word,
            "da" | "dá" | "di" | "dí" | "ve" | "pon" | "sal" | "ten" | "ven" | "haz" | "se" | "sé"
        )
    }

    /// Verifica si una base es un infinitivo (termina en -ar, -er, -ir)
    pub fn is_infinitive(base: &str) -> bool {
        base.ends_with("ar") || base.ends_with("er") || base.ends_with("ir")
    }

    /// Verifica si una base es un gerundio (termina en -ando, -iendo, -yendo)
    pub fn is_gerund(base: &str) -> bool {
        base.ends_with("ando")
            || base.ends_with("iendo")
            || base.ends_with("yendo")
            || base.ends_with("ándo")
            || base.ends_with("iéndo")
            || base.ends_with("yéndo")
    }

    /// Verifica si una base podría ser un imperativo
    /// (esto es más difícil de determinar sin contexto)
    pub fn could_be_imperative(base: &str) -> bool {
        // Imperativos irregulares monosilábicos
        if Self::is_likely_monosyllabic_imperative(base) {
            return true;
        }

        // Exhortativo de 1ª plural: "hagamos", "analicemos", "demos".
        if base.ends_with("amos")
            || base.ends_with("emos")
            || base.ends_with("imos")
            || base.ends_with("ámos")
            || base.ends_with("émos")
            || base.ends_with("ímos")
        {
            return true;
        }

        let chars: Vec<char> = base.chars().collect();
        let len = chars.len();

        // Imperativos vosotros: -ad, -ed, -id
        if len >= 2 {
            let last2: String = chars[len - 2..].iter().collect();
            if matches!(last2.as_str(), "ad" | "ed" | "id") {
                return true;
            }
        }

        // Imperativo tú: termina en -a (canta) o -e (come, vive)
        if len >= 1 {
            let last = chars[len - 1];
            if matches!(last, 'a' | 'e') {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_single_enclitic() {
        let result = EncliticsAnalyzer::strip_enclitics("decirle").unwrap();
        assert_eq!(result.base, "decir");
        assert_eq!(result.pronouns, vec!["le"]);

        // "cantando" no tiene enclítico, debería devolver None
        assert!(EncliticsAnalyzer::strip_enclitics("cantando").is_none());
    }

    #[test]
    fn test_strip_double_enclitic() {
        let result = EncliticsAnalyzer::strip_enclitics("dármelo").unwrap();
        assert_eq!(result.base, "dar");
        assert_eq!(result.pronouns, vec!["me", "lo"]);

        let result = EncliticsAnalyzer::strip_enclitics("diciéndotelo").unwrap();
        assert_eq!(result.base, "diciendo");
        assert_eq!(result.pronouns, vec!["te", "lo"]);
    }

    #[test]
    fn test_strip_enclitic_imperative() {
        let result = EncliticsAnalyzer::strip_enclitics("dámelo").unwrap();
        assert_eq!(result.base, "da");
        assert_eq!(result.pronouns, vec!["me", "lo"]);

        let result = EncliticsAnalyzer::strip_enclitics("dime").unwrap();
        assert_eq!(result.base, "di");
        assert_eq!(result.pronouns, vec!["me"]);

        let result = EncliticsAnalyzer::strip_enclitics("ponlo").unwrap();
        assert_eq!(result.base, "pon");
        assert_eq!(result.pronouns, vec!["lo"]);
    }

    #[test]
    fn test_strip_enclitic_gerund() {
        let result = EncliticsAnalyzer::strip_enclitics("cantándose").unwrap();
        assert_eq!(result.base, "cantando");
        assert_eq!(result.pronouns, vec!["se"]);
    }

    #[test]
    fn test_no_enclitic() {
        assert!(EncliticsAnalyzer::strip_enclitics("cantar").is_none());
        assert!(EncliticsAnalyzer::strip_enclitics("canto").is_none());
        assert!(EncliticsAnalyzer::strip_enclitics("hablo").is_none());
    }

    #[test]
    fn test_is_infinitive() {
        assert!(EncliticsAnalyzer::is_infinitive("cantar"));
        assert!(EncliticsAnalyzer::is_infinitive("comer"));
        assert!(EncliticsAnalyzer::is_infinitive("vivir"));
        assert!(!EncliticsAnalyzer::is_infinitive("canto"));
    }

    #[test]
    fn test_is_gerund() {
        assert!(EncliticsAnalyzer::is_gerund("cantando"));
        assert!(EncliticsAnalyzer::is_gerund("comiendo"));
        assert!(EncliticsAnalyzer::is_gerund("yendo"));
        assert!(EncliticsAnalyzer::is_gerund("yéndo"));
        assert!(!EncliticsAnalyzer::is_gerund("cantar"));
    }

    #[test]
    fn test_could_be_imperative() {
        // Irregulares
        assert!(EncliticsAnalyzer::could_be_imperative("da"));
        assert!(EncliticsAnalyzer::could_be_imperative("di"));
        assert!(EncliticsAnalyzer::could_be_imperative("pon"));
        // Regulares
        assert!(EncliticsAnalyzer::could_be_imperative("canta"));
        assert!(EncliticsAnalyzer::could_be_imperative("come"));
        assert!(EncliticsAnalyzer::could_be_imperative("cantad"));
    }
}
