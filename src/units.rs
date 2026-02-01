//! Detección de unidades de medida
//!
//! Módulo centralizado para detectar unidades y evitar marcarlas como errores.

/// Sufijos típicos de unidades (case-sensitive)
pub const UNIT_SUFFIXES: &[&str] = &[
    // Energía/potencia
    "Wh", "kWh", "MWh", "GWh", "TWh",
    "Ah", "mAh",
    "W", "kW", "MW", "GW",
    "VA", "kVA", "MVA",
    // Frecuencia
    "Hz", "kHz", "MHz", "GHz", "THz",
    // Datos/velocidad
    "bps", "kbps", "Kbps", "Mbps", "Gbps",
    "bit", "kbit", "Mbit", "Gbit",
    "B", "kB", "KB", "MB", "GB", "TB", "PB",
    "iB", "KiB", "MiB", "GiB", "TiB",
    // Sonido/señal
    "dB", "dBm", "dBi",
    // Presión
    "Pa", "kPa", "MPa", "hPa",
    "ppm", "ppb",
    // Tiempo
    "ms", "ns", "µs",
];

/// Unidades comunes en minúsculas
pub const LOWERCASE_UNITS: &[&str] = &[
    // Longitud
    "km", "m", "cm", "mm", "mi", "ft", "in", "yd", "nm",
    // Peso
    "kg", "g", "mg", "lb", "oz", "t",
    // Volumen
    "l", "ml", "cl", "dl", "gal",
    // Tiempo
    "h", "min", "s",
    // Digital (minúsculas alternativas)
    "kb", "mb", "gb", "tb", "pb",
    // Otros
    "rpm",
];

/// Unidades y abreviaturas ALL CAPS conocidas
const UPPERCASE_UNITS: &[&str] = &[
    // Digital
    "KB", "MB", "GB", "TB", "PB", "EB",
    "CPU", "GPU", "RAM", "ROM", "SSD", "HDD",
    // Frecuencia/señal
    "HZ", "KHZ", "MHZ", "GHZ",
    "DB", "DBM",
    // Energía
    "KW", "MW", "GW", "WH", "KWH", "MWH",
    "VA", "KVA", "MVA",
    // Otros
    "RPM", "BPS", "FPS",
    "PA", "KPA", "MPA",
    "PPM", "PPB",
];

/// Verifica si una palabra es una unidad de medida conocida
pub fn is_known_unit(word: &str) -> bool {
    // Verificar sufijos exactos (case-sensitive)
    if UNIT_SUFFIXES.contains(&word) {
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
}
