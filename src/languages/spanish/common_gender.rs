//! Analizador de concordancia de género común con referente explícito
//!
//! Detecta errores de concordancia en sustantivos de género común (periodista, líder, premio)
//! cuando hay un referente explícito (nombre propio) que indica el género.
//!
//! Ejemplo:
//! - "el periodista María García" → "la periodista María García"
//! - "la premio Nobel Juan Pérez" → "el premio Nobel Juan Pérez"
//!
//! También detecta cuando la gramática corrigió incorrectamente:
//! - Original "la premio Nobel María" → Gramática sugirió "el" → Anulamos porque "María" es femenino

use crate::dictionary::{Gender, ProperNames, Trie};
use crate::grammar::{has_sentence_boundary, Token, TokenType};

use super::exceptions::is_common_gender_noun;
use super::names_gender::get_name_gender;

/// Tipo de acción a realizar
#[derive(Debug, Clone, PartialEq)]
pub enum CommonGenderAction {
    /// Corregir el artículo a la sugerencia
    Correct(String),
    /// Anular una corrección gramatical previa (el original era correcto)
    ClearCorrection,
}

/// Corrección de género común
#[derive(Debug, Clone)]
pub struct CommonGenderCorrection {
    pub token_index: usize,
    pub original: String,
    pub action: CommonGenderAction,
    pub message: String,
}

impl CommonGenderCorrection {
    /// Compatibility helper - returns suggestion if it's a Correct action
    pub fn suggestion(&self) -> Option<&str> {
        match &self.action {
            CommonGenderAction::Correct(s) => Some(s),
            CommonGenderAction::ClearCorrection => None,
        }
    }
}

/// Analizador de género común con referente
pub struct CommonGenderAnalyzer;

impl CommonGenderAnalyzer {
    /// Analiza tokens buscando sustantivos de género común con referente explícito
    pub fn analyze(
        tokens: &[Token],
        dictionary: &Trie,
        proper_names: &ProperNames,
    ) -> Vec<CommonGenderCorrection> {
        let mut corrections = Vec::new();

        // Recopilar índices de tokens de tipo Word
        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Buscar patrón: artículo + sustantivo_género_común + [adjetivos]* + nombre_propio
        for (word_idx, (token_idx, token)) in word_tokens.iter().enumerate() {
            // Verificar si es un artículo usando el texto POST-ortografía pero PRE-gramática
            // Esto es: corrected_spelling si existe, sino el texto original
            // NO usamos effective_text() porque incluiría corrected_grammar
            let post_spelling_article = token.corrected_spelling
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or(&token.text);
            let post_spelling_article_lower = post_spelling_article.to_lowercase();

            let post_spelling_gender = match post_spelling_article_lower.as_str() {
                "el" | "un" => Some(Gender::Masculine),
                "la" | "una" => Some(Gender::Feminine),
                _ => None,
            };

            let Some(post_spelling_gender) = post_spelling_gender else {
                continue;
            };

            // Verificar si el siguiente token es un sustantivo de género común
            if word_idx + 1 >= word_tokens.len() {
                continue;
            }

            let (noun_token_idx, noun_token) = word_tokens[word_idx + 1];
            let noun_lower = noun_token.effective_text().to_lowercase();

            if !is_common_gender_noun(&noun_lower) {
                continue;
            }

            // Verificar que no hay límite de oración entre artículo y sustantivo
            if has_sentence_boundary(tokens, *token_idx, noun_token_idx) {
                continue;
            }

            // Buscar un nombre propio en los siguientes 4 tokens
            // Saltar adjetivos intermedios
            let referent_result = Self::find_referent_gender(
                tokens,
                &word_tokens,
                word_idx + 2,
                *token_idx,
                dictionary,
                proper_names,
            );

            let Some(referent_gender) = referent_result else {
                // Sin referente explícito, no podemos determinar el género correcto
                continue;
            };

            // Comparar el artículo POST-ORTOGRAFÍA con el género del referente
            if post_spelling_gender == referent_gender {
                // El artículo (tras ortografía) es correcto para el referente
                // Si hay una corrección gramatical previa, debemos anularla
                if token.corrected_grammar.is_some() {
                    corrections.push(CommonGenderCorrection {
                        token_index: *token_idx,
                        original: token.text.clone(),
                        action: CommonGenderAction::ClearCorrection,
                        message: format!(
                            "Artículo '{}' correcto para referente femenino/masculino (anulando corrección previa)",
                            post_spelling_article
                        ),
                    });
                }
                // Si no hay corrección previa, no hacer nada (ya está correcto)
            } else {
                // El artículo post-ortografía NO concuerda con el referente → corregir
                let correct_article = match (&post_spelling_article_lower.as_str(), &referent_gender) {
                    (&"el", &Gender::Feminine) => "la",
                    (&"la", &Gender::Masculine) => "el",
                    (&"un", &Gender::Feminine) => "una",
                    (&"una", &Gender::Masculine) => "un",
                    _ => continue,
                };

                // Preservar mayúsculas
                let suggestion = if token.text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    let mut chars = correct_article.chars();
                    match chars.next() {
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                        None => correct_article.to_string(),
                    }
                } else {
                    correct_article.to_string()
                };

                corrections.push(CommonGenderCorrection {
                    token_index: *token_idx,
                    original: token.text.clone(),
                    action: CommonGenderAction::Correct(suggestion),
                    message: format!(
                        "Concordancia con referente: '{}' debería ser '{}' (referente femenino/masculino)",
                        token.text, correct_article
                    ),
                });
            }
        }

