//! Reconocedor de formas verbales conjugadas

use std::collections::{HashMap, HashSet};

use crate::dictionary::{Trie, WordCategory};

use super::enclitics::EncliticsAnalyzer;
use super::irregular::get_irregular_forms;
use super::prefixes::PrefixAnalyzer;
use super::regular::{self, CONDICIONAL, FUTURO};
use super::stem_changing::{self, get_stem_changing_verbs, StemChangeType};
use super::VerbClass;

/// Reconocedor de formas verbales
///
/// Permite verificar si una palabra es una forma verbal válida
/// sin necesidad de tenerla explícitamente en el diccionario.
pub struct VerbRecognizer {
    /// Conjunto de infinitivos conocidos del diccionario
    infinitives: HashSet<String>,
    /// Mapa de formas irregulares a infinitivos
    irregular_lookup: HashMap<String, String>,
    /// Mapa de infinitivos con cambio de raíz a su tipo de cambio
    stem_changing_verbs: HashMap<String, StemChangeType>,
    /// Mapa de infinitivo sin "se" → infinitivo pronominal completo
    pronominal_verbs: HashMap<String, String>,
}

impl VerbRecognizer {
    /// Crea un nuevo reconocedor a partir de un diccionario Trie
    pub fn from_dictionary(trie: &Trie) -> Self {
        let mut infinitives = HashSet::new();
        let mut pronominal_verbs = HashMap::new();

        // Extraer todos los verbos en infinitivo del diccionario
        for (word, info) in trie.get_all_words() {
            if info.category == WordCategory::Verbo {
                // Verificar que termine en -ar, -er, -ir (incluyendo pronominales -arse, -erse, -irse)
                let is_regular_inf =
                    word.ends_with("ar") || word.ends_with("er") || word.ends_with("ir");
                let is_pronominal =
                    word.ends_with("arse") || word.ends_with("erse") || word.ends_with("irse");

                if is_regular_inf || is_pronominal {
                    infinitives.insert(word.clone());

                    // Si es un verbo pronominal, también añadir la versión sin "se"
                    if is_pronominal {
                        let base = &word[..word.len() - 2]; // quitar "se"
                        pronominal_verbs.insert(base.to_string(), word.clone());
                        infinitives.insert(base.to_string());
                    }
                }
            }
        }

        // Cargar formas irregulares
        let irregular_forms = get_irregular_forms();
        let irregular_lookup: HashMap<String, String> = irregular_forms
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        // Cargar verbos con cambio de raíz
        let stem_changing_map = get_stem_changing_verbs();
        let stem_changing_verbs: HashMap<String, StemChangeType> = stem_changing_map
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        Self {
            infinitives,
            irregular_lookup,
            stem_changing_verbs,
            pronominal_verbs,
        }
    }

    /// Verifica si una palabra es una forma verbal válida
    pub fn is_valid_verb_form(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();

        // 1. Primero buscar en formas irregulares
        if self.irregular_lookup.contains_key(&word_lower) {
            return true;
        }

        // 2. Intentar reconocer como forma regular
        if self.try_recognize_regular(&word_lower) {
            return true;
        }

        // 3. Intentar reconocer como forma con cambio de raíz
        if self.try_recognize_stem_changing(&word_lower) {
            return true;
        }

        // 4. Intentar reconocer cambio ortográfico z→c (verbos -zar)
        if self.try_recognize_orthographic_zar(&word_lower) {
            return true;
        }

        // 4b. Intentar reconocer cambio ortográfico g→gu (verbos -gar)
        if self.try_recognize_orthographic_gar(&word_lower) {
            return true;
        }

        // 4c. Intentar reconocer cambio ortográfico c→qu (verbos -car)
        if self.try_recognize_orthographic_car(&word_lower) {
            return true;
        }

        // 5. Intentar reconocer como forma con prefijo
        if self.try_recognize_prefixed(&word_lower) {
            return true;
        }

        // 6. Intentar reconocer como forma con enclíticos
        self.try_recognize_with_enclitics(&word_lower)
    }

    /// Verifica si una palabra es un gerundio de un verbo conocido
    ///
    /// A diferencia de simplemente verificar si termina en -ando/-iendo/-yendo,
    /// este método confirma que el gerundio corresponde a un infinitivo real.
    ///
    /// Ejemplos:
    /// - "abandonando" → true (gerundio de "abandonar")
    /// - "comiendo" → true (gerundio de "comer")
    /// - "mando" → false (1ª persona de "mandar", no gerundio)
    /// - "blando" → false (adjetivo, no gerundio)
    pub fn is_gerund(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();

        // Solo considerar palabras que terminen con sufijos de gerundio
        if !EncliticsAnalyzer::is_gerund(&word_lower) {
            return false;
        }

        // Verificar que corresponde a un infinitivo conocido
        self.try_recognize_gerund_base(&word_lower)
    }

    /// Obtiene el infinitivo de una forma verbal (si es reconocida)
    pub fn get_infinitive(&self, word: &str) -> Option<String> {
        let word_lower = word.to_lowercase();

        // 1. Primero buscar en formas irregulares
        if let Some(inf) = self.irregular_lookup.get(&word_lower) {
            // Si el infinitivo tiene versión pronominal, devolverla
            if let Some(pronominal) = self.pronominal_verbs.get(inf) {
                return Some(pronominal.clone());
            }
            return Some(inf.clone());
        }

        // 2. Intentar extraer infinitivo de forma regular
        if let Some(inf) = self.extract_infinitive_regular(&word_lower) {
            // Si el infinitivo tiene versión pronominal, devolverla
            if let Some(pronominal) = self.pronominal_verbs.get(&inf) {
                return Some(pronominal.clone());
            }
            return Some(inf);
        }

        // 3. Intentar extraer infinitivo de forma con cambio de raíz
        if let Some(inf) = self.extract_infinitive_stem_changing(&word_lower) {
            // Si el infinitivo tiene versión pronominal, devolverla
            if let Some(pronominal) = self.pronominal_verbs.get(&inf) {
                return Some(pronominal.clone());
            }
            return Some(inf);
        }

        // 4. Intentar extraer infinitivo de forma con cambio ortográfico z→c
        if let Some(inf) = self.extract_infinitive_orthographic_zar(&word_lower) {
            if let Some(pronominal) = self.pronominal_verbs.get(&inf) {
                return Some(pronominal.clone());
            }
            return Some(inf);
        }

        // 4b. Intentar extraer infinitivo de forma con cambio ortográfico g→gu
        if let Some(inf) = self.extract_infinitive_orthographic_gar(&word_lower) {
            if let Some(pronominal) = self.pronominal_verbs.get(&inf) {
                return Some(pronominal.clone());
            }
            return Some(inf);
        }

        // 4c. Intentar extraer infinitivo de forma con cambio ortográfico c→qu
        if let Some(inf) = self.extract_infinitive_orthographic_car(&word_lower) {
            if let Some(pronominal) = self.pronominal_verbs.get(&inf) {
                return Some(pronominal.clone());
            }
            return Some(inf);
        }

        // 5. Intentar extraer infinitivo de forma con prefijo
        if let Some(inf) = self.extract_infinitive_prefixed(&word_lower) {
            return Some(inf);
        }

        // 6. Intentar extraer infinitivo de forma con enclíticos
        self.extract_infinitive_with_enclitics(&word_lower)
    }

