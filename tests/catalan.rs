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

// ==========================================================================
// Punt volat (·)
// ==========================================================================

#[test]
fn test_catalan_punt_volat_correct() {
    let corrector = create_catalan_corrector();

    // Palabras con punt volat deben reconocerse como una sola unidad
    let result = corrector.correct("El col·legi és gran");
    assert!(!result.contains('|'), "No debería marcar 'col·legi' como error: {}", result);

    let result = corrector.correct("És intel·ligent");
    assert!(!result.contains('|'), "No debería marcar 'intel·ligent' como error: {}", result);

    let result = corrector.correct("Va col·laborar amb nosaltres");
    assert!(!result.contains('|'), "No debería marcar 'col·laborar' como error: {}", result);
}

#[test]
fn test_catalan_punt_volat_typo() {
    let corrector = create_catalan_corrector();

    // Typo en palabra con punt volat debe generar sugerencias con punt volat
    let result = corrector.correct("intel·lignt");
    assert!(result.contains('|'), "Debería marcar 'intel·lignt' como error: {}", result);
    // La sugerencia debe contener el punt volat
    assert!(result.contains("intel·ligent"), "La sugerencia debería incluir 'intel·ligent': {}", result);
}

// ==========================================================================
// Elisiones con apóstrofo
// ==========================================================================

#[test]
fn test_catalan_elision_correct() {
    let corrector = create_catalan_corrector();

    // l' + sustantivo
    let result = corrector.correct("l'home");
    assert!(!result.contains('|'), "No debería marcar 'l'home' como error: {}", result);

    // d' + demostrativo
    let result = corrector.correct("d'aquest");
    assert!(!result.contains('|'), "No debería marcar 'd'aquest' como error: {}", result);

    // n' + pronombre
    let result = corrector.correct("n'hi ha");
    assert!(!result.contains('|'), "No debería marcar 'n'hi' como error: {}", result);

    // s' + verbo
    let result = corrector.correct("s'ha acabat");
    assert!(!result.contains('|'), "No debería marcar 's'ha' como error: {}", result);

    // l' + sustantivo femenino
    let result = corrector.correct("l'escola");
    assert!(!result.contains('|'), "No debería marcar 'l'escola' como error: {}", result);
}

#[test]
fn test_catalan_elision_typo() {
    let corrector = create_catalan_corrector();

    // Typo en la parte tras el apóstrofo
    let result = corrector.correct("l'hme");
    assert!(result.contains('|'), "Debería marcar 'l'hme' como error: {}", result);
    assert!(result.contains("l'home"), "La sugerencia debería ser 'l'home': {}", result);
}

// ==========================================================================
// Combinación: punt volat + elisiones en contexto
// ==========================================================================

#[test]
fn test_catalan_mixed_features() {
    let corrector = create_catalan_corrector();

    let result = corrector.correct("L'home va col·laborar amb l'escola");
    assert!(!result.contains('|'), "No debería marcar errores en texto con elisiones y punt volat: {}", result);
}
