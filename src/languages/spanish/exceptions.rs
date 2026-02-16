//! Excepciones y casos especiales del español

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Obtiene el conjunto de excepciones conocidas
pub fn get_exceptions() -> HashSet<String> {
    let exceptions = vec![
        // Sustantivos femeninos que empiezan con 'a' tónica y usan "el"
        "agua", "águila", "alma", "área", "arma", "aula", "ave", "acta", "hacha", "hada", "hambre",
        "álgebra", // Palabras que pueden ser masculinas o femeninas según contexto
        "mar", "arte", "azúcar",
        // Sustantivos epicenos (un solo género para ambos sexos)
        "persona", "víctima", "genio",
        // Otros casos especiales
        "día",      // masculino aunque termine en 'a'
        "mapa",     // masculino aunque termine en 'a'
        "problema", // masculino aunque termine en 'a'
        "sistema",  // masculino aunque termine en 'a'
        "tema",     // masculino aunque termine en 'a'
        "programa", // masculino aunque termine en 'a'
        "idioma",   // masculino aunque termine en 'a'
        "clima",    // masculino aunque termine en 'a'
        "planeta",  // masculino aunque termine en 'a'
        "poema",    // masculino aunque termine en 'a'
        "drama",    // masculino aunque termine en 'a'
        "fantasma", // masculino aunque termine en 'a'
        "pijama",   // masculino aunque termine en 'a'
        "sofá",     // masculino aunque termine en 'a'
        "mano",     // femenino aunque termine en 'o'
        "radio",    // femenino aunque termine en 'o' (cuando es aparato)
        "foto",     // femenino (abreviatura de fotografía)
        "moto",     // femenino (abreviatura de motocicleta)
        "grosso",   // latinismo en la locución "grosso modo"
    ];

    exceptions.into_iter().map(String::from).collect()
}

/// Verifica si una palabra usa "el" aunque sea femenina (a tónica)
/// Regla: sustantivos femeninos que empiezan con "a" tónica usan "el/un" en singular
pub fn uses_el_with_feminine(word: &str) -> bool {
    let word_lower = word.to_lowercase();

    // Caso 1: Palabras que empiezan con "á" o "há" (tilde = sílaba tónica segura)
    // Ejemplos: águila, área, álgebra, ánfora, áncora, ánima, árabe (f), ágata
    if word_lower.starts_with('á') || word_lower.starts_with("há") {
        return true;
    }

    // Caso 2: Palabras sin tilde pero con "a" tónica inicial (lista conocida)
    // Estas son palabras llanas (acento en penúltima) cuya primera sílaba es "a" o "ha"
    matches!(
        word_lower.as_str(),
        // Empiezan con "a" tónica
        "agua" | "acta" | "ala" | "alba" | "alga" | "alma" | "alta" | "alza" |
        "ama" | "ancla" | "ansia" | "ara" | "arca" | "arma" | "arpa" |
        "asa" | "asma" | "aspa" | "aula" | "ave" | "ancha" |
        // Empiezan con "ha" tónica
        "habla" | "hacha" | "hada" | "haya" | "hambre" | "hampa" |
        // Plurales no aplican (usan "las/unas"), pero los incluimos por si se buscan
        // en singular con typo o el diccionario tiene número incorrecto
        "aguas" | "actas" | "alas" | "almas" | "armas" | "aulas" | "aves" | "hachas" | "hadas"
    )
}

/// Sustantivos masculinos que terminan en 'a'
pub fn is_masculine_ending_a(word: &str) -> bool {
    let normalized = normalize_spanish(word);
    if is_masculine_ending_a_singular(normalized.as_str()) {
        return true;
    }

    singularize_spanish(normalized.as_str())
        .map(|singular| is_masculine_ending_a_singular(singular.as_str()))
        .unwrap_or(false)
}

fn is_masculine_ending_a_singular(word: &str) -> bool {
    matches!(
        word,
        "dia"
            | "mapa"
            | "problema"
            | "sistema"
            | "tema"
            | "programa"
            | "panorama"
            | "idioma"
            | "clima"
            | "planeta"
            | "poema"
            | "drama"
            | "fantasma"
            | "pijama"
            | "sofa"
    )
}

/// Sustantivos femeninos que terminan en 'o'
pub fn is_feminine_ending_o(word: &str) -> bool {
    let word_lower = word.to_lowercase();
    matches!(word_lower.as_str(), "mano" | "radio" | "foto" | "moto")
}

/// Sustantivos cuyo género depende del significado y aceptan artículo masculino o femenino.
/// Esto evita falsas correcciones en pares como "el cometa"/"la cometa",
/// "el capital"/"la capital" o "el cólera"/"la cólera".
pub fn allows_both_gender_articles(word: &str) -> bool {
    let lemmas = ambiguous_gender_lemmas();

    let normalized = normalize_spanish(word);
    if normalized == "covid" || normalized.starts_with("covid-") {
        return true;
    }
    if lemmas.contains(normalized.as_str()) {
        return true;
    }

    if let Some(singular) = singularize_spanish(&normalized) {
        if lemmas.contains(singular.as_str()) {
            return true;
        }
    }

    // Cubre plurales en -es cuyo singular mantiene la -e:
    // "lentes" -> "lente", "márgenes" -> "margen" (ya cubierto por singularize_spanish).
    if let Some(singular_s) = normalized.strip_suffix('s') {
        if lemmas.contains(singular_s) {
            return true;
        }
    }

    false
}

