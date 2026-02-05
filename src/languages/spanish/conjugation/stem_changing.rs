//! Verbos con cambio de raíz (stem-changing verbs)
//!
//! Maneja los tipos de cambios vocálicos y consonánticos:
//! - e→ie: pensar → pienso, entender → entiendo
//! - o→ue: contar → cuento, dormir → duermo
//! - e→i: pedir → pido, servir → sirvo
//! - c→zc: conocer → conozco, parecer → parezco

use std::collections::HashMap;
use std::sync::OnceLock;

/// Tipo de cambio de raíz
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StemChangeType {
    /// e → ie (pensar, entender, querer)
    EToIe,
    /// o → ue (contar, dormir, poder)
    OToUe,
    /// e → i (pedir, servir) - solo verbos -ir
    EToI,
    /// u → ue (jugar - único verbo común)
    UToUe,
    /// c → zc (conocer, parecer) - verbos -ecer, -ocer, -ucir
    CToZc,
}

impl StemChangeType {
    /// Obtiene el par (original, cambiado) para el cambio
    pub fn change_pair(&self) -> (&'static str, &'static str) {
        match self {
            StemChangeType::EToIe => ("e", "ie"),
            StemChangeType::OToUe => ("o", "ue"),
            StemChangeType::EToI => ("e", "i"),
            StemChangeType::UToUe => ("u", "ue"),
            StemChangeType::CToZc => ("c", "zc"),
        }
    }

    /// Alias para compatibilidad
    pub fn vowel_pair(&self) -> (&'static str, &'static str) {
        self.change_pair()
    }

    /// Revierte el cambio: dado un stem cambiado, devuelve el original
    pub fn reverse_change(&self, stem: &str) -> Option<String> {
        let (original, changed) = self.change_pair();

        // Para c→zc, buscar "zc" al final del stem (antes de la terminación)
        if *self == StemChangeType::CToZc {
            if stem.ends_with("zc") {
                let mut result = stem[..stem.len() - 2].to_string();
                result.push_str("c");
                return Some(result);
            }
            return None;
        }

        // Para cambios vocálicos, buscar la última ocurrencia
        if let Some(pos) = stem.rfind(changed) {
            let mut result = String::with_capacity(stem.len());
            result.push_str(&stem[..pos]);
            result.push_str(original);
            result.push_str(&stem[pos + changed.len()..]);
            Some(result)
        } else {
            None
        }
    }
}

