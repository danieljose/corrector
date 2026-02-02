//! Detección de género de nombres propios
//!
//! Proporciona funcionalidades para determinar el género de nombres propios comunes.

use crate::dictionary::Gender;

/// Nombres femeninos más comunes en español
const FEMININE_NAMES: &[&str] = &[
    // Nombres tradicionales españoles
    "maría", "carmen", "ana", "isabel", "laura", "elena", "rosa", "paula",
    "marta", "cristina", "lucía", "andrea", "sara", "alba", "silvia", "beatriz",
    "patricia", "susana", "mónica", "raquel", "julia", "luisa", "inés", "alicia",
    "teresa", "pilar", "dolores", "mercedes", "rocío", "irene", "victoria", "clara",
    "nuria", "eva", "olga", "sonia", "lourdes", "amparo", "consuelo", "esperanza",
    "soledad", "margarita", "carolina", "natalia", "adriana", "verónica", "lorena",
    "alejandra", "claudia", "daniela", "gabriela", "valentina", "camila", "sofía",
    "valeria", "fernanda", "marina", "rebeca", "diana", "emma", "nerea", "miriam",
    "noelia", "vanessa", "sandra", "esther", "yolanda", "inmaculada", "encarnación",
    "antonia", "josefa", "francisca", "manuela", "concepción", "asunción", "rosario",
    // Nombres modernos/internacionales comunes
    "jennifer", "jessica", "tiffany", "nicole", "stephanie", "michelle", "ashley",
    "amanda", "elizabeth", "catherine", "samantha", "sarah", "emily", "emma",
];

/// Nombres masculinos más comunes en español
const MASCULINE_NAMES: &[&str] = &[
    // Nombres tradicionales españoles
    "josé", "antonio", "juan", "manuel", "francisco", "pedro", "luis", "carlos",
    "miguel", "ángel", "david", "pablo", "jorge", "alberto", "fernando", "rafael",
    "javier", "sergio", "alejandro", "daniel", "roberto", "eduardo", "enrique",
    "ramón", "vicente", "andrés", "diego", "mario", "jesús", "tomás", "gabriel",
    "felipe", "ignacio", "jaime", "alfonso", "ricardo", "arturo", "marcos", "emilio",
    "agustín", "iván", "óscar", "hugo", "rubén", "raúl", "adrián", "víctor",
    "álvaro", "guillermo", "gonzalo", "nicolás", "santiago", "martín", "rodrigo",
    "samuel", "lucas", "mateo", "leo", "marc", "pau", "pol", "álex", "adrià",
    "arnau", "eric", "ivan", "adam", "bruno", "izan", "alex", "mario", "leo",
    // Nombres modernos/internacionales comunes
    "michael", "christopher", "matthew", "joshua", "daniel", "james", "john",
    "robert", "william", "david", "richard", "joseph", "thomas", "charles",
];

/// Determina el género de un nombre propio conocido
/// Devuelve None si el nombre no está en la lista o es ambiguo
pub fn get_name_gender(name: &str) -> Option<Gender> {
    let name_lower = name.to_lowercase();

    // Verificar si es nombre femenino conocido
    if FEMININE_NAMES.contains(&name_lower.as_str()) {
        return Some(Gender::Feminine);
    }

    // Verificar si es nombre masculino conocido
    if MASCULINE_NAMES.contains(&name_lower.as_str()) {
        return Some(Gender::Masculine);
    }

    // Heurísticas para nombres no listados
    // Solo aplicar con alta confianza
    guess_name_gender_heuristic(&name_lower)
}

/// Intenta adivinar el género de un nombre por heurísticas
/// Es conservador para evitar falsos positivos
fn guess_name_gender_heuristic(name: &str) -> Option<Gender> {
    // Apellidos comunes que terminan en 'a' pero no son femeninos
    // No usar heurística si puede ser apellido
    let possible_surnames = [
        "garcía", "mejía", "iniesta", "peña", "ojeda", "estrada", "ortega",
        "sosa", "vega", "mora", "herrera", "silva", "rivera", "serna", "sierra",
        "victoria", "valencia", "costa", "rocha", "ayala", "acosta", "espinoza",
        "ochoa", "araya", "guerra", "varela", "zúñiga", "barrera", "aldea",
        "roca", "iglesia", "rueda", "baena", "cuesta", "dueña", "estrella",
    ];

    if possible_surnames.contains(&name) {
        return None;
    }

    // Nombres claramente masculinos terminados en -o
    // Excepciones: amparo (es femenino pero termina en -o es raro)
    if name.ends_with('o') && name.len() > 2 {
        // Excepciones conocidas
        if name == "amparo" || name == "rosario" || name == "consuelo" || name == "socorro" {
            return Some(Gender::Feminine);
        }
        return Some(Gender::Masculine);
    }

    // Nombres terminados en consonante + variaciones típicas
    // No aplicar heurística general - demasiado arriesgado

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feminine_names() {
        assert_eq!(get_name_gender("María"), Some(Gender::Feminine));
        assert_eq!(get_name_gender("MARÍA"), Some(Gender::Feminine));
        assert_eq!(get_name_gender("carmen"), Some(Gender::Feminine));
        assert_eq!(get_name_gender("Laura"), Some(Gender::Feminine));
        assert_eq!(get_name_gender("Sofía"), Some(Gender::Feminine));
    }

    #[test]
    fn test_masculine_names() {
        assert_eq!(get_name_gender("José"), Some(Gender::Masculine));
        assert_eq!(get_name_gender("JOSÉ"), Some(Gender::Masculine));
        assert_eq!(get_name_gender("juan"), Some(Gender::Masculine));
        assert_eq!(get_name_gender("Carlos"), Some(Gender::Masculine));
        assert_eq!(get_name_gender("Miguel"), Some(Gender::Masculine));
    }

    #[test]
    fn test_unknown_names() {
        // Nombres no listados sin heurística clara
        assert_eq!(get_name_gender("Xylophone"), None);
    }

    #[test]
    fn test_heuristic_masculine_o() {
        // Nombres no listados pero terminados en -o
        assert_eq!(get_name_gender("Alejandro"), Some(Gender::Masculine));
        assert_eq!(get_name_gender("Rodrigo"), Some(Gender::Masculine));
    }

    #[test]
    fn test_heuristic_exceptions() {
        // Nombres femeninos que terminan en -o (excepciones)
        assert_eq!(get_name_gender("Amparo"), Some(Gender::Feminine));
        assert_eq!(get_name_gender("Rosario"), Some(Gender::Feminine));
        assert_eq!(get_name_gender("Consuelo"), Some(Gender::Feminine));
    }

    #[test]
    fn test_not_surname_confusion() {
        // Apellidos comunes no deben devolver género
        assert_eq!(get_name_gender("García"), None);
        assert_eq!(get_name_gender("Mejía"), None);
    }
}