        corrections
    }

    /// Busca el género del referente (nombre propio o adjetivo inmediato) en una ventana de tokens
    /// Prioridad: nombre propio > adjetivo con género explícito
    /// Solo considera adjetivos inmediatos (adyacentes o con adverbio de grado intermedio)
    /// Verifica límites de oración durante la búsqueda
    fn find_referent_gender(
        all_tokens: &[Token],
        word_tokens: &[(usize, &Token)],
        start_idx: usize,
        article_token_idx: usize,
        dictionary: &Trie,
        proper_names: &ProperNames,
    ) -> Option<Gender> {
        use crate::dictionary::WordCategory;

        // Ventana base de 4 tokens, ampliable dinámicamente si hay inciso
        // La ventana se extiende hasta el cierre del inciso + tokens extra para el nombre
        const BASE_WINDOW: usize = 4;
        const FALLBACK_EXTENDED: usize = 10; // fallback si no se encuentra cierre
        const TOKENS_AFTER_CLOSER: usize = 4; // tokens extra tras el cierre para buscar nombre
        let max_possible = word_tokens.len().saturating_sub(start_idx);
        let mut window_size = BASE_WINDOW.min(max_possible);

        // Recordar el género del primer adjetivo inmediato con género explícito
        let mut adjective_gender: Option<Gender> = None;
        // Contador de tokens procesados para limitar búsqueda de adjetivos
        let mut tokens_since_noun = 0;
        // Contador de adverbios de grado encontrados (permite "muy muy buena")
        let mut degree_adverb_count: usize = 0;

        // Índice del sustantivo de género común (token anterior al start_idx)
        let noun_token_idx = if start_idx > 0 {
            word_tokens.get(start_idx - 1).map(|(idx, _)| *idx)
        } else {
            None
        };

        // Flag para indicar si hay inciso (bloquea adjetivos pero no nombres propios)
        let mut incise_found = false;

        let mut offset = 0;
        while offset < window_size {
            let idx = start_idx + offset;
            if idx >= word_tokens.len() {
                break;
            }

            let (current_token_idx, token) = &word_tokens[idx];

            // Verificar que no hay límite de oración entre el artículo y este token
            if has_sentence_boundary(all_tokens, article_token_idx, *current_token_idx) {
                break;
            }

            // Detectar incisos (bloquean adjetivos pero no nombres propios)
            // Incluye: comas, paréntesis, guiones largos, punto y coma
            if !incise_found {
                if let Some(noun_idx) = noun_token_idx {
                    if let Some((opener_pos, opener_type)) = Self::find_incise_between(all_tokens, noun_idx, *current_token_idx) {
                        incise_found = true;
                        // Buscar el cierre del inciso para calcular ventana exacta
                        if let Some(closer_pos) = Self::find_incise_closer(all_tokens, opener_pos, opener_type) {
                            // Convertir posición en all_tokens a offset en word_tokens
                            // Buscar cuántos word_tokens hay hasta el closer
                            let mut tokens_to_closer = 0;
                            for (wi, (ti, _)) in word_tokens.iter().enumerate().skip(start_idx) {
                                if *ti > closer_pos {
                                    tokens_to_closer = wi.saturating_sub(start_idx);
                                    break;
                                }
                            }
                            // Ventana = hasta el cierre + tokens extra para el nombre
                            let new_window = tokens_to_closer + TOKENS_AFTER_CLOSER;
                            window_size = new_window.min(max_possible);
                        } else {
                            // Sin cierre encontrado, usar fallback
                            window_size = FALLBACK_EXTENDED.min(max_possible);
                        }
                    }
                }
            }

            let text = token.effective_text();
            let lower = text.to_lowercase();

            // Saltar palabras especiales como "Nobel", "Pulitzer" (títulos de premios)
            if matches!(lower.as_str(), "nobel" | "pulitzer" | "cervantes" | "goya" |
                        "príncipe" | "princesa" | "nacional" | "internacional") {
                offset += 1;
                continue;
            }

            // PRIORIDAD MÁXIMA: Verificar nombres propios ANTES de cualquier decisión
            // Esto evita que nombres como "Sofía" (termina en -ía) se confundan con verbos
            if Self::is_proper_name_candidate(text, proper_names) {
                if let Some(gender) = get_name_gender(text) {
                    return Some(gender);
                }
            }

            // Verificar si es adverbio de grado ANTES del check de categorías
            // (más/menos están en diccionario como otras categorías, pero aquí funcionan como adverbios)
            // Solo si no hay coma y aún buscamos adjetivos
            if !incise_found && adjective_gender.is_none() {
                // Caso especial: "mas" sin tilde seguido de adjetivo (o adverbio + adjetivo)
                // Lo tratamos como adverbio de grado antes de que diacríticas lo corrija
                let is_degree = if lower == "mas" {
                    // Verificar si hay adjetivo adelante (permite "mas muy buena")
                    Self::has_adjective_ahead(word_tokens, idx + 1, dictionary)
                } else {
                    Self::is_degree_adverb(&lower)
                };

                if is_degree {
                    degree_adverb_count += 1;
                    tokens_since_noun += 1;
                    offset += 1;
                    continue;
                }
            }

            // Verificar categoría en el diccionario
            let dict_info = dictionary.get(&lower);

            // Flag para indicar si esta palabra bloquea la búsqueda de adjetivos
            // (pero NO la de nombres propios, que ya verificamos arriba)
            let mut blocks_adjective_search = false;

            // Si es verbo, preposición, conjunción, pronombre o determinante, bloquear adjetivos
            if let Some(info) = &dict_info {
                if matches!(info.category, WordCategory::Verbo | WordCategory::Preposicion |
                           WordCategory::Conjuncion | WordCategory::Pronombre |
                           WordCategory::Determinante | WordCategory::Articulo) {
                    blocks_adjective_search = true;
                }
            }

            // Detectar verbos conjugados no en diccionario usando heurísticas
            if dict_info.is_none() && Self::looks_like_conjugated_verb(&lower) {
                blocks_adjective_search = true;
            }

            // Si encontramos algo que bloquea adjetivos, marcar pero seguir buscando nombres
            if blocks_adjective_search {
                tokens_since_noun = 10; // Forzar que no busque más adjetivos
                offset += 1;
                continue; // Pero seguir buscando nombres propios
            }

            // Para adjetivos: solo considerar posiciones inmediatas y sin coma
            let max_adj_distance = 1 + degree_adverb_count;

            if !incise_found && adjective_gender.is_none() && tokens_since_noun < max_adj_distance {
                // Solo aceptar adjetivos del diccionario (no sustantivos ni heurísticas)
                // El diccionario tiene ~52,000 adjetivos, suficiente cobertura
                if let Some(info) = &dict_info {
                    if info.category == WordCategory::Adjetivo && info.gender != Gender::None {
                        adjective_gender = Some(info.gender);
                        tokens_since_noun += 1;
                        offset += 1;
                        continue;
                    }
                }
            }

            tokens_since_noun += 1;
            offset += 1;
        }

        // Si no encontramos nombre propio, usar el género del adjetivo (si lo hay)
        adjective_gender
    }

    /// Verifica si una palabra es un adverbio de grado
    /// Se verifica ANTES del check de categorías del diccionario para que
    /// "más/menos" funcionen como adverbios aunque estén categorizados de otra forma
    fn is_degree_adverb(word: &str) -> bool {
        // Incluimos más/menos porque en "más famosa" funcionan como adverbios de grado
        // aunque en el diccionario puedan estar como adverbio/conjunción
        // Excluimos: algo/nada (casi siempre pronombres en este contexto)
        // Nota: "mas" sin tilde se maneja aparte con lookahead
        matches!(word, "muy" | "tan" | "bastante" | "poco" | "bien" | "demasiado" |
                 "más" | "menos" |
                 "sumamente" | "extremadamente" | "tremendamente" | "increíblemente" |
                 "absolutamente" | "totalmente" | "completamente" | "realmente")
    }

    /// Verifica si hay un adjetivo con género en los siguientes tokens
    /// Permite adverbios de grado intermedios: "mas muy buena" → true
    /// Bloquea si hay preposición, pronombre, verbo, etc.
    /// Usado para tratar "mas" (sin tilde) como adverbio de grado antes de diacríticas
    fn has_adjective_ahead(
        word_tokens: &[(usize, &Token)],
        start_idx: usize,
        dictionary: &Trie,
    ) -> bool {
        use crate::dictionary::WordCategory;

        // Buscar en máximo 2 tokens adelante (permite "mas muy buena")
        for offset in 0..2 {
            let idx = start_idx + offset;
            if idx >= word_tokens.len() {
                return false;
            }

            let (_, token) = &word_tokens[idx];
            let lower = token.effective_text().to_lowercase();

            // Si es adjetivo con género, éxito
            if let Some(info) = dictionary.get(&lower) {
                if info.category == WordCategory::Adjetivo && info.gender != Gender::None {
                    return true;
                }

                // Si es adverbio de grado, continuar buscando
                if Self::is_degree_adverb(&lower) {
                    continue;
                }

                // Si es preposición, pronombre, verbo, etc., bloquear
                if matches!(info.category,
                    WordCategory::Preposicion | WordCategory::Pronombre |
                    WordCategory::Verbo | WordCategory::Conjuncion |
                    WordCategory::Articulo | WordCategory::Determinante) {
                    return false;
                }
            }

            // Verificar si parece verbo conjugado
            if Self::looks_like_conjugated_verb(&lower) {
                return false;
            }

            // Si es adverbio de grado (puede no estar en diccionario con esa categoría)
            if Self::is_degree_adverb(&lower) {
                continue;
            }

            // Token desconocido en posición 0 → no es adjetivo directo
            // Token desconocido en posición 1 → ya pasamos el posible adverbio, bloquear
            return false;
        }

        false
    }

    /// Heurística para detectar verbos conjugados que no están en el diccionario
    fn looks_like_conjugated_verb(word: &str) -> bool {
        // Verbos copulativos comunes (pueden no estar en diccionario con todas sus formas)
        if matches!(word, "es" | "está" | "son" | "están" | "era" | "eran" |
                   "fue" | "fueron" | "será" | "serán" | "sería" | "serían" |
                   "estaba" | "estaban" | "estuvo" | "estuvieron" |
                   "parece" | "parecen" | "resulta" | "resultan") {
            return true;
        }

        // Terminaciones típicas de verbos conjugados
        // Presente indicativo: -a, -e, -an, -en (3ra persona)
        // Pretérito: -ó, -ió, -aron, -ieron
        // Imperfecto: -aba, -ía, -aban, -ían
        if word.len() >= 4 {
            if word.ends_with("aba") || word.ends_with("aban") ||
               word.ends_with("ía") || word.ends_with("ían") ||
               word.ends_with("aron") || word.ends_with("ieron") ||
               word.ends_with("ando") || word.ends_with("iendo") {
                return true;
            }
        }

        false
    }

    /// Verifica si una palabra es candidata a nombre propio
    fn is_proper_name_candidate(word: &str, proper_names: &ProperNames) -> bool {
        // Debe estar capitalizado
        let first_char = word.chars().next();
        if !first_char.map(|c| c.is_uppercase()).unwrap_or(false) {
            return false;
        }

        // Debe estar en la lista de nombres propios
        proper_names.is_proper_name(word)
    }

    /// Verifica si hay un marcador de inciso entre dos posiciones de tokens
    /// Incluye: comas, paréntesis, guiones largos, punto y coma, comillas
    /// Devuelve Some((posición_apertura, tipo_inciso)) si encuentra uno
    fn find_incise_between(tokens: &[Token], start_idx: usize, end_idx: usize) -> Option<(usize, char)> {
        for i in start_idx..end_idx {
            if i < tokens.len() {
                let text = tokens[i].effective_text();
                // Coma, paréntesis, guiones largos, comillas
                match text {
                    "(" => return Some((i, '(')),
                    "," => return Some((i, ',')),
                    "—" => return Some((i, '—')),
                    "–" => return Some((i, '–')),
                    ";" => return Some((i, ';')), // sin cierre claro
                    // Comillas (evita que adjetivos citados den pista de género)
                    "«" => return Some((i, '«')),
                    "\"" => return Some((i, '"')),
                    "\u{201C}" => return Some((i, '\u{201C}')), // " (comilla tipográfica apertura)
                    "-" => {
                        // Guion simple solo si está rodeado de espacios
                        let prev_is_space = i > 0 && tokens[i - 1].token_type == TokenType::Whitespace;
                        let next_is_space = i + 1 < tokens.len() && tokens[i + 1].token_type == TokenType::Whitespace;
                        if prev_is_space && next_is_space {
                            return Some((i, '-'));
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    /// Busca la posición del cierre de un inciso
    /// Devuelve la posición del token de cierre, o None si no se encuentra
    fn find_incise_closer(tokens: &[Token], opener_pos: usize, opener_type: char) -> Option<usize> {
        let closer = match opener_type {
            '(' => ')',
            ',' => ',',
            '—' => '—',
            '–' => '–',
            '-' => '-',
            // Comillas
            '«' => '»',
            '"' => '"', // comillas rectas: mismo carácter cierra
            '\u{201C}' => '\u{201D}', // " → " (comillas tipográficas)
            ';' => return None, // punto y coma no tiene cierre claro
            _ => return None,
        };

        // Buscar el cierre después del opener
        for i in (opener_pos + 1)..tokens.len() {
            let text = tokens[i].effective_text();
            if opener_type == '-' {
                // Para guion simple, verificar espacios
                if text == "-" {
                    let prev_is_space = i > 0 && tokens[i - 1].token_type == TokenType::Whitespace;
                    let next_is_space = i + 1 < tokens.len() && tokens[i + 1].token_type == TokenType::Whitespace;
                    if prev_is_space && next_is_space {
                        return Some(i);
                    }
                }
            } else if text.chars().next() == Some(closer) {
                return Some(i);
            }
        }
        None
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::DictionaryLoader;
    use crate::grammar::Tokenizer;
    use std::path::Path;

    fn setup() -> (Trie, ProperNames) {
        let dict_path = Path::new("data/es/words.txt");
        let dictionary = if dict_path.exists() {
            DictionaryLoader::load_from_file(dict_path).unwrap_or_else(|_| Trie::new())
        } else {
            Trie::new()
        };

        let names_path = Path::new("data/names.txt");
        let proper_names = if names_path.exists() {
            ProperNames::load_from_file(names_path).unwrap_or_default()
        } else {
            ProperNames::default()
        };

        (dictionary, proper_names)
    }

    #[test]
    fn test_el_periodista_maria_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista María García informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "el");
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_la_periodista_maria_no_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la periodista María García informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "No debería haber corrección para 'la periodista María'");
    }

    #[test]
    fn test_la_periodista_juan_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la periodista Juan López informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "la");
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("el".to_string()));
    }

    #[test]
    fn test_el_periodista_juan_no_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista Juan López informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "No debería haber corrección para 'el periodista Juan'");
    }

    #[test]
    fn test_common_gender_with_adjective() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista española María García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "María" es femenino y sugerir "la"
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_common_gender_without_referent() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista informó sobre el evento");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Sin referente explícito, no debería haber corrección
        assert!(corrections.is_empty(), "Sin referente no debe haber corrección");
    }

    #[test]
    fn test_el_premio_nobel_maria_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el premio Nobel María Curie");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "premio" es de género común en contextos como "la premio Nobel"
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_la_lider_ana_no_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la líder Ana García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "No debería haber corrección para 'la líder Ana'");
    }

    #[test]
    fn test_un_artista_carmen_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("un artista Carmen López");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "un");
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("una".to_string()));
    }

    #[test]
    fn test_preserves_uppercase() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("El periodista María García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("La".to_string()));
    }

    // ==========================================================================
    // Tests de límites de oración
    // ==========================================================================

    #[test]
    fn test_sentence_boundary_blocks_referent() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // El punto separa "periodista" de "María", no debería encontrar referente
        let tokens = tokenizer.tokenize("El periodista. María llegó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debería haber corrección porque "María" está en otra oración
        assert!(corrections.is_empty(), "No debería cruzar límites de oración: {:?}", corrections);
    }

    #[test]
    fn test_sentence_boundary_with_question_mark() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("¿Quién es el periodista? María lo sabe");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debería haber corrección porque "María" está en otra oración
        assert!(corrections.is_empty(), "No debería cruzar límites de oración con ?: {:?}", corrections);
    }

    // ==========================================================================
    // Test de anulación de corrección gramatical
    // ==========================================================================

    #[test]
    fn test_clear_grammar_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize("la premio Nobel María Curie");

        // Simular que la gramática corrigió "la" a "el" (porque "premio" es masculino en dict)
        tokens[0].corrected_grammar = Some("el".to_string());

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "la" era correcto y pedir anular la corrección
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::ClearCorrection);
    }

    #[test]
    fn test_no_clear_when_grammar_was_right() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el premio Nobel Juan Pérez");

        // Simular que la gramática dejó "el" sin cambiar (correcto)
        // No hay corrected_grammar

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debería haber corrección porque "el" ya es correcto para "Juan"
        assert!(corrections.is_empty(), "No debería haber corrección cuando ya está correcto");
    }

    // ==========================================================================
    // Test de corrección ortográfica previa
    // ==========================================================================

    #[test]
    fn test_spelling_corrected_article_detected() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize("laa premio Nobel María Curie");

        // Simular que ortografía corrigió "laa" → "la"
        tokens[0].corrected_spelling = Some("la".to_string());
        // Y gramática después cambió a "el" (porque premio es masc en dict)
        tokens[0].corrected_grammar = Some("el".to_string());

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "la" (effective) es correcto para "María" y anular
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::ClearCorrection);
    }

    #[test]
    fn test_spelling_corrected_article_wrong_for_referent() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let mut tokens = tokenizer.tokenize("ell periodista María García");

        // Simular que ortografía corrigió "ell" → "el"
        tokens[0].corrected_spelling = Some("el".to_string());

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debería detectar que "el" (effective) no concuerda con "María" y sugerir "la"
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    // ==========================================================================
    // Tests de adjetivo como pista de género (sin nombre propio)
    // ==========================================================================

    #[test]
    fn test_adjective_as_gender_hint_feminine() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "buena" está en diccionario como adjetivo femenino
        let tokens = tokenizer.tokenize("el periodista buena informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "buena" indica género femenino → corregir "el" a "la"
        assert_eq!(corrections.len(), 1, "Debería detectar género por adjetivo: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_adjective_as_gender_hint_masculine() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "nuevo" está en diccionario como adjetivo masculino
        let tokens = tokenizer.tokenize("la periodista nuevo comentó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "nuevo" indica género masculino → corregir "la" a "el"
        assert_eq!(corrections.len(), 1, "Debería detectar género por adjetivo: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("el".to_string()));
    }

    #[test]
    fn test_adjective_correct_no_correction() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la periodista española informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "la" + "española" concuerdan → sin corrección
        assert!(corrections.is_empty(), "No debería corregir cuando concuerdan: {:?}", corrections);
    }

    #[test]
    fn test_invariable_adjective_no_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista inteligente habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "inteligente" es invariable → no hay pista de género → sin corrección
        assert!(corrections.is_empty(), "Adjetivo invariable no debería dar pista: {:?}", corrections);
    }

    #[test]
    fn test_proper_name_overrides_adjective() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Caso raro: adjetivo masculino pero nombre femenino
        // El nombre propio tiene prioridad
        let tokens = tokenizer.tokenize("el periodista famoso María García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "María" tiene prioridad sobre "famoso" → corregir a "la"
        assert_eq!(corrections.len(), 1, "Nombre propio debe tener prioridad: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_multiple_adjectives_first_wins() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el artista talentosa reconocida ganó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Primer adjetivo "talentosa" da la pista de género femenino
        assert_eq!(corrections.len(), 1, "Debería usar primer adjetivo: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_adjective_hint_with_un_una() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("un artista talentosa participó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "talentosa" indica femenino → "un" debería ser "una"
        assert_eq!(corrections.len(), 1, "Debería corregir un→una: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("una".to_string()));
    }

    // ==========================================================================
    // Tests de casos que NO deben usar como pista de género
    // ==========================================================================

    #[test]
    fn test_pronoun_not_used_as_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "la" es pronombre CD, no debe usarse como pista de género
        let tokens = tokenizer.tokenize("el periodista la entrevistó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir "el" basándose en el pronombre "la"
        assert!(corrections.is_empty(), "Pronombre no debe ser pista de género: {:?}", corrections);
    }

    #[test]
    fn test_predicative_adjective_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "cansado" es predicativo (tras verbo copulativo), no inmediato
        let tokens = tokenizer.tokenize("el periodista está cansada");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir "el" basándose en "cansada" (predicativo)
        assert!(corrections.is_empty(), "Predicativo no debe ser pista de género: {:?}", corrections);
    }

    #[test]
    fn test_predicative_with_ser_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista es buena");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir "el" basándose en "buena" (predicativo tras "es")
        assert!(corrections.is_empty(), "Predicativo tras 'ser' no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_noun_apposition_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "estrella" es sustantivo en aposición, no adjetivo
        let tokens = tokenizer.tokenize("el periodista estrella informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir "el" basándose en "estrella" (sustantivo)
        assert!(corrections.is_empty(), "Sustantivo aposicional no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_noun_victima_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "víctima" es sustantivo, no adjetivo
        let tokens = tokenizer.tokenize("el periodista víctima declaró");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir basándose en "víctima"
        assert!(corrections.is_empty(), "Sustantivo 'víctima' no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_degree_adverb_allows_adjective() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "muy buena" - adverbio de grado + adjetivo
        let tokens = tokenizer.tokenize("el periodista muy buena informó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe corregir porque "buena" es adjetivo tras adverbio de grado
        assert_eq!(corrections.len(), 1, "Debería detectar adjetivo tras adverbio de grado: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_degree_adverb_bastante() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la periodista bastante famoso habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "famoso" es masculino, debe corregir "la" a "el"
        assert_eq!(corrections.len(), 1, "Debería detectar adjetivo tras 'bastante': {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("el".to_string()));
    }

    #[test]
    fn test_consecutive_degree_adverbs() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "muy muy buena" - dos adverbios de grado consecutivos
        let tokens = tokenizer.tokenize("el periodista muy muy buena habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe detectar "buena" como pista de género femenino
        assert_eq!(corrections.len(), 1, "Debería detectar adjetivo tras dos adverbios: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_determiner_not_used_as_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "esta" es determinante, no adjetivo
        let tokens = tokenizer.tokenize("el periodista esta mañana habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir basándose en "esta" (determinante)
        assert!(corrections.is_empty(), "Determinante no debe ser pista: {:?}", corrections);
    }

    // ==========================================================================
    // Tests adicionales: incisos, clíticos y casos edge
    // ==========================================================================

    #[test]
    fn test_comma_blocks_adjective_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Coma indica inciso, no debería usar "buena" como pista
        let tokens = tokenizer.tokenize("el periodista, buena persona, habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir porque la coma separa el adjetivo
        assert!(corrections.is_empty(), "Adjetivo tras coma no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_parenthesis_blocks_adjective_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Paréntesis indica inciso
        let tokens = tokenizer.tokenize("el periodista (buena persona) habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Adjetivo en paréntesis no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_em_dash_blocks_adjective_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Guión largo indica inciso
        let tokens = tokenizer.tokenize("el periodista — buena persona — habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Adjetivo tras guión largo no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_hyphen_with_spaces_blocks_adjective() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Guión simple con espacios funciona como inciso
        let tokens = tokenizer.tokenize("el periodista - buena persona - habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Guión con espacios debe bloquear adjetivo: {:?}", corrections);
    }

    #[test]
    fn test_hyphen_in_compound_word_no_block() {
        // Verifica que compuestos con guión se tokenizan juntos
        // y por tanto no generan un token "-" que bloquee
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("ex-buena");

        // El tokenizer debe producir "ex-buena" como un solo token Word
        let word_tokens: Vec<_> = tokens.iter()
            .filter(|t| t.token_type == TokenType::Word)
            .collect();
        assert_eq!(word_tokens.len(), 1, "Compuesto debe ser un solo token: {:?}", word_tokens);
        assert_eq!(word_tokens[0].text, "ex-buena");
    }

    #[test]
    fn test_guillemets_block_adjective_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Comillas angulares « » bloquean adjetivos citados
        let tokens = tokenizer.tokenize("el periodista «buena persona» habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Adjetivo en comillas « » no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_straight_quotes_block_adjective_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Comillas rectas " " bloquean adjetivos citados
        let tokens = tokenizer.tokenize("el periodista \"buena persona\" habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Adjetivo en comillas rectas no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_curly_quotes_block_adjective_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Comillas tipográficas " " bloquean adjetivos citados
        // Usamos \u{201C} y \u{201D} para las comillas tipográficas
        let tokens = tokenizer.tokenize("el periodista \u{201C}buena persona\u{201D} habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Adjetivo en comillas tipográficas no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_semicolon_blocks_adjective_hint() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Punto y coma separa cláusulas
        let tokens = tokenizer.tokenize("el periodista; buena persona habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Adjetivo tras punto y coma no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_clitic_lo_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "lo" es pronombre clítico
        let tokens = tokenizer.tokenize("el periodista lo sabe");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir basándose en "lo"
        assert!(corrections.is_empty(), "Clítico 'lo' no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_clitic_les_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("el periodista les preguntó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Clítico 'les' no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_article_after_noun_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "la" es artículo del siguiente sustantivo
        let tokens = tokenizer.tokenize("el periodista la noticia cubrió");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        assert!(corrections.is_empty(), "Artículo de otro sustantivo no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_adjective_after_two_words_not_used() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "buena" está demasiado lejos (no inmediato)
        let tokens = tokenizer.tokenize("el periodista de Madrid buena");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "de" es preposición, debería cortar la búsqueda de adjetivos
        assert!(corrections.is_empty(), "Adjetivo no inmediato no debe ser pista: {:?}", corrections);
    }

    #[test]
    fn test_name_ending_in_ia_not_confused_with_verb() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "Sofía" termina en "-ía" pero es nombre propio, no verbo
        let tokens = tokenizer.tokenize("el periodista Sofía García habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe corregir "el" a "la" porque "Sofía" es femenino
        assert_eq!(corrections.len(), 1, "Debe detectar Sofía como nombre: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_name_after_comma_still_detected() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Nombre propio tras coma debe seguir detectándose
        let tokens = tokenizer.tokenize("el periodista, María García, habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe corregir porque "María" es nombre femenino (coma no bloquea nombres)
        assert_eq!(corrections.len(), 1, "Nombre tras coma debe detectarse: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_adjective_blocked_but_name_found_after_comma() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // El adjetivo "buena" tras coma no debe usarse, pero "María" sí
        let tokens = tokenizer.tokenize("el periodista, buena persona, María García");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe corregir por "María", no por "buena"
        assert_eq!(corrections.len(), 1, "Nombre tras inciso debe detectarse: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_mas_as_degree_adverb() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "más famosa" - más funciona como adverbio de grado
        let tokens = tokenizer.tokenize("el periodista más buena habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "más buena" indica femenino
        assert_eq!(corrections.len(), 1, "Debería detectar 'más buena': {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_menos_as_degree_adverb() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("la periodista menos nuevo llegó");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "menos nuevo" indica masculino
        assert_eq!(corrections.len(), 1, "Debería detectar 'menos nuevo': {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("el".to_string()));
    }

    #[test]
    fn test_name_after_preposition_incise() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Nombre propio tras inciso con preposición
        let tokens = tokenizer.tokenize("el periodista, de Madrid, María García habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe encontrar "María" a pesar de la preposición "de"
        assert_eq!(corrections.len(), 1, "Nombre tras preposición en inciso: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_name_after_long_incise() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Inciso largo: más de 4 tokens antes del nombre propio
        // Ventana dinámica basada en el cierre del inciso
        let tokens = tokenizer.tokenize("el periodista, de la ciudad de Madrid, María García habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe encontrar "María" a pesar del inciso largo
        assert_eq!(corrections.len(), 1, "Nombre tras inciso largo: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_name_after_very_long_incise() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // Inciso MUY largo: más de 10 tokens (superaría el fallback fijo)
        // La ventana dinámica debe extenderse hasta el cierre ")" + tokens extra
        let tokens = tokenizer.tokenize(
            "el periodista (que como todos sabemos es experto en política internacional desde hace muchos años) María García habló"
        );

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // Debe encontrar "María" gracias a la ventana dinámica basada en cierre de paréntesis
        assert_eq!(corrections.len(), 1, "Nombre tras inciso muy largo: {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_mas_without_accent_before_adjective() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "mas" sin tilde seguido de adjetivo → tratar como adverbio de grado
        // (antes de que diacríticas lo corrija)
        let tokens = tokenizer.tokenize("el periodista mas buena habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "mas buena" indica femenino (mas se trata como adverbio de grado)
        assert_eq!(corrections.len(), 1, "Debería detectar 'mas buena': {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_mas_without_accent_with_degree_adverb() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "mas muy buena" → mas + adverbio de grado + adjetivo
        let tokens = tokenizer.tokenize("el periodista mas muy buena habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // "mas muy buena" indica femenino
        assert_eq!(corrections.len(), 1, "Debería detectar 'mas muy buena': {:?}", corrections);
        assert_eq!(corrections[0].action, CommonGenderAction::Correct("la".to_string()));
    }

    #[test]
    fn test_mas_blocked_by_preposition() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "mas de" → mas seguido de preposición, no es adverbio de grado aquí
        let tokens = tokenizer.tokenize("el periodista mas de Madrid habló");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe corregir - "mas de" no indica género
        assert!(corrections.is_empty(), "'mas de' no debe activar: {:?}", corrections);
    }

    #[test]
    fn test_mas_without_accent_not_before_adjective() {
        let (dictionary, proper_names) = setup();
        let tokenizer = Tokenizer::new();
        // "mas" sin tilde NO seguido de adjetivo → no es adverbio de grado
        let tokens = tokenizer.tokenize("el periodista mas el director hablaron");

        let corrections = CommonGenderAnalyzer::analyze(&tokens, &dictionary, &proper_names);

        // No debe haber corrección de género (mas aquí es conjunción)
        assert!(corrections.is_empty(), "'mas' + no-adjetivo no debe activar: {:?}", corrections);
    }
}
