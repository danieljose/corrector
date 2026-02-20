//! Tokenizador de texto

use crate::dictionary::WordInfo;

/// Tipo de token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Word,
    Punctuation,
    Whitespace,
    Number,
    Unknown,
}

/// Token individual
#[derive(Debug, Clone)]
pub struct Token {
    pub text: String,
    pub token_type: TokenType,
    pub start: usize,
    pub end: usize,
    pub word_info: Option<WordInfo>,
    pub corrected_spelling: Option<String>,
    pub corrected_grammar: Option<String>,
    /// Indica que la palabra debe eliminarse (tachada en la salida)
    pub strikethrough: bool,
}

impl Token {
    pub fn new(text: String, token_type: TokenType, start: usize, end: usize) -> Self {
        Self {
            text,
            token_type,
            start,
            end,
            word_info: None,
            corrected_spelling: None,
            corrected_grammar: None,
            strikethrough: false,
        }
    }

    pub fn is_word(&self) -> bool {
        self.token_type == TokenType::Word
    }

    pub fn lowercase(&self) -> String {
        self.text.to_lowercase()
    }

    /// Devuelve el texto efectivo del token para análisis en fases posteriores.
    /// Prioriza: corrección gramatical > corrección ortográfica > texto original.
    /// Filtra correcciones que son mensajes (no texto válido).
    pub fn effective_text(&self) -> &str {
        // Primero intentar corrección gramatical
        if let Some(ref correction) = self.corrected_grammar {
            // Filtrar mensajes que no son texto de reemplazo válido
            if !correction.starts_with("falta")
                && !correction.starts_with("sobra")
                && correction != "desbalanceado"
                && correction != "?"
            {
                // Si hay múltiples opciones separadas por coma, usar la primera
                return correction.split(',').next().unwrap_or(correction);
            }
        }
        // Luego intentar corrección ortográfica
        if let Some(ref correction) = self.corrected_spelling {
            if correction != "?" {
                return correction.split(',').next().unwrap_or(correction);
            }
        }
        // Por defecto, texto original
        &self.text
    }

    /// Verifica si este token es un límite de oración.
    /// Signos reconocidos: . ! ? ; : " " » (y puntos suspensivos ... o …),
    /// además de saltos de línea explícitos.
    pub fn is_sentence_boundary(&self) -> bool {
        if self.token_type == TokenType::Whitespace
            && (self.text.contains('\n') || self.text.contains('\r'))
        {
            return true;
        }
        if self.token_type == TokenType::Punctuation {
            return matches!(
                self.text.as_str(),
                "."
                    | "!"
                    | "?"
                    | ";"
                    | ":"
                    | "..."
                    | "\u{2026}"
                    | "\""
                    | "\u{201C}"
                    | "\u{201D}"
                    | "\u{00AB}"
                    | "\u{00BB}"
            );
        }
        false
    }
}

/// Verifica si hay un signo de límite de oración entre dos índices de tokens.
/// Útil para que los analizadores eviten cruzar límites de oración.
pub struct SentenceBoundaryIndex {
    prefix: Vec<usize>,
}

impl SentenceBoundaryIndex {
    pub fn new(tokens: &[Token]) -> Self {
        let mut prefix = Vec::with_capacity(tokens.len() + 1);
        prefix.push(0);
        for token in tokens {
            let next =
                prefix.last().copied().unwrap_or(0) + usize::from(token.is_sentence_boundary());
            prefix.push(next);
        }
        Self { prefix }
    }

    #[inline]
    pub fn has_between(&self, start_idx: usize, end_idx: usize) -> bool {
        let (start, end) = if start_idx < end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };
        if end <= start + 1 {
            return false;
        }
        self.prefix[end] > self.prefix[start + 1]
    }
}

pub fn has_sentence_boundary(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
    let (start, end) = if start_idx < end_idx {
        (start_idx, end_idx)
    } else {
        (end_idx, start_idx)
    };

    for i in (start + 1)..end {
        if tokens[i].is_sentence_boundary() {
            return true;
        }
    }
    false
}

