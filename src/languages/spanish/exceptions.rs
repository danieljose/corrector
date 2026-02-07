//! Excepciones y casos especiales del español

use std::collections::HashSet;

/// Obtiene el conjunto de excepciones conocidas
pub fn get_exceptions() -> HashSet<String> {
    let exceptions = vec![
        // Sustantivos femeninos que empiezan con 'a' tónica y usan "el"
        "agua",
        "águila",
        "alma",
        "área",
        "arma",
        "aula",
        "ave",
        "hacha",
        "hada",
        "hambre",
        "álgebra",
        // Palabras que pueden ser masculinas o femeninas según contexto
        "mar",
        "arte",
        "azúcar",
        // Sustantivos epicenos (un solo género para ambos sexos)
        "persona",
        "víctima",
        "genio",
        // Otros casos especiales
        "día",     // masculino aunque termine en 'a'
        "mapa",    // masculino aunque termine en 'a'
        "problema", // masculino aunque termine en 'a'
        "sistema", // masculino aunque termine en 'a'
        "tema",    // masculino aunque termine en 'a'
        "programa", // masculino aunque termine en 'a'
        "idioma",  // masculino aunque termine en 'a'
        "clima",   // masculino aunque termine en 'a'
        "planeta", // masculino aunque termine en 'a'
        "poema",   // masculino aunque termine en 'a'
        "drama",   // masculino aunque termine en 'a'
        "fantasma", // masculino aunque termine en 'a'
        "pijama",  // masculino aunque termine en 'a'
        "sofá",    // masculino aunque termine en 'a'
        "mano",    // femenino aunque termine en 'o'
        "radio",   // femenino aunque termine en 'o' (cuando es aparato)
        "foto",    // femenino (abreviatura de fotografía)
        "moto",    // femenino (abreviatura de motocicleta)
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
        "agua" | "ala" | "alba" | "alga" | "alma" | "alta" | "alza" |
        "ama" | "ancla" | "ansia" | "ara" | "arca" | "arma" | "arpa" |
        "asa" | "asma" | "aspa" | "aula" | "ave" | "ancha" |
        // Empiezan con "ha" tónica
        "habla" | "hacha" | "hada" | "haya" | "hambre" | "hampa" |
        // Plurales no aplican (usan "las/unas"), pero los incluimos por si se buscan
        // en singular con typo o el diccionario tiene número incorrecto
        "aguas" | "alas" | "almas" | "armas" | "aulas" | "aves" | "hachas" | "hadas"
    )
}

/// Sustantivos masculinos que terminan en 'a'
pub fn is_masculine_ending_a(word: &str) -> bool {
    let word_lower = word.to_lowercase();
    matches!(
        word_lower.as_str(),
        "día"
            | "mapa"
            | "problema"
            | "sistema"
            | "tema"
            | "programa"
            | "idioma"
            | "clima"
            | "planeta"
            | "poema"
            | "drama"
            | "fantasma"
            | "pijama"
            | "sofá"
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
    let word_lower = word.to_lowercase();
    matches!(
        word_lower.as_str(),
        "c\u{00f3}lera"
            | "colera"
            | "c\u{00f3}leras"
            | "coleras"
            | "az\u{00fa}car"
            | "azucar"
            | "az\u{00fa}cares"
            | "azucares"
            | "cometa"
            | "cometas"
            | "capital"
            | "capitales"
            | "cura"
            | "curas"
            | "frente"
            | "frentes"
            | "orden"
            | "\u{00f3}rdenes"
            | "ordenes"
            | "pendiente"
            | "pendientes"
            | "editorial"
            | "editoriales"
            | "corte"
            | "cortes"
            | "moral"
            | "morales"
            | "parte"
            | "partes"
            | "margen"
            | "m\u{00e1}rgenes"
            | "margenes"
            | "mar"
            | "mares"
            | "sart\u{00e9}n"
            | "sarten"
            | "sartenes"
            | "internet"
            | "radio"
            | "radios"
            | "calor"
            | "calores"
            | "marat\u{00f3}n"
            | "maraton"
            | "maratones"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_both_gender_articles_examples() {
        assert!(allows_both_gender_articles("c\u{00f3}lera"));
        assert!(allows_both_gender_articles("cometa"));
        assert!(allows_both_gender_articles("capitales"));
        assert!(allows_both_gender_articles("\u{00f3}rdenes"));
        assert!(allows_both_gender_articles("margenes"));
        assert!(allows_both_gender_articles("mar"));
        assert!(allows_both_gender_articles("az\u{00fa}car"));
        assert!(allows_both_gender_articles("sart\u{00e9}n"));
        assert!(allows_both_gender_articles("internet"));
        assert!(allows_both_gender_articles("radio"));
        assert!(allows_both_gender_articles("calor"));
        assert!(allows_both_gender_articles("marat\u{00f3}n"));
    }

    #[test]
    fn test_allows_both_gender_articles_non_ambiguous() {
        assert!(!allows_both_gender_articles("casa"));
        assert!(!allows_both_gender_articles("problema"));
        assert!(!allows_both_gender_articles("silla"));
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
        "astronauta" | "cosmonauta" | "corresponsal" | "chef" | "sommelier" |
        // Títulos y cargos
        "testigo" | "cómplice" | "mártir" | "rehén" | "prócer" |
        // Descriptores personales
        "joven" | "menor" | "mayor" | "adolescente" | "bebé" |
        // Casos especiales (en contextos como "la premio Nobel")
        "premio" | "personaje"
    )
}
