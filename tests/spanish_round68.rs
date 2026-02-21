use corrector::{Config, Corrector};
use std::path::PathBuf;

fn create_test_corrector() -> Corrector {
    let config = Config {
        language: "es".to_string(),
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
    Corrector::new(&config).expect("Failed to create corrector")
}

#[test]
fn test_round68_sentence_initial_que_si_que_needs_accent() {
    let corrector = create_test_corrector();

    let emphatic = corrector.correct("Que si que es verdad.");
    assert!(
        emphatic.contains("si [s\u{00ED}]") || emphatic.contains("Si [S\u{00ED}]"),
        "Debe acentuar 's\u{00ED}' en 'Que si que ...': {}",
        emphatic
    );

    let conditional = corrector.correct("Dijo que si vienes, avise.");
    assert!(
        !conditional.contains("si [s\u{00ED}]") && !conditional.contains("Si [S\u{00ED}]"),
        "No debe acentuar condicional en 'que si vienes': {}",
        conditional
    );
}
