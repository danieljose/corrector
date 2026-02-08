//! Tablas de terminaciones para verbos regulares españoles

use super::VerbClass;

/// Terminaciones del presente de indicativo
pub const PRESENTE_AR: [&str; 6] = ["o", "as", "a", "amos", "áis", "an"];
pub const PRESENTE_ER: [&str; 6] = ["o", "es", "e", "emos", "éis", "en"];
pub const PRESENTE_IR: [&str; 6] = ["o", "es", "e", "imos", "ís", "en"];

/// Terminaciones del pretérito indefinido
pub const PRETERITO_AR: [&str; 6] = ["é", "aste", "ó", "amos", "asteis", "aron"];
pub const PRETERITO_ER: [&str; 6] = ["í", "iste", "ió", "imos", "isteis", "ieron"];
pub const PRETERITO_IR: [&str; 6] = ["í", "iste", "ió", "imos", "isteis", "ieron"];

/// Terminaciones del pretérito imperfecto
pub const IMPERFECTO_AR: [&str; 6] = ["aba", "abas", "aba", "ábamos", "abais", "aban"];
pub const IMPERFECTO_ER: [&str; 6] = ["ía", "ías", "ía", "íamos", "íais", "ían"];
pub const IMPERFECTO_IR: [&str; 6] = ["ía", "ías", "ía", "íamos", "íais", "ían"];

/// Terminaciones del futuro simple (se añaden al infinitivo completo)
pub const FUTURO: [&str; 6] = ["é", "ás", "á", "emos", "éis", "án"];

/// Terminaciones del condicional simple (se añaden al infinitivo completo)
pub const CONDICIONAL: [&str; 6] = ["ía", "ías", "ía", "íamos", "íais", "ían"];

/// Terminaciones del presente de subjuntivo
pub const SUBJUNTIVO_PRESENTE_AR: [&str; 6] = ["e", "es", "e", "emos", "éis", "en"];
pub const SUBJUNTIVO_PRESENTE_ER: [&str; 6] = ["a", "as", "a", "amos", "áis", "an"];
pub const SUBJUNTIVO_PRESENTE_IR: [&str; 6] = ["a", "as", "a", "amos", "áis", "an"];

/// Terminaciones del imperfecto de subjuntivo (-ra)
pub const SUBJUNTIVO_IMPERFECTO_RA_AR: [&str; 6] =
    ["ara", "aras", "ara", "áramos", "arais", "aran"];
pub const SUBJUNTIVO_IMPERFECTO_RA_ER: [&str; 6] =
    ["iera", "ieras", "iera", "iéramos", "ierais", "ieran"];
pub const SUBJUNTIVO_IMPERFECTO_RA_IR: [&str; 6] =
    ["iera", "ieras", "iera", "iéramos", "ierais", "ieran"];

/// Terminaciones del imperfecto de subjuntivo (-se)
pub const SUBJUNTIVO_IMPERFECTO_SE_AR: [&str; 6] =
    ["ase", "ases", "ase", "ásemos", "aseis", "asen"];
pub const SUBJUNTIVO_IMPERFECTO_SE_ER: [&str; 6] =
    ["iese", "ieses", "iese", "iésemos", "ieseis", "iesen"];
pub const SUBJUNTIVO_IMPERFECTO_SE_IR: [&str; 6] =
    ["iese", "ieses", "iese", "iésemos", "ieseis", "iesen"];

/// Terminaciones del futuro de subjuntivo (poco usado, pero válido)
pub const SUBJUNTIVO_FUTURO_AR: [&str; 6] = ["are", "ares", "are", "áremos", "areis", "aren"];
pub const SUBJUNTIVO_FUTURO_ER: [&str; 6] = ["iere", "ieres", "iere", "iéremos", "iereis", "ieren"];
pub const SUBJUNTIVO_FUTURO_IR: [&str; 6] = ["iere", "ieres", "iere", "iéremos", "iereis", "ieren"];

/// Terminación del gerundio
pub const GERUNDIO_AR: &str = "ando";
pub const GERUNDIO_ER: &str = "iendo";
pub const GERUNDIO_IR: &str = "iendo";

/// Terminación del participio
pub const PARTICIPIO_AR: &str = "ado";
pub const PARTICIPIO_ER: &str = "ido";
pub const PARTICIPIO_IR: &str = "ido";

/// Terminación del imperativo vosotros
/// (las demás formas ya coinciden con presente/subjuntivo)
pub const IMPERATIVO_VOSOTROS_AR: &str = "ad";
pub const IMPERATIVO_VOSOTROS_ER: &str = "ed";
pub const IMPERATIVO_VOSOTROS_IR: &str = "id";

