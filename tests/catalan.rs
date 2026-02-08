//! Tests de integración del corrector para catalán.
//!
//! Ejecutar solo estos tests:  cargo test --test catalan

use std::path::PathBuf;
use corrector::{Config, Corrector};

fn create_catalan_corrector() -> Corrector {
    let config = Config {
        language: "ca".to_string(),
        data_dir: PathBuf::from("data"),
        custom_dict: None,
        input_file: None,
        output_file: None,
        add_word: None,
        text: None,
        show_help: false,
        spelling_separator: "|".to_string(),
        grammar_separator: ("[".to_string(), "]".to_string()),
    };
    Corrector::new(&config).expect("Failed to create Catalan corrector")
}

#[test]
fn test_catalan_correct_words_no_errors() {
    let corrector = create_catalan_corrector();
    // Palabras catalanas comunes que deben estar en el diccionario
    let result = corrector.correct("Barcelona és una ciutat");
    assert!(!result.contains('|'), "No debería marcar errores ortográficos en texto correcto: {}", result);
    assert!(!result.contains('['), "No debería marcar errores gramaticales: {}", result);
}

#[test]
fn test_catalan_incorrect_word_gets_suggestions() {
    let corrector = create_catalan_corrector();
    let result = corrector.correct("Barcelonaa");
    assert!(result.contains('|'), "Debería marcar 'Barcelonaa' como error ortográfico: {}", result);
}

#[test]
fn test_catalan_proper_names_recognized() {
    let corrector = create_catalan_corrector();
    // Nombres propios catalanes fusionados en names.txt
    let result = corrector.correct("Montserrat");
    assert!(!result.contains('|'), "No debería marcar 'Montserrat' como error: {}", result);
}

#[test]
fn test_catalan_no_grammar_corrections() {
    let corrector = create_catalan_corrector();
    // Sin reglas gramaticales, no debe haber correcciones gramaticales
    let result = corrector.correct("el gat és negre");
    assert!(!result.contains('['), "No debería aplicar correcciones gramaticales: {}", result);
}

#[test]
fn test_catalan_no_spanish_rules_applied() {
    // Verifica que no se aplican reglas españolas (tildes, vocativos, etc.)
    let corrector = create_catalan_corrector();

    // "Hola Joan" en español generaría coma vocativa → "Hola, Joan"
    // En catalán no debe pasar
    let result = corrector.correct("Hola Joan");
    assert!(!result.contains('['), "No debería aplicar coma vocativa española: {}", result);
}
