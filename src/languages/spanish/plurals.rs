//! Despluralización para español
//!
//! Genera candidatos de singular a partir de una forma plural,
//! según las reglas de pluralización del español.

use std::collections::HashSet;

/// Devuelve candidatos de singular para una palabra plural en español.
///
/// Genera candidatos por reglas comunes de pluralización y
/// se usa como base para `Trie::derive_plural_info()`. No valida contra diccionario.
pub fn depluralize_candidates(word: &str) -> Vec<String> {
    let w = word.to_lowercase();
    let mut candidates: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let mut push_unique = |s: String| {
        if seen.insert(s.clone()) {
            candidates.push(s);
        }
    };

    // Reglas de más específicas a menos específicas.
    if let Some(stem) = w.strip_suffix("ces") {
        if !stem.is_empty() {
            push_unique(format!("{stem}z"));
        }
    }

    if let Some(stem) = w.strip_suffix("iones") {
        if !stem.is_empty() {
            push_unique(format!("{stem}ión"));
        }
    }

    if let Some(stem) = w.strip_suffix("anes") {
        if !stem.is_empty() {
            push_unique(format!("{stem}án"));
        }
    }

    if let Some(stem) = w.strip_suffix("enes") {
        if !stem.is_empty() {
            push_unique(format!("{stem}én"));
        }
    }

    if let Some(stem) = w.strip_suffix("eses") {
        if !stem.is_empty() {
            push_unique(format!("{stem}és"));
        }
    }

    if let Some(stem) = w.strip_suffix("ines") {
        if !stem.is_empty() {
            push_unique(format!("{stem}ín"));
        }
    }

    // "-ones" (pero no "-iones") -> "-ón"
    if w.ends_with("ones") && !w.ends_with("iones") {
        if let Some(stem) = w.strip_suffix("ones") {
            if !stem.is_empty() {
                push_unique(format!("{stem}ón"));
            }
        }
    }

    if let Some(stem) = w.strip_suffix("unes") {
        if !stem.is_empty() {
            push_unique(format!("{stem}ún"));
        }
    }

    // Vocal tónica + -es: rubíes -> rubí, tabúes -> tabú
    if let Some(stem) = w.strip_suffix("íes") {
        if !stem.is_empty() {
            push_unique(format!("{stem}í"));
        }
    }
    if let Some(stem) = w.strip_suffix("úes") {
        if !stem.is_empty() {
            push_unique(format!("{stem}ú"));
        }
    }

    // -es tras consonante (incluye 'y'): ciudades -> ciudad, leyes -> ley
    if let Some(stem) = w.strip_suffix("es") {
        if let Some(last) = stem.chars().last() {
            if !is_vowel(last) {
                push_unique(stem.to_string());
            }
        }
    }

    // -s tras vocal: abuelas -> abuela, cafés -> café
    if let Some(stem) = w.strip_suffix('s') {
        if let Some(last) = stem.chars().last() {
            if is_vowel(last) {
                push_unique(stem.to_string());
            }
        }
    }

    // -s tras consonante (anglicismos): banners -> banner, pellets -> pellet
    // Última prioridad; solo funciona si el singular existe en el diccionario.
    if let Some(stem) = w.strip_suffix('s') {
        if let Some(last) = stem.chars().last() {
            if !is_vowel(last) {
                push_unique(stem.to_string());
            }
        }
    }

    candidates
}

fn is_vowel(ch: char) -> bool {
    matches!(
        ch,
        'a' | 'e' | 'i' | 'o' | 'u' | 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ü'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_depluralize_candidates_rules() {
        let cases = [
            ("veces", "vez"),         // -ces -> -z
            ("canciones", "canción"), // -iones -> -ión
            ("alemanes", "alemán"),   // -anes -> -án
            ("almacenes", "almacén"), // -enes -> -én
            ("franceses", "francés"), // -eses -> -és
            ("jardines", "jardín"),   // -ines -> -ín
            ("leones", "león"),       // -ones -> -ón
            ("comunes", "común"),     // -unes -> -ún
            ("rubíes", "rubí"),       // -íes -> -í
            ("tabúes", "tabú"),       // -úes -> -ú
            ("ciudades", "ciudad"),   // consonante + es
            ("leyes", "ley"),         // 'y' + es
            ("abuelas", "abuela"),    // vocal + s
            ("cafés", "café"),        // vocal + s (tónica)
            ("banners", "banner"),    // consonante + s (anglicismo)
            ("pellets", "pellet"),    // consonante + s (anglicismo)
        ];

        for (plural, expected) in cases {
            let cands = depluralize_candidates(plural);
            assert!(
                cands.contains(&expected.to_string()),
                "Expected '{}' to produce candidate '{}', got {:?}",
                plural,
                expected,
                cands
            );
        }
    }
}
