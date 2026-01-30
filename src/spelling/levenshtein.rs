//! Algoritmo de distancia de Levenshtein

/// Calcula la distancia de Levenshtein entre dos cadenas
///
/// La distancia de Levenshtein es el número mínimo de operaciones
/// (inserción, eliminación, sustitución) necesarias para transformar
/// una cadena en otra.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let a_len = a_chars.len();
    let b_len = b_chars.len();

    // Casos base
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Optimización: usar solo dos filas en lugar de matriz completa
    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row: Vec<usize> = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr_row[0] = i;

        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };

            curr_row[j] = (prev_row[j] + 1) // eliminación
                .min(curr_row[j - 1] + 1) // inserción
                .min(prev_row[j - 1] + cost); // sustitución
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Calcula la distancia de Damerau-Levenshtein
///
/// Igual que Levenshtein pero también permite transposiciones
/// (intercambio de dos caracteres adyacentes).
pub fn damerau_levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    // Matriz completa necesaria para Damerau-Levenshtein
    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };

            matrix[i][j] = (matrix[i - 1][j] + 1) // eliminación
                .min(matrix[i][j - 1] + 1) // inserción
                .min(matrix[i - 1][j - 1] + cost); // sustitución

            // Transposición
            if i > 1
                && j > 1
                && a_chars[i - 1] == b_chars[j - 2]
                && a_chars[i - 2] == b_chars[j - 1]
            {
                matrix[i][j] = matrix[i][j].min(matrix[i - 2][j - 2] + cost);
            }
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(levenshtein_distance("hola", "hola"), 0);
    }

    #[test]
    fn test_levenshtein_empty() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
    }

    #[test]
    fn test_levenshtein_single_edit() {
        assert_eq!(levenshtein_distance("casa", "caza"), 1); // sustitución
        assert_eq!(levenshtein_distance("casa", "casas"), 1); // inserción
        assert_eq!(levenshtein_distance("casas", "casa"), 1); // eliminación
    }

    #[test]
    fn test_levenshtein_multiple_edits() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("problema", "probelma"), 2);
    }

    #[test]
    fn test_damerau_levenshtein_transposition() {
        // "probelma" -> "problema" requiere solo 1 transposición con Damerau
        assert_eq!(damerau_levenshtein_distance("probelma", "problema"), 1);
        // Pero 2 operaciones con Levenshtein estándar
        assert_eq!(levenshtein_distance("probelma", "problema"), 2);
    }
}
