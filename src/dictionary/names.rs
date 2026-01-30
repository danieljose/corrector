//! Cargador de nombres propios y apellidos
//!
//! Proporciona una estructura para verificar si una palabra es un nombre propio.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Conjunto de nombres propios y apellidos
pub struct ProperNames {
    /// Nombres en su forma original (capitalizada)
    names: HashSet<String>,
    /// Nombres en minúscula para búsqueda insensible a mayúsculas
    names_lower: HashSet<String>,
}

impl ProperNames {
    /// Crea un conjunto vacío
    pub fn new() -> Self {
        Self {
            names: HashSet::new(),
            names_lower: HashSet::new(),
        }
    }

    /// Carga nombres desde un archivo (un nombre por línea)
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path.as_ref())
            .map_err(|e| format!("No se pudo abrir el archivo de nombres: {}", e))?;

        let reader = BufReader::new(file);
        let mut names = HashSet::new();
        let mut names_lower = HashSet::new();

        for line_result in reader.lines() {
            let line = line_result.map_err(|e| format!("Error leyendo: {}", e))?;
            let name = line.trim();

            if !name.is_empty() && !name.starts_with('#') {
                names.insert(name.to_string());
                names_lower.insert(name.to_lowercase());
            }
        }

        Ok(Self { names, names_lower })
    }

    /// Verifica si una palabra es un nombre propio (búsqueda exacta)
    pub fn contains(&self, word: &str) -> bool {
        self.names.contains(word)
    }

    /// Verifica si una palabra es un nombre propio (insensible a mayúsculas)
    pub fn contains_ignore_case(&self, word: &str) -> bool {
        self.names_lower.contains(&word.to_lowercase())
    }

    /// Verifica si una palabra capitalizada es un nombre propio
    ///
    /// Retorna true si:
    /// - La palabra empieza con mayúscula Y
    /// - Está en la lista de nombres (comparación insensible a mayúsculas)
    pub fn is_proper_name(&self, word: &str) -> bool {
        // Verificar que empiece con mayúscula
        let first_char = match word.chars().next() {
            Some(c) => c,
            None => return false,
        };

        if !first_char.is_uppercase() {
            return false;
        }

        // Buscar en la lista (insensible a mayúsculas)
        self.names_lower.contains(&word.to_lowercase())
    }

    /// Número de nombres en la base de datos
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Verifica si la base de datos está vacía
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

impl Default for ProperNames {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_load_names() {
        let test_file = "test_names.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "Juan").unwrap();
        writeln!(file, "María").unwrap();
        writeln!(file, "García").unwrap();
        writeln!(file, "# comentario").unwrap();
        writeln!(file, "Smith").unwrap();
        drop(file);

        let names = ProperNames::load_from_file(test_file).unwrap();

        assert_eq!(names.len(), 4);
        assert!(names.contains("Juan"));
        assert!(names.contains("María"));
        assert!(!names.contains("comentario"));

        fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_is_proper_name() {
        let test_file = "test_names2.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "Juan").unwrap();
        writeln!(file, "García").unwrap();
        drop(file);

        let names = ProperNames::load_from_file(test_file).unwrap();

        // Con mayúscula y en lista -> true
        assert!(names.is_proper_name("Juan"));
        assert!(names.is_proper_name("García"));

        // Con mayúscula pero no en lista -> false
        assert!(!names.is_proper_name("Pedro"));

        // Sin mayúscula -> false (aunque esté en lista)
        assert!(!names.is_proper_name("juan"));

        // Variantes de capitalización
        assert!(names.is_proper_name("JUAN")); // Empieza con mayúscula

        // Sin tilde no coincide con versión con tilde (búsqueda exacta)
        assert!(!names.is_proper_name("Garcia")); // "Garcia" != "García"

        fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_contains_ignore_case() {
        let test_file = "test_names3.txt";
        let mut file = File::create(test_file).unwrap();
        writeln!(file, "Mohammed").unwrap();
        drop(file);

        let names = ProperNames::load_from_file(test_file).unwrap();

        assert!(names.contains_ignore_case("Mohammed"));
        assert!(names.contains_ignore_case("mohammed"));
        assert!(names.contains_ignore_case("MOHAMMED"));
        assert!(!names.contains_ignore_case("John"));

        fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_empty() {
        let names = ProperNames::new();
        assert!(names.is_empty());
        assert_eq!(names.len(), 0);
        assert!(!names.contains("Juan"));
    }
}
