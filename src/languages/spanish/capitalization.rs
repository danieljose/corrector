//! Corrección de mayúsculas
//!
//! Detecta errores de capitalización:
//! - Inicio de oración debe ser mayúscula
//! - Después de punto, signos ¿? ¡! debe ser mayúscula
//! - Excepto después de abreviaturas (EE.UU., Dr., Sr., etc.)

use crate::grammar::{Token, TokenType};

/// Conectores que pueden seguir a un título con signos (subtítulos)
/// Ejemplo: "¡De Viernes! con Ana" - "con" es un conector de subtítulo
const SUBTITLE_CONNECTORS: &[&str] = &["con", "de", "del", "al", "en", "para", "por", "y", "e"];

/// Abreviaturas comunes que terminan en punto pero no indican fin de oración
const COMMON_ABBREVIATIONS: &[&str] = &[
    // Títulos y tratamientos
    "Dr",
    "Dra",
    "Sr",
    "Sra",
    "Srta",
    "Prof",
    "Lic",
    "Ing",
    "Arq",
    "D",
    "Dña",
    "Dn", // Don, Doña
    "Ud",
    "Uds",
    "Vd",
    "Vds", // Usted(es)
    "Excmo",
    "Excma",
    "Ilmo",
    "Ilma", // Excelentísimo, Ilustrísimo
    "Rvdo",
    "Rvda",
    "Mons",
    "Fr",
    "Hno",
    "Hna", // Religiosos
    "Gral",
    "Cnel",
    "Tte",
    "Cap",
    "Cmte",
    "Sgt", // Militares
    "Sto",
    "Sta", // Santo/a
    "Mtro",
    "Mtra", // Maestro/a
    // Siglas dobles (EE.UU., RR.HH., etc.)
    "EE",
    "UU",
    "RR",
    "HH",
    "AA",
    "CC",
    "OO",
    "SS",
    "FF",
    "VV",
    "JJ",
    "PP",
    // Direcciones y lugares
    "Av",
    "Avda",
    "Avd", // Avenida
    "C",
    "Cl",
    "Cll", // Calle
    "Pza",
    "Pl",   // Plaza
    "Ctra", // Carretera
    "Urb",  // Urbanización
    "Edif", // Edificio
    "Esc",  // Escalera
    "Izq",
    "Izda",
    "Dcha",
    "Drcha", // Izquierda, Derecha
    "Pta",
    "Pso", // Puerta, Piso
    "Dpto",
    "Depto",
    "Dept", // Departamento
    "Prov", // Provincia
    "Mun",  // Municipio
    "Col",  // Colonia (México)
    "Apdo", // Apartado
    // Unidades y medidas
    "kg",
    "km",
    "cm",
    "mm",
    "m",
    "g",
    "gr",
    "mg",
    "ml",
    "Lt",
    "l",
    "dl",
    "cl",
    "ha", // Hectárea
    "min",
    "seg",
    "h", // Tiempo
    // Tiempo y fechas
    "a",
    "d",
    "p", // a.m., d.C., p.m.
    "a",
    "s",  // año, siglo
    "ss", // siguientes
    // Bibliografía y referencias
    "pág",
    "págs",
    "p",
    "pp", // Página(s)
    "vol",
    "vols", // Volumen(es)
    "núm",
    "nº",
    "n", // Número
    "ed",
    "eds",  // Editor(es), Edición
    "trad", // Traductor
    "col",  // Colección
    "cap",
    "caps", // Capítulo(s)
    "fig",
    "figs", // Figura(s)
    "tab",  // Tabla
    "op",
    "cit", // Op. cit.
    "ibíd",
    "ibid",
    "íd",
    "id",  // Ibídem, ídem
    "vid", // Véase
    "cf",  // Confer
    "et",
    "al", // et al.
    "sic",
    // Comerciales y legales
    "Cía",
    "Cia",  // Compañía
    "Hnos", // Hermanos
    "Inc",  // Incorporated
    "Ltd",  // Limited
    "Corp", // Corporation
    "Art",
    "art",   // Artículo
    "Admón", // Administración
    "Ayto",  // Ayuntamiento
    "Gob",   // Gobierno
    "Sec",   // Secretaría
    "Ref",   // Referencia
    // Comunicación
    "tel",
    "teléf",
    "tfno",
    "tlf", // Teléfono
    "fax",
    "Att", // A la atención
    "c",
    "cc",  // Con copia
    "adj", // Adjunto
    // Otras comunes
    "etc",
    "etcétera",
    "ej",
    "Ej",    // Ejemplo
    "aprox", // Aproximadamente
    "máx",
    "mín",  // Máximo, mínimo
    "prom", // Promedio
    "obs",  // Observación
    "sig",
    "sigs", // Siguiente(s)
    "ant",  // Anterior
    "ppal", // Principal
    "gral", // General
    "part", // Particular
    "comp", // Compárese
    "vs",   // Versus
    "ca",   // Circa
    "Atte", // Atentamente
    // Meses abreviados
    "ene",
    "feb",
    "mar",
    "abr",
    "may",
    "jun",
    "jul",
    "ago",
    "sep",
    "sept",
    "oct",
    "nov",
    "dic",
];