/// Sustantivos colectivos/partitivos que admiten concordancia variable
/// con verbo o relativo: puede concordar con el núcleo o con el complemento.
/// Ejemplo: "el conjunto de alumnos llegó/llegaron".
pub fn is_variable_collective_noun(word: &str) -> bool {
    let normalized = normalize_spanish(word);
    variable_collective_nouns().contains(normalized.as_str())
}

fn normalize_spanish(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            _ => c,
        })
        .collect()
}

fn singularize_spanish(word: &str) -> Option<String> {
    if let Some(stem) = word.strip_suffix("es") {
        if stem.len() >= 2 {
            return Some(stem.to_string());
        }
    }
    if let Some(stem) = word.strip_suffix('s') {
        if stem.len() >= 2 {
            return Some(stem.to_string());
        }
    }
    None
}

fn ambiguous_gender_lemmas() -> &'static HashSet<String> {
    static LEMMAS: OnceLock<HashSet<String>> = OnceLock::new();
    LEMMAS.get_or_init(|| {
        let mut lemmas: HashSet<String> = include_str!("data/ambiguous_gender_lemmas.txt")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(ToString::to_string)
            .collect();

        // Refuerzo automático: si words.txt trae el mismo sustantivo
        // como masculino y femenino, añadirlo sin parche manual.
        // Si el archivo no está disponible, usamos solo la lista curada.
        lemmas.extend(load_ambiguous_nouns_from_dictionary_file());
        lemmas
    })
}

fn load_ambiguous_nouns_from_dictionary_file() -> HashSet<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("es")
        .join("words.txt");

    let Ok(content) = fs::read_to_string(path) else {
        return HashSet::new();
    };

    parse_ambiguous_nouns_from_words_data(&content)
}

fn parse_ambiguous_nouns_from_words_data(data: &str) -> HashSet<String> {
    let mut gender_mask_by_lemma: HashMap<String, u8> = HashMap::new();

    for line in data.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split('|');
        let Some(raw_word) = parts.next() else {
            continue;
        };
        let Some(raw_category) = parts.next() else {
            continue;
        };
        let Some(raw_gender) = parts.next() else {
            continue;
        };

        let category = raw_category.trim().to_lowercase();
        if !matches!(category.as_str(), "sustantivo" | "noun" | "n") {
            continue;
        }

        let gender_mask = match raw_gender.trim().to_lowercase().as_str() {
            "m" | "masc" | "masculino" | "masculine" => 0b01,
            "f" | "fem" | "femenino" | "feminine" => 0b10,
            _ => 0,
        };
        if gender_mask == 0 {
            continue;
        }

        let lemma = normalize_spanish(raw_word.trim());
        if lemma.is_empty() {
            continue;
        }

        let entry = gender_mask_by_lemma.entry(lemma).or_insert(0);
        *entry |= gender_mask;
    }

    gender_mask_by_lemma
        .into_iter()
        .filter_map(|(lemma, mask)| if mask == 0b11 { Some(lemma) } else { None })
        .collect()
}