/// Tokenizador de texto
pub struct Tokenizer {
    /// Caracteres internos de palabra específicos del idioma
    /// (ej: · en catalán para col·legi, intel·ligent)
    word_internal_chars: Vec<char>,
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            word_internal_chars: Vec::new(),
        }
    }

    /// Configura caracteres internos de palabra específicos del idioma
    pub fn with_word_internal_chars(mut self, chars: &[char]) -> Self {
        self.word_internal_chars = chars.to_vec();
        self
    }

    /// Tokeniza un texto en tokens individuales
    pub fn tokenize(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = text.char_indices().peekable();

        while let Some((start, ch)) = chars.next() {
            // Filtrar prefijos mojibake frecuentes (p.ej. "Â¿", "Ã‚Â¿", "Â¡")
            // para que no generen tokens espurios antes de signos invertidos.
            if is_mojibake_prefix_artifact(ch) {
                if let Some(&(_, next_ch)) = chars.peek() {
                    if is_mojibake_follow_artifact(next_ch) {
                        continue;
                    }
                }
            }

            // Detectar emails como token atómico: "juan@gmail.com"
            if ch.is_ascii_alphanumeric() {
                if let Some(email_end) = Self::scan_email_end(text, start) {
                    while let Some(&(next_idx, _)) = chars.peek() {
                        if next_idx < email_end {
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::new(
                        text[start..email_end].to_string(),
                        TokenType::Word,
                        start,
                        email_end,
                    ));
                    continue;
                }
            }

            let token = if ch.is_alphabetic() {
                // Palabra o término alfanumérico (ej: USB2.0, MP3, B2B)
                let mut end = start + ch.len_utf8();
                let mut word = String::from(ch);

                while let Some(&(_, next_ch)) = chars.peek() {
                    if next_ch.is_alphanumeric() {
                        word.push(next_ch);
                        end += next_ch.len_utf8();
                        chars.next();
                    } else if next_ch == '-' {
                        // Solo incluir guión si va seguido de letra/dígito (palabra compuesta)
                        // "Madrid-Sevilla" → incluir, "Lucifer-" → no incluir
                        let mut lookahead = chars.clone();
                        lookahead.next(); // saltar el guión
                        match lookahead.peek() {
                            Some(&(_, c)) if c.is_alphanumeric() => {
                                word.push(next_ch);
                                end += next_ch.len_utf8();
                                chars.next();
                            }
                            _ => break, // Guión final o seguido de espacio/puntuación
                        }
                    } else if next_ch == '\'' {
                        // Solo incluir apóstrofo si va seguido de letra (contracción: l'eau)
                        let mut lookahead = chars.clone();
                        lookahead.next(); // saltar el apóstrofo
                        match lookahead.peek() {
                            Some(&(_, c)) if c.is_alphabetic() => {
                                word.push(next_ch);
                                end += next_ch.len_utf8();
                                chars.next();
                            }
                            _ => break, // Apóstrofo final, no incluir
                        }
                    } else if self.word_internal_chars.contains(&next_ch) {
                        // Carácter interno de palabra específico del idioma (ej: · catalán)
                        let mut lookahead = chars.clone();
                        lookahead.next();
                        match lookahead.peek() {
                            Some(&(_, c)) if c.is_alphabetic() => {
                                word.push(next_ch);
                                end += next_ch.len_utf8();
                                chars.next();
                            }
                            _ => break,
                        }
                    } else if next_ch == '.' {
                        // Verificar si es abreviatura de número: N.º
                        let mut lookahead = chars.clone();
                        lookahead.next(); // saltar el punto
                        if let Some(&(_, after_dot)) = lookahead.peek() {
                            if after_dot == 'º' || after_dot == 'ª' {
                                // Abreviatura: N.º, n.º
                                word.push(next_ch); // añadir el punto
                                end += next_ch.len_utf8();
                                chars.next();
                                word.push(after_dot); // añadir º/ª
                                end += after_dot.len_utf8();
                                chars.next();
                                break; // Terminar el token
                            }
                        }
                        break;
                    } else if next_ch == '^' {
                        // Incluir ^ para exponentes ASCII (m^2, s^-1) si va seguido de dígito o -
                        let mut lookahead = chars.clone();
                        lookahead.next();
                        match lookahead.peek() {
                            Some(&(_, c)) if c.is_ascii_digit() || c == '-' => {
                                word.push(next_ch);
                                end += next_ch.len_utf8();
                                chars.next();
                            }
                            _ => break,
                        }
                    } else {
                        break;
                    }
                }

                Token::new(word, TokenType::Word, start, end)
            } else if ch.is_numeric() {
                // Número o término alfanumérico (ej: 6K, 4K, M4)
                let mut end = start + ch.len_utf8();
                let mut text = String::from(ch);
                let mut has_letters = false;
                let mut is_ordinal = false;

                while let Some(&(_, next_ch)) = chars.peek() {
                    if next_ch.is_alphanumeric() {
                        // Letras y dígitos siempre se incluyen
                        if next_ch.is_alphabetic() {
                            has_letters = true;
                        }
                        text.push(next_ch);
                        end += next_ch.len_utf8();
                        chars.next();
                    } else if next_ch == '-' {
                        // Solo incluir guión si va seguido de letra/dígito
                        let mut lookahead = chars.clone();
                        lookahead.next();
                        match lookahead.peek() {
                            Some(&(_, c)) if c.is_alphanumeric() => {
                                text.push(next_ch);
                                end += next_ch.len_utf8();
                                chars.next();
                            }
                            _ => break,
                        }
                    } else if next_ch == '^' {
                        // Incluir ^ para exponentes ASCII (m^2, s^-1) si va seguido de dígito o -
                        let mut lookahead = chars.clone();
                        lookahead.next();
                        match lookahead.peek() {
                            Some(&(_, c)) if c.is_ascii_digit() || c == '-' => {
                                text.push(next_ch);
                                end += next_ch.len_utf8();
                                chars.next();
                            }
                            _ => break,
                        }
                    } else if (next_ch == '.' || next_ch == ',') && !has_letters {
                        // Punto o coma solo para números decimales (cuando aún no hay letras)
                        // O para ordinales (número.º, número.ª)
                        let mut lookahead = chars.clone();
                        lookahead.next(); // saltar el punto/coma
                        if let Some(&(_, after_dot)) = lookahead.peek() {
                            if after_dot.is_numeric() {
                                // Número decimal
                                text.push(next_ch);
                                end += next_ch.len_utf8();
                                chars.next();
                            } else if next_ch == '.' && (after_dot == 'º' || after_dot == 'ª') {
                                // Ordinal: 20.º, 75.ª
                                text.push(next_ch); // añadir el punto
                                end += next_ch.len_utf8();
                                chars.next();
                                text.push(after_dot); // añadir º/ª
                                end += after_dot.len_utf8();
                                chars.next();
                                is_ordinal = true;
                                break; // Terminar el token ordinal
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Si contiene letras, es un término alfanumérico (Word), sino un número
                // Los ordinales se tratan como números
                let token_type = if has_letters && !is_ordinal {
                    TokenType::Word
                } else {
                    TokenType::Number
                };
                Token::new(text, token_type, start, end)
            } else if ch.is_whitespace() {
                // Espacio en blanco
                let mut end = start + ch.len_utf8();
                let mut whitespace = String::from(ch);

                while let Some(&(_, next_ch)) = chars.peek() {
                    if next_ch.is_whitespace() {
                        whitespace.push(next_ch);
                        end += next_ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }

                Token::new(whitespace, TokenType::Whitespace, start, end)
            } else if is_punctuation(ch) {
                // Puntuación - agrupar "..." en un solo token
                if ch == '.' {
                    let mut end = start + ch.len_utf8();
                    let mut dots = String::from(ch);

                    while let Some(&(_, next_ch)) = chars.peek() {
                        if next_ch == '.' {
                            dots.push(next_ch);
                            end += next_ch.len_utf8();
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    Token::new(dots, TokenType::Punctuation, start, end)
                } else {
                    Token::new(
                        ch.to_string(),
                        TokenType::Punctuation,
                        start,
                        start + ch.len_utf8(),
                    )
                }
            } else {
                // Desconocido
                Token::new(
                    ch.to_string(),
                    TokenType::Unknown,
                    start,
                    start + ch.len_utf8(),
                )
            };

            tokens.push(token);
        }

        tokens
    }

    fn is_email_local_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '%' | '+' | '-')
    }

    fn is_valid_email_domain(domain: &str) -> bool {
        let mut labels = domain.split('.');
        let Some(first_label) = labels.next() else {
            return false;
        };
        if first_label.is_empty()
            || first_label.starts_with('-')
            || first_label.ends_with('-')
            || !first_label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return false;
        }

        let mut saw_dot = false;
        let mut last_label = first_label;
        for label in labels {
            saw_dot = true;
            if label.is_empty()
                || label.starts_with('-')
                || label.ends_with('-')
                || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return false;
            }
            last_label = label;
        }

        saw_dot && last_label.len() >= 2
    }

    fn scan_email_end(text: &str, start: usize) -> Option<usize> {
        let slice = text.get(start..)?;

        let mut at_offset: Option<usize> = None;
        let mut local_len = 0usize;
        let mut last_local_char = '\0';

        for (offset, ch) in slice.char_indices() {
            if ch == '@' {
                if local_len == 0 || last_local_char == '.' {
                    return None;
                }
                at_offset = Some(offset);
                break;
            }
            if Self::is_email_local_char(ch) {
                local_len += 1;
                last_local_char = ch;
            } else {
                return None;
            }
        }

        let at = at_offset?;
        let domain_slice = slice.get((at + 1)..)?;
        let mut domain_end = 0usize;
        for (offset, ch) in domain_slice.char_indices() {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
                domain_end = offset + ch.len_utf8();
            } else {
                break;
            }
        }

        if domain_end == 0 {
            return None;
        }
        let domain = domain_slice.get(..domain_end)?;
        if !Self::is_valid_email_domain(domain) {
            return None;
        }

        Some(start + at + 1 + domain_end)
    }

    /// Reconstruye texto desde tokens
    pub fn reconstruct(&self, tokens: &[Token]) -> String {
        tokens.iter().map(|t| t.text.as_str()).collect()
    }

    /// Obtiene solo los tokens de tipo palabra
    pub fn get_words<'a>(&self, tokens: &'a [Token]) -> Vec<&'a Token> {
        tokens.iter().filter(|t| t.is_word()).collect()
    }
}