fn build_stem_changing_verbs() -> HashMap<&'static str, StemChangeType> {
    let mut map = HashMap::new();

    // ========== e → ie ==========

    // Verbos -ar con e→ie
    for verb in [
        "acertar", "apretar", "atravesar", "calentar", "cerrar", "comenzar",
        "confesar", "despertar", "empezar", "encerrar", "gobernar", "helar",
        "manifestar", "merendar", "negar", "nevar", "pensar", "plegar", "recomendar",
        "regar", "sembrar", "sentar", "temblar", "tropezar",
    ] {
        map.insert(verb, StemChangeType::EToIe);
    }

    // Verbos -er con e→ie
    for verb in [
        "ascender", "atender", "defender", "descender", "encender", "entender",
        "extender", "perder", "tender", "trascender", "verter",
    ] {
        map.insert(verb, StemChangeType::EToIe);
    }

    // Verbos -ir con e→ie
    for verb in [
        "advertir", "arrepentirse", "conferir", "consentir", "convertir", "divertir",
        "herir", "hervir", "inferir", "invertir", "mentir", "preferir", "presentir",
        "referir", "sentir", "sugerir", "transferir",
    ] {
        map.insert(verb, StemChangeType::EToIe);
    }

    // ========== o → ue ==========

    // Verbos -ar con o→ue
    // Nota: verbos -zar como forzar/reforzar/almorzar también tienen cambio z→c en subjuntivo
    for verb in [
        "acordar", "acostar", "almorzar", "apostar", "aprobar", "colgar",
        "comprobar", "contar", "costar", "demostrar", "encontrar", "esforzar",
        "forzar", "mostrar", "probar", "recordar", "reforzar", "renovar",
        "rodar", "rogar", "soltar", "sonar", "soñar", "tostar", "volar", "volcar",
    ] {
        map.insert(verb, StemChangeType::OToUe);
    }

    // Verbos -er con o→ue
    for verb in [
        "absolver", "conmover", "devolver", "disolver", "doler", "envolver",
        "llover", "morder", "mover", "oler", "promover", "remover", "resolver",
        "revolver", "soler", "torcer", "volver",
    ] {
        map.insert(verb, StemChangeType::OToUe);
    }
    map.insert("cocer", StemChangeType::OToUe);

    // Verbos -ir con o→ue
    for verb in ["dormir", "morir"] {
        map.insert(verb, StemChangeType::OToUe);
    }

    // ========== e → i (solo -ir) ==========
    for verb in [
        "adherir", "competir", "concebir", "conseguir", "corregir", "derretir", "despedir",
        "elegir", "freír", "gemir", "impedir", "medir", "pedir", "perseguir",
        "proseguir", "reír", "rendir", "repetir", "reñir", "seguir", "servir",
        "sonreír", "teñir", "vestir",
    ] {
        map.insert(verb, StemChangeType::EToI);
    }

    // ========== u → ue ==========
    map.insert("jugar", StemChangeType::UToUe);

    // ========== c → zc (verbos -ecer, -ocer, -ucir) ==========
    for verb in [
        // -ecer
        "agradecer", "amanecer", "anochecer", "aparecer", "apetecer",
        "carecer", "compadecer", "complacer", "conocer", "crecer",
        "desaparecer", "desconocer", "desobedecer", "embellecer", "empobrecer",
        "enloquecer", "enmudecer", "enorgullecer", "enriquecer", "enternecer",
        "envejecer", "esclarecer", "establecer", "estremecer", "favorecer", "florecer",
        "fortalecer", "humedecer", "merecer", "nacer", "obedecer",
        "obscurecer", "ofrecer", "oscurecer", "padecer", "palidecer",
        "parecer", "perecer", "permanecer", "pertenecer", "prevalecer",
        "reconocer", "rejuvenecer", "resplandecer", "restablecer",
        // -ucir
        "conducir", "deducir", "inducir", "introducir", "lucir",
        "producir", "reducir", "reproducir", "seducir", "traducir",
    ] {
        map.insert(verb, StemChangeType::CToZc);
    }

    map
}

/// Obtiene el mapa de infinitivos a su tipo de cambio de raíz.
///
/// Se cachea en un `OnceLock` para evitar reconstrucciones y asignaciones
/// repetidas en cada llamada.
pub fn get_stem_changing_verbs() -> &'static HashMap<&'static str, StemChangeType> {
    static STEM_CHANGING_VERBS: OnceLock<HashMap<&'static str, StemChangeType>> = OnceLock::new();
    STEM_CHANGING_VERBS.get_or_init(build_stem_changing_verbs)
}

/// Terminaciones que activan el cambio de raíz en presente indicativo
/// (1ª, 2ª, 3ª singular y 3ª plural)
pub const PRESENTE_STEM_CHANGE_ENDINGS_AR: [&str; 4] = ["o", "as", "a", "an"];
pub const PRESENTE_STEM_CHANGE_ENDINGS_ER: [&str; 4] = ["o", "es", "e", "en"];
pub const PRESENTE_STEM_CHANGE_ENDINGS_IR: [&str; 4] = ["o", "es", "e", "en"];

/// Terminaciones que activan el cambio en presente subjuntivo
pub const SUBJUNTIVO_STEM_CHANGE_ENDINGS_AR: [&str; 4] = ["e", "es", "e", "en"];
pub const SUBJUNTIVO_STEM_CHANGE_ENDINGS_ER: [&str; 4] = ["a", "as", "a", "an"];
pub const SUBJUNTIVO_STEM_CHANGE_ENDINGS_IR: [&str; 4] = ["a", "as", "a", "an"];