/// Corrección de mayúscula sugerida
#[derive(Debug, Clone)]
pub struct CapitalizationCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

/// Analizador de mayúsculas
pub struct CapitalizationAnalyzer;

impl CapitalizationAnalyzer {
    /// Analiza los tokens y detecta errores de mayúsculas
    pub fn analyze(tokens: &[Token]) -> Vec<CapitalizationCorrection> {
        let mut corrections = Vec::new();
        let mut expect_uppercase = true; // Al inicio de texto
        let mut last_word: Option<&str> = None; // Para detectar abreviaturas

        for (idx, token) in tokens.iter().enumerate() {
            match token.token_type {
                TokenType::Word => {
                    if expect_uppercase {
                        if let Some(correction) = Self::check_needs_uppercase(idx, token) {
                            corrections.push(correction);
                        }
                    }
                    last_word = Some(&token.text);
                    expect_uppercase = false;
                }
                TokenType::Punctuation => {
                    // Después de estos signos, la siguiente palabra debe ser mayúscula
                    // EXCEPTO si es parte de una abreviatura o cita directa
                    if Self::is_sentence_ending(&token.text) {
                        if token.text == "." {
                            // Verificar si el punto es parte de una abreviatura
                            if let Some(word) = last_word {
                                if Self::is_abbreviation(word) {
                                    // No activar mayúscula después de abreviatura
                                    continue;
                                }
                            }
                        }
                        // Verificar si es fin de cita directa (!" o ?" seguido de coma)
                        // En ese caso, la oración continúa: "¡Hola!", dijo él.
                        if Self::is_end_of_quote(tokens, idx) {
                            continue;
                        }
                        // Verificar si es diálogo con guion largo (!-- o ?--)
                        // "¡Hola!--dijo Juan" no requiere mayúscula después del guion
                        if Self::is_dialog_marker(tokens, idx) {
                            continue;
                        }
                        // Verificar si es un título con signos seguido de conector
                        // "¡De Viernes! con Ana" - "con" no necesita mayúscula
                        if Self::is_title_with_signs(tokens, idx) {
                            continue;
                        }
                        expect_uppercase = true;
                    }
                }
                TokenType::Whitespace | TokenType::Unknown => {
                    // No cambia el estado
                }
                TokenType::Number => {
                    // Un número al inicio de oración "usa" el turno de mayúscula
                    // Ejemplo: ". 227 millones" - el número inicia la oración, "millones" no necesita mayúscula
                    if expect_uppercase {
                        expect_uppercase = false;
                    }
                }
            }
        }

        corrections
    }

    /// Verifica si una palabra es una abreviatura conocida o sigue patrón de abreviatura
    fn is_abbreviation(word: &str) -> bool {
        // Verificar en lista de abreviaturas conocidas (case-insensitive)
        let word_lower = word.to_lowercase();
        if COMMON_ABBREVIATIONS
            .iter()
            .any(|&abbr| abbr.to_lowercase() == word_lower)
        {
            return true;
        }

        // Heurística: palabras cortas (1-4 letras) todas en mayúsculas
        // probablemente son siglas/abreviaturas (EE, UU, AA, etc.)
        if word.len() <= 4 && word.chars().all(|c| c.is_uppercase()) {
            return true;
        }

        // Heurística: términos alfanuméricos que terminan en unidades de temperatura (C, F, K)
        // Ejemplos: 52,2C, 100F, 273K
        if word.chars().any(|c| c.is_numeric()) {
            let last_char = word.chars().last();
            if matches!(last_char, Some('C') | Some('F') | Some('K')) {
                return true;
            }
        }

        false
    }