fn is_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '.' | ','
            | ';'
            | ':'
            | '!'
            | '?'
            | '¡'
            | '¿'
            | '"'
            | '\''
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '-'
            | '—'
            | '–'
            | '«'
            | '»'
            | '…'
            | '/'
            | '°'
    )
}

fn is_mojibake_prefix_artifact(ch: char) -> bool {
    matches!(
        ch,
        '\u{00C2}' // Â
            | '\u{00C3}' // Ã
            | '\u{0082}' // control byte rendered in mojibake chains
            | '\u{201A}' // ‚
            | '\u{00EF}' // ï (full-width punctuation mojibake)
            | '\u{00BC}' // ¼
            | '\u{009F}' // control byte for ï¼Ÿ/ï¼ chains
            | '\u{0081}'
    )
}

fn is_mojibake_follow_artifact(ch: char) -> bool {
    matches!(
        ch,
        '\u{00BF}' // ¿
            | '\u{00A1}' // ¡
            | '\u{00B0}' // °
            | '\u{00BA}' // º
            | '\u{00AA}' // ª
            | '\u{00AB}' // «
            | '\u{00BB}' // »
            | '\u{FF1F}' // ？
            | '\u{FF01}' // ！
            | '\u{00C2}'
            | '\u{00C3}'
            | '\u{0082}'
            | '\u{201A}'
            | '\u{00EF}'
            | '\u{00BC}'
            | '\u{009F}'
            | '\u{0081}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hola mundo");

        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "Hola");
        assert_eq!(tokens[0].token_type, TokenType::Word);
        assert_eq!(tokens[1].token_type, TokenType::Whitespace);
        assert_eq!(tokens[2].text, "mundo");
    }

    #[test]
    fn test_tokenize_punctuation() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("¡Hola, mundo!");

        assert_eq!(tokens[0].text, "¡");
        assert_eq!(tokens[0].token_type, TokenType::Punctuation);
        assert_eq!(tokens[1].text, "Hola");
        assert_eq!(tokens[2].text, ",");
        assert_eq!(tokens[2].token_type, TokenType::Punctuation);
    }

    #[test]
    fn test_tokenize_numbers() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Tengo 42 años");

        assert_eq!(tokens[2].text, "42");
        assert_eq!(tokens[2].token_type, TokenType::Number);
    }

    #[test]
    fn test_reconstruct() {
        let tokenizer = Tokenizer::new();
        let original = "¡Hola, mundo!";
        let tokens = tokenizer.tokenize(original);
        let reconstructed = tokenizer.reconstruct(&tokens);

        assert_eq!(original, reconstructed);
    }

    #[test]
    fn test_positions() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("ab cd");

        assert_eq!(tokens[0].start, 0);
        assert_eq!(tokens[0].end, 2);
        assert_eq!(tokens[1].start, 2);
        assert_eq!(tokens[1].end, 3);
        assert_eq!(tokens[2].start, 3);
        assert_eq!(tokens[2].end, 5);
    }

    #[test]
    fn test_trailing_quote_not_included_in_word() {
        let tokenizer = Tokenizer::new();
        // Comilla simple al final de palabra no debe incluirse
        let tokens = tokenizer.tokenize("mundo' es");

        assert_eq!(tokens[0].text, "mundo");
        assert_eq!(tokens[0].token_type, TokenType::Word);
        assert_eq!(tokens[1].text, "'");
        assert_eq!(tokens[1].token_type, TokenType::Punctuation);
    }

    #[test]
    fn test_mid_word_apostrophe_included() {
        let tokenizer = Tokenizer::new();
        // Apóstrofo seguido de letra (contracción) sí se incluye
        let tokens = tokenizer.tokenize("l'eau");

        assert_eq!(tokens[0].text, "l'eau");
        assert_eq!(tokens[0].token_type, TokenType::Word);
    }

    #[test]
    fn test_quoted_word() {
        let tokenizer = Tokenizer::new();
        // Palabra entre comillas simples
        let tokens = tokenizer.tokenize("'hola'");

        assert_eq!(tokens[0].text, "'");
        assert_eq!(tokens[0].token_type, TokenType::Punctuation);
        assert_eq!(tokens[1].text, "hola");
        assert_eq!(tokens[1].token_type, TokenType::Word);
        assert_eq!(tokens[2].text, "'");
        assert_eq!(tokens[2].token_type, TokenType::Punctuation);
    }

    #[test]
    fn test_alphanumeric_tokens() {
        let tokenizer = Tokenizer::new();

        // Términos alfanuméricos como resoluciones de pantalla
        let tokens = tokenizer.tokenize("6K 4K 8K");
        assert_eq!(tokens[0].text, "6K");
        assert_eq!(tokens[0].token_type, TokenType::Word);
        assert_eq!(tokens[2].text, "4K");
        assert_eq!(tokens[2].token_type, TokenType::Word);
        assert_eq!(tokens[4].text, "8K");
        assert_eq!(tokens[4].token_type, TokenType::Word);

        // Otros términos alfanuméricos
        let tokens = tokenizer.tokenize("MP3 USB2 B2B");
        assert_eq!(tokens[0].text, "MP3");
        assert_eq!(tokens[0].token_type, TokenType::Word);
        assert_eq!(tokens[2].text, "USB2");
        assert_eq!(tokens[2].token_type, TokenType::Word);
        assert_eq!(tokens[4].text, "B2B");
        assert_eq!(tokens[4].token_type, TokenType::Word);

        // Números puros siguen siendo números
        let tokens = tokenizer.tokenize("42 3.14");
        assert_eq!(tokens[0].text, "42");
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert_eq!(tokens[2].text, "3.14");
        assert_eq!(tokens[2].token_type, TokenType::Number);
    }

    #[test]
    fn test_ordinal_numbers() {
        let tokenizer = Tokenizer::new();

        // Ordinales masculinos
        let tokens = tokenizer.tokenize("el 20.º presidente");
        assert_eq!(tokens[2].text, "20.º");
        assert_eq!(tokens[2].token_type, TokenType::Number);

        // Ordinales femeninos
        let tokens = tokenizer.tokenize("la 75.ª edición");
        assert_eq!(tokens[2].text, "75.ª");
        assert_eq!(tokens[2].token_type, TokenType::Number);

        // Abreviatura N.º (número)
        let tokens = tokenizer.tokenize("Ley N.º 26.571");
        assert_eq!(tokens[2].text, "N.º");
        assert_eq!(tokens[2].token_type, TokenType::Word);
    }

    #[test]
    fn test_trailing_hyphen_separate() {
        let tokenizer = Tokenizer::new();

        // Guión final debe ser token separado (puntuación)
        let tokens = tokenizer.tokenize("rima con Lucifer-");
        let word_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word)
            .collect();
        let punct_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Punctuation)
            .collect();

        assert!(
            word_tokens.iter().any(|t| t.text == "Lucifer"),
            "Debe tener 'Lucifer' como palabra"
        );
        assert!(
            punct_tokens.iter().any(|t| t.text == "-"),
            "Debe tener '-' como puntuación"
        );
    }

    #[test]
    fn test_compound_word_hyphen_included() {
        let tokenizer = Tokenizer::new();

        // Palabra compuesta: guión incluido en el token
        let tokens = tokenizer.tokenize("Madrid-Sevilla");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "Madrid-Sevilla");
        assert_eq!(tokens[0].token_type, TokenType::Word);
    }

    #[test]
    fn test_double_hyphen_separate() {
        let tokenizer = Tokenizer::new();

        // Doble guión: tokens separados
        let tokens = tokenizer.tokenize("Madrid--Sevilla");
        let word_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Word)
            .collect();
        let punct_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Punctuation)
            .collect();

        assert_eq!(word_tokens.len(), 2, "Debe tener 2 palabras");
        assert!(word_tokens.iter().any(|t| t.text == "Madrid"));
        assert!(word_tokens.iter().any(|t| t.text == "Sevilla"));
        assert_eq!(
            punct_tokens.len(),
            2,
            "Debe tener 2 guiones como puntuación"
        );
    }

    // ==========================================================================
    // Tests de límites de oración
    // ==========================================================================

    #[test]
    fn test_sentence_boundary_period() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hola. Mundo");
        assert!(has_sentence_boundary(&tokens, 0, 4)); // "Hola" a "Mundo"
    }

    #[test]
    fn test_sentence_boundary_semicolon() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hola; mundo");
        assert!(has_sentence_boundary(&tokens, 0, 4)); // "Hola" a "mundo"
    }

    #[test]
    fn test_sentence_boundary_colon() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Dijo: hola");
        assert!(has_sentence_boundary(&tokens, 0, 4)); // "Dijo" a "hola"
    }

    #[test]
    fn test_sentence_boundary_quotes() {
        let tokenizer = Tokenizer::new();
        // Comilla de cierre tipográfica
        let tokens = tokenizer.tokenize("dijo \"hola\" luego");
        // Buscar índices de "dijo" y "luego"
        let dijo_idx = tokens.iter().position(|t| t.text == "dijo").unwrap();
        let luego_idx = tokens.iter().position(|t| t.text == "luego").unwrap();
        assert!(has_sentence_boundary(&tokens, dijo_idx, luego_idx));
    }

    #[test]
    fn test_sentence_boundary_guillemets() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("dijo «hola» luego");
        let dijo_idx = tokens.iter().position(|t| t.text == "dijo").unwrap();
        let luego_idx = tokens.iter().position(|t| t.text == "luego").unwrap();
        assert!(has_sentence_boundary(&tokens, dijo_idx, luego_idx));
    }

    #[test]
    fn test_no_sentence_boundary_comma() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hola, mundo");
        assert!(!has_sentence_boundary(&tokens, 0, 4)); // coma NO es límite
    }

    #[test]
    fn test_mojibake_inverted_question_prefix_is_ignored() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Â¿y que quieres?");
        let texts: Vec<&str> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert!(!texts.contains(&"Â"), "No debe generar token espurio 'Â': {:?}", texts);
        assert!(
            texts.contains(&"¿"),
            "Debe conservar el signo invertido de apertura: {:?}",
            texts
        );
    }

    #[test]
    fn test_token_is_sentence_boundary() {
        let period = Token::new(".".to_string(), TokenType::Punctuation, 0, 1);
        let comma = Token::new(",".to_string(), TokenType::Punctuation, 0, 1);
        let semicolon = Token::new(";".to_string(), TokenType::Punctuation, 0, 1);
        let colon = Token::new(":".to_string(), TokenType::Punctuation, 0, 1);
        let ellipsis_ascii = Token::new("...".to_string(), TokenType::Punctuation, 0, 3);
        let ellipsis_unicode = Token::new("…".to_string(), TokenType::Punctuation, 0, 1);
        let guillemet_open = Token::new("«".to_string(), TokenType::Punctuation, 0, 1);
        let guillemet_close = Token::new("»".to_string(), TokenType::Punctuation, 0, 1);
        let newline_ws = Token::new("\n".to_string(), TokenType::Whitespace, 0, 1);
        let word = Token::new("hola".to_string(), TokenType::Word, 0, 4);

        assert!(period.is_sentence_boundary());
        assert!(!comma.is_sentence_boundary());
        assert!(semicolon.is_sentence_boundary());
        assert!(colon.is_sentence_boundary());
        assert!(ellipsis_ascii.is_sentence_boundary());
        assert!(ellipsis_unicode.is_sentence_boundary());
        assert!(guillemet_open.is_sentence_boundary());
        assert!(guillemet_close.is_sentence_boundary());
        assert!(newline_ws.is_sentence_boundary());
        assert!(!word.is_sentence_boundary());
    }

    #[test]
    fn test_has_sentence_boundary_across_newline() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("hola\nmundo");
        let hola_idx = tokens.iter().position(|t| t.text == "hola").unwrap();
        let mundo_idx = tokens.iter().position(|t| t.text == "mundo").unwrap();
        assert!(
            has_sentence_boundary(&tokens, hola_idx, mundo_idx),
            "El salto de línea debe cortar contexto entre oraciones"
        );
    }

    #[test]
    fn test_tokenize_exponent_caret() {
        let tokenizer = Tokenizer::new();

        // "100m^2/s" debe tokenizarse como ["100m^2", "/", "s"]
        let tokens = tokenizer.tokenize("100m^2/s");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "100m^2");
        assert_eq!(tokens[0].token_type, TokenType::Word);
        assert_eq!(tokens[1].text, "/");
        assert_eq!(tokens[2].text, "s");

        // "m^-1" exponente negativo
        let tokens = tokenizer.tokenize("m^-1");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "m^-1");

        // "10s^-2" exponente negativo con número
        let tokens = tokenizer.tokenize("10s^-2");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "10s^-2");

        // "m/s^2" con barra
        let tokens = tokenizer.tokenize("m/s^2");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].text, "m");
        assert_eq!(tokens[1].text, "/");
        assert_eq!(tokens[2].text, "s^2");
    }

    #[test]
    fn test_tokenize_email_as_single_word() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Escribeme a juan@gmail.com, por favor.");
        assert!(
            tokens
                .iter()
                .any(|t| t.token_type == TokenType::Word && t.text == "juan@gmail.com"),
            "Debe tokenizar el email como una sola palabra: {:?}",
            tokens.iter().map(|t| t.text.as_str()).collect::<Vec<_>>()
        );
        assert!(
            !tokens.iter().any(|t| t.text == "@"),
            "No debe separar '@' como token independiente"
        );
    }
}