const STEM_CHANGE_ENDINGS_AR: [&str; 11] = [
    "o", "as", "a", "an", "e", "es", "e", "en", "ue", "ues", "uen",
];
const STEM_CHANGE_ENDINGS_ER: [&str; 8] = ["o", "es", "e", "en", "a", "as", "a", "an"];
const STEM_CHANGE_ENDINGS_IR: [&str; 11] = [
    "o", "es", "e", "en", "a", "as", "a", "an", "iendo", "ió", "ieron",
];
const C_TO_ZC_ENDINGS_ER: [&str; 6] = ["o", "a", "as", "amos", "áis", "an"];

/// Todas las terminaciones que pueden tener cambio de raíz para verbos -ar
pub fn get_stem_change_endings_ar() -> &'static [&'static str] {
    &STEM_CHANGE_ENDINGS_AR
}

/// Todas las terminaciones que pueden tener cambio de raíz para verbos -er
pub fn get_stem_change_endings_er() -> &'static [&'static str] {
    &STEM_CHANGE_ENDINGS_ER
}

/// Todas las terminaciones que pueden tener cambio de raíz para verbos -ir
pub fn get_stem_change_endings_ir() -> &'static [&'static str] {
    &STEM_CHANGE_ENDINGS_IR
}

/// Terminaciones que activan el cambio c→zc para verbos -ecer/-ocer/-ucir
/// Solo 1ª persona presente y todo el subjuntivo
pub fn get_c_to_zc_endings_er() -> &'static [&'static str] {
    &C_TO_ZC_ENDINGS_ER
}

/// Terminaciones c→zc para verbos -ucir (igual que -er)
pub fn get_c_to_zc_endings_ir() -> &'static [&'static str] {
    get_c_to_zc_endings_er()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stem_change_types() {
        assert_eq!(StemChangeType::EToIe.change_pair(), ("e", "ie"));
        assert_eq!(StemChangeType::OToUe.change_pair(), ("o", "ue"));
        assert_eq!(StemChangeType::EToI.change_pair(), ("e", "i"));
        assert_eq!(StemChangeType::UToUe.change_pair(), ("u", "ue"));
        assert_eq!(StemChangeType::CToZc.change_pair(), ("c", "zc"));
    }

    #[test]
    fn test_reverse_change() {
        // e→ie: pienspensar
        assert_eq!(
            StemChangeType::EToIe.reverse_change("piens"),
            Some("pens".to_string())
        );

        // o→ue: cuent → cont
        assert_eq!(
            StemChangeType::OToUe.reverse_change("cuent"),
            Some("cont".to_string())
        );

        // e→i: pid → ped
        assert_eq!(
            StemChangeType::EToI.reverse_change("pid"),
            Some("ped".to_string())
        );

        // u→ue: jueg → jug
        assert_eq!(
            StemChangeType::UToUe.reverse_change("jueg"),
            Some("jug".to_string())
        );

        // c→zc: conozc → conoc
        assert_eq!(
            StemChangeType::CToZc.reverse_change("conozc"),
            Some("conoc".to_string())
        );
        assert_eq!(
            StemChangeType::CToZc.reverse_change("parezc"),
            Some("parec".to_string())
        );
    }

    #[test]
    fn test_get_stem_changing_verbs() {
        let verbs = get_stem_changing_verbs();

        assert_eq!(verbs.get("pensar"), Some(&StemChangeType::EToIe));
        assert_eq!(verbs.get("entender"), Some(&StemChangeType::EToIe));
        assert_eq!(verbs.get("contar"), Some(&StemChangeType::OToUe));
        assert_eq!(verbs.get("dormir"), Some(&StemChangeType::OToUe));
        assert_eq!(verbs.get("pedir"), Some(&StemChangeType::EToI));
        assert_eq!(verbs.get("jugar"), Some(&StemChangeType::UToUe));
        assert_eq!(verbs.get("cocer"), Some(&StemChangeType::OToUe));
    }
}