    /// Verifica si el token necesita mayúscula inicial
    fn check_needs_uppercase(idx: usize, token: &Token) -> Option<CapitalizationCorrection> {
        let first_char = token.text.chars().next()?;

        // Si ya empieza con mayúscula, está bien
        if first_char.is_uppercase() {
            return None;
        }

        // Si empieza con minúscula pero tiene mayúsculas internas (xAI, iOS, eBay),
        // es probable que sea un nombre propio/marca que no debe modificarse
        if token.text.chars().skip(1).any(|c| c.is_uppercase()) {
            return None;
        }

        // Si empieza con minúscula, necesita corrección
        if first_char.is_lowercase() {
            let capitalized = Self::capitalize(&token.text);
            return Some(CapitalizationCorrection {
                token_index: idx,
                original: token.text.clone(),
                suggestion: capitalized,
                reason: "Inicio de oración requiere mayúscula".to_string(),
            });
        }

        None
    }

    /// Verifica si estamos al final de una cita directa o paréntesis que continúa la oración
    /// Patrón: [!?] + [comillas/paréntesis] + [coma/palabra] → la oración sigue en minúscula
    /// Ejemplos: "¡Hola!", dijo. / "¿Qué?", preguntó. / (¿algo?) y luego
    fn is_end_of_quote(tokens: &[Token], current_idx: usize) -> bool {
        // Buscar los siguientes tokens (saltando espacios)
        let mut found_quote = false;
        let mut found_paren = false;
        let mut found_comma = false;

        for token in tokens.iter().skip(current_idx + 1) {
            match token.token_type {
                TokenType::Whitespace => continue,
                TokenType::Punctuation => {
                    let text = token.text.as_str();
                    // Comillas de cierre
                    if text == "\"" || text == "\u{201D}" || text == "'" || text == "»" {
                        found_quote = true;
                    }
                    // Paréntesis de cierre - indica que la interrogación/exclamación está dentro de un paréntesis
                    else if text == ")" {
                        found_paren = true;
                    }
                    // Coma después de comillas
                    else if text == "," && found_quote {
                        found_comma = true;
                        break;
                    }
                    // Otro signo de puntuación - salir
                    else if found_quote {
                        break;
                    }
                }
                TokenType::Word => {
                    // Si encontramos paréntesis de cierre y luego una palabra, la oración continúa
                    if found_paren {
                        return true;
                    }
                    break; // Palabra sin paréntesis previo - salir
                }
                _ => break,
            }
        }

        found_quote && found_comma
    }

    /// Verifica si es un marcador de diálogo con guion largo
    /// Detecta patrones como "!--" o "?--" donde no se requiere mayúscula después
    fn is_dialog_marker(tokens: &[Token], punct_idx: usize) -> bool {
        // Buscar guion largo después del signo de puntuación
        let mut dash_count = 0;
        for i in (punct_idx + 1)..tokens.len() {
            let token = &tokens[i];
            match token.token_type {
                TokenType::Whitespace => continue,
                TokenType::Punctuation => {
                    // Guion largo tipográfico
                    if token.text == "—" || token.text == "–" {
                        return true;
                    }
                    // Dos guiones seguidos (-- simulando guion largo)
                    if token.text == "-" {
                        dash_count += 1;
                        if dash_count >= 2 {
                            return true;
                        }
                        continue;
                    }
                    break;
                }
                _ => break,
            }
        }
        false
    }