/// Obtiene todas las terminaciones posibles para una clase de verbo
pub fn get_all_endings(class: VerbClass) -> Vec<&'static str> {
    let mut endings = Vec::with_capacity(80);

    match class {
        VerbClass::Ar => {
            endings.extend_from_slice(&PRESENTE_AR);
            endings.extend_from_slice(&PRETERITO_AR);
            endings.extend_from_slice(&IMPERFECTO_AR);
            endings.extend_from_slice(&SUBJUNTIVO_PRESENTE_AR);
            endings.extend_from_slice(&SUBJUNTIVO_IMPERFECTO_RA_AR);
            endings.extend_from_slice(&SUBJUNTIVO_IMPERFECTO_SE_AR);
            endings.extend_from_slice(&SUBJUNTIVO_FUTURO_AR);
            endings.push(GERUNDIO_AR);
            endings.push(PARTICIPIO_AR);
            endings.push(IMPERATIVO_VOSOTROS_AR);
        }
        VerbClass::Er => {
            endings.extend_from_slice(&PRESENTE_ER);
            endings.extend_from_slice(&PRETERITO_ER);
            endings.extend_from_slice(&IMPERFECTO_ER);
            endings.extend_from_slice(&SUBJUNTIVO_PRESENTE_ER);
            endings.extend_from_slice(&SUBJUNTIVO_IMPERFECTO_RA_ER);
            endings.extend_from_slice(&SUBJUNTIVO_IMPERFECTO_SE_ER);
            endings.extend_from_slice(&SUBJUNTIVO_FUTURO_ER);
            endings.push(GERUNDIO_ER);
            endings.push(PARTICIPIO_ER);
            endings.push(IMPERATIVO_VOSOTROS_ER);
        }
        VerbClass::Ir => {
            endings.extend_from_slice(&PRESENTE_IR);
            endings.extend_from_slice(&PRETERITO_IR);
            endings.extend_from_slice(&IMPERFECTO_IR);
            endings.extend_from_slice(&SUBJUNTIVO_PRESENTE_IR);
            endings.extend_from_slice(&SUBJUNTIVO_IMPERFECTO_RA_IR);
            endings.extend_from_slice(&SUBJUNTIVO_IMPERFECTO_SE_IR);
            endings.extend_from_slice(&SUBJUNTIVO_FUTURO_IR);
            endings.push(GERUNDIO_IR);
            endings.push(PARTICIPIO_IR);
            endings.push(IMPERATIVO_VOSOTROS_IR);
        }
    }

    // Añadir terminaciones de futuro y condicional (basadas en infinitivo completo)
    // Estas se manejan de forma especial en el reconocedor

    endings
}

/// Obtiene la terminación del gerundio para una clase de verbo
pub fn get_gerund_ending(class: VerbClass) -> &'static str {
    match class {
        VerbClass::Ar => GERUNDIO_AR,
        VerbClass::Er | VerbClass::Ir => GERUNDIO_ER,
    }
}

/// Obtiene la terminación del participio para una clase de verbo
pub fn get_participle_ending(class: VerbClass) -> &'static str {
    match class {
        VerbClass::Ar => PARTICIPIO_AR,
        VerbClass::Er | VerbClass::Ir => PARTICIPIO_ER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_all_endings_ar() {
        let endings = get_all_endings(VerbClass::Ar);
        assert!(endings.contains(&"o"));
        assert!(endings.contains(&"amos"));
        assert!(endings.contains(&"aron"));
        assert!(endings.contains(&"aba"));
        assert!(endings.contains(&"ando"));
        assert!(endings.contains(&"ado"));
    }

    #[test]
    fn test_get_all_endings_er() {
        let endings = get_all_endings(VerbClass::Er);
        assert!(endings.contains(&"o"));
        assert!(endings.contains(&"emos"));
        assert!(endings.contains(&"ieron"));
        assert!(endings.contains(&"ía"));
        assert!(endings.contains(&"iendo"));
        assert!(endings.contains(&"ido"));
    }

    #[test]
    fn test_get_all_endings_ir() {
        let endings = get_all_endings(VerbClass::Ir);
        assert!(endings.contains(&"o"));
        assert!(endings.contains(&"imos"));
        assert!(endings.contains(&"ieron"));
        assert!(endings.contains(&"ía"));
        assert!(endings.contains(&"iendo"));
        assert!(endings.contains(&"ido"));
    }

    #[test]
    fn test_imperativo_vosotros_endings() {
        let ar_endings = get_all_endings(VerbClass::Ar);
        let er_endings = get_all_endings(VerbClass::Er);
        let ir_endings = get_all_endings(VerbClass::Ir);

        assert!(ar_endings.contains(&"ad")); // cantad
        assert!(er_endings.contains(&"ed")); // comed
        assert!(ir_endings.contains(&"id")); // vivid
    }
}
