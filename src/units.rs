//! Detección de unidades de medida
//!
//! Módulo centralizado para detectar unidades y evitar marcarlas como errores.

/// Sufijos típicos de unidades (case-sensitive)
pub const UNIT_SUFFIXES: &[&str] = &[
    // Energía/potencia
    "Wh", "kWh", "MWh", "GWh", "TWh", "Ah", "mAh", "W", "kW", "MW", "GW", "V", "VA", "kVA",
    "MVA",
    // Frecuencia
    "Hz", "kHz", "MHz", "GHz", "THz", // Datos/velocidad
    "bps", "kbps", "Kbps", "Mbps", "Gbps", "bit", "kbit", "Mbit", "Gbit", "B", "kB", "KB", "MB",
    "GB", "TB", "PB", "iB", "KiB", "MiB", "GiB", "TiB", // Sonido/señal
    "dB", "dBm", "dBi", // Presión
    "Pa", "kPa", "MPa", "hPa", "ppm", "ppb", // Tiempo
    "ms", "ns", "µs", // Temperatura
    "C", "F", "K", // Usados tras ° o º
    "ºC", "ºF", // Con símbolo de grado ordinal
    // Unidades con superíndices
    "m²", "m³", "cm²", "cm³", "km²", "s²", "s⁻¹", "m⁻¹", // Newton y derivados
    "N", "kN", "MN",
];

/// Unidades comunes en minúsculas
pub const LOWERCASE_UNITS: &[&str] = &[
    // Longitud
    "km", "m", "cm", "mm", "mi", "ft", "in", "yd", "nm", // Peso
    "kg", "g", "mg", "lb", "oz", "t", // Volumen
    "l", "ml", "cl", "dl", "gal", // Tiempo
    "h", "min", "s", // Digital (minúsculas alternativas)
    "kb", "mb", "gb", "tb", "pb", // Otros
    "rpm",
];

/// Unidades y abreviaturas ALL CAPS conocidas
const UPPERCASE_UNITS: &[&str] = &[
    // Digital
    "KB", "MB", "GB", "TB", "PB", "EB", "CPU", "GPU", "RAM", "ROM", "SSD", "HDD",
    // Frecuencia/señal
    "HZ", "KHZ", "MHZ", "GHZ", "DB", "DBM", // Energía
    "KW", "MW", "GW", "WH", "KWH", "MWH", "VA", "KVA", "MVA", // Otros
    "RPM", "BPS", "FPS", "PA", "KPA", "MPA", "PPM", "PPB",
];

/// Normaliza exponentes ASCII a superíndices Unicode
/// Ejemplos: "m^2" → "m²", "s^-1" → "s⁻¹", "m2" → "m²" (si dígito final tras letra)
fn normalize_exponents(word: &str) -> String {
    let mut result = word.to_string();

    // Convertir ^n a superíndice
    result = result
        .replace("^-1", "⁻¹")
        .replace("^0", "⁰")
        .replace("^1", "¹")
        .replace("^2", "²")
        .replace("^3", "³")
        .replace("^4", "⁴")
        .replace("^5", "⁵")
        .replace("^6", "⁶")
        .replace("^7", "⁷")
        .replace("^8", "⁸")
        .replace("^9", "⁹");

    // Si termina en dígito tras letra (m2, s2, km2), convertir a superíndice
    let chars: Vec<char> = result.chars().collect();
    if chars.len() >= 2 {
        let last = chars[chars.len() - 1];
        let prev = chars[chars.len() - 2];

        if last.is_ascii_digit() && prev.is_alphabetic() {
            let superscript = match last {
                '0' => '⁰',
                '1' => '¹',
                '2' => '²',
                '3' => '³',
                '4' => '⁴',
                '5' => '⁵',
                '6' => '⁶',
                '7' => '⁷',
                '8' => '⁸',
                '9' => '⁹',
                _ => last,
            };
            result = format!("{}{}", &result[..result.len() - 1], superscript);
        }
    }

    result
}