    /// Verifica si el signo de cierre (! o ?) es parte de un título con signos
    /// y la palabra siguiente es un conector de subtítulo.
    /// Ejemplo: "¡De Viernes! con Ana" - "con" no necesita mayúscula
    ///
    /// Criterios para detectar un título:
    /// 1. Encontrar el signo de apertura (¡/¿) hacia atrás sin otro fin de oración
    /// 2. Contar palabras entre apertura y cierre: >= 2 y <= 5
    /// 3. Todas las palabras internas están en Title Case o ALL CAPS
    /// 4. La palabra siguiente es un conector de subtítulo
    fn is_title_with_signs(tokens: &[Token], close_idx: usize) -> bool {
        let close_sign = &tokens[close_idx].text;

        // Solo aplica a ! y ?
        if close_sign != "!" && close_sign != "?" {
            return false;
        }

        // Determinar qué signo de apertura buscar
        let open_sign = if close_sign == "!" { "¡" } else { "¿" };

        // Buscar el signo de apertura hacia atrás
        let mut open_idx = None;
        for i in (0..close_idx).rev() {
            let token = &tokens[i];
            match token.token_type {
                TokenType::Punctuation => {
                    if token.text == open_sign {
                        open_idx = Some(i);
                        break;
                    }
                    // Si encontramos otro fin de oración, no es un título válido
                    if Self::is_sentence_ending(&token.text) {
                        return false;
                    }
                }
                _ => {}
            }
        }

        let open_idx = match open_idx {
            Some(idx) => idx,
            None => return false, // No encontramos el signo de apertura
        };

        // Contar palabras entre apertura y cierre
        let mut words_inside: Vec<&str> = Vec::new();
        for i in (open_idx + 1)..close_idx {
            if tokens[i].token_type == TokenType::Word {
                words_inside.push(&tokens[i].text);
            }
        }

        // Criterio: entre 2 y 5 palabras
        if words_inside.len() < 2 || words_inside.len() > 5 {
            return false;
        }

        // Criterio: todas las palabras en Title Case o ALL CAPS
        // Title Case = primera letra mayúscula, resto minúsculas
        // ALL CAPS = todas mayúsculas
        for word in &words_inside {
            if !Self::is_title_case_or_caps(word) {
                return false;
            }
        }

        // Buscar la siguiente palabra después del cierre
        // Saltar comillas y paréntesis de cierre que puedan envolver el título
        let mut next_word: Option<&str> = None;
        for i in (close_idx + 1)..tokens.len() {
            match tokens[i].token_type {
                TokenType::Whitespace => continue,
                TokenType::Word => {
                    next_word = Some(&tokens[i].text);
                    break;
                }
                TokenType::Punctuation => {
                    let punct = tokens[i].text.as_str();
                    // Saltar comillas y paréntesis de cierre
                    if punct == "\""
                        || punct == "'"
                        || punct == "»"
                        || punct == "\u{201D}"
                        || punct == ")"
                    {
                        continue;
                    }
                    // Otro signo de puntuación: parar
                    break;
                }
                _ => break,
            }
        }

        // Verificar si la siguiente palabra es un conector de subtítulo (en minúscula)
        if let Some(word) = next_word {
            let word_lower = word.to_lowercase();
            return SUBTITLE_CONNECTORS.contains(&word_lower.as_str());
        }

        false
    }

    /// Verifica si una palabra está en Title Case (Primera mayúscula, resto minúsculas)
    /// o en ALL CAPS (todas mayúsculas)
    fn is_title_case_or_caps(word: &str) -> bool {
        let chars: Vec<char> = word.chars().collect();
        if chars.is_empty() {
            return false;
        }

        // ALL CAPS: todas son mayúsculas
        if chars.iter().all(|c| !c.is_alphabetic() || c.is_uppercase()) {
            return true;
        }

        // Title Case: primera mayúscula, resto minúsculas
        let first = chars[0];
        if first.is_lowercase() {
            return false;
        }

        // El resto deben ser minúsculas (o no alfabéticos como tildes procesadas)
        for c in chars.iter().skip(1) {
            if c.is_alphabetic() && c.is_uppercase() {
                return false;
            }
        }

        true
    }

    /// Verifica si la puntuación termina una oración
    /// Nota: "..." y ".." no se incluyen porque a menudo indican pausa, no fin de oración
    fn is_sentence_ending(punct: &str) -> bool {
        // Solo un punto suelto termina oración, no múltiples puntos
        if punct.starts_with('.') && punct.len() > 1 {
            return false;
        }
        // Nota: ¿ y ¡ son signos de APERTURA, no de fin de oración.
        // Solo . ? ! terminan oraciones y requieren mayúscula después.
        // ¿ y ¡ no deben activar mayúscula porque pueden estar en medio
        // de una oración: "Dijo una ¿confusa? respuesta"
        matches!(punct, "." | "?" | "!")
    }

    /// Convierte la primera letra a mayúscula
    fn capitalize(word: &str) -> String {
        let mut chars = word.chars();
        match chars.next() {
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
            None => word.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<CapitalizationCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        CapitalizationAnalyzer::analyze(&tokens)
    }

    #[test]
    fn test_start_of_text_lowercase() {
        let corrections = analyze_text("hola mundo");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "hola");
        assert_eq!(corrections[0].suggestion, "Hola");
    }

    #[test]
    fn test_start_of_text_uppercase_ok() {
        let corrections = analyze_text("Hola mundo");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_after_period() {
        let corrections = analyze_text("Hola. mundo");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "mundo");
        assert_eq!(corrections[0].suggestion, "Mundo");
    }