fn variable_collective_nouns() -> &'static HashSet<&'static str> {
    static NOUNS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    NOUNS.get_or_init(|| {
        include_str!("data/variable_collective_nouns.txt")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_both_gender_articles_examples() {
        assert!(allows_both_gender_articles("c\u{00f3}lera"));
        assert!(allows_both_gender_articles("c\u{00f3}leras"));
        assert!(allows_both_gender_articles("cometa"));
        assert!(allows_both_gender_articles("capitales"));
        assert!(allows_both_gender_articles("\u{00f3}rdenes"));
        assert!(allows_both_gender_articles("az\u{00fa}cares"));
        assert!(allows_both_gender_articles("margenes"));
        assert!(allows_both_gender_articles("mar"));
        assert!(allows_both_gender_articles("mares"));
        assert!(allows_both_gender_articles("az\u{00fa}car"));
        assert!(allows_both_gender_articles("sart\u{00e9}n"));
        assert!(allows_both_gender_articles("sartenes"));
        assert!(allows_both_gender_articles("internet"));
        assert!(allows_both_gender_articles("radio"));
        assert!(allows_both_gender_articles("radios"));
        assert!(allows_both_gender_articles("calor"));
        assert!(allows_both_gender_articles("calores"));
        assert!(allows_both_gender_articles("marat\u{00f3}n"));
        assert!(allows_both_gender_articles("maratones"));
        assert!(allows_both_gender_articles("lente"));
        assert!(allows_both_gender_articles("lentes"));
        assert!(allows_both_gender_articles("caza"));
    }

    #[test]
    fn test_allows_both_gender_articles_non_ambiguous() {
        assert!(!allows_both_gender_articles("casa"));
        assert!(!allows_both_gender_articles("problema"));
        assert!(!allows_both_gender_articles("silla"));
    }

    #[test]
    fn test_parse_ambiguous_nouns_from_words_data() {
        let data = r#"
# comentario
cólera|sustantivo|f|s||3
cólera|sustantivo|m|s||1
cometa|sustantivo|f|s||4
cometa|sustantivo|m|s||1
casa|sustantivo|f|s||10
rápido|adjetivo|m|s||5
"#;

        let parsed = parse_ambiguous_nouns_from_words_data(data);
        assert!(parsed.contains("colera"));
        assert!(parsed.contains("cometa"));
        assert!(!parsed.contains("casa"));
        assert!(!parsed.contains("rapido"));
    }

    #[test]
    fn test_is_variable_collective_noun_examples() {
        assert!(is_variable_collective_noun("conjunto"));
        assert!(is_variable_collective_noun("mayoría"));
        assert!(is_variable_collective_noun("mayoria"));
        assert!(is_variable_collective_noun("sumatoria"));
        assert!(is_variable_collective_noun("montón"));
        assert!(is_variable_collective_noun("monton"));
        assert!(!is_variable_collective_noun("problema"));
        assert!(!is_variable_collective_noun("casa"));
    }
}

/// Sustantivos invariables (misma forma en singular y plural)
/// Estos no deben generar errores de concordancia de número
pub fn is_invariable_noun(word: &str) -> bool {
    let word_lower = word.to_lowercase();
    matches!(
        word_lower.as_str(),
        // Palabras terminadas en -is (crisis, análisis, tesis, etc.)
        "crisis" | "análisis" | "analisis" | "tesis" | "hipótesis" | "hipotesis" |
        "síntesis" | "sintesis" | "diagnosis" | "prognosis" | "dosis" | "osis" |
        "metamorfosis" | "simbiosis" | "ósmosis" | "osmosis" | "psicosis" |
        "neurosis" | "cirrosis" | "tuberculosis" | "arteriosclerosis" |
        // Palabras terminadas en -us
        "virus" | "campus" | "corpus" | "fetus" | "feto" | "nexus" | "versus" |
        "bonus" | "estatus" | "status" | "cactus" | "eucaliptus" | "ómnibus" |
        // Otras invariables
        "lunes" | "martes" | "miércoles" | "jueves" | "viernes" |
        "paraguas" | "paracaídas" | "sacacorchos" | "rascacielos" |
        "cumpleaños" | "portaaviones" | "saltamontes" | "trabalenguas"
    )
}

/// Sustantivos de género común (aceptan ambos artículos según el referente)
/// Ejemplos: "la periodista María", "el periodista Juan"
pub fn is_common_gender_noun(word: &str) -> bool {
    let word_lower = word.to_lowercase();
    matches!(
        word_lower.as_str(),
        // Profesiones terminadas en -ista
        "periodista" | "artista" | "dentista" | "pianista" | "taxista" |
        "futbolista" | "ciclista" | "economista" | "especialista" | "analista" |
        "protagonista" | "antagonista" | "novelista" | "ensayista" | "cronista" |
        "activista" | "terrorista" | "comunista" | "socialista" | "capitalista" |
        "feminista" | "pacifista" | "oculista" | "electricista" | "recepcionista" |
        "maquinista" | "accionista" | "finalista" | "velocista" | "tenista" |
        "guitarrista" | "baterista" | "violinista" | "flautista" | "solista" |
        "columnista" | "comentarista" | "deportista" | "equilibrista" | "trapecista" |
        // Profesiones terminadas en -ante/-ente
        "estudiante" | "cantante" | "representante" | "agente" | "gerente" |
        "presidente" | "asistente" | "ayudante" | "amante" | "comandante" |
        "comerciante" | "navegante" | "dibujante" | "votante" | "manifestante" |
        "participante" | "concursante" | "aspirante" | "informante" | "postulante" |
        "conferenciante" | "integrante" | "visitante" | "vigilante" | "militante" |
        "dirigente" | "dependiente" | "creyente" | "sobreviviente" | "superviviente" |
        // Otras profesiones de género común
        "atleta" | "colega" | "modelo" | "líder" | "portavoz" | "cónsul" |
        "piloto" | "guía" | "conserje" | "bedel" | "detective" | "intérprete" |
        "astronauta" | "cosmonauta" | "corresponsal" | "chef" | "sommelier" | "concejal" |
        // Títulos y cargos
        "testigo" | "cómplice" | "mártir" | "rehén" | "prócer" |
        // Descriptores personales
        "joven" | "menor" | "mayor" | "adolescente" | "bebé" |
        // Casos especiales (en contextos como "la premio Nobel")
        "premio" | "personaje"
    )
}
