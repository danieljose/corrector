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
        let file =
            File::open(path.as_ref()).map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;

        let reader = BufReader::new(file);
        let mut trie = Trie::new();
        let mut line_num = 0;

        for line_result in reader.lines() {
            line_num += 1;
            let line =
                line_result.map_err(|e| format!("Error leyendo línea {}: {}", line_num, e))?;

            let line = line.trim();

            // Ignorar líneas vacías y comentarios
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parsear línea
            let mut parts = line.split('|');
            let word = parts.next().unwrap_or("").trim();
            if word.is_empty() {
                continue;
            }

            let category = parts.next();
            let gender = parts.next();
            let number = parts.next();
            let extra = parts.next();
            let frequency = parts.next();

            let info = if extra.is_some() {
                WordInfo {
                    category: WordCategory::from_str(category.unwrap_or("")),
                    gender: Gender::from_str(gender.unwrap_or("")),
                    number: Number::from_str(number.unwrap_or("")),
                    extra: extra.unwrap_or("").to_string(),
                    frequency: frequency.and_then(|s| s.parse().ok()).unwrap_or(1),
                }
            } else if category.is_some() {
                // Formato simplificado: palabra|categoría
                WordInfo {
                    category: WordCategory::from_str(category.unwrap_or("")),
                    gender: Gender::from_str(gender.unwrap_or("")),
                    number: Number::from_str(number.unwrap_or("")),
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
        let file =
            File::open(path.as_ref()).map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;

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
        let file =
            File::open(path.as_ref()).map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;

        let reader = BufReader::new(file);
        let mut count = 0;

        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("Error leyendo: {}", e))?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.split('|');
            let word = parts.next().unwrap_or("").trim();

            if word.is_empty() {
                continue;
            }

            let category = parts.next();
            let gender = parts.next();
            let number = parts.next();
            let extra = parts.next();
            let frequency = parts.next();

            let info = if extra.is_some() {
                WordInfo {
                    category: WordCategory::from_str(category.unwrap_or("")),
                    gender: Gender::from_str(gender.unwrap_or("")),
                    number: Number::from_str(number.unwrap_or("")),
                    extra: extra.unwrap_or("").to_string(),
                    frequency: frequency.and_then(|s| s.parse().ok()).unwrap_or(1),
                }
            } else if category.is_some() {
                WordInfo {
                    category: WordCategory::from_str(category.unwrap_or("")),
                    gender: Gender::from_str(gender.unwrap_or("")),
                    number: Number::from_str(number.unwrap_or("")),
                    extra: String::new(),
                    frequency: 1,
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
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_file(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("corrector_{name}_{nanos}.txt"));
        path
    }

    #[test]
    fn test_load_simple() {
        let test_file = temp_test_file("dict_simple");
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "hola").unwrap();
        writeln!(file, "mundo").unwrap();
        writeln!(file, "# comentario").unwrap();
        writeln!(file, "").unwrap();
        writeln!(file, "prueba").unwrap();
        drop(file);

        let trie = DictionaryLoader::load_simple(&test_file).unwrap();
        assert!(trie.contains("hola"));
        assert!(trie.contains("mundo"));
        assert!(trie.contains("prueba"));
        assert!(!trie.contains("comentario"));

        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_load_numeric_prefix_words() {
        // Create a test file with numeric prefix words
        let test_file = temp_test_file("numeric_words");
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "6K|sustantivo|m|s||200").unwrap();
        writeln!(file, "4K|sustantivo|m|s||300").unwrap();
        drop(file);

        let trie = DictionaryLoader::load_from_file(&test_file).unwrap();

        // Check if the words are found
        assert!(
            trie.contains("6K"),
            "Should find 6K after loading from file"
        );
        assert!(
            trie.contains("6k"),
            "Should find 6k after loading from file"
        );
        assert!(
            trie.contains("4K"),
            "Should find 4K after loading from file"
        );
        assert!(
            trie.contains("4k"),
            "Should find 4k after loading from file"
        );

        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_append_from_file_preserves_four_field_metadata() {
        let test_file = temp_test_file("append_4_fields");
        let mut file = File::create(&test_file).unwrap();
        writeln!(file, "mesa|sustantivo|f|s").unwrap();
        drop(file);

        let mut trie = Trie::new();
        let count = DictionaryLoader::append_from_file(&mut trie, &test_file).unwrap();
        assert_eq!(count, 1);

        let info = trie.get_or_derive("mesa").expect("mesa should exist");
        assert_eq!(info.category, WordCategory::Sustantivo);
        assert_eq!(info.gender, Gender::Feminine);
        assert_eq!(info.number, Number::Singular);

        let _ = fs::remove_file(&test_file);
    }
}