/// Verifica si una palabra es una unidad de medida conocida
pub fn is_known_unit(word: &str) -> bool {
    // Verificar sufijos exactos (case-sensitive)
    if UNIT_SUFFIXES.contains(&word) {
        return true;
    }

    // Verificar con exponentes normalizados (m^2 → m², s2 → s²)
    let normalized = normalize_exponents(word);
    if normalized != word && UNIT_SUFFIXES.contains(&normalized.as_str()) {
        return true;
    }

    // Verificar unidades en minúsculas
    let lower = word.to_lowercase();
    if LOWERCASE_UNITS.contains(&lower.as_str()) {
        return true;
    }

    // Verificar unidades ALL CAPS conocidas (no cualquier palabra en mayúsculas)
    let upper = word.to_uppercase();
    if UPPERCASE_UNITS.contains(&upper.as_str()) {
        return true;
    }

    false
}

/// Verifica si una palabra parece una unidad por su patrón
/// (mezcla mayúscula/minúscula corta típica de SI)
pub fn is_unit_like(word: &str) -> bool {
    if word.is_empty() || word.len() > 10 {
        return false;
    }

    // Primero verificar si es unidad conocida
    if is_known_unit(word) {
        return true;
    }

    // Heurística: mezcla de mayúscula/minúscula corta (2-5 chars)
    if word.len() >= 2 && word.len() <= 5 {
        let has_upper = word.chars().any(|c| c.is_uppercase());
        let has_lower = word.chars().any(|c| c.is_lowercase());
        let all_alpha = word.chars().all(|c| c.is_alphabetic());

        if has_upper && has_lower && all_alpha {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_units() {
        assert!(is_known_unit("kWh"));
        assert!(is_known_unit("mAh"));
        assert!(is_known_unit("MHz"));
        assert!(is_known_unit("dB"));
        assert!(is_known_unit("V"));
        assert!(is_known_unit("km"));
        assert!(is_known_unit("MB"));
        assert!(is_known_unit("GB"));
    }

    #[test]
    fn test_unit_like() {
        assert!(is_unit_like("kWh"));
        assert!(is_unit_like("mAh"));
        assert!(is_unit_like("Mbps"));
        assert!(is_unit_like("GHz"));
        // Patrón mixto desconocido pero válido
        assert!(is_unit_like("kVAr"));
    }

    #[test]
    fn test_not_units() {
        assert!(!is_unit_like("casa"));
        assert!(!is_unit_like("CASA"));
        assert!(!is_unit_like(""));
    }

    #[test]
    fn test_normalize_exponents() {
        assert_eq!(normalize_exponents("m^2"), "m²");
        assert_eq!(normalize_exponents("s^2"), "s²");
        assert_eq!(normalize_exponents("m^3"), "m³");
        assert_eq!(normalize_exponents("s^-1"), "s⁻¹");
        assert_eq!(normalize_exponents("m2"), "m²");
        assert_eq!(normalize_exponents("s2"), "s²");
        assert_eq!(normalize_exponents("km2"), "km²");
        // Sin cambio si ya tiene superíndice
        assert_eq!(normalize_exponents("m²"), "m²");
        // Sin cambio si no tiene exponente
        assert_eq!(normalize_exponents("km"), "km");
    }

    #[test]
    fn test_units_with_ascii_exponents() {
        // Unidades con exponente ^ deben reconocerse
        assert!(is_known_unit("m^2"));
        assert!(is_known_unit("s^2"));
        assert!(is_known_unit("cm^2"));
        assert!(is_known_unit("km^2"));
        assert!(is_known_unit("s^-1"));
        // Unidades con dígito final deben reconocerse
        assert!(is_known_unit("m2"));
        assert!(is_known_unit("s2"));
        assert!(is_known_unit("cm2"));
    }

    #[test]
    fn test_unit_like_with_exponents() {
        // is_unit_like debe aceptar exponentes ASCII
        assert!(is_unit_like("m^2"));
        assert!(is_unit_like("s^2"));
        assert!(is_unit_like("m2"));
        assert!(is_unit_like("s2"));
        // Y también superíndices
        assert!(is_unit_like("m²"));
        assert!(is_unit_like("s²"));
        assert!(is_unit_like("m³"));
    }
}
