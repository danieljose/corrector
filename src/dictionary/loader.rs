//! Cargador de diccionarios desde archivos

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::trie::{Gender, Number, Trie, WordCategory, WordInfo};

pub struct DictionaryLoader;

impl DictionaryLoader {
    /// Carga un diccionario desde un archivo
    ///
    /// Formato esperado: palabra|categoría|género|número|extra
    /// Ejemplo: casa|sustantivo|f|s|
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Trie, String> {
        let file = File::open(path.as_ref())
            .map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;

        let reader = BufReader::new(file);
        let mut trie = Trie::new();
        let mut line_num = 0;

        for line_result in reader.lines() {
            line_num += 1;
            let line = line_result
                .map_err(|e| format!("Error leyendo línea {}: {}", line_num, e))?;

            let line = line.trim();

            // Ignorar líneas vacías y comentarios
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parsear línea
            let parts: Vec<&str> = line.split('|').collect();

            if parts.is_empty() {
                continue;
            }

            let word = parts[0].trim();
            if word.is_empty() {
                continue;
            }

            let info = if parts.len() >= 5 {
                WordInfo {
                    category: WordCategory::from_str(parts.get(1).unwrap_or(&"")),
                    gender: Gender::from_str(parts.get(2).unwrap_or(&"")),
                    number: Number::from_str(parts.get(3).unwrap_or(&"")),
                    extra: parts.get(4).unwrap_or(&"").to_string(),
                    frequency: parts
                        .get(5)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1),
                }
            } else if parts.len() >= 2 {
                // Formato simplificado: palabra|categoría
                WordInfo {
                    category: WordCategory::from_str(parts[1]),
                    gender: Gender::from_str(parts.get(2).unwrap_or(&"")),
                    number: Number::from_str(parts.get(3).unwrap_or(&"")),
                    extra: String::new(),
                    frequency: 1,
                }
            } else {
                // Solo palabra
                WordInfo::default()
            };

            trie.insert(word, info);
        }

        Ok(trie)
    }

    /// Carga un diccionario simple (una palabra por línea)
    pub fn load_simple<P: AsRef<Path>>(path: P) -> Result<Trie, String> {
        let file = File::open(path.as_ref())
            .map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;

        let reader = BufReader::new(file);
        let mut trie = Trie::new();

        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("Error leyendo: {}", e))?;
            let word = line.trim();

            if !word.is_empty() && !word.starts_with('#') {
                trie.insert_word(word);
            }
        }

        Ok(trie)
    }

    /// Combina múltiples tries en uno
    pub fn merge(tries: Vec<Trie>) -> Trie {
        let mut result = Trie::new();

        for trie in tries {
            for (word, info) in trie.get_all_words() {
                result.insert(&word, info);
            }
        }

        result
    }

    /// Añade palabras de un archivo a un trie existente
    pub fn append_from_file<P: AsRef<Path>>(trie: &mut Trie, path: P) -> Result<usize, String> {
        let file = File::open(path.as_ref())
            .map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;

        let reader = BufReader::new(file);
        let mut count = 0;

        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("Error leyendo: {}", e))?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            let word = parts[0].trim();

            if word.is_empty() {
                continue;
            }

            let info = if parts.len() >= 5 {
                WordInfo {
                    category: WordCategory::from_str(parts.get(1).unwrap_or(&"")),
                    gender: Gender::from_str(parts.get(2).unwrap_or(&"")),
                    number: Number::from_str(parts.get(3).unwrap_or(&"")),
                    extra: parts.get(4).unwrap_or(&"").to_string(),
                    frequency: parts
                        .get(5)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1),
                }
            } else {
                WordInfo::default()
            };

            trie.insert(word, info);
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::fs;

    #[test]
    fn test_load_simple() {
        let test_file = "test_dict_simple.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "hola").unwrap();
        writeln!(file, "mundo").unwrap();
        writeln!(file, "# comentario").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "prueba").unwrap();
        drop(file);

        let trie = DictionaryLoader::load_simple(test_file).unwrap();
        assert!(trie.contains("hola"));
        assert!(trie.contains("mundo"));
        assert!(trie.contains("prueba"));
        assert!(!trie.contains("comentario"));

        fs::remove_file(test_file).unwrap();
    }
}