    /// Intenta reconocer una palabra como forma verbal regular
    fn try_recognize_regular(&self, word: &str) -> bool {
        // Probar cada clase de verbo
        for class in [VerbClass::Ar, VerbClass::Er, VerbClass::Ir] {
            if self.try_class(word, class) {
                return true;
            }
        }

        // Probar futuro y condicional (basados en infinitivo completo)
        self.try_future_conditional(word)
    }

    /// Intenta reconocer una palabra como forma de una clase específica
    fn try_class(&self, word: &str, class: VerbClass) -> bool {
        let endings = regular::get_all_endings(class);
        let inf_ending = class.infinitive_ending();

        for ending in endings {
            if let Some(stem) = word.strip_suffix(ending) {
                if !stem.is_empty() {
                    // Construir el infinitivo candidato
                    let candidate = format!("{}{}", stem, inf_ending);
                    if self.infinitives.contains(&candidate) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Intenta reconocer formas de futuro y condicional
    fn try_future_conditional(&self, word: &str) -> bool {
        // Futuro: infinitivo + é/ás/á/emos/éis/án
        for ending in FUTURO.iter() {
            if let Some(base) = word.strip_suffix(ending) {
                // El base debería ser un infinitivo
                if self.infinitives.contains(base) {
                    return true;
                }
                // Probar raíces irregulares de futuro/condicional
                if self.try_irregular_future_stem(base) {
                    return true;
                }
            }
        }

        // Condicional: infinitivo + ía/ías/ía/íamos/íais/ían
        for ending in CONDICIONAL.iter() {
            if let Some(base) = word.strip_suffix(ending) {
                if self.infinitives.contains(base) {
                    return true;
                }
                // Probar raíces irregulares de futuro/condicional
                if self.try_irregular_future_stem(base) {
                    return true;
                }
            }
        }

        false
    }

    /// Intenta reconocer una raíz irregular de futuro/condicional
    /// Ejemplo: "valdr" → "valer", "equivaldr" → "equivaler"
    fn try_irregular_future_stem(&self, stem: &str) -> bool {
        // Patrones de raíces irregulares:
        // - Verbos -er con raíz en -dr: valer→valdr, tener→tendr, poner→pondr
        // - Verbos -ir con raíz en -dr: salir→saldr, venir→vendr
        // - Verbos -er con raíz en -br: caber→cabr, saber→sabr, haber→habr
        // - Verbos -er con raíz en -rr: querer→querr

        // Patrón: raíz termina en "dr" → probar infinitivos -er e -ir
        if stem.ends_with("dr") {
            let base = &stem[..stem.len() - 2];
            // Probar -er (valer, tener, poner, equivaler, contener, etc.)
            let candidate_er = format!("{}er", base);
            if self.infinitives.contains(&candidate_er) {
                return true;
            }
            // Probar -ir (salir, venir, convenir, prevenir, etc.)
            let candidate_ir = format!("{}ir", base);
            if self.infinitives.contains(&candidate_ir) {
                return true;
            }
        }

        // Patrón: raíz termina en "br" → probar infinitivo -ber
        if stem.ends_with("br") {
            let base = &stem[..stem.len() - 2];
            let candidate = format!("{}ber", base);
            if self.infinitives.contains(&candidate) {
                return true;
            }
        }

        // Patrón: raíz termina en "rr" → probar infinitivo -rer (querer)
        if stem.ends_with("rr") {
            let base = &stem[..stem.len() - 2];
            let candidate = format!("{}rer", base);
            if self.infinitives.contains(&candidate) {
                return true;
            }
        }

        // Patrón: raíz termina en "odr" → probar infinitivo -oder (poder)
        if stem.ends_with("odr") {
            let base = &stem[..stem.len() - 3];
            let candidate = format!("{}oder", base);
            if self.infinitives.contains(&candidate) {
                return true;
            }
        }

        false
    }

    /// Intenta reconocer una forma verbal con cambio de raíz
    fn try_recognize_stem_changing(&self, word: &str) -> bool {
        // Probar cada clase de verbo con sus terminaciones que activan cambio de raíz
        for (_class, endings, inf_ending) in [
            (VerbClass::Ar, stem_changing::get_stem_change_endings_ar(), "ar"),
            (VerbClass::Er, stem_changing::get_stem_change_endings_er(), "er"),
            (VerbClass::Ir, stem_changing::get_stem_change_endings_ir(), "ir"),
        ] {
            if self.try_stem_change_class(word, &endings, inf_ending) {
                return true;
            }
        }

        // Probar c→zc para verbos -ecer/-ocer y -ucir
        if self.try_c_to_zc_class(word, "er") {
            return true;
        }
        if self.try_c_to_zc_class(word, "ir") {
            return true;
        }

        false
    }

    /// Intenta reconocer una forma con cambio c→zc
    fn try_c_to_zc_class(&self, word: &str, inf_ending: &str) -> bool {
        let endings = stem_changing::get_c_to_zc_endings_er();

        for ending in endings {
            if let Some(changed_stem) = word.strip_suffix(ending) {
                if changed_stem.is_empty() {
                    continue;
                }

                if let Some(original_stem) = StemChangeType::CToZc.reverse_change(changed_stem) {
                    let candidate = format!("{}{}", original_stem, inf_ending);

                    if self.infinitives.contains(&candidate) {
                        if let Some(&verb_change_type) = self.stem_changing_verbs.get(&candidate) {
                            if verb_change_type == StemChangeType::CToZc {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Intenta reconocer cambio ortográfico z→c para verbos -zar
    ///
    /// En español, la 'z' cambia a 'c' antes de 'e' para mantener el sonido /θ/ o /s/.
    /// Afecta a:
    /// - Subjuntivo presente: garantice, garantices, garantice, garanticemos, garanticéis, garanticen
    /// - Pretérito 1ª persona: garanticé
    ///
    /// También maneja combinación con cambio de raíz o→ue (forzar→fuerce, almorzar→almuerce)
    fn try_recognize_orthographic_zar(&self, word: &str) -> bool {
        // Terminaciones del subjuntivo presente -ar que empiezan con 'e'
        // y pretérito 1ª persona que empieza con 'e'
        let endings_with_e = ["e", "es", "emos", "éis", "en", "é"];

        for ending in endings_with_e {
            if let Some(stem) = word.strip_suffix(ending) {
                // El stem debe terminar en 'c' (que viene de 'z')
                if stem.ends_with('c') && stem.len() > 1 {
                    // Revertir c→z para obtener el stem original
                    let original_stem = format!("{}z", &stem[..stem.len() - 1]);
                    let candidate = format!("{}ar", original_stem);

                    if self.infinitives.contains(&candidate) {
                        return true;
                    }

                    // Intentar también revertir cambio de raíz ue→o
                    // para verbos como forzar (fuerce→forzar), almorzar (almuerce→almorzar)
                    if original_stem.contains("ue") {
                        let stem_with_o = original_stem.replacen("ue", "o", 1);
                        let candidate_with_o = format!("{}ar", stem_with_o);
                        if self.infinitives.contains(&candidate_with_o) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Extrae el infinitivo de una forma con cambio ortográfico z→c
    fn extract_infinitive_orthographic_zar(&self, word: &str) -> Option<String> {
        let endings_with_e = ["e", "es", "emos", "éis", "en", "é"];

        for ending in endings_with_e {
            if let Some(stem) = word.strip_suffix(ending) {
                if stem.ends_with('c') && stem.len() > 1 {
                    let original_stem = format!("{}z", &stem[..stem.len() - 1]);
                    let candidate = format!("{}ar", original_stem);

                    if self.infinitives.contains(&candidate) {
                        return Some(candidate);
                    }

                    // Intentar también revertir cambio de raíz ue→o
                    if original_stem.contains("ue") {
                        let stem_with_o = original_stem.replacen("ue", "o", 1);
                        let candidate_with_o = format!("{}ar", stem_with_o);
                        if self.infinitives.contains(&candidate_with_o) {
                            return Some(candidate_with_o);
                        }
                    }
                }
            }
        }

        None
    }

    /// Intenta reconocer cambio ortográfico g→gu para verbos -gar
    ///
    /// En español, la 'g' cambia a 'gu' antes de 'e' para mantener el sonido /g/.
    /// Afecta a:
    /// - Subjuntivo presente: largue, largues, largue, larguemos, larguéis, larguen
    /// - Pretérito 1ª persona: largué
    fn try_recognize_orthographic_gar(&self, word: &str) -> bool {
        // Terminaciones del subjuntivo presente -ar que empiezan con 'e'
        // y pretérito 1ª persona que empieza con 'e'
        let endings_with_e = ["ue", "ues", "uemos", "uéis", "uen", "ué"];

        for ending in endings_with_e {
            if let Some(stem) = word.strip_suffix(ending) {
                // El stem debe terminar en 'g' (que viene de 'gu' + e → gue)
                if stem.ends_with('g') && stem.len() >= 1 {
                    // El infinitivo sería stem + "ar"
                    let candidate = format!("{}ar", stem);

                    if self.infinitives.contains(&candidate) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Extrae el infinitivo de una forma con cambio ortográfico g→gu
    fn extract_infinitive_orthographic_gar(&self, word: &str) -> Option<String> {
        let endings_with_e = ["ue", "ues", "uemos", "uéis", "uen", "ué"];

        for ending in endings_with_e {
            if let Some(stem) = word.strip_suffix(ending) {
                if stem.ends_with('g') && stem.len() >= 1 {
                    let candidate = format!("{}ar", stem);

                    if self.infinitives.contains(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }

        None
    }

    /// Intenta reconocer cambio ortográfico c→qu para verbos -car
    ///
    /// En español, la 'c' cambia a 'qu' antes de 'e' para mantener el sonido /k/.
    /// Afecta a:
    /// - Subjuntivo presente: indique, indiques, indique, indiquemos, indiquéis, indiquen
    /// - Pretérito 1ª persona: indiqué
    fn try_recognize_orthographic_car(&self, word: &str) -> bool {
        // Terminaciones del subjuntivo presente -ar que empiezan con 'e'
        // y pretérito 1ª persona que empieza con 'e'
        let endings_with_e = ["que", "ques", "quemos", "quéis", "quen", "qué"];

        for ending in endings_with_e {
            if let Some(stem) = word.strip_suffix(ending) {
                // El stem debe existir (no vacío)
                if !stem.is_empty() {
                    // El infinitivo sería stem + "car" (c→qu, así que stem termina antes del cambio)
                    let candidate = format!("{}car", stem);

                    if self.infinitives.contains(&candidate) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Extrae el infinitivo de una forma con cambio ortográfico c→qu
    fn extract_infinitive_orthographic_car(&self, word: &str) -> Option<String> {
        let endings_with_e = ["que", "ques", "quemos", "quéis", "quen", "qué"];

        for ending in endings_with_e {
            if let Some(stem) = word.strip_suffix(ending) {
                if !stem.is_empty() {
                    let candidate = format!("{}car", stem);

                    if self.infinitives.contains(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }

        None
    }

    /// Intenta reconocer una forma con cambio de raíz para una clase específica
    fn try_stem_change_class(&self, word: &str, endings: &[&str], inf_ending: &str) -> bool {
        // Terminaciones donde verbos -ir con e→ie usan e→i en su lugar
        let ir_preterite_gerund_endings = ["ió", "ieron", "iendo"];

        for ending in endings {
            if let Some(changed_stem) = word.strip_suffix(ending) {
                if changed_stem.is_empty() {
                    continue;
                }

                // Probar cada tipo de cambio de raíz (vocálicos)
                for change_type in [
                    StemChangeType::EToIe,
                    StemChangeType::OToUe,
                    StemChangeType::EToI,
                    StemChangeType::UToUe,
                ] {
                    if let Some(original_stem) = change_type.reverse_change(changed_stem) {
                        // Construir el infinitivo candidato
                        let candidate = format!("{}{}", original_stem, inf_ending);

                        // Verificar que el infinitivo existe y tiene este tipo de cambio
                        if self.infinitives.contains(&candidate) {
                            if let Some(&verb_change_type) = self.stem_changing_verbs.get(&candidate) {
                                if verb_change_type == change_type {
                                    return true;
                                }
                                // Caso especial: verbos -ir con e→ie usan e→i en pretérito y gerundio
                                // Ej: "invertir" (e→ie) → "invirtió", "invirtieron", "invirtiendo" (e→i)
                                if inf_ending == "ir"
                                    && verb_change_type == StemChangeType::EToIe
                                    && change_type == StemChangeType::EToI
                                    && ir_preterite_gerund_endings.contains(&ending)
                                {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Extrae el infinitivo de una forma verbal regular
    fn extract_infinitive_regular(&self, word: &str) -> Option<String> {
        // Probar cada clase de verbo
        for class in [VerbClass::Ar, VerbClass::Er, VerbClass::Ir] {
            if let Some(inf) = self.extract_from_class(word, class) {
                return Some(inf);
            }
        }

        // Probar futuro y condicional
        self.extract_from_future_conditional(word)
    }

    /// Extrae infinitivo de una clase específica
    fn extract_from_class(&self, word: &str, class: VerbClass) -> Option<String> {
        let endings = regular::get_all_endings(class);
        let inf_ending = class.infinitive_ending();

        for ending in endings {
            if let Some(stem) = word.strip_suffix(ending) {
                if !stem.is_empty() {
                    let candidate = format!("{}{}", stem, inf_ending);
                    if self.infinitives.contains(&candidate) {
                        return Some(candidate);
                    }
                }
            }
        }

        None
    }

    /// Extrae infinitivo de formas de futuro o condicional
    fn extract_from_future_conditional(&self, word: &str) -> Option<String> {
        // Futuro
        for ending in FUTURO.iter() {
            if let Some(base) = word.strip_suffix(ending) {
                if self.infinitives.contains(base) {
                    return Some(base.to_string());
                }
                // Probar raíces irregulares
                if let Some(inf) = self.extract_infinitive_from_irregular_stem(base) {
                    return Some(inf);
                }
            }
        }

        // Condicional
        for ending in CONDICIONAL.iter() {
            if let Some(base) = word.strip_suffix(ending) {
                if self.infinitives.contains(base) {
                    return Some(base.to_string());
                }
                // Probar raíces irregulares
                if let Some(inf) = self.extract_infinitive_from_irregular_stem(base) {
                    return Some(inf);
                }
            }
        }

        None
    }

    /// Extrae el infinitivo de una raíz irregular de futuro/condicional
    fn extract_infinitive_from_irregular_stem(&self, stem: &str) -> Option<String> {
        // Patrón: raíz termina en "dr" → probar infinitivos -er e -ir
        if stem.ends_with("dr") {
            let base = &stem[..stem.len() - 2];
            for suffix in ["er", "ir"] {
                let candidate = format!("{}{}", base, suffix);
                if self.infinitives.contains(&candidate) {
                    return Some(candidate);
                }
            }
        }

        // Patrón: raíz termina en "br" → probar infinitivo -ber
        if stem.ends_with("br") {
            let base = &stem[..stem.len() - 2];
            let candidate = format!("{}ber", base);
            if self.infinitives.contains(&candidate) {
                return Some(candidate);
            }
        }

        // Patrón: raíz termina en "rr" → probar infinitivo -rer
        if stem.ends_with("rr") {
            let base = &stem[..stem.len() - 2];
            let candidate = format!("{}rer", base);
            if self.infinitives.contains(&candidate) {
                return Some(candidate);
            }
        }

        // Patrón: raíz termina en "odr" → probar infinitivo -oder
        if stem.ends_with("odr") {
            let base = &stem[..stem.len() - 3];
            let candidate = format!("{}oder", base);
            if self.infinitives.contains(&candidate) {
                return Some(candidate);
            }
        }

        None
    }

    /// Extrae el infinitivo de una forma con cambio de raíz
    fn extract_infinitive_stem_changing(&self, word: &str) -> Option<String> {
        // Terminaciones donde verbos -ir con e→ie usan e→i en su lugar
        let ir_preterite_gerund_endings = ["ió", "ieron", "iendo"];

        // Primero probar cambios vocálicos
        for (_, endings, inf_ending) in [
            (VerbClass::Ar, stem_changing::get_stem_change_endings_ar(), "ar"),
            (VerbClass::Er, stem_changing::get_stem_change_endings_er(), "er"),
            (VerbClass::Ir, stem_changing::get_stem_change_endings_ir(), "ir"),
        ] {
            for ending in &endings {
                if let Some(changed_stem) = word.strip_suffix(ending) {
                    if changed_stem.is_empty() {
                        continue;
                    }

                    for change_type in [
                        StemChangeType::EToIe,
                        StemChangeType::OToUe,
                        StemChangeType::EToI,
                        StemChangeType::UToUe,
                    ] {
                        if let Some(original_stem) = change_type.reverse_change(changed_stem) {
                            let candidate = format!("{}{}", original_stem, inf_ending);

                            if self.infinitives.contains(&candidate) {
                                if let Some(&verb_change_type) = self.stem_changing_verbs.get(&candidate) {
                                    if verb_change_type == change_type {
                                        return Some(candidate);
                                    }
                                    // Caso especial: verbos -ir con e→ie usan e→i en pretérito y gerundio
                                    if inf_ending == "ir"
                                        && verb_change_type == StemChangeType::EToIe
                                        && change_type == StemChangeType::EToI
                                        && ir_preterite_gerund_endings.contains(ending)
                                    {
                                        return Some(candidate);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Luego probar c→zc para -ecer/-ocer y -ucir
        if let Some(inf) = self.extract_infinitive_c_to_zc(word, "er") {
            return Some(inf);
        }
        if let Some(inf) = self.extract_infinitive_c_to_zc(word, "ir") {
            return Some(inf);
        }

        None
    }

    /// Extrae infinitivo de forma con cambio c→zc
    fn extract_infinitive_c_to_zc(&self, word: &str, inf_ending: &str) -> Option<String> {
        let endings = stem_changing::get_c_to_zc_endings_er();

        for ending in endings {
            if let Some(changed_stem) = word.strip_suffix(ending) {
                if changed_stem.is_empty() {
                    continue;
                }

                if let Some(original_stem) = StemChangeType::CToZc.reverse_change(changed_stem) {
                    let candidate = format!("{}{}", original_stem, inf_ending);

                    if self.infinitives.contains(&candidate) {
                        if let Some(&verb_change_type) = self.stem_changing_verbs.get(&candidate) {
                            if verb_change_type == StemChangeType::CToZc {
                                return Some(candidate);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Intenta reconocer una forma verbal con prefijo
    fn try_recognize_prefixed(&self, word: &str) -> bool {
        if let Some((prefix, base)) = PrefixAnalyzer::strip_prefix(word) {
            // Verificar si la base es una forma válida
            if self.irregular_lookup.contains_key(base) {
                return true;
            }
            if self.try_recognize_regular(base) {
                return true;
            }
            if self.try_recognize_stem_changing(base) {
                return true;
            }
            // Verificar si la base es un infinitivo conocido
            if self.infinitives.contains(base) {
                return true;
            }
            // Verificar si el infinitivo con prefijo existe en el diccionario
            if let Some(base_inf) = self.extract_base_infinitive(base) {
                let prefixed_inf = PrefixAnalyzer::reconstruct_infinitive(prefix, &base_inf);
                if self.infinitives.contains(&prefixed_inf) {
                    return true;
                }
            }
        }
        false
    }

    /// Extrae el infinitivo de una forma con prefijo
    fn extract_infinitive_prefixed(&self, word: &str) -> Option<String> {
        if let Some((prefix, base)) = PrefixAnalyzer::strip_prefix(word) {
            // Intentar obtener el infinitivo de la base
            if let Some(base_inf) = self.extract_base_infinitive(base) {
                let prefixed_inf = PrefixAnalyzer::reconstruct_infinitive(prefix, &base_inf);
                // Si el infinitivo con prefijo existe en el diccionario, devolverlo
                if self.infinitives.contains(&prefixed_inf) {
                    return Some(prefixed_inf);
                }
                // Si no, devolver la forma reconstruida (puede no estar en diccionario)
                return Some(prefixed_inf);
            }
        }
        None
    }

    /// Extrae el infinitivo base de una forma verbal (sin considerar prefijos)
    fn extract_base_infinitive(&self, word: &str) -> Option<String> {
        // Buscar en irregulares
        if let Some(inf) = self.irregular_lookup.get(word) {
            return Some(inf.clone());
        }
        // Buscar en regulares
        if let Some(inf) = self.extract_infinitive_regular(word) {
            return Some(inf);
        }
        // Buscar en formas con cambio de raíz
        self.extract_infinitive_stem_changing(word)
    }

    /// Intenta reconocer una forma verbal con pronombres enclíticos
    fn try_recognize_with_enclitics(&self, word: &str) -> bool {
        if let Some(result) = EncliticsAnalyzer::strip_enclitics(word) {
            let base = &result.base;

            // Verificar si la base es un infinitivo
            if EncliticsAnalyzer::is_infinitive(base) && self.infinitives.contains(base) {
                return true;
            }

            // Verificar si la base es un gerundio reconocible
            if EncliticsAnalyzer::is_gerund(base) {
                // Primero verificar si es un gerundio irregular conocido
                let base_no_accent = Self::remove_accent(base);
                if self.irregular_lookup.contains_key(&base_no_accent) {
                    return true;
                }
                // Luego intentar extraer infinitivo del gerundio regular
                if self.try_recognize_gerund_base(base) {
                    return true;
                }
            }

            // Verificar si la base es un imperativo
            if EncliticsAnalyzer::could_be_imperative(base) {
                // Verificar si es forma irregular conocida
                if self.irregular_lookup.contains_key(base) {
                    return true;
                }
                // Verificar si es forma regular de imperativo
                if self.try_recognize_imperative_base(base) {
                    return true;
                }
            }
        }
        false
    }

    /// Extrae el infinitivo de una forma con enclíticos
    fn extract_infinitive_with_enclitics(&self, word: &str) -> Option<String> {
        if let Some(result) = EncliticsAnalyzer::strip_enclitics(word) {
            let base = &result.base;

            // Si la base es un infinitivo
            if EncliticsAnalyzer::is_infinitive(base) && self.infinitives.contains(base) {
                if let Some(pronominal) = self.pronominal_verbs.get(base) {
                    return Some(pronominal.clone());
                }
                return Some(base.clone());
            }

            // Si la base es un gerundio
            if EncliticsAnalyzer::is_gerund(base) {
                if let Some(inf) = self.extract_infinitive_from_gerund(base) {
                    return Some(inf);
                }
            }

            // Si la base es un imperativo
            if EncliticsAnalyzer::could_be_imperative(base) {
                if let Some(inf) = self.irregular_lookup.get(base) {
                    return Some(inf.clone());
                }
                if let Some(inf) = self.extract_infinitive_from_imperative(base) {
                    return Some(inf);
                }
            }
        }
        None
    }

    /// Verifica si una base de gerundio corresponde a un verbo conocido
    fn try_recognize_gerund_base(&self, base: &str) -> bool {
        // Gerundio -ando → -ar
        if let Some(stem) = base.strip_suffix("ando") {
            let inf = format!("{}ar", stem);
            if self.infinitives.contains(&inf) {
                return true;
            }
        }
        // Gerundio -iendo/-yendo → -er/-ir
        for suffix in ["iendo", "yendo"] {
            if let Some(stem) = base.strip_suffix(suffix) {
                for ending in ["er", "ir"] {
                    let inf = format!("{}{}", stem, ending);
                    if self.infinitives.contains(&inf) {
                        return true;
                    }
                }
            }
        }
        // Gerundios con acento (diciéndo → decir)
        if let Some(stem) = base.strip_suffix("ándo") {
            let inf = format!("{}ar", stem);
            if self.infinitives.contains(&inf) {
                return true;
            }
        }
        if let Some(stem) = base.strip_suffix("iéndo") {
            for ending in ["er", "ir"] {
                let inf = format!("{}{}", stem, ending);
                if self.infinitives.contains(&inf) {
                    return true;
                }
            }
        }
        false
    }

    /// Extrae el infinitivo de una base de gerundio
    fn extract_infinitive_from_gerund(&self, base: &str) -> Option<String> {
        // Gerundio -ando → -ar
        if let Some(stem) = base.strip_suffix("ando") {
            let inf = format!("{}ar", stem);
            if self.infinitives.contains(&inf) {
                return Some(inf);
            }
        }
        // Gerundio -iendo/-yendo → -er/-ir
        for suffix in ["iendo", "yendo"] {
            if let Some(stem) = base.strip_suffix(suffix) {
                for ending in ["er", "ir"] {
                    let inf = format!("{}{}", stem, ending);
                    if self.infinitives.contains(&inf) {
                        return Some(inf);
                    }
                }
            }
        }
        // Gerundios con acento
        if let Some(stem) = base.strip_suffix("ándo") {
            let inf = format!("{}ar", stem);
            if self.infinitives.contains(&inf) {
                return Some(inf);
            }
        }
        if let Some(stem) = base.strip_suffix("iéndo") {
            for ending in ["er", "ir"] {
                let inf = format!("{}{}", stem, ending);
                if self.infinitives.contains(&inf) {
                    return Some(inf);
                }
            }
        }
        None
    }

    /// Verifica si una base de imperativo corresponde a un verbo conocido
    fn try_recognize_imperative_base(&self, base: &str) -> bool {
        // Quitar acento del imperativo si existe (cánta → canta)
        let base_no_accent = Self::remove_accent(base);
        let base = base_no_accent.as_str();

        // Imperativo vosotros: -ad → -ar, -ed → -er, -id → -ir
        if let Some(stem) = base.strip_suffix("ad") {
            let inf = format!("{}ar", stem);
            if self.infinitives.contains(&inf) {
                return true;
            }
        }
        if let Some(stem) = base.strip_suffix("ed") {
            let inf = format!("{}er", stem);
            if self.infinitives.contains(&inf) {
                return true;
            }
        }
        if let Some(stem) = base.strip_suffix("id") {
            let inf = format!("{}ir", stem);
            if self.infinitives.contains(&inf) {
                return true;
            }
        }
        // Imperativo tú: coincide con 3ª persona presente
        // -a (canta) → -ar, -e (come, vive) → -er/-ir
        if let Some(stem) = base.strip_suffix("a") {
            if !stem.is_empty() {
                let inf = format!("{}ar", stem);
                if self.infinitives.contains(&inf) {
                    return true;
                }
            }
        }
        if let Some(stem) = base.strip_suffix("e") {
            if !stem.is_empty() {
                for ending in ["er", "ir"] {
                    let inf = format!("{}{}", stem, ending);
                    if self.infinitives.contains(&inf) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Quita el acento de una palabra
    fn remove_accent(word: &str) -> String {
        word.replace('á', "a")
            .replace('é', "e")
            .replace('í', "i")
            .replace('ó', "o")
            .replace('ú', "u")
    }

    /// Extrae el infinitivo de una base de imperativo
    fn extract_infinitive_from_imperative(&self, base: &str) -> Option<String> {
        // Quitar acento del imperativo si existe (cánta → canta)
        let base_no_accent = Self::remove_accent(base);
        let base = base_no_accent.as_str();

        // Imperativo vosotros
        if let Some(stem) = base.strip_suffix("ad") {
            let inf = format!("{}ar", stem);
            if self.infinitives.contains(&inf) {
                return Some(inf);
            }
        }
        if let Some(stem) = base.strip_suffix("ed") {
            let inf = format!("{}er", stem);
            if self.infinitives.contains(&inf) {
                return Some(inf);
            }
        }
        if let Some(stem) = base.strip_suffix("id") {
            let inf = format!("{}ir", stem);
            if self.infinitives.contains(&inf) {
                return Some(inf);
            }
        }
        // Imperativo tú
        if let Some(stem) = base.strip_suffix("a") {
            if !stem.is_empty() {
                let inf = format!("{}ar", stem);
                if self.infinitives.contains(&inf) {
                    return Some(inf);
                }
            }
        }
        if let Some(stem) = base.strip_suffix("e") {
            if !stem.is_empty() {
                for ending in ["er", "ir"] {
                    let inf = format!("{}{}", stem, ending);
                    if self.infinitives.contains(&inf) {
                        return Some(inf);
                    }
                }
            }
        }
        None
    }

    /// Número de infinitivos conocidos
    pub fn infinitive_count(&self) -> usize {
        self.infinitives.len()
    }

    /// Número de formas irregulares conocidas
    pub fn irregular_count(&self) -> usize {
        self.irregular_lookup.len()
    }

    /// Número de verbos pronominales conocidos
    pub fn pronominal_count(&self) -> usize {
        self.pronominal_verbs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::{Gender, Number, WordInfo};

    fn create_test_trie() -> Trie {
        let mut trie = Trie::new();

        // Añadir algunos verbos de prueba
        let verb_info = WordInfo {
            category: WordCategory::Verbo,
            gender: Gender::None,
            number: Number::None,
            extra: String::new(),
            frequency: 100,
        };

        // Verbos regulares
        trie.insert("cantar", verb_info.clone());
        trie.insert("comer", verb_info.clone());
        trie.insert("vivir", verb_info.clone());
        trie.insert("hablar", verb_info.clone());
        trie.insert("bailar", verb_info.clone());

        // Verbos con cambio de raíz
        trie.insert("pensar", verb_info.clone());   // e→ie
        trie.insert("entender", verb_info.clone()); // e→ie
        trie.insert("contar", verb_info.clone());   // o→ue
        trie.insert("dormir", verb_info.clone());   // o→ue
        trie.insert("pedir", verb_info.clone());    // e→i
        trie.insert("jugar", verb_info.clone());    // u→ue

        // Verbos con cambio c→zc
        trie.insert("conocer", verb_info.clone());  // c→zc
        trie.insert("parecer", verb_info.clone());  // c→zc
        trie.insert("conducir", verb_info.clone()); // c→zc (-ucir)

        trie
    }

    #[test]
    fn test_from_dictionary() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        assert!(recognizer.infinitive_count() > 0);
        assert!(recognizer.irregular_count() > 0);
    }

    #[test]
    fn test_regular_ar_verbs() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Presente
        assert!(recognizer.is_valid_verb_form("canto"));
        assert!(recognizer.is_valid_verb_form("cantas"));
        assert!(recognizer.is_valid_verb_form("canta"));
        assert!(recognizer.is_valid_verb_form("cantamos"));
        assert!(recognizer.is_valid_verb_form("cantáis"));
        assert!(recognizer.is_valid_verb_form("cantan"));

        // Pretérito
        assert!(recognizer.is_valid_verb_form("canté"));
        assert!(recognizer.is_valid_verb_form("cantaste"));
        assert!(recognizer.is_valid_verb_form("cantó"));
        assert!(recognizer.is_valid_verb_form("cantaron"));

        // Imperfecto
        assert!(recognizer.is_valid_verb_form("cantaba"));
        assert!(recognizer.is_valid_verb_form("cantabas"));
        assert!(recognizer.is_valid_verb_form("cantaban"));

        // Gerundio y participio
        assert!(recognizer.is_valid_verb_form("cantando"));
        assert!(recognizer.is_valid_verb_form("cantado"));
    }

    #[test]
    fn test_regular_er_verbs() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Presente
        assert!(recognizer.is_valid_verb_form("como"));
        assert!(recognizer.is_valid_verb_form("comes"));
        assert!(recognizer.is_valid_verb_form("come"));
        assert!(recognizer.is_valid_verb_form("comemos"));
        assert!(recognizer.is_valid_verb_form("comen"));

        // Pretérito
        assert!(recognizer.is_valid_verb_form("comí"));
        assert!(recognizer.is_valid_verb_form("comió"));
        assert!(recognizer.is_valid_verb_form("comieron"));

        // Imperfecto
        assert!(recognizer.is_valid_verb_form("comía"));
        assert!(recognizer.is_valid_verb_form("comías"));
        assert!(recognizer.is_valid_verb_form("comían"));

        // Gerundio y participio
        assert!(recognizer.is_valid_verb_form("comiendo"));
        assert!(recognizer.is_valid_verb_form("comido"));
    }

    #[test]
    fn test_regular_ir_verbs() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Presente
        assert!(recognizer.is_valid_verb_form("vivo"));
        assert!(recognizer.is_valid_verb_form("vives"));
        assert!(recognizer.is_valid_verb_form("vive"));
        assert!(recognizer.is_valid_verb_form("vivimos"));
        assert!(recognizer.is_valid_verb_form("viven"));

        // Pretérito
        assert!(recognizer.is_valid_verb_form("viví"));
        assert!(recognizer.is_valid_verb_form("vivió"));
        assert!(recognizer.is_valid_verb_form("vivieron"));

        // Gerundio y participio
        assert!(recognizer.is_valid_verb_form("viviendo"));
        assert!(recognizer.is_valid_verb_form("vivido"));
    }

    #[test]
    fn test_future_conditional() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Futuro
        assert!(recognizer.is_valid_verb_form("cantaré"));
        assert!(recognizer.is_valid_verb_form("cantarás"));
        assert!(recognizer.is_valid_verb_form("cantará"));
        assert!(recognizer.is_valid_verb_form("cantaremos"));
        assert!(recognizer.is_valid_verb_form("cantarán"));

        // Condicional
        assert!(recognizer.is_valid_verb_form("cantaría"));
        assert!(recognizer.is_valid_verb_form("cantarías"));
        assert!(recognizer.is_valid_verb_form("cantaríamos"));
        assert!(recognizer.is_valid_verb_form("cantarían"));
    }

    #[test]
    fn test_irregular_verbs() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // ser
        assert!(recognizer.is_valid_verb_form("soy"));
        assert!(recognizer.is_valid_verb_form("eres"));
        assert!(recognizer.is_valid_verb_form("fue"));
        assert!(recognizer.is_valid_verb_form("sido"));

        // estar
        assert!(recognizer.is_valid_verb_form("estoy"));
        assert!(recognizer.is_valid_verb_form("estuvo"));

        // ir
        assert!(recognizer.is_valid_verb_form("voy"));
        assert!(recognizer.is_valid_verb_form("iba"));

        // haber
        assert!(recognizer.is_valid_verb_form("he"));
        assert!(recognizer.is_valid_verb_form("hay"));
        assert!(recognizer.is_valid_verb_form("había"));

        // tener
        assert!(recognizer.is_valid_verb_form("tengo"));
        assert!(recognizer.is_valid_verb_form("tuvo"));

        // hacer
        assert!(recognizer.is_valid_verb_form("hago"));
        assert!(recognizer.is_valid_verb_form("hizo"));
        assert!(recognizer.is_valid_verb_form("hecho"));
    }

    #[test]
    fn test_get_infinitive() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Formas regulares
        assert_eq!(
            recognizer.get_infinitive("cantamos"),
            Some("cantar".to_string())
        );
        assert_eq!(
            recognizer.get_infinitive("comieron"),
            Some("comer".to_string())
        );
        assert_eq!(
            recognizer.get_infinitive("viviendo"),
            Some("vivir".to_string())
        );

        // Formas irregulares
        assert_eq!(recognizer.get_infinitive("soy"), Some("ser".to_string()));
        assert_eq!(recognizer.get_infinitive("tengo"), Some("tener".to_string()));
        assert_eq!(recognizer.get_infinitive("hecho"), Some("hacer".to_string()));
    }

    #[test]
    fn test_non_verb_words() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Palabras que NO son verbos y no deberían coincidir
        assert!(!recognizer.is_valid_verb_form("casa"));
        assert!(!recognizer.is_valid_verb_form("perro"));
        assert!(!recognizer.is_valid_verb_form("azul"));
        assert!(!recognizer.is_valid_verb_form("rapidamente"));
    }

    #[test]
    fn test_subjunctive() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Subjuntivo presente -ar
        assert!(recognizer.is_valid_verb_form("cante"));
        assert!(recognizer.is_valid_verb_form("cantes"));
        assert!(recognizer.is_valid_verb_form("cantemos"));
        assert!(recognizer.is_valid_verb_form("canten"));

        // Subjuntivo presente -er/-ir
        assert!(recognizer.is_valid_verb_form("coma"));
        assert!(recognizer.is_valid_verb_form("viva"));

        // Subjuntivo imperfecto (-ra)
        assert!(recognizer.is_valid_verb_form("cantara"));
        assert!(recognizer.is_valid_verb_form("comiera"));
        assert!(recognizer.is_valid_verb_form("viviera"));

        // Subjuntivo imperfecto (-se)
        assert!(recognizer.is_valid_verb_form("cantase"));
        assert!(recognizer.is_valid_verb_form("comiese"));
        assert!(recognizer.is_valid_verb_form("viviese"));
    }

    #[test]
    fn test_stem_changing_e_to_ie() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // pensar (e→ie)
        assert!(recognizer.is_valid_verb_form("pienso"));
        assert!(recognizer.is_valid_verb_form("piensas"));
        assert!(recognizer.is_valid_verb_form("piensa"));
        assert!(recognizer.is_valid_verb_form("piensan"));
        // pensamos/pensáis no tienen cambio (formas regulares)
        assert!(recognizer.is_valid_verb_form("pensamos"));

        // entender (e→ie)
        assert!(recognizer.is_valid_verb_form("entiendo"));
        assert!(recognizer.is_valid_verb_form("entiendes"));
        assert!(recognizer.is_valid_verb_form("entiende"));
        assert!(recognizer.is_valid_verb_form("entienden"));

        // Subjuntivo con cambio
        assert!(recognizer.is_valid_verb_form("piense"));
        assert!(recognizer.is_valid_verb_form("pienses"));
        assert!(recognizer.is_valid_verb_form("entienda"));
        assert!(recognizer.is_valid_verb_form("entiendan"));
    }

    #[test]
    fn test_stem_changing_o_to_ue() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // contar (o→ue)
        assert!(recognizer.is_valid_verb_form("cuento"));
        assert!(recognizer.is_valid_verb_form("cuentas"));
        assert!(recognizer.is_valid_verb_form("cuenta"));
        assert!(recognizer.is_valid_verb_form("cuentan"));
        // contamos no tiene cambio
        assert!(recognizer.is_valid_verb_form("contamos"));

        // dormir (o→ue)
        assert!(recognizer.is_valid_verb_form("duermo"));
        assert!(recognizer.is_valid_verb_form("duermes"));
        assert!(recognizer.is_valid_verb_form("duerme"));
        assert!(recognizer.is_valid_verb_form("duermen"));

        // Subjuntivo con cambio
        assert!(recognizer.is_valid_verb_form("cuente"));
        assert!(recognizer.is_valid_verb_form("duerma"));
    }

    #[test]
    fn test_stem_changing_e_to_i() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // pedir (e→i)
        assert!(recognizer.is_valid_verb_form("pido"));
        assert!(recognizer.is_valid_verb_form("pides"));
        assert!(recognizer.is_valid_verb_form("pide"));
        assert!(recognizer.is_valid_verb_form("piden"));

        // Subjuntivo con cambio
        assert!(recognizer.is_valid_verb_form("pida"));
        assert!(recognizer.is_valid_verb_form("pidas"));
        assert!(recognizer.is_valid_verb_form("pidan"));

        // Gerundio con cambio e→i
        assert!(recognizer.is_valid_verb_form("pidiendo"));

        // Pretérito 3ª persona con cambio
        assert!(recognizer.is_valid_verb_form("pidió"));
        assert!(recognizer.is_valid_verb_form("pidieron"));
    }

    #[test]
    fn test_stem_changing_u_to_ue() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // jugar (u→ue)
        assert!(recognizer.is_valid_verb_form("juego"));
        assert!(recognizer.is_valid_verb_form("juegas"));
        assert!(recognizer.is_valid_verb_form("juega"));
        assert!(recognizer.is_valid_verb_form("juegan"));
        // jugamos no tiene cambio
        assert!(recognizer.is_valid_verb_form("jugamos"));

        // Subjuntivo
        assert!(recognizer.is_valid_verb_form("juegue"));
        assert!(recognizer.is_valid_verb_form("jueguen"));
    }

    #[test]
    fn test_get_infinitive_stem_changing() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // e→ie
        assert_eq!(recognizer.get_infinitive("pienso"), Some("pensar".to_string()));
        assert_eq!(recognizer.get_infinitive("entienden"), Some("entender".to_string()));

        // o→ue
        assert_eq!(recognizer.get_infinitive("cuento"), Some("contar".to_string()));
        assert_eq!(recognizer.get_infinitive("duermen"), Some("dormir".to_string()));

        // e→i
        assert_eq!(recognizer.get_infinitive("pido"), Some("pedir".to_string()));
        assert_eq!(recognizer.get_infinitive("pidiendo"), Some("pedir".to_string()));

        // u→ue
        assert_eq!(recognizer.get_infinitive("juego"), Some("jugar".to_string()));
    }

    #[test]
    fn test_stem_changing_c_to_zc() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // conocer (c→zc)
        assert!(recognizer.is_valid_verb_form("conozco"));
        assert!(recognizer.is_valid_verb_form("conozca"));
        assert!(recognizer.is_valid_verb_form("conozcas"));
        assert!(recognizer.is_valid_verb_form("conozcamos"));
        assert!(recognizer.is_valid_verb_form("conozcan"));
        // conoces/conoce no tienen cambio (formas regulares)
        assert!(recognizer.is_valid_verb_form("conocemos"));

        // parecer (c→zc)
        assert!(recognizer.is_valid_verb_form("parezco"));
        assert!(recognizer.is_valid_verb_form("parezca"));
        assert!(recognizer.is_valid_verb_form("parezcan"));

        // conducir (c→zc en -ucir)
        assert!(recognizer.is_valid_verb_form("conduzco"));
        assert!(recognizer.is_valid_verb_form("conduzca"));
        assert!(recognizer.is_valid_verb_form("conduzcan"));
    }

    #[test]
    fn test_get_infinitive_c_to_zc() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        assert_eq!(recognizer.get_infinitive("conozco"), Some("conocer".to_string()));
        assert_eq!(recognizer.get_infinitive("parezcan"), Some("parecer".to_string()));
        assert_eq!(recognizer.get_infinitive("conduzca"), Some("conducir".to_string()));
    }

    #[test]
    fn test_imperativo_vosotros() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // -ar
        assert!(recognizer.is_valid_verb_form("cantad"));
        assert!(recognizer.is_valid_verb_form("hablad"));
        // -er
        assert!(recognizer.is_valid_verb_form("comed"));
        // -ir
        assert!(recognizer.is_valid_verb_form("vivid"));
    }

    fn create_test_trie_with_pronominal() -> Trie {
        let mut trie = create_test_trie();

        let verb_info = WordInfo {
            category: WordCategory::Verbo,
            gender: Gender::None,
            number: Number::None,
            extra: String::new(),
            frequency: 100,
        };

        // Verbos pronominales
        trie.insert("sentirse", verb_info.clone());
        trie.insert("acostarse", verb_info.clone());
        trie.insert("convertirse", verb_info.clone());
        trie.insert("arrepentirse", verb_info.clone());

        // Verbos con prefijo
        trie.insert("deshacer", verb_info.clone());
        trie.insert("rehacer", verb_info.clone());
        trie.insert("predecir", verb_info.clone());

        trie
    }

    #[test]
    fn test_pronominal_verbs() {
        let trie = create_test_trie_with_pronominal();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Formas de verbos pronominales (sentirse → e→ie)
        assert!(recognizer.is_valid_verb_form("siento"));
        assert!(recognizer.is_valid_verb_form("sientes"));
        assert!(recognizer.is_valid_verb_form("sienten"));

        // acostarse (o→ue)
        assert!(recognizer.is_valid_verb_form("acuesto"));
        assert!(recognizer.is_valid_verb_form("acuestas"));

        // Infinitivo devuelto debe ser el pronominal
        assert_eq!(
            recognizer.get_infinitive("siento"),
            Some("sentirse".to_string())
        );
    }

    #[test]
    fn test_prefixed_verbs() {
        let trie = create_test_trie_with_pronominal();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // des- + hacer
        assert!(recognizer.is_valid_verb_form("deshago"));
        assert!(recognizer.is_valid_verb_form("deshice"));
        assert_eq!(
            recognizer.get_infinitive("deshago"),
            Some("deshacer".to_string())
        );

        // re- + hacer
        assert!(recognizer.is_valid_verb_form("rehago"));
        assert!(recognizer.is_valid_verb_form("rehice"));
        assert_eq!(
            recognizer.get_infinitive("rehago"),
            Some("rehacer".to_string())
        );

        // pre- + decir
        assert!(recognizer.is_valid_verb_form("predigo"));
        assert_eq!(
            recognizer.get_infinitive("predigo"),
            Some("predecir".to_string())
        );
    }

    fn create_test_trie_with_enclitics() -> Trie {
        let mut trie = create_test_trie();

        let verb_info = WordInfo {
            category: WordCategory::Verbo,
            gender: Gender::None,
            number: Number::None,
            extra: String::new(),
            frequency: 100,
        };

        // Añadir verbos adicionales para tests de enclíticos
        trie.insert("dar", verb_info.clone());

        trie
    }

    #[test]
    fn test_enclitics_infinitive() {
        let trie = create_test_trie_with_enclitics();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Infinitivo + enclítico
        assert!(recognizer.is_valid_verb_form("cantarle"));
        assert!(recognizer.is_valid_verb_form("comerlo"));
        assert!(recognizer.is_valid_verb_form("vivirla"));

        // Infinitivo + doble enclítico
        assert!(recognizer.is_valid_verb_form("cantármelo"));
        assert!(recognizer.is_valid_verb_form("dárselo"));
    }

    #[test]
    fn test_enclitics_gerund() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Gerundio + enclítico
        assert!(recognizer.is_valid_verb_form("cantándole"));
        assert!(recognizer.is_valid_verb_form("comiéndolo"));
    }

    #[test]
    fn test_enclitics_imperative() {
        let trie = create_test_trie();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // Imperativo irregular + enclítico (formas conocidas)
        assert!(recognizer.is_valid_verb_form("dime"));
        assert!(recognizer.is_valid_verb_form("ponlo"));
        assert!(recognizer.is_valid_verb_form("hazlo"));

        // Imperativo regular + enclítico
        assert!(recognizer.is_valid_verb_form("cántame"));
        // "cómelo" requires special handling since "come" + "lo" needs
        // recognition of imperative forms ending in 'e' mapped to -er verbs
        assert!(recognizer.is_valid_verb_form("cómelo"));
    }

    fn create_test_trie_with_car_verbs() -> Trie {
        let mut trie = create_test_trie();

        let verb_info = WordInfo {
            category: WordCategory::Verbo,
            gender: Gender::None,
            number: Number::None,
            extra: String::new(),
            frequency: 100,
        };

        // Verbos -car (cambio ortográfico c→qu antes de e)
        trie.insert("indicar", verb_info.clone());
        trie.insert("aplicar", verb_info.clone());
        trie.insert("explicar", verb_info.clone());
        trie.insert("buscar", verb_info.clone());
        trie.insert("tocar", verb_info.clone());

        trie
    }

    #[test]
    fn test_orthographic_car_verbs() {
        let trie = create_test_trie_with_car_verbs();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        // indicar (c→qu)
        assert!(recognizer.is_valid_verb_form("indique"));
        assert!(recognizer.is_valid_verb_form("indiques"));
        assert!(recognizer.is_valid_verb_form("indiquemos"));
        assert!(recognizer.is_valid_verb_form("indiquéis"));
        assert!(recognizer.is_valid_verb_form("indiquen"));
        // Pretérito 1ª persona
        assert!(recognizer.is_valid_verb_form("indiqué"));

        // aplicar (c→qu)
        assert!(recognizer.is_valid_verb_form("aplique"));
        assert!(recognizer.is_valid_verb_form("apliquen"));

        // explicar (c→qu)
        assert!(recognizer.is_valid_verb_form("explique"));
        assert!(recognizer.is_valid_verb_form("expliquen"));

        // buscar (c→qu)
        assert!(recognizer.is_valid_verb_form("busque"));
        assert!(recognizer.is_valid_verb_form("busqué"));

        // Formas regulares (sin cambio ortográfico) también deben funcionar
        assert!(recognizer.is_valid_verb_form("indica"));
        assert!(recognizer.is_valid_verb_form("indicamos"));
        assert!(recognizer.is_valid_verb_form("indicó"));
    }

    #[test]
    fn test_get_infinitive_orthographic_car() {
        let trie = create_test_trie_with_car_verbs();
        let recognizer = VerbRecognizer::from_dictionary(&trie);

        assert_eq!(recognizer.get_infinitive("indique"), Some("indicar".to_string()));
        assert_eq!(recognizer.get_infinitive("indiqué"), Some("indicar".to_string()));
        assert_eq!(recognizer.get_infinitive("apliquen"), Some("aplicar".to_string()));
        assert_eq!(recognizer.get_infinitive("explique"), Some("explicar".to_string()));
        assert_eq!(recognizer.get_infinitive("busqué"), Some("buscar".to_string()));
    }

}

    #[test]
    fn test_mando_recognized_as_verb() {
        // "mando" es 1ª persona singular de "mandar", debe ser reconocido como verbo
        use crate::dictionary::DictionaryLoader;

        let dict_path = std::path::Path::new("data/es/words.txt");
        if !dict_path.exists() {
            return; // Skip si el diccionario no existe
        }
        let dictionary = DictionaryLoader::load_from_file(dict_path).unwrap();
        let recognizer = VerbRecognizer::from_dictionary(&dictionary);

        assert!(recognizer.is_valid_verb_form("mando"),
            "'mando' debería ser reconocido como forma válida de 'mandar'");
    }