    #[test]
    fn test_after_question_mark() {
        let corrections = analyze_text("Hola? que tal");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "que");
        assert_eq!(corrections[0].suggestion, "Que");
    }

    #[test]
    fn test_after_exclamation() {
        let corrections = analyze_text("Hola! como estas");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "como");
        assert_eq!(corrections[0].suggestion, "Como");
    }

    #[test]
    fn test_after_inverted_question() {
        // ¿ es signo de apertura, no de fin de oración.
        // No debe activar mayúscula automáticamente porque puede estar
        // en medio de una oración: "una ¿confusa? respuesta"
        let corrections = analyze_text("Bien ¿que tal?");
        // Ahora no se corrige porque ¿ no activa mayúscula
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_question_in_middle_of_sentence() {
        // ¿ no activa mayúscula, así que "confusa" no se corrige.
        // Pero ? sí termina oración, así que "reflexión" se corrige.
        // "una ¿confusa? reflexión" → "Una ¿confusa? Reflexión"
        let corrections = analyze_text("una ¿confusa? reflexión");
        assert_eq!(corrections.len(), 2);
        assert_eq!(corrections[0].original, "una");
        assert_eq!(corrections[0].suggestion, "Una");
        assert_eq!(corrections[1].original, "reflexión");
        assert_eq!(corrections[1].suggestion, "Reflexión");
    }

    #[test]
    fn test_inverted_exclamation_no_uppercase() {
        // ¡ tampoco debe activar mayúscula
        let corrections = analyze_text("Vino ¡qué sorpresa! dijo");
        // "qué" después de ¡ no necesita corrección (¡ no activa mayúscula)
        // "dijo" después de ! sí necesita mayúscula
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "dijo");
        assert_eq!(corrections[0].suggestion, "Dijo");
    }

    #[test]
    fn test_multiple_sentences() {
        let corrections = analyze_text("hola. mundo. bien");
        assert_eq!(corrections.len(), 3);
        assert_eq!(corrections[0].suggestion, "Hola");
        assert_eq!(corrections[1].suggestion, "Mundo");
        assert_eq!(corrections[2].suggestion, "Bien");
    }

    #[test]
    fn test_comma_no_uppercase() {
        // Después de coma NO se requiere mayúscula
        let corrections = analyze_text("Hola, mundo");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_ellipsis_no_uppercase() {
        // Después de "..." NO se requiere mayúscula (puede ser pausa)
        let corrections = analyze_text("Hola... mundo");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_number_after_period() {
        // Número después de punto "inicia" la oración
        // "Total. 5 items" - el número usa el turno de mayúscula, "items" no necesita corrección
        let corrections = analyze_text("Total. 5 items");
        assert_eq!(corrections.len(), 0);

        // Pero si no hay número, la palabra sí necesita mayúscula
        let corrections2 = analyze_text("Total. items aquí");
        assert_eq!(corrections2.len(), 1);
        assert_eq!(corrections2[0].original, "items");
    }

    // Tests para abreviaturas

    #[test]
    fn test_abbreviation_eeuu() {
        // No debe sugerir mayúscula después de "EE.UU."
        let corrections = analyze_text("Senado de EE.UU. que aprobó");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'que' después de EE.UU."
        );
    }

    #[test]
    fn test_abbreviation_dr() {
        // No debe sugerir mayúscula después de "Dr."
        let corrections = analyze_text("El Dr. García llegó");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'García' después de Dr."
        );
    }

    #[test]
    fn test_abbreviation_sra() {
        // No debe sugerir mayúscula después de "Sra."
        let corrections = analyze_text("La Sra. López llamó");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'López' después de Sra."
        );
    }

    #[test]
    fn test_abbreviation_etc() {
        // No debe sugerir mayúscula después de "etc."
        let corrections = analyze_text("Manzanas, peras, etc. todo fresco");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'todo' después de etc."
        );
    }

    #[test]
    fn test_abbreviation_siglas_mayusculas() {
        // Siglas cortas en mayúsculas se tratan como abreviaturas
        let corrections = analyze_text("Según la ONU. las negociaciones");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'las' después de ONU."
        );
    }

    #[test]
    fn test_normal_word_before_period() {
        // Palabra normal antes de punto SÍ activa mayúscula
        let corrections = analyze_text("Llegó tarde. ella se fue");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "ella");
        assert_eq!(corrections[0].suggestion, "Ella");
    }

    #[test]
    fn test_abbreviation_av_address() {
        // Abreviatura de dirección
        let corrections = analyze_text("Vive en la Av. principal");
        assert!(corrections.is_empty(), "No debe corregir después de Av.");
    }

    #[test]
    fn test_abbreviation_pag_bibliography() {
        // Abreviatura bibliográfica
        let corrections = analyze_text("Ver pág. siguiente");
        assert!(corrections.is_empty(), "No debe corregir después de pág.");
    }

    #[test]
    fn test_abbreviation_gral_military() {
        // Abreviatura militar
        let corrections = analyze_text("El Gral. comandó las tropas");
        assert!(corrections.is_empty(), "No debe corregir después de Gral.");
    }

    #[test]
    fn test_abbreviation_tel_contact() {
        // Abreviatura de contacto
        let corrections = analyze_text("Contactar al tel. indicado");
        assert!(corrections.is_empty(), "No debe corregir después de tel.");
    }

    #[test]
    fn test_mixed_case_proper_names() {
        // Nombres propios con mayúsculas internas no deben corregirse
        let corrections = analyze_text("xAI es una empresa");
        assert!(
            corrections.is_empty(),
            "No debe corregir xAI (tiene mayúsculas internas)"
        );

        let corrections2 = analyze_text("iOS es un sistema operativo");
        assert!(corrections2.is_empty(), "No debe corregir iOS");

        let corrections3 = analyze_text("eBay vende productos");
        assert!(corrections3.is_empty(), "No debe corregir eBay");
    }

    #[test]
    fn test_temperature_units() {
        // Unidades de temperatura después de números no deben activar mayúscula
        // "en" no debe corregirse después de "52,2C."
        let corrections = analyze_text("Temperatura de 52,2C. en verano");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'en' después de 52,2C."
        );

        // "muy" no debe corregirse después de "100F."
        let corrections2 = analyze_text("Alcanzó los 100F. muy caliente");
        assert!(
            corrections2.is_empty(),
            "No debe corregir 'muy' después de 100F."
        );

        // "se" no debe corregirse después de "273K."
        let corrections3 = analyze_text("A 273K. se congela");
        assert!(
            corrections3.is_empty(),
            "No debe corregir 'se' después de 273K."
        );
    }

    // Tests para títulos con signos

    #[test]
    fn test_title_with_signs_de_viernes() {
        // "¡De Viernes! con Ana" - "con" no debe corregirse
        let corrections = analyze_text("¡De Viernes! con Ana");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'con' en título '¡De Viernes! con Ana'"
        );
    }

    #[test]
    fn test_title_with_signs_quien_es_quien() {
        // "¿Quién Es Quién? en política" - "en" no debe corregirse
        let corrections = analyze_text("¿Quién Es Quién? en política");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'en' en título '¿Quién Es Quién?'"
        );
    }

    #[test]
    fn test_title_with_signs_normal_sentence() {
        // "¡Hola Mundo! Volvemos" - "Volvemos" ya tiene mayúscula, pero si fuera minúscula sí debería corregirse
        // porque no tiene un conector de subtítulo
        let corrections = analyze_text("¡Hola Mundo! volvemos");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "volvemos");
        assert_eq!(corrections[0].suggestion, "Volvemos");
    }

    #[test]
    fn test_title_with_signs_too_short() {
        // "¡Basta! con eso" - solo 1 palabra dentro, no es título
        let corrections = analyze_text("¡Basta! con eso");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "con");
        assert_eq!(corrections[0].suggestion, "Con");
    }

    #[test]
    fn test_title_with_signs_lowercase_inside() {
        // "Algo ¡vamos a ver! con María" - palabras internas en minúscula, no es título
        // Por tanto "con" sí debe corregirse a "Con"
        let corrections = analyze_text("Algo ¡vamos a ver! con María");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "con");
        assert_eq!(corrections[0].suggestion, "Con");
    }

    #[test]
    fn test_title_with_signs_all_caps() {
        // "¡EXCLUSIVO MUNDIAL! de última hora" - ALL CAPS es válido como título
        let corrections = analyze_text("¡EXCLUSIVO MUNDIAL! de última hora");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'de' en título ALL CAPS"
        );
    }

    #[test]
    fn test_title_with_signs_quoted() {
        // "¡De Viernes!" con Ana - título entrecomillado
        // Debe saltar las comillas de cierre para encontrar el conector "con"
        let corrections = analyze_text("\"¡De Viernes!\" con Ana");
        assert!(
            corrections.is_empty(),
            "No debe corregir 'con' en título entrecomillado"
        );

        // Con comillas españolas »
        let corrections2 = analyze_text("«¡De Viernes!» con Ana");
        assert!(
            corrections2.is_empty(),
            "No debe corregir 'con' con comillas españolas"
        );
    }
}
