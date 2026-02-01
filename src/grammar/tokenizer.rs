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
}

/// Tokenizador de texto
pub struct Tokenizer;

impl Tokenizer {
    pub fn new() -> Self {
        Self
    }

    /// Tokeniza un texto en tokens individuales
    pub fn tokenize(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = text.char_indices().peekable();

        while let Some((start, ch)) = chars.next() {
            let token = if ch.is_alphabetic() {
                // Palabra o término alfanumérico (ej: USB2.0, MP3, B2B)
                let mut end = start + ch.len_utf8();
                let mut word = String::from(ch);

                while let Some(&(_, next_ch)) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '-' {
                        word.push(next_ch);
                        end += next_ch.len_utf8();
                        chars.next();
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
                    if next_ch.is_alphanumeric() || next_ch == '-' {
                        // Letras y dígitos siempre se incluyen
                        if next_ch.is_alphabetic() {
                            has_letters = true;
                        }
                        text.push(next_ch);
                        end += next_ch.len_utf8();
                        chars.next();
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
        '.' | ',' | ';' | ':' | '!' | '?' | '¡' | '¿' | '"' | '\'' | '(' | ')' | '[' | ']'
            | '{' | '}' | '-' | '—' | '–' | '«' | '»' | '…'
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
}
