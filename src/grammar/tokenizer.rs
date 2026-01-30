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
                // Palabra
                let mut end = start + ch.len_utf8();
                let mut word = String::from(ch);

                while let Some(&(_, next_ch)) = chars.peek() {
                    if next_ch.is_alphabetic() || next_ch == '\'' || next_ch == '-' {
                        word.push(next_ch);
                        end += next_ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }

                Token::new(word, TokenType::Word, start, end)
            } else if ch.is_numeric() {
                // Número
                let mut end = start + ch.len_utf8();
                let mut number = String::from(ch);

                while let Some(&(_, next_ch)) = chars.peek() {
                    if next_ch.is_numeric() || next_ch == '.' || next_ch == ',' {
                        number.push(next_ch);
                        end += next_ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }

                Token::new(number, TokenType::Number, start, end)
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
}
