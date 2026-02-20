//! Tests de integración del corrector para español.
//!
//! Ejecutar solo estos tests:  cargo test --test spanish_corrector

use corrector::{Config, Corrector};
use std::path::PathBuf;

fn create_test_corrector_with_language(language: &str) -> Corrector {
    let config = Config {
        language: language.to_string(),
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

fn create_test_corrector() -> Corrector {
    create_test_corrector_with_language("es")
}

#[test]
fn test_spanish_alias_uses_canonical_dictionary_path() {
    let corrector = create_test_corrector_with_language("spanish");
    let result = corrector.correct("El casa es bonita");
    assert!(
        result.contains("[La]"),
        "Debería mantener reglas con alias 'spanish': {}",
        result
    );
    assert!(
        !result.contains("|?|"),
        "No debería quedar con diccionario vacío al usar alias 'spanish': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_then_subject_verb() {
    // Flujo: "tu canto" → (diacríticas) "tú" → (sujeto-verbo) "cantas"
    // Este test verifica que effective_text() propaga correcciones entre fases
    let corrector = create_test_corrector();
    let result = corrector.correct("tu canto muy bien");

    // Debe corregir AMBOS: "tu" → "tú" Y "canto" → "cantas"
    assert!(
        result.contains("[Tú]"),
        "Debería corregir 'tu' → 'tú': {}",
        result
    );
    assert!(
        result.contains("[cantas]"),
        "Debería corregir 'canto' → 'cantas': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_then_subject_verb_hablo() {
    // Otro caso: "tu hablo" → "tú hablas"
    let corrector = create_test_corrector();
    let result = corrector.correct("tu hablo español");

    assert!(
        result.contains("[Tú]"),
        "Debería corregir 'tu' → 'tú': {}",
        result
    );
    assert!(
        result.contains("[hablas]"),
        "Debería corregir 'hablo' → 'hablas': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_si() {
    let corrector = create_test_corrector();
    let result = corrector.correct("no se si");

    assert!(
        result.contains("se [sé]"),
        "Debería corregir 'se' -> 'sé' en 'no se si': {}",
        result
    );
    assert!(
        !result.contains("si [sí]"),
        "No debería corregir 'si' -> 'sí' en 'no se si': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_solo_se_que_no_se_nada_both_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Solo se que no se nada");
    let lower = result.to_lowercase();
    let occurrences = lower.match_indices("se [s").count();

    assert!(
        occurrences >= 2,
        "Debe corregir ambos 'se' como 's\u{00E9}' en 'Solo se que no se nada': {}",
        result
    );
    assert!(
        lower.contains("solo se [s"),
        "Debe corregir tambi\u{00E9}n el primer 'se' tras adverbio: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_se_imperative_with_more_adjectives() {
    let corrector = create_test_corrector();

    for input in [
        "Se feliz",
        "Se fuerte",
        "Se humilde",
        "Se libre",
        "Se util",
        "Se fiel",
        "Se breve",
        "Se justo",
        "Se claro",
        "Se firme",
        "Se digno",
        "Se honesto",
        "Se sincero",
    ] {
        let result = corrector.correct(input);
        let result_lower = result.to_lowercase();
        assert!(
            result_lower.contains("se [s"),
            "Deberia corregir 'se' -> 'sé' en imperativo para '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_se_tu_mismo_dual_accent() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Se tu mismo");
    let lower = result.to_lowercase();

    assert!(
        lower.contains("se [s"),
        "Deberia corregir 'se' -> 'se' (se con tilde) en 'Se tu mismo': {}",
        result
    );
    assert!(
        lower.contains("tu [t"),
        "Deberia corregir 'tu' -> 'tu' (tu con tilde) en 'Se tu mismo': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_como_hacerlo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se como hacerlo");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se como hacerlo': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_cuanto_cuesta() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se cuanto cuesta");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se cuanto cuesta': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_por_que_vino() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se por que vino");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se por que vino': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_por_donde_empezar() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se por donde empezar");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se por donde empezar': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_de_quien_hablas() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se de quien hablas");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se de quien hablas': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_con_quien_vino() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se con quien vino");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se con quien vino': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_para_el_es_dificil() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Para el es difícil");

    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Debería corregir 'el' -> 'él' en 'Para el es difícil': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_no_sabia_que_hacer() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El no sabia que hacer");

    assert!(
        result.contains("El [Él]") || result.contains("el [él]"),
        "Debería corregir pronombre 'El' -> 'Él': {}",
        result
    );
    assert!(
        result.contains("sabia [sabía]"),
        "Debería corregir forma verbal 'sabia' -> 'sabía': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_sentence_start_with_conjugated_verb() {
    let corrector = create_test_corrector();
    let cases = [
        "El sabe",
        "El es",
        "El fue",
        "El estudia medicina",
        "El camina rapido",
        "El juega al futbol",
        "El baila salsa",
        "El nada rapido",
        "El pinta cuadros",
        "El llama a su madre",
        "El cuenta una historia",
        "El corta el pan",
        "El limpia la mesa",
        "El busca trabajo",
        "El toca la guitarra",
        "El gana siempre",
        "El rie mucho",
        "El llora poco",
        "El marcha rapido",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains("El [") && result.contains("] "),
            "Deberia corregir pronombre 'El' al inicio en '{}': {}",
            text,
            result
        );
        assert!(
            !result.contains("El [La]") && !result.contains("el [la]"),
            "No deberia reinterpretar '{}' como articulo+sustantivo: {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_sentence_start_nominal_no_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El perro corre");

    assert!(
        !result.contains("El ["),
        "No deberia corregir 'El' como articulo al inicio: {}",
        result
    );
}

#[test]
fn test_integration_dictionary_spurious_noun_entries_do_not_trigger_false_agreement() {
    let corrector = create_test_corrector();
    let cases = [
        "El caf\u{00E9} se enfr\u{00ED}a",
        "El ni\u{00F1}o se aburr\u{00ED}a mucho",
        "El barco se desv\u{00ED}a",
        "El tema se ampl\u{00ED}a",
        "El proyecto se beneficiar\u{00ED}a",
        "El equipo se clasificar\u{00ED}a",
        "El rumor se difund\u{00ED}a",
        "El perro se enfr\u{00ED}a r\u{00E1}pido",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains('[') && !result.contains('|'),
            "No deberia generar correcciones espurias para '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_common_nouns_in_o_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = [
        "El libro es nuevo",
        "El cambio climatico",
        "El centro de la ciudad",
        "El caso fue resuelto",
        "El paso del tiempo",
        "El canto del gallo",
        "El marco legal",
        "El caso de Maria",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [") && !result.contains("el ["),
            "No deberia convertir articulo en pronombre en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_common_nouns_in_e_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = [
        "El viaje fue largo",
        "El nombre del autor",
        "El corte de pelo",
        "El debate presidencial",
        "El baile de mascaras",
        "El detalle del contrato",
        "El parte medico",
        "El cierre de la tienda",
        "El avance tecnologico",
        "El combate fue duro",
        "El enlace fue exitoso",
        "El arma es peligrosa",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [") && !result.contains("el ["),
            "No deberia convertir articulo en pronombre en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_nominal_plus_se_not_pronoun() {
    let corrector = create_test_corrector();
    let samples = [
        "El hecho se produjo",
        "El partido se jugo ayer",
        "El sentido se perdio",
        "El tejido se rompio",
        "El gobierno se reunio",
        "El equipo se preparo",
        "El consumo se redujo",
        "El cambio se produjo",
        "El acuerdo se firmo",
        "El trabajo se termino",
        "El proyecto se cancelo",
        "El pago se realizo",
        "El resultado se publico",
        "El contenido se filtro",
        "El pedido se envio",
        "El vestido se mancho",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [") && !result.contains("el ["),
            "No deberia convertir articulo en pronombre en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_apocopated_adjective_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = [
        "El buen libro es interesante",
        "El gran cambio llego",
        "El buen hombre",
        "El gran poeta",
        "El buen vivir",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [") && !result.contains("el ["),
            "No deberia convertir articulo en pronombre en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_de_demonstrative_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = [
        "El de la derecha es mejor",
        "El de azul es mi amigo",
        "El de ayer era mejor",
        "El de siempre",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [") && !result.contains("el ["),
            "No deberia convertir 'El de ...' en pronombre personal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_hecho_que_does_not_block_queismo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El hecho que este aqui");

    assert!(
        !result.contains("El [") && !result.contains("el ["),
        "No deberia convertir articulo en pronombre en 'El hecho que...': {}",
        result
    );
    assert!(
        result.to_lowercase().contains("que [de que]"),
        "Deberia corregir queismo en 'El hecho que...': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_nominal_homograph_with_clitics_or_adverbs_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = [
        "El gobierno lo decidió",
        "El equipo lo ganó todo",
        "El trabajo lo terminó Juan",
        "El gobierno me preocupa",
        "El cambio te beneficia",
        "El trabajo nos agota",
        "El resultado me sorprendió",
        "El gobierno le informó",
        "El equipo les ganó",
        "El gobierno no funciona",
        "El cambio ya llegó",
        "El trabajo también importa",
        "El gobierno siempre miente",
        "El cambio nunca llegó",
        "El proceso aún continúa",
        "El cambio lo notamos todos",
        "El proceso lo llevan ellos",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [Él]") && !result.contains("el [él]"),
            "No deberia convertir articulo en pronombre en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_irrealis_conditional_si_tendria() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si tendria dinero, compraria una casa");

    assert!(
        result.contains("[tuviera]"),
        "Debe corregir 'si + condicional' a subjuntivo imperfecto: {}",
        result
    );
}

#[test]
fn test_integration_irrealis_conditional_si_podrias() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si podrias venir, te avisaria");

    assert!(
        result.contains("[pudieras]"),
        "Debe corregir 'podrias' -> 'pudieras' tras 'si': {}",
        result
    );
}

#[test]
fn test_irrealis_analyzer_first_plural_direct() {
    use corrector::dictionary::DictionaryLoader;
    use corrector::grammar::Tokenizer;
    use corrector::languages::spanish::{IrrealisConditionalAnalyzer, Spanish, VerbRecognizer};
    use corrector::languages::Language;

    let mut dictionary = DictionaryLoader::load_from_file("data/es/words.txt")
        .expect("Debe cargar data/es/words.txt");
    let spanish = Spanish::new();
    spanish.configure_dictionary(&mut dictionary);
    let recognizer = VerbRecognizer::from_dictionary(&dictionary);

    let tokenizer = Tokenizer::new();
    let tokens = tokenizer.tokenize("Si tendr\u{00ED}amos dinero, viajar\u{00ED}amos");
    let corrections = IrrealisConditionalAnalyzer::analyze(&tokens, Some(&recognizer));
    let infinitive =
        corrector::languages::VerbFormRecognizer::get_infinitive(&recognizer, "tendr\u{00ED}amos");
    let knows_tener =
        corrector::languages::VerbFormRecognizer::knows_infinitive(&recognizer, "tener");

    assert!(
        corrections
            .iter()
            .any(|c| c.suggestion == "tuvi\u{00E9}ramos"),
        "Irrealis 1a plural sin detectar. corrections={:?}, infinitive={:?}, knows_tener={}",
        corrections,
        infinitive,
        knows_tener
    );
}

#[test]
fn test_integration_irrealis_conditional_first_plural_forms() {
    let corrector = create_test_corrector();

    let result_tener = corrector.correct("Si tendr\u{00ED}amos dinero, viajar\u{00ED}amos");
    assert!(
        result_tener.contains("[tuvi\u{00E9}ramos]"),
        "Debe corregir 'si tendr\u{00ED}amos' -> 'si tuvi\u{00E9}ramos': {}",
        result_tener
    );

    let result_hacer = corrector.correct("Si har\u{00ED}amos eso, saldr\u{00ED}a mal");
    assert!(
        result_hacer.contains("[hici\u{00E9}ramos]"),
        "Debe corregir 'si har\u{00ED}amos' -> 'si hici\u{00E9}ramos': {}",
        result_hacer
    );

    let result_decir = corrector.correct("Si dir\u{00ED}amos la verdad, se enojar\u{00ED}an");
    assert!(
        result_decir.contains("[dij\u{00E9}ramos]"),
        "Debe corregir 'si dir\u{00ED}amos' -> 'si dij\u{00E9}ramos': {}",
        result_decir
    );
}

#[test]
fn test_integration_irrealis_conditional_regular_er() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si comeria mas, engordaria");

    assert!(
        result.contains("[comiera]"),
        "Debe corregir condicional regular tras 'si': {}",
        result
    );
}

#[test]
fn test_integration_irrealis_conditional_stem_changing_ir() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si sentiria dolor, iria al medico");

    assert!(
        result.contains("[sintiera]"),
        "Debe corregir condicional irregular de -ir tras 'si': {}",
        result
    );
}

#[test]
fn test_integration_irrealis_conditional_uir_verbs() {
    let corrector = create_test_corrector();
    let result_huir = corrector.correct("Si huiria escaparia");
    let result_incluir = corrector.correct("Si incluiria cambios mejoraria");

    assert!(
        result_huir.contains("[huyera]"),
        "Debe corregir condicional de '-uir' a subjuntivo en 'huir': {}",
        result_huir
    );
    assert!(
        result_incluir.contains("[incluyera]"),
        "Debe corregir condicional de '-uir' a subjuntivo en 'incluir': {}",
        result_incluir
    );
}

#[test]
fn test_integration_irrealis_conditional_no_correction_in_indirect_question() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se si tendria tiempo");

    assert!(
        !result.contains("[tuviera]"),
        "No debe corregir en interrogativa indirecta 'no se si...': {}",
        result
    );
}

#[test]
fn test_integration_irrealis_conditional_no_correction_in_que_si_indirect_question() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Me pregunto que si tendria tiempo");

    assert!(
        !result.contains("[tuviera]"),
        "No debe corregir en interrogativa indirecta 'que si...': {}",
        result
    );
}

#[test]
fn test_integration_irrealis_conditional_correct_subjunctive_unchanged() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si tuviera dinero, compraria una casa");

    assert!(
        !result.contains("tuviera ["),
        "No debe tocar forma ya correcta en subjuntivo: {}",
        result
    );
}

#[test]
fn test_integration_irrealis_conditional_does_not_touch_apodosis_without_comma() {
    let corrector = create_test_corrector();
    let cases = [
        ("Si pudiera iria", "iria ["),
        ("Si fuera rico viajaria", "viajaria ["),
        ("Si lo supiera no preguntaria", "preguntaria ["),
        ("Si tuviera dinero compraria una casa", "compraria ["),
    ];

    for (text, unexpected) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(unexpected),
            "No debe corregir condicional en la apódosis sin coma: '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_irrealis_conditional_como_si_conditional() {
    let corrector = create_test_corrector();
    let cases = [
        ("Habla como si sabria la verdad", "[supiera]"),
        ("Actua como si seria el jefe", "[fuera]"),
        ("Corre como si tendria prisa", "[tuviera]"),
        ("Gasta como si seria millonario", "[fuera]"),
        ("Habla como si podria hacerlo", "[pudiera]"),
        ("Me mira como si conoceria mi secreto", "[conociera]"),
        ("Trabaja como si no habria manana", "[hubiera]"),
    ];

    for (text, expected) in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains(expected),
            "Debe corregir condicional tras 'como si' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_irrealis_conditional_ojala_conditional() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ojalá llovería mañana");
    assert!(
        result.contains("[lloviera]"),
        "Debe corregir condicional tras 'ojalá': {}",
        result
    );
}

#[test]
fn test_integration_irrealis_conditional_ojala_subjunctive_unchanged() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ojalá llueva mañana");
    assert!(
        !result.contains("llueva ["),
        "No debe tocar subjuntivo correcto tras 'ojalá': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_sentence_end() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se nada': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_impersonal_no_accent() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada en la piscina");

    assert!(
        !result.contains("se [s"),
        "Should not force saber accent in impersonal context: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_de_eso() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada de eso");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se nada de eso': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_de_espaldas_impersonal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada de espaldas");

    assert!(
        !result.contains("se [s"),
        "Should not force saber accent in impersonal 'nadar' context: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_mucho() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se mucho");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se mucho': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_yo_no_se_bien() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Yo no se bien");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'Yo no se bien': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_mucho_en_la_piscina_impersonal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se mucho en la piscina");

    assert!(
        !result.contains("se [s"),
        "Should keep impersonal/locative reading without forcing saber: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_yo_no_se_mucho_en_la_piscina_still_saber() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Yo no se mucho en la piscina");

    assert!(
        result.contains("se [s"),
        "Should still correct saber with explicit subject 'yo': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_casi_nada() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se casi nada");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se casi nada': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_absolutamente_nada_sobre_quimica() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se absolutamente nada sobre química");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber with modifier + topic tail: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_aqui_no_se_casi_nada_en_la_piscina_impersonal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Aquí no se casi nada en la piscina");

    assert!(
        !result.contains("se [s"),
        "Should keep impersonal locative reading without forcing saber: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_aqui_no_se_nada_impersonal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Aquí no se nada");

    assert!(
        !result.contains("se [s"),
        "Should keep impersonal/ambiguous locative pattern without forcing saber: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_aqui_no_se_nada_de_eso() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Aquí no se nada de eso");

    assert!(
        result.contains("se [s"),
        "Should still correct saber with explicit 'de eso' tail: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_de_quimica() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada de química");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se nada de química': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_de_braza_impersonal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada de braza");

    assert!(
        !result.contains("se [s"),
        "Should keep impersonal swimming-mode reading without forcing saber: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_sobre_quimica() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada sobre química");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se nada sobre química': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_acerca_de_quimica() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada acerca de química");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se nada acerca de química': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_nada_respecto_a_quimica() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se nada respecto a química");

    assert!(
        result.contains("se [s"),
        "Should correct 'se' as saber in 'No se nada respecto a química': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_si_sentence_start_with_comma() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si, claro que puedo");

    assert!(
        result.contains("Si [Sí]"),
        "Debería corregir 'Si,' inicial a 'Sí,': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_si_sentence_start_conditional_no_accent() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si vienes, avisa");

    assert!(
        !result.contains("Si [Sí]") && !result.contains("si [sí]"),
        "No debería corregir 'si' condicional al inicio: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_no_se_siembra() {
    let corrector = create_test_corrector();
    let result = corrector.correct("no se siembra arroz");

    assert!(
        !result.contains("se [sé]"),
        "No debería corregir 'se' en pasiva/reflexiva: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_tu_pregunta_with_adverb_no_false_positive() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Sobre tu pregunta ya respondo");
    assert!(
        !result.contains("tu [tú]") && !result.contains("tu [Tú]"),
        "No debería corregir 'tu' en sintagma posesivo: {}",
        result
    );

    let result = corrector.correct("Sobre tu pregunta mañana respondo");
    assert!(
        !result.contains("tu [tú]") && !result.contains("tu [Tú]"),
        "No debería corregir 'tu' en sintagma posesivo: {}",
        result
    );

    let result = corrector.correct("Sobre tu pregunta claramente no respondo");
    assert!(
        !result.contains("tu [tú]") && !result.contains("tu [Tú]"),
        "No debería corregir 'tu' en sintagma posesivo con adverbio+no+verbo: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_mismo_plus_noun_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = [
        "Los estudiantes obtienen el mismo título",
        "Le dieron el mismo trato",
        "Reciben el mismo sueldo",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("el [él]") && !result.contains("el [Él]"),
            "No debería corregir 'el mismo + sustantivo' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_before_nominal_head_after_preposition_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = ["Para el partido", "Segun el informe", "Con el resultado"];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("el [él]") && !result.contains("el [Él]"),
            "No debería corregir 'el' -> 'él' antes de núcleo nominal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_en_el_de_no_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No juega en el de las promesas, sino en el de los hechos");
    assert!(
        !result.contains("el [él]") && !result.contains("el [Él]"),
        "No debe acentuar 'el' en 'en el de ...': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_cocina_bien_not_rewritten_as_article() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El cocina bien");

    assert!(
        result.contains("El ["),
        "Deberia corregir 'El' como pronombre en 'El cocina bien': {}",
        result
    );
    assert!(
        !result.contains("La cocina"),
        "No deberia reinterpretar 'El cocina bien' como articulo+sustantivo: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_cuenta_una_historia_not_rewritten_as_article() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El cuenta una historia");

    assert!(
        result.contains("El ["),
        "Deberia corregir 'El' como pronombre en 'El cuenta una historia': {}",
        result
    );
    assert!(
        !result.contains("La cuenta"),
        "No deberia reinterpretar 'El cuenta...' como articulo+sustantivo: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_marcha_rapido_not_rewritten_as_article() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El marcha rapido");

    assert!(
        result.contains("El ["),
        "Deberia corregir 'El' como pronombre en 'El marcha rapido': {}",
        result
    );
    assert!(
        !result.contains("La marcha"),
        "No deberia reinterpretar 'El marcha...' como articulo+sustantivo: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_plus_name_not_converted_to_pronoun() {
    let corrector = create_test_corrector();
    let samples = ["El Juan es simpatico", "El Juan no vino"];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [Él]") && !result.contains("el [él]"),
            "No deberia convertir articulo + nombre propio en pronombre en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_sentence_start_la_plus_verb_not_forced_to_article() {
    let corrector = create_test_corrector();
    let samples = ["La traje flores", "La traje de Paris"];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("La [El]") && !result.contains("la [el]"),
            "No deberia reinterpretar clitico + verbo como articulo+sustantivo en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_spelling_averio_not_marked_as_error() {
    let corrector = create_test_corrector();
    let text = "El coche se averió";
    let result = corrector.correct(text);

    assert_eq!(
        result, text,
        "No deberia marcar 'averió' como error ortografico: {}",
        result
    );
}

#[test]
fn test_integration_spelling_ceramica_and_informatica_are_marked() {
    let corrector = create_test_corrector();
    let ceramica = corrector.correct("La ceramica es util");
    let informatica = corrector.correct("La informatica avanza");

    assert!(
        ceramica.contains("cer\u{00E1}mica"),
        "Debe sugerir tilde en 'ceramica': {}",
        ceramica
    );
    assert!(
        informatica.contains("inform\u{00E1}tica"),
        "Debe sugerir tilde en 'informatica': {}",
        informatica
    );
}

#[test]
fn test_integration_masculine_ending_a_articles_are_corrected_for_tema_family() {
    let corrector = create_test_corrector();
    let samples = [
        "La tema es interesante",
        "La sistema es bueno",
        "La drama fue intenso",
        "La programa no funciona",
        "La panorama general",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            result.contains("La [El]") || result.contains("la [el]"),
            "Deberia corregir articulo femenino ante sustantivo masculino en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_masculine_ending_a_plural_articles_are_corrected() {
    let corrector = create_test_corrector();
    let samples = [
        "Las temas son dif\u{00ED}ciles",
        "Las dramas son intensos",
        "Las sistemas funcionan",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            result.contains("Las [Los]") || result.contains("las [los]"),
            "Deberia corregir articulo plural femenino ante sustantivo masculino en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_determiner_temporal_phrase_not_treated_as_predicative() {
    let corrector = create_test_corrector();
    let samples = [
        "Compr\u{00F3} la casa este a\u{00F1}o",
        "El coche esta semana",
        "La casa este verano",
        "El ni\u{00F1}o estas vacaciones",
        "la liga este a\u{00F1}o",
    ];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("a\u{00F1}o [a\u{00F1}a]")
                && !result.contains("semana [semano]")
                && !result.contains("verano [verana]")
                && !result.contains("vacaciones [vacacion]"),
            "No debe tratar determinantes temporales como adjetivos predicativos en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_el_tema_programa_sentence_end_no_false_positive() {
    let corrector = create_test_corrector();
    let samples = ["El tema", "El programa"];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("El [\u{00C9}l]") && !result.contains("el [\u{00E9}l]"),
            "No deberia convertir articulo en pronombre al final de oracion en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_entre_el_y_yo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Entre el y yo");

    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Debería corregir 'Entre el y yo' -> 'Entre él y yo': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_hasta_el_lo_sabe() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hasta el lo sabe");

    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Deberia corregir 'Hasta el lo sabe' a pronombre tónico: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_por_el_no_te_preocupes() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Por el no te preocupes");

    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Deberia corregir 'Por el no te preocupes' a pronombre tónico: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_fue_el_quien_lo_hizo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Fue el quien lo hizo");

    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Deberia corregir 'Fue el quien lo hizo' a pronombre tónico: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_excepto_el_todos_vinieron() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Excepto el todos vinieron");

    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Deberia corregir 'Excepto el todos vinieron' a pronombre tónico: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_salvo_el_nadie_lo_sabe() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Salvo el nadie lo sabe");

    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Deberia corregir 'Salvo el nadie lo sabe' a pronombre tónico: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_el_mismo_pronoun_still_corrects() {
    let corrector = create_test_corrector();
    let result = corrector.correct("el mismo lo hizo");

    assert!(
        result.contains("el [él]") || result.contains("el [Él]"),
        "Debería corregir 'el mismo lo hizo' a pronombre tónico: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_tu_no_plus_verb() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Tu no puedes hacer eso");
    assert!(
        result.contains("Tu [Tú]"),
        "Debería corregir 'Tu no puedes...' a 'Tú': {}",
        result
    );

    let result = corrector.correct("Tu no sabes nada");
    assert!(
        result.contains("Tu [Tú]"),
        "Debería corregir 'Tu no sabes...' a 'Tú': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_tu_adverb_no_plus_verb() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Tu claramente no sabes nada");
    assert!(
        result.contains("Tu [Tú]"),
        "Debería corregir 'Tu claramente no sabes...' a 'Tú': {}",
        result
    );

    let result = corrector.correct("Tu ahora no quieres venir");
    assert!(
        result.contains("Tu [Tú]"),
        "Debería corregir 'Tu ahora no quieres...' a 'Tú': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_tu_no_nominal_no_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("tu no rotundo me sorprendió");

    assert!(
        !result.contains("tu [tú]") && !result.contains("tu [Tú]"),
        "No debería corregir posesivo en 'tu no rotundo': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_te_apoyo_cuento_no_false_tea() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Te apoyo en esta decisión");
    assert!(
        !result.contains("Te [Té]") && !result.contains("te [té]"),
        "No debería corregir 'Te apoyo...' a 'Té': {}",
        result
    );

    let result = corrector.correct("Te cuento un secreto importante");
    assert!(
        !result.contains("Te [Té]") && !result.contains("te [té]"),
        "No debería corregir 'Te cuento...' a 'Té': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_te_complica_no_false_tea() {
    let corrector = create_test_corrector();
    let cases = ["Te complica", "No te complica", "Se te complica"];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains("Te [Té]")
                && !result.contains("te [té]")
                && !result.contains("Te [té]")
                && !result.contains("te [Té]"),
            "No debería corregir 'te' a 'té' en contexto clítico con verbo: {} => {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_te_after_possessive_not_tea_before_verb() {
    let corrector = create_test_corrector();
    let cases = [
        "tu te vas ahora",
        "yo no se porque tu te fuistes sin decir nada",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains("te [té]")
                && !result.contains("Te [té]")
                && !result.contains("te [Té]")
                && !result.contains("Te [Té]"),
            "No debe corregir 'te' a 'té' en contexto pronominal: '{}' => {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_arrepentirse_forms_recognized() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Se arrepiente de todo");
    assert!(
        !result.contains("Se [S\u{00E9}]"),
        "No deberia corregir 'Se' a 'Se con tilde' cuando va seguido de verbo pronominal: {}",
        result
    );
    assert!(
        !result.contains("|"),
        "No deberia marcar errores ortograficos en 'arrepiente': {}",
        result
    );

    let result = corrector.correct("Se arrepienten de todo");
    assert!(
        !result.contains("|"),
        "No deberia marcar errores ortograficos en 'arrepienten': {}",
        result
    );

    let result = corrector.correct("Se arrepinti\u{00F3} de todo");
    assert!(
        !result.contains("|"),
        "No deberia marcar errores ortograficos en 'arrepintio': {}",
        result
    );
}

#[test]
fn test_integration_subject_verb_tu_temo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("tú temo");

    assert!(
        result.contains("[temes]"),
        "Debería corregir 'temo' → 'temes': {}",
        result
    );
}

#[test]
fn test_integration_collective_noun_gente_requires_singular_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La gente vinieron temprano");

    assert!(
        result.contains("vinieron [vino]"),
        "Debe corregir colectivo singular 'gente' con verbo plural: {}",
        result
    );
}

#[test]
fn test_integration_collective_noun_familia_requires_singular_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La familia llegaron tarde");

    assert!(
        result.contains("llegaron [llegó]"),
        "Debe corregir colectivo singular 'familia' con verbo plural: {}",
        result
    );
}

#[test]
fn test_integration_collective_noun_equipo_requires_singular_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El equipo ganaron");

    assert!(
        result.contains("ganaron [ganó]"),
        "Debe corregir colectivo singular 'equipo' con verbo plural: {}",
        result
    );
}

#[test]
fn test_integration_variable_collective_without_de_keeps_singular_agreement() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El grupo llegaron tarde");

    assert!(
        result.contains("llegaron [llegó]"),
        "Sin complemento partitivo, 'grupo' debe concordar en singular: {}",
        result
    );
}

#[test]
fn test_integration_variable_collective_with_de_allows_plural_agreement() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El grupo de alumnos llegaron tarde");

    assert!(
        !result.contains("llegaron ["),
        "Con estructura partitiva 'grupo de ...', no debe forzar singular: {}",
        result
    );
}

#[test]
fn test_integration_prefixed_hacer_preterite_pronoun_no_false_positive() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Ella rehizo el trabajo");
    assert!(
        !result.contains("rehizo [rehace]") && !result.contains("rehizo [rehizo]"),
        "No debería corregir 'rehizo' con sujeto singular: {}",
        result
    );

    let result = corrector.correct("Él deshizo el nudo");
    assert!(
        !result.contains("deshizo [deshace]") && !result.contains("deshizo [deshizo]"),
        "No debería corregir 'deshizo' con sujeto singular: {}",
        result
    );
}

#[test]
fn test_integration_prefixed_hacer_preterite_plural_suggestion() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ellos rehizo el trabajo");

    assert!(
        result.contains("rehizo [rehicieron]"),
        "Debería corregir a pretérito plural prefijado: {}",
        result
    );
}

#[test]
fn test_integration_prefixed_irregular_nonclassic_prefixes_recognized() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Ellos depuso el cargo");
    assert!(
        result.contains("depuso [depusieron]"),
        "Debe corregir 'depuso' -> 'depusieron': {}",
        result
    );
    assert!(
        !result.contains("depuso |"),
        "No debe marcar 'depuso' como error ortografico: {}",
        result
    );

    let result = corrector.correct("Ellos opuso resistencia");
    assert!(
        result.contains("opuso [opusieron]"),
        "Debe corregir 'opuso' -> 'opusieron': {}",
        result
    );
    assert!(
        !result.contains("opuso |"),
        "No debe marcar 'opuso' como error ortografico: {}",
        result
    );

    let result = corrector.correct("Ella bendicen");
    assert!(
        result.contains("bendicen [bendice]"),
        "Debe corregir 'bendicen' -> 'bendice': {}",
        result
    );
    assert!(
        !result.contains("bendicen |"),
        "No debe marcar 'bendicen' como error ortografico: {}",
        result
    );

    let result = corrector.correct("Ellos oponga resistencia");
    assert!(
        !result.contains("oponga |"),
        "No debe marcar 'oponga' como error ortografico: {}",
        result
    );

    let result = corrector.correct("Ellos atenga la calma");
    assert!(
        !result.contains("atenga |"),
        "No debe marcar 'atenga' como error ortografico: {}",
        result
    );

    let result = corrector.correct("Ellos decaiga en su animo");
    assert!(
        !result.contains("decaiga |"),
        "No debe marcar 'decaiga' como error ortografico: {}",
        result
    );

    let result = corrector.correct("Que ellos oponga resistencia");
    assert!(
        result.contains("oponga [opongan]"),
        "Debe corregir 'oponga' -> 'opongan' en subjuntivo: {}",
        result
    );

    let result = corrector.correct("Que ella opongan resistencia");
    assert!(
        result.contains("opongan [oponga]"),
        "Debe corregir 'opongan' -> 'oponga' en subjuntivo: {}",
        result
    );

    let result = corrector.correct("Que nosotros oponga resistencia");
    assert!(
        result.contains("oponga [opongamos]"),
        "Debe corregir 'oponga' -> 'opongamos' en subjuntivo: {}",
        result
    );

    let result = corrector.correct("Que vosotros oponga resistencia");
    assert!(
        result.contains("oponga [opongáis]"),
        "Debe corregir 'oponga' -> 'opongáis' en subjuntivo: {}",
        result
    );

    let result = corrector.correct("Que los alumnos oponga resistencia");
    assert!(
        result.contains("oponga [opongan]"),
        "Debe corregir con sujeto nominal plural en subjuntivo: {}",
        result
    );

    let result = corrector.correct("Que el alumno opongan resistencia");
    assert!(
        result.contains("opongan [oponga]"),
        "Debe corregir con sujeto nominal singular en subjuntivo: {}",
        result
    );

    let result = corrector.correct("Que mañana ellos oponga resistencia");
    assert!(
        result.contains("oponga [opongan]"),
        "Debe corregir subjuntivo con adverbio intercalado y sujeto pronominal: {}",
        result
    );

    let result = corrector.correct("Que mañana los alumnos oponga resistencia");
    assert!(
        result.contains("oponga [opongan]"),
        "Debe corregir subjuntivo con adverbio intercalado y sujeto nominal: {}",
        result
    );

    let result = corrector.correct("Tal vez ellos oponga resistencia");
    assert!(
        result.contains("oponga [opongan]"),
        "Debe corregir subjuntivo tras 'tal vez' con sujeto pronominal: {}",
        result
    );

    let result = corrector.correct("Quizás ella opongan resistencia");
    assert!(
        result.contains("opongan [oponga]"),
        "Debe corregir subjuntivo tras 'quizás' con sujeto pronominal: {}",
        result
    );
}

#[test]
fn test_integration_j_to_g_before_e_i_common_verbs_suggested() {
    let corrector = create_test_corrector();
    let cases = [
        ("Coje el paraguas", "coge"),
        ("Recoje los platos", "recoge"),
        ("Escoje el que quieras", "escoge"),
        ("Proteje a tu familia", "protege"),
    ];

    for (text, expected) in cases {
        let result = corrector.correct(text);
        let result_lower = result.to_lowercase();
        assert!(
            result_lower.contains(&format!("|{expected}")),
            "Debe sugerir '{}' como primera opción en '{}': {}",
            expected,
            text,
            result
        );
    }
}

#[test]
fn test_integration_enclitic_usted_double_pronouns_not_marked_as_spelling_error() {
    let corrector = create_test_corrector();
    let samples = ["Dígamelo", "Muéstremelo", "Tráigamelo", "Cuéntemelo"];

    for text in samples {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debería marcar '{}' como error ortográfico: {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_nada_pronoun_not_treated_as_verb() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Yo no se nada");
    assert!(
        !result.contains("nada [nado]"),
        "No debería corregir 'nada' a 'nado' en uso pronominal: {}",
        result
    );

    let result = corrector.correct("Yo nada sé");
    assert!(
        !result.contains("nada [nado]"),
        "No debería corregir 'nada' a 'nado' cuando hay otro verbo finito: {}",
        result
    );
}

#[test]
fn test_integration_possessive_tu_not_corrected() {
    // "tu casa" NO debe cambiar "tu" a "tú" (es posesivo válido)
    let corrector = create_test_corrector();
    let result = corrector.correct("tu casa es bonita");

    // No debe sugerir "tú" cuando es posesivo seguido de sustantivo
    assert!(
        !result.contains("[tú]"),
        "No debería corregir 'tu' posesivo: {}",
        result
    );
}

#[test]
fn test_integration_correct_tu_cantas_unchanged() {
    // "tú cantas" ya es correcto, no debe haber corrección de sujeto-verbo
    let corrector = create_test_corrector();
    let result = corrector.correct("tú cantas muy bien");

    // No debe sugerir cambio en "cantas" (ya concuerda con "tú")
    assert!(
        !result.contains("[cantas]"),
        "No debería cambiar 'cantas' correcto: {}",
        result
    );
    assert!(
        !result.contains("[canto]"),
        "No debería cambiar a 'canto': {}",
        result
    );
}

#[test]
fn test_integration_possessive_after_preposition_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("la casa de nuestro Gobierno");

    assert!(
        !result.contains("nuestro ["),
        "No debería corregir 'nuestro' cuando concuerda con el sustantivo siguiente: {}",
        result
    );
}

#[test]
fn test_integration_cuecen_not_spell_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("yo cuecen");

    assert!(
        result.contains("[cuezo]"),
        "Debería sugerir 'cuezo' en 'yo cuecen': {}",
        result
    );
    assert!(
        !result.contains("[crezco]"),
        "No debería autocorregir hacia 'crecer': {}",
        result
    );
    assert!(
        !result.contains("crecen"),
        "No debería sugerir 'crecen' por ortografía: {}",
        result
    );
}

#[test]
fn test_integration_compound_durmieron_not_spell_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("he durmieron");

    assert!(
        result.contains("[dormido]"),
        "Debería sugerir 'dormido' en 'he durmieron': {}",
        result
    );
    assert!(
        !result.contains("[muerto]"),
        "No debería sugerir 'muerto' por autocorrección a 'murieron': {}",
        result
    );
    assert!(
        !result.to_lowercase().contains("murieron"),
        "No debería autocorregir 'durmieron' a 'murieron': {}",
        result
    );
}

#[test]
fn test_integration_compound_des_prefixed_participle_not_truncated() {
    let corrector = create_test_corrector();
    let cases = [
        "Ha desarticulado una banda",
        "Ha desaconsejado el pago",
        "Ha desconectado el aparato",
        "Ha deshabilitado la cuenta",
        "Ha desinstalado el programa",
        "Ha descentralizado el poder",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains("[articulado]")
                && !result.contains("[aconsejado]")
                && !result.contains("[conectado]")
                && !result.contains("[habilitado]")
                && !result.contains("[instalado]")
                && !result.contains("[centralizado]"),
            "No debería eliminar prefijo 'des-' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_compound_haya_lo_que_haya_no_cross_clause_without_comma() {
    let corrector = create_test_corrector();
    let result = corrector.correct("haya lo que haya seguiremos");

    assert!(
        !result.contains("seguiremos [seguido]"),
        "No debería tratar 'seguiremos' como participio tras clausula concesiva: {}",
        result
    );
}

#[test]
fn test_integration_compound_haya_o_no_haya_no_cross_clause_without_comma() {
    let corrector = create_test_corrector();
    let result = corrector.correct("haya o no haya seguiremos");

    assert!(
        !result.contains("seguiremos [seguido]"),
        "No debería tratar 'seguiremos' como participio tras clausula concesiva: {}",
        result
    );
}

#[test]
fn test_integration_durmieron_not_spell_marked() {
    let corrector = create_test_corrector();
    let result = corrector.correct("durmieron bien");

    assert!(
        !result.contains("durmieron |"),
        "No debería marcar 'durmieron' como error ortográfico: {}",
        result
    );
}

#[test]
fn test_integration_royo_not_spell_marked() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El perro royó el hueso");

    assert!(
        !result.contains("royó |"),
        "No debería marcar 'royó' como error ortográfico: {}",
        result
    );
}

#[test]
fn test_integration_leyeron_not_spell_marked() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ellos leyeron el libro");

    assert!(
        !result.contains("leyeron |"),
        "No debería marcar 'leyeron' como error ortográfico: {}",
        result
    );
}

#[test]
fn test_integration_derived_plural_not_spell_marked() {
    // "abuelas" no está en el diccionario, pero "abuela" sí.
    // Debe reconocerse como plural derivado y NO marcarse como error ortográfico.
    let corrector = create_test_corrector();
    let result = corrector.correct("las abuelas son sabias");

    assert!(
        !result.contains("abuelas |"),
        "No debería marcar 'abuelas' como error ortográfico: {}",
        result
    );
}

#[test]
fn test_integration_article_agreement_with_derived_plural() {
    // Sin plural derivado, "el abuelas" no podía corregirse por falta de word_info.
    let corrector = create_test_corrector();
    let result = corrector.correct("el abuelas");

    assert!(
        result.to_lowercase().contains("[las]"),
        "Debería corregir 'el' → 'las' con plural derivado: {}",
        result
    );
    assert!(
        !result.contains("abuelas |"),
        "No debería marcar 'abuelas' como error ortográfico: {}",
        result
    );
}

#[test]
fn test_integration_possessive_vuestro_after_preposition_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("la sede de vuestro partido");

    assert!(
        !result.contains("vuestro ["),
        "No debería corregir 'vuestro' cuando concuerda con el sustantivo siguiente: {}",
        result
    );
}

#[test]
fn test_integration_possessive_vuestra_partido_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("vuestra partido gana");

    assert!(
        result.contains("[Vuestro]"),
        "Debería corregir 'vuestra partido' -> 'vuestro': {}",
        result
    );
}

#[test]
fn test_integration_indefinite_quantifier_muchas_problema_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Muchas problema");

    assert!(
        result.contains("Muchas [") || result.contains("problema ["),
        "Deberia corregir cuantificador indefinido en genero/numero: {}",
        result
    );
}

#[test]
fn test_integration_indefinite_quantifier_varios_problema_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Varios problema");

    assert!(
        result.contains("Varios [") || result.contains("problema ["),
        "Deberia detectar concordancia con cuantificador indefinido: {}",
        result
    );
}

#[test]
fn test_integration_indefinite_quantifier_todas_los_ninos_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Todas los niños");

    assert!(
        result.contains("Todas [Todos]"),
        "Deberia corregir cuantificador en 'Todas los niños': {}",
        result
    );
}

#[test]
fn test_integration_indefinite_quantifier_todos_las_casas_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Todos las casas");

    assert!(
        result.contains("Todos [Todas]"),
        "Deberia corregir cuantificador en 'Todos las casas': {}",
        result
    );
}

#[test]
fn test_integration_indefinite_quantifier_ningun_personas_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ningun personas");

    assert!(
        result.contains("personas [persona]"),
        "Deberia singularizar el sustantivo en 'Ningun personas': {}",
        result
    );
    assert!(
        !result.to_lowercase().contains("ningunos") && !result.to_lowercase().contains("ningunas"),
        "No deberia proponer formas no estandar tipo 'ningunos/ningunas': {}",
        result
    );
}

#[test]
fn test_integration_indefinite_quantifier_ningun_libros_singularizes_noun() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ningun libros");

    assert!(
        result.contains("libros [libro]"),
        "Deberia singularizar el sustantivo en 'Ningun libros': {}",
        result
    );
    assert!(
        !result.to_lowercase().contains("ningunos") && !result.to_lowercase().contains("ningunas"),
        "No deberia proponer 'ningunos' en 'Ningun libros': {}",
        result
    );
}

#[test]
fn test_integration_indefinite_quantifier_algun_problemas_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Algún problemas");

    assert!(
        result.contains("Algún ["),
        "Deberia corregir cuantificador indefinido en 'Algún problemas': {}",
        result
    );
}

#[test]
fn test_integration_quantifier_mucho_as_adverb_not_forced_plural() {
    let corrector = create_test_corrector();
    let cases = [
        "Me gustan mucho los libros",
        "Trabaja mucho las tardes",
        "Habla mucho las noches",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.to_lowercase().contains("mucho [much"),
            "No deberia convertir 'mucho' adverbial en cuantificador en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_quantifier_demasiado_adverb_before_caras_not_forced() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Son demasiado caras");
    assert!(
        !result.to_lowercase().contains("demasiado [demasiad"),
        "No deberia convertir 'demasiado' adverbial en determinante en 'Son demasiado caras': {}",
        result
    );

    let result = corrector.correct("Tengo demasiado casas");
    assert!(
        result.to_lowercase().contains("demasiado [demasiadas]"),
        "Debe mantener la correccion de cuantificador cuando si modifica un sustantivo: {}",
        result
    );
}

#[test]
fn test_integration_quantifier_demasiado_adverb_before_tarde_not_forced() {
    let corrector = create_test_corrector();
    let cases = [
        "Llego demasiado tarde",
        "El aviso llego demasiado tarde",
        "Demasiado tarde llegaron",
        "Demasiado tarde para actuar",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.to_lowercase().contains("demasiado [demasiada]"),
            "No deberia convertir 'demasiado' adverbial en determinante en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_capitalizes_possessive_sentence_start() {
    let corrector = create_test_corrector();
    let result = corrector.correct("nuestro partido gana");

    assert!(
        result.contains("[Nuestro]"),
        "Debería capitalizar el determinante al inicio de oración: {}",
        result
    );
}

#[test]
fn test_integration_capitalizes_gender_correction_sentence_start() {
    let corrector = create_test_corrector();
    let result = corrector.correct("la partido gana");

    assert!(
        result.contains("[El]"),
        "Debería capitalizar la corrección de género al inicio de oración: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público y privado");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos coordinados: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_commas_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público, privado y mixto");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con comas: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_ni_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público ni privado");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con 'ni': {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_asyndetic_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público, privado, mixto");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos sin conjunción: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_o_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público o privado");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con 'o': {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_u_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público u oficial");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con 'u': {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_ni_twice_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público ni privado ni mixto");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con 'ni... ni...': {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_parentheses_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público (y privado)");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con paréntesis: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_parenthetical_list_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público (privado y mixto)");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con lista parentética: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_quotes_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público \"privado\" y mixto");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con comillas: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_angle_quotes_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público «privado» y mixto");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con comillas angulares: {}",
        result
    );
}

#[test]
fn test_integration_distributive_adjectives_with_em_dash_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("los sectores público — privado — y mixto");

    assert!(
        !result.contains("[públicos]"),
        "No debería corregir adjetivos distributivos con guiones largos: {}",
        result
    );
}

#[test]
fn test_integration_tu_mando_corrected() {
    // "tu mando" → "tú mandas"
    // "mando" termina en -ando pero NO es gerundio; es 1ª persona de "mandar"
    let corrector = create_test_corrector();
    let result = corrector.correct("tu mando aquí");

    // Debe corregir "tu" → "tú" (porque "mando" es verbo)
    assert!(
        result.contains("[tú]") || result.contains("[Tú]"),
        "Debería corregir 'tu' → 'tú' cuando va seguido de verbo 'mando': {}",
        result
    );
    // Debe corregir "mando" → "mandas" (concordancia tú + verbo)
    assert!(
        result.contains("[mandas]"),
        "Debería corregir 'mando' → 'mandas' (concordancia con tú): {}",
        result
    );
}

#[test]
fn test_integration_spelling_then_grammar() {
    // Verifica que correcciones ortográficas son visibles para gramática
    // (si hay un error ortográfico que al corregirse afecta el análisis)
    let corrector = create_test_corrector();

    // "el" + sustantivo femenino corregido ortográficamente
    // Este test verifica el flujo ortografía → gramática
    let result = corrector.correct("la casa blanco");
    assert!(
        result.contains("[blanca]"),
        "Debería corregir concordancia: {}",
        result
    );
}

#[test]
fn test_integration_spelling_propagates_to_article_noun() {
    // Verifica que la corrección ortográfica propaga word_info a la gramática
    // "este cassa" → spelling: "cassa"→"casa" → grammar debe ver "casa" (fem)
    // y corregir "este"→"esta"
    let corrector = create_test_corrector();
    let result = corrector.correct("este cassa blanca");

    // Debe corregir ortografía: "cassa" (primera sugerencia es "casa")
    assert!(
        result.contains("|casa,") || result.contains("|casa|"),
        "Debería sugerir 'casa' para 'cassa': {}",
        result
    );
    // Debe corregir gramática: "este" → "esta" (porque "casa" es femenino)
    assert!(
        result.contains("[Esta]"),
        "Debería corregir 'este' → 'esta': {}",
        result
    );
}

#[test]
fn test_integration_spelling_propagates_to_adjective() {
    // Similar pero con adjetivo: "la cassa blanco"
    // spelling: "cassa"→"casa" → grammar debe ver "casa" (fem) y corregir "blanco"→"blanca"
    let corrector = create_test_corrector();
    let result = corrector.correct("la cassa blanco");

    // Debe corregir ortografía: "cassa" (primera sugerencia es "casa")
    assert!(
        result.contains("|casa,") || result.contains("|casa|"),
        "Debería sugerir 'casa' para 'cassa': {}",
        result
    );
    // Debe corregir gramática: "blanco" → "blanca"
    assert!(
        result.contains("[blanca]"),
        "Debería corregir 'blanco' → 'blanca': {}",
        result
    );
}

#[test]
fn test_integration_unknown_word_suggestions_do_not_force_article_gender() {
    let corrector = create_test_corrector();
    let result = corrector.correct("el stock");

    assert!(
        !result.contains("[La]"),
        "No debería forzar corrección de artículo con sugerencias ortográficas ambiguas: {}",
        result
    );
}

#[test]
fn test_integration_unknown_word_suggestions_do_not_force_determiner_gender() {
    let corrector = create_test_corrector();
    let result = corrector.correct("este stock");

    assert!(
        !result.contains("[Esta]"),
        "No debería forzar corrección de determinante con sugerencias ortográficas ambiguas: {}",
        result
    );
}

#[test]
fn test_integration_unknown_ascii_word_suggestions_do_not_force_adjective_gender() {
    let corrector = create_test_corrector();
    let result = corrector.correct("el stock caro");

    assert!(
        !result.contains("[cara]"),
        "No debería forzar corrección de adjetivo con sustantivo ASCII desconocido ambiguo: {}",
        result
    );
}

#[test]
fn test_integration_unknown_ascii_word_uses_determiner_for_adjective_number() {
    let corrector = create_test_corrector();
    let result = corrector.correct("el stock caros");

    assert!(
        result.contains("[caro]"),
        "Debería corregir número del adjetivo usando el determinante previo: {}",
        result
    );
    assert!(
        !result.contains("[cara]"),
        "No debería forzar género de candidato ortográfico ambiguo: {}",
        result
    );
}

// ==========================================================================
// Tests de palabras compuestas con guión
// ==========================================================================

#[test]
fn test_compound_word_proper_names() {
    // "Madrid-Sevilla" debe ser aceptado (dos nombres propios)
    let corrector = create_test_corrector();
    let result = corrector.correct("la línea Madrid-Sevilla");
    assert!(
        !result.contains("|?|"),
        "No debería marcar Madrid-Sevilla como desconocida: {}",
        result
    );
    assert!(
        !result.contains("Madrid-Sevilla |"),
        "No debería haber corrección para Madrid-Sevilla: {}",
        result
    );
}

#[test]
fn test_compound_word_mixed() {
    // "norte-sur" debe ser aceptado (dos palabras del diccionario)
    let corrector = create_test_corrector();
    let result = corrector.correct("dirección norte-sur");
    assert!(
        !result.contains("|?|"),
        "No debería marcar norte-sur como desconocida: {}",
        result
    );
}

#[test]
fn test_compound_word_invalid() {
    // "asdfg-qwerty" no debe ser aceptado (palabras inexistentes)
    let corrector = create_test_corrector();
    let result = corrector.correct("esto es asdfg-qwerty");
    assert!(
        result.contains("|?|") || result.contains("|"),
        "Debería marcar como desconocida: {}",
        result
    );
}

#[test]
fn test_proper_name_ai() {
    // "AI" debe ser reconocido como nombre propio (siglas)
    let corrector = create_test_corrector();
    let result = corrector.correct("Figure AI apunta a la industria");
    // No debe sugerir corrección para "AI"
    assert!(
        !result.contains("AI |"),
        "No debería corregir AI como error ortográfico: {}",
        result
    );
    assert!(
        !result.contains("[Ay]"),
        "No debería sugerir 'Ay' para AI: {}",
        result
    );
}

#[test]
fn test_verb_car_orthographic_change() {
    // "indique" es subjuntivo de "indicar" (c→qu antes de e)
    let corrector = create_test_corrector();

    // Test is_word_known directly
    assert!(
        corrector.is_word_known("indique"),
        "'indique' debería ser reconocido como forma verbal de 'indicar'"
    );
    assert!(
        corrector.is_word_known("aplique"),
        "'aplique' debería ser reconocido"
    );
    assert!(
        corrector.is_word_known("busqué"),
        "'busqué' debería ser reconocido"
    );

    // Test in full correction context
    let result = corrector.correct("a menos que el fabricante indique lo contrario");
    assert!(
        !result.contains("indique |"),
        "No debería marcar 'indique' como error: {}",
        result
    );
}

#[test]
fn test_whitespace_preserved_after_correction() {
    // Los saltos de línea y tabs deben preservarse después de correcciones
    let corrector = create_test_corrector();

    // Test con salto de línea después de palabra corregida
    let result = corrector.correct("cassa grande\ncasa pequeña");
    assert!(
        result.contains('\n'),
        "Debería preservar el salto de línea: {:?}",
        result
    );

    // Verificar que hay exactamente 2 líneas
    let line_count = result.lines().count();
    assert_eq!(
        line_count, 2,
        "Debería tener 2 líneas, tiene {}: {:?}",
        line_count, result
    );
}

#[test]
fn test_whitespace_preserved_tab_after_grammar_correction() {
    // Los tabs deben preservarse después de correcciones gramaticales
    let corrector = create_test_corrector();

    // "el casa" → "la casa" con tab después
    let result = corrector.correct("el casa\tgrande");
    assert!(
        result.contains("[La]"),
        "Debería corregir 'el' → 'la': {}",
        result
    );
    assert!(
        result.contains('\t'),
        "Debería preservar el tab: {:?}",
        result
    );
}

#[test]
fn test_whitespace_preserved_multiple_newlines() {
    // Múltiples saltos de línea deben preservarse
    let corrector = create_test_corrector();

    let result = corrector.correct("el casa\n\ngrande");
    assert!(
        result.contains("\n\n"),
        "Debería preservar los dos saltos de línea: {:?}",
        result
    );
}

#[test]
fn test_whitespace_preserved_crlf() {
    // CRLF (Windows) debe preservarse
    let corrector = create_test_corrector();

    let result = corrector.correct("el casa\r\ngrande");
    assert!(
        result.contains("\r\n"),
        "Debería preservar CRLF: {:?}",
        result
    );
}

// ==========================================================================
// Tests de integración para número entre artículo y sustantivo
// ==========================================================================

#[test]
fn test_number_between_unit_no_correction() {
    // "los 10 MB" no debe corregirse - MB es unidad invariable
    let corrector = create_test_corrector();
    let result = corrector.correct("los 10 MB de RAM");

    // No debe haber corrección de artículo (solo mayúscula inicial)
    assert!(
        !result.contains("[las]"),
        "No debería corregir 'los' a 'las' con unidad MB: {}",
        result
    );
}

#[test]
fn test_number_between_currency_corrects() {
    // "la 10 euros" debe corregirse a "los 10 euros"
    let corrector = create_test_corrector();
    let result = corrector.correct("cuesta la 10 euros");

    assert!(
        result.contains("[los]"),
        "Debería corregir 'la' a 'los' con moneda: {}",
        result
    );
}

#[test]
fn test_number_between_regular_noun_corrects() {
    // "los 3 casas" debe corregirse a "las 3 casas"
    let corrector = create_test_corrector();
    let result = corrector.correct("tengo los 3 casas");

    assert!(
        result.contains("[las]"),
        "Debería corregir 'los' a 'las' con sustantivo regular: {}",
        result
    );
}

#[test]
fn test_integration_numeral_noun_singular_is_corrected() {
    let corrector = create_test_corrector();

    let result_libro = corrector.correct("Compre dos libro");
    assert!(
        result_libro.contains("libro [libros]"),
        "Deberia corregir 'dos libro' -> 'dos libros': {}",
        result_libro
    );

    let result_gato = corrector.correct("Hay tres gato");
    assert!(
        result_gato.contains("gato [gatos]"),
        "Deberia corregir 'tres gato' -> 'tres gatos': {}",
        result_gato
    );

    let result_entrada = corrector.correct("Quedan cinco entrada");
    assert!(
        result_entrada.contains("entrada [entradas]"),
        "Deberia corregir 'cinco entrada' -> 'cinco entradas': {}",
        result_entrada
    );
}

// ==========================================================================
// Tests de fallback para verbos sin infinitivo en diccionario
// ==========================================================================

#[test]
fn test_verb_fallback_with_subject_pronoun() {
    // "Ellos cliquearon" no debe marcarse como error (aunque "cliquear" no está)
    let corrector = create_test_corrector();
    let result = corrector.correct("Ellos cliquearon el botón");

    assert!(
        !result.contains("|?|"),
        "No debería marcar 'cliquearon' como desconocida: {}",
        result
    );
    assert!(
        !result.contains("cliquearon |"),
        "No debería haber corrección para 'cliquearon': {}",
        result
    );
}

#[test]
fn test_verb_fallback_with_que() {
    // "que instanciaron" no debe marcarse como error
    let corrector = create_test_corrector();
    let result = corrector.correct("Los objetos que instanciaron funcionan");

    assert!(
        !result.contains("|?|"),
        "No debería marcar 'instanciaron' como desconocida: {}",
        result
    );
    assert!(
        !result.contains("instanciaron |"),
        "No debería haber corrección para 'instanciaron': {}",
        result
    );
}

#[test]
fn test_verb_fallback_with_object_pronoun() {
    // "los cliquearon" no debe marcarse (pronombre objeto precede verbo)
    let corrector = create_test_corrector();
    let result = corrector.correct("Los usuarios los cliquearon");

    assert!(
        !result.contains("|?|"),
        "No debería marcar 'cliquearon' como desconocida: {}",
        result
    );
}

#[test]
fn test_verb_fallback_without_context_marks_error() {
    // "El zumbificaron" debe marcarse como error (artículo, no pronombre)
    // Usamos verbo inventado para que no esté en diccionario
    let corrector = create_test_corrector();
    let result = corrector.correct("El zumbificaron fue rápido");

    assert!(
        result.contains("|?|"),
        "Debería marcar 'zumbificaron' sin contexto verbal: {}",
        result
    );
}

#[test]
fn test_verb_fallback_gerund_with_se() {
    // "se renderizando" no debe marcarse (se + gerundio)
    let corrector = create_test_corrector();
    let result = corrector.correct("La página se está renderizando");

    assert!(
        !result.contains("renderizando |"),
        "No debería marcar 'renderizando': {}",
        result
    );
}

#[test]
fn test_verb_fallback_imperfect_with_pronoun() {
    // "Nosotros deployábamos" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Nosotros deployábamos el código");

    assert!(
        !result.contains("|?|"),
        "No debería marcar 'deployábamos' como desconocida: {}",
        result
    );
}

// ==========================================================================
// Tests de unidades mixtas (kWh, mAh, dB, Mbps, etc.)
// ==========================================================================

#[test]
fn test_unit_mah_with_number() {
    // "5000 mAh" no debe marcarse como error
    let corrector = create_test_corrector();
    let result = corrector.correct("La batería de 5000 mAh dura mucho");

    assert!(
        !result.contains("mAh |"),
        "No debería marcar 'mAh' como error: {}",
        result
    );
}

#[test]
fn test_unit_mbps_with_number() {
    // "100 Mbps" no debe marcarse como error
    let corrector = create_test_corrector();
    let result = corrector.correct("Conexión de 100 Mbps");

    assert!(
        !result.contains("Mbps |"),
        "No debería marcar 'Mbps' como error: {}",
        result
    );
}

#[test]
fn test_unit_kwh_with_number() {
    // "100 kWh" no debe marcarse como error
    let corrector = create_test_corrector();
    let result = corrector.correct("El coche tiene 100 kWh de batería");

    assert!(
        !result.contains("kWh |"),
        "No debería marcar 'kWh' como error: {}",
        result
    );
}

#[test]
fn test_unit_db_with_number() {
    // "85 dB" no debe marcarse como error
    let corrector = create_test_corrector();
    let result = corrector.correct("Potencia de 85 dB");

    assert!(
        !result.contains("dB |"),
        "No debería marcar 'dB' como error: {}",
        result
    );
}

#[test]
fn test_unit_without_number_marks_error() {
    // "El mAh es" debe marcarse (no hay número precedente)
    let corrector = create_test_corrector();
    let result = corrector.correct("El mAh es una unidad");

    assert!(
        result.contains("mAh |"),
        "Debería marcar 'mAh' sin número: {}",
        result
    );
}

// ==========================================================================
// Tests de unidades con barra (km/h, m/s, etc.)
// ==========================================================================

#[test]
fn test_unit_km_per_h() {
    // "100 km/h" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Velocidad de 100 km/h");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '100 km/h': {}",
        result
    );
}

#[test]
fn test_unit_m_per_s_squared() {
    // "10 m/s²" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Aceleración de 10 m/s²");

    assert!(
        !result.contains("s² |"),
        "No debería marcar 's²': {}",
        result
    );
}

#[test]
fn test_unit_m3_per_s() {
    // "5 m³/s" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Flujo de 5 m³/s");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '5 m³/s': {}",
        result
    );
}

// ==========================================================================
// Tests de temperatura (°C, °F)
// ==========================================================================

#[test]
fn test_unit_celsius() {
    // "20 °C" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Temperatura de 20 °C");

    assert!(
        !result.contains("C |"),
        "No debería marcar 'C' tras °: {}",
        result
    );
}

#[test]
fn test_unit_fahrenheit() {
    // "68 °F" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Temperatura de 68 °F");

    assert!(
        !result.contains("F |"),
        "No debería marcar 'F' tras °: {}",
        result
    );
}

// ==========================================================================
// Tests de mediciones técnicas sin espacio (100km/h, 10m/s²)
// ==========================================================================

#[test]
fn test_unit_km_per_h_no_space() {
    // "100km/h" (sin espacio) no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Velocidad de 100km/h");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '100km/h': {}",
        result
    );
}

#[test]
fn test_unit_m_per_s_squared_no_space() {
    // "10m/s²" (sin espacio) no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Aceleración de 10m/s²");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '10m/s²': {}",
        result
    );
}

// ==========================================================================
// Tests de exponentes ASCII (m/s^2, m/s2)
// ==========================================================================

#[test]
fn test_unit_ascii_exponent_caret() {
    // "10 m/s^2" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Aceleración de 10 m/s^2");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '10 m/s^2': {}",
        result
    );
}

#[test]
fn test_unit_ascii_exponent_digit() {
    // "10 m/s2" no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Aceleración de 10 m/s2");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '10 m/s2': {}",
        result
    );
}

#[test]
fn test_unit_superscript_no_space() {
    // "100m²/s" (sin espacio, superíndice en numerador) no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Flujo de 100m²/s");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '100m²/s': {}",
        result
    );
}

#[test]
fn test_unit_ascii_exponent_no_space() {
    // "100m^2/s" (sin espacio, exponente ASCII en numerador) no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Flujo de 100m^2/s");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '100m^2/s': {}",
        result
    );
}

#[test]
fn test_unit_negative_exponent() {
    // "5 s^-1" (exponente negativo con espacio) no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Frecuencia de 5 s^-1");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '5 s^-1': {}",
        result
    );
}

#[test]
fn test_unit_negative_exponent_no_space() {
    // "5s^-1" (exponente negativo sin espacio) no debe marcarse
    let corrector = create_test_corrector();
    let result = corrector.correct("Frecuencia de 5s^-1");

    assert!(
        !result.contains("|"),
        "No debería haber errores en '5s^-1': {}",
        result
    );
}

// ==========================================================================
// Tests de integración para género común con referente
// ==========================================================================

#[test]
fn test_integration_common_gender_el_periodista_maria() {
    // Pipeline completo: "el periodista María" → "la periodista María"
    let corrector = create_test_corrector();
    let result = corrector.correct("el periodista María García informó");

    assert!(
        result.contains("[La]"),
        "Debería corregir 'el' → 'la' por referente femenino: {}",
        result
    );
}

#[test]
fn test_integration_common_gender_la_premio_nobel_maria() {
    // Pipeline completo: "la premio Nobel María" → NO debe corregirse
    // La gramática quiere cambiar "la" a "el" pero el referente "María" es femenino
    let corrector = create_test_corrector();
    let result = corrector.correct("la premio Nobel María Curie fue científica");

    // NO debe haber corrección de artículo (la gramática lo intentó pero fue anulado)
    assert!(
        !result.contains("[el]"),
        "No debería corregir 'la' a 'el' cuando hay referente femenino: {}",
        result
    );
}

#[test]
fn test_integration_common_gender_el_premio_nobel_maria() {
    // Pipeline completo: "el premio Nobel María" → "la premio Nobel María"
    let corrector = create_test_corrector();
    let result = corrector.correct("el premio Nobel María Curie fue científica");

    assert!(
        result.contains("[La]"),
        "Debería corregir 'el' → 'la' por referente femenino: {}",
        result
    );
}

#[test]
fn test_integration_common_gender_premio_non_title_no_false_positive() {
    let corrector = create_test_corrector();

    let result = corrector.correct("El premio lo ganó María");
    assert!(
        !result.contains("El [La] premio") && !result.contains("el [la] premio"),
        "No debería corregir 'el premio' en uso nominal normal: {}",
        result
    );

    let result = corrector.correct("El premio fue para María");
    assert!(
        !result.contains("El [La] premio") && !result.contains("el [la] premio"),
        "No debería corregir 'el premio' en uso nominal normal: {}",
        result
    );

    let result = corrector.correct("El premio lo recibió Ana");
    assert!(
        !result.contains("El [La] premio") && !result.contains("el [la] premio"),
        "No debería corregir 'el premio' en uso nominal normal: {}",
        result
    );
}

#[test]
fn test_integration_common_gender_without_referent() {
    // Sin referente, la gramática decide según el género del diccionario
    let corrector = create_test_corrector();
    let result = corrector.correct("el premio Nobel es importante");

    // "premio" es masculino en el diccionario, "el premio" es correcto
    assert!(
        !result.contains("[la]"),
        "Sin referente no debe cambiar el artículo: {}",
        result
    );
}

#[test]
fn test_integration_feminine_tonic_a_el_acta_no_correction() {
    let corrector = create_test_corrector();

    let result = corrector.correct("el acta");
    assert!(
        !result.contains("el [la]") && !result.contains("El [La]"),
        "No debería corregir 'el acta': {}",
        result
    );

    let result = corrector.correct("he revisado el acta");
    assert!(
        !result.contains("el [la]") && !result.contains("El [La]"),
        "No debería corregir el artículo en 'he revisado el acta': {}",
        result
    );
}

#[test]
fn test_integration_common_gender_sentence_boundary() {
    // El punto impide que "María" sea referente de "periodista"
    let corrector = create_test_corrector();
    let result = corrector.correct("el periodista llegó. María también llegó");

    // No debe haber corrección de "el" porque "María" está en otra oración
    assert!(
        !result.contains("el [la]"),
        "No debería cruzar límite de oración: {}",
        result
    );
}

#[test]
fn test_integration_common_gender_la_lider_ana() {
    // "la líder Ana" es correcto (femenino + referente femenino)
    let corrector = create_test_corrector();
    let result = corrector.correct("la líder Ana García habló");

    // No debe haber corrección
    assert!(
        !result.contains("[el]"),
        "No debería cambiar 'la' cuando es correcto: {}",
        result
    );
}

#[test]
fn test_integration_common_gender_plural_relative_postposed_subject_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La noticia que publicaron los periodistas");

    assert!(
        !result.contains("los [las]"),
        "No debería forzar género en plural de sustantivo común dentro de relativa: {}",
        result
    );
}

#[test]
fn test_integration_relative_transitive_object_implicit_subject_redactar_not_forced() {
    let corrector = create_test_corrector();
    let result = corrector.correct("He revisado el acta que redactaron durante toda la noche");

    assert!(
        !result.contains("redactaron [redactó]"),
        "No debería forzar singular en relativo de objeto transitivo con sujeto implícito plural: {}",
        result
    );
}

#[test]
fn test_integration_relative_transitive_object_implicit_subject_revisar_not_forced() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El informe que revisaron durante toda la tarde está listo");

    assert!(
        !result.contains("revisaron [revisó]"),
        "No debería forzar singular en relativo de objeto transitivo con 'revisar': {}",
        result
    );
}

#[test]
fn test_integration_relative_transitive_object_implicit_subject_definir_not_forced() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La estrategia que definieron en la sesión anterior se mantuvo");

    assert!(
        !result.contains("definieron [definió]"),
        "No debería forzar singular en relativo de objeto transitivo con 'definir': {}",
        result
    );
}

#[test]
fn test_integration_relative_transitive_object_implicit_subject_presentar_not_forced() {
    let corrector = create_test_corrector();
    let result =
        corrector.correct("El resumen que presentaron en la reunión pasada quedó aprobado");

    assert!(
        !result.contains("presentaron [presentó]"),
        "No debería forzar singular en relativo de objeto transitivo con 'presentar': {}",
        result
    );
}

#[test]
fn test_integration_relative_temporal_preposition_not_forced_singular_main_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Las paredes que pintaron durante toda la noche están secas");

    assert!(
        !result.contains("están [está]"),
        "No debería forzar singular en verbo principal por 'durante toda la noche' dentro de relativa: {}",
        result
    );
}

#[test]
fn test_integration_relative_temporal_preposition_object_not_forced_plural_main_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct(
        "El comité que revisó durante toda la mañana los informes confirma los resultados",
    );

    assert!(
        !result.contains("confirma [confirman]"),
        "No debería forzar plural en verbo principal por objeto dentro de relativa temporal: {}",
        result
    );
}

#[test]
fn test_integration_relative_postposed_proper_name_maria_no_hang_or_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La lista que completaron María está actualizada");

    assert!(
        !result.contains("completaron ["),
        "No debería corregir 'completaron' con sujeto pospuesto de nombre propio: {}",
        result
    );
}

#[test]
fn test_integration_gustar_like_postposed_plural_subject_agreement() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Me gusta los perros");

    assert!(
        result.contains("gusta [gustan]"),
        "Debería corregir 'Me gusta los perros' -> 'Me gustan los perros': {}",
        result
    );
}

#[test]
fn test_integration_gustar_like_postposed_plural_subject_variants() {
    let corrector = create_test_corrector();
    let cases = [
        ("Le molesta los ruidos", "molesta [molestan]"),
        ("Nos preocupa las noticias", "preocupa [preocupan]"),
        ("Te interesa los libros", "interesa [interesan]"),
        ("Me duele las piernas", "duele [duelen]"),
        ("Nos falta dos días", "falta [faltan]"),
        ("Le sobra motivos", "sobra [sobran]"),
        ("Me encanta los planes", "encanta [encantan]"),
        ("Le fascina los documentales", "fascina [fascinan]"),
        ("Nos apetece unas vacaciones", "apetece [apetecen]"),
        ("Te agrada los cambios", "agrada [agradan]"),
        ("Me disgusta los ruidos", "disgusta [disgustan]"),
        ("Le importa los detalles", "importa [importan]"),
        ("Nos conviene las medidas", "conviene [convienen]"),
        ("Les corresponde los premios", "corresponde [corresponden]"),
        ("Le pertenece esos terrenos", "pertenece [pertenecen]"),
        ("Nos basta dos ejemplos", "basta [bastan]"),
    ];

    for (text, expected_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains(expected_fragment),
            "Debería corregir concordancia en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_gustar_like_no_false_positive_in_clause_or_infinitive() {
    let corrector = create_test_corrector();

    let result_clause = corrector.correct("Me importa que vengas");
    assert!(
        !result_clause.contains("importa [importan]"),
        "No deberia forzar plural con subordinada como sujeto: {}",
        result_clause
    );

    let result_inf = corrector.correct("Me encanta correr");
    assert!(
        !result_inf.contains("encanta [encantan]"),
        "No deberia forzar plural con infinitivo como sujeto: {}",
        result_inf
    );

    let result_clause_with_como = corrector.correct("Le gusta como cocinas");
    assert!(
        !result_clause_with_como.contains("gusta [gustan]"),
        "No deberia forzar plural con subordinada interrogativa como sujeto: {}",
        result_clause_with_como
    );
}

#[test]
fn test_integration_reflexive_passive_se_singular_with_postposed_plural() {
    let corrector = create_test_corrector();

    let result_vende = corrector.correct("Se vende pisos");
    assert!(
        result_vende.contains("vende [venden]"),
        "Deberia corregir 'Se vende pisos' -> 'Se venden pisos': {}",
        result_vende
    );

    let result_busca = corrector.correct("Se busca empleados");
    assert!(
        result_busca.contains("busca [buscan]"),
        "Deberia corregir 'Se busca empleados' -> 'Se buscan empleados': {}",
        result_busca
    );

    let result_prohibe = corrector.correct("Se proh\u{00ED}be las motos");
    assert!(
        result_prohibe.contains("proh\u{00ED}be [proh\u{00ED}ben]"),
        "Deberia corregir 'Se proh\u{00ED}be las motos' -> 'Se proh\u{00ED}ben las motos': {}",
        result_prohibe
    );

    let result_busca_adverb = corrector.correct("Se busca urgentemente empleados");
    assert!(
        result_busca_adverb.contains("busca [buscan]"),
        "Deberia corregir pasiva refleja con adverbio en -mente: {}",
        result_busca_adverb
    );
}

#[test]
fn test_integration_reflexive_passive_se_singular_with_postposed_singular_no_change() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Se vende piso");

    assert!(
        !result.contains("vende [venden]"),
        "No deberia corregir cuando el SN pospuesto es singular: {}",
        result
    );
}

#[test]
fn test_integration_reflexive_body_part_not_forced_to_passive_plural() {
    let corrector = create_test_corrector();
    let cases = [
        ("Maria se lava las manos", "lava [lavan]"),
        ("Juan se pone los zapatos", "pone [ponen]"),
        ("Ana se cepilla los dientes", "cepilla [cepillan]"),
        ("Se corta las unas", "corta [cortan]"),
        ("Se pinta las unas", "pinta [pintan]"),
        ("Se pone los zapatos", "pone [ponen]"),
        ("Se quita los guantes", "quita [quitan]"),
        ("Se ata los cordones", "ata [atan]"),
        ("Se seca las lagrimas", "seca [secan]"),
        ("Se frota los ojos", "frota [frotan]"),
        ("Se muerde las unas", "muerde [muerden]"),
        ("Se toca los labios", "toca [tocan]"),
        ("Se rasca las piernas", "rasca [rascan]"),
    ];

    for (input, wrong_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            !result.contains(wrong_fragment),
            "No deberia forzar pasiva refleja en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_homophone_hecho_de_menos() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hecho de menos a mi familia");

    assert!(
        result.contains("Hecho [Echo]"),
        "Debería corregir 'Hecho de menos' -> 'Echo de menos': {}",
        result
    );
}

#[test]
fn test_integration_homophone_un_echo_noun() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es un echo conocido");

    assert!(
        result.contains("echo [hecho]"),
        "Debería corregir 'un echo' -> 'un hecho': {}",
        result
    );
}

#[test]
fn test_integration_homophone_el_echo_de_que() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El echo de que no viniera");

    assert!(
        result.contains("echo [hecho]"),
        "Debería corregir 'el echo de que' -> 'el hecho de que': {}",
        result
    );
}

#[test]
fn test_integration_homophone_cada_echo_noun() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Cada echo preocupa");

    assert!(
        result.contains("echo [hecho]"),
        "Deberia corregir 'cada echo' -> 'cada hecho': {}",
        result
    );
}

#[test]
fn test_integration_homophone_todo_echo_noun() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Todo echo quedo sin explicar");

    assert!(
        result.contains("echo [hecho]"),
        "Deberia corregir 'todo echo' -> 'todo hecho': {}",
        result
    );
}

#[test]
fn test_integration_homophone_los_echos_noun() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los echos importan");

    assert!(
        result.contains("echos") && result.contains("[hechos]"),
        "Debería corregir 'los echos' -> 'los hechos': {}",
        result
    );
}

#[test]
fn test_integration_homophone_son_echos_conocidos() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Son echos conocidos");

    assert!(
        result.contains("echos") && result.contains("[hechos]"),
        "Debería corregir 'son echos conocidos' -> 'son hechos conocidos': {}",
        result
    );
}

#[test]
fn test_integration_homophone_haber_si() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Haber si vienes mañana");

    assert!(
        result.contains("Haber [A ver]"),
        "Debería corregir 'Haber si' -> 'A ver si': {}",
        result
    );
}

#[test]
fn test_integration_homophone_haber_que() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Haber que pasa");

    assert!(
        result.contains("Haber [A ver]"),
        "Debería corregir 'Haber que' -> 'A ver qué': {}",
        result
    );
    assert!(
        result.contains("que [qué]"),
        "Debería acentuar interrogativo en 'A ver qué': {}",
        result
    );
}

#[test]
fn test_integration_homophone_haber_cuando() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Haber cuando vienes");

    assert!(
        result.contains("Haber [A ver]"),
        "Debería corregir 'Haber cuando' -> 'A ver cuando': {}",
        result
    );
    assert!(
        !result.contains("cuando [cuándo]"),
        "No debería forzar tilde interrogativa en 'Haber cuando ...': {}",
        result
    );
}

#[test]
fn test_integration_homophone_a_ver_que_accented() {
    let corrector = create_test_corrector();
    let result = corrector.correct("A ver que pasa");

    assert!(
        result.contains("que [qué]"),
        "Debería corregir 'A ver que...' -> 'A ver qué...': {}",
        result
    );
}

#[test]
fn test_integration_homophone_pues_haber_si() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Pues haber si vienes");

    assert!(
        result.contains("haber [a ver]") || result.contains("Haber [A ver]"),
        "Debería corregir 'Pues haber si' -> 'Pues a ver si': {}",
        result
    );
}

#[test]
fn test_integration_homophone_vamos_haber_que() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Vamos haber que pasa");

    assert!(
        result.contains("haber [a ver]") || result.contains("Haber [A ver]"),
        "Debería corregir 'Vamos haber que' -> 'Vamos a ver que': {}",
        result
    );
}

#[test]
fn test_integration_homophone_voy_haber_si() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Voy haber si puedo");

    assert!(
        result.contains("haber [a ver]") || result.contains("Haber [A ver]"),
        "Debería corregir 'Voy haber si' -> 'Voy a ver si': {}",
        result
    );
}

#[test]
fn test_integration_homophone_puede_haber_que_no_correction() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Puede haber que esperar");

    assert!(
        !result.contains("haber [a ver]") && !result.contains("Haber [A ver]"),
        "No debería cambiar 'haber' verbal en 'puede haber que esperar': {}",
        result
    );
    assert!(
        !result.contains("que [qué]"),
        "No debería acentuar 'que' en uso verbal de 'haber': {}",
        result
    );
}

#[test]
fn test_integration_homophone_halla_haya_with_interposed_clitics() {
    let corrector = create_test_corrector();
    let cases = [
        "No creo que lo halla hecho",
        "No creo que se halla ido",
        "No creo que me halla visto",
        "Dudo que lo halla entendido",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains("halla [haya]"),
            "Debería corregir 'halla' -> 'haya' con clítico interpuesto en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_porque_direct_question() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Porque vienes?");

    assert!(
        result.contains("Porque [Por qu\u{00E9}]") || result.contains("porque [por qu\u{00E9}]"),
        "Deberia corregir interrogativo directo 'Porque' -> 'Por que': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_direct_question_que_hora() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Que hora es?");
    let lower = result.to_lowercase();

    assert!(
        lower.contains("que [q"),
        "Deberia corregir interrogativo directo 'Que' -> 'Qué': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_direct_question_como_te_llamas() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Como te llamas?");
    let lower = result.to_lowercase();

    assert!(
        lower.contains("como [c"),
        "Deberia corregir interrogativo directo 'Como' -> 'Cómo': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_direct_exclamation_que_bonito() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¡Que bonito!");
    let lower = result.to_lowercase();

    assert!(
        lower.contains("que [q"),
        "Deberia corregir exclamativo directo 'Que' -> 'Qué': {}",
        result
    );
}

#[test]
fn test_integration_diacritics_direct_question_subordinate_que_not_accented() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Crees que es posible?");

    assert!(
        !result.contains("que [qué]") && !result.contains("Que [Qué]"),
        "No deberia acentuar 'que' conjunción en interrogativa directa: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_direct_question_double_que_only_first() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Que quieres que haga?");

    assert!(
        result.contains("Que [Qué]") || result.contains("que [qué]"),
        "Deberia acentuar solo el primer 'que': {}",
        result
    );
    assert!(
        !result.contains("que [qué] haga") && !result.contains("que [Qué] haga"),
        "No deberia acentuar el segundo 'que' conjuntivo: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_direct_question_comparative_como_not_accented() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Es tan bueno como dicen?");

    assert!(
        !result.contains("como [cómo]") && !result.contains("Como [Cómo]"),
        "No deberia acentuar 'como' comparativo: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_direct_question_prep_plus_que() {
    let corrector = create_test_corrector();
    let cases = [
        "\u{00BF}De que hablas?",
        "\u{00BF}En que piensas?",
        "\u{00BF}A que hora llegas?",
        "\u{00BF}Para que sirve?",
        "\u{00BF}Con que lo abriste?",
        "\u{00BF}Sobre que discutian?",
        "\u{00BF}Hasta que hora trabajas?",
        "\u{00BF}Desde que ciudad llamas?",
        "\u{00BF}Hacia que direccion miras?",
        "\u{00BF}Contra que juegan hoy?",
        "\u{00BF}Con que lo hiciste?",
        "\u{00BF}Sobre que trato?",
        "\u{00BF}Hacia que lugar iban?",
    ];

    for text in cases {
        let result = corrector.correct(text);
        let lower = result.to_lowercase();
        assert!(
            lower.contains("que [q"),
            "Debe acentuar 'que' en interrogativa con preposicion inicial '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_direct_question_connector_led_interrogative() {
    let corrector = create_test_corrector();
    let cases = [
        ("\u{00BF}Y que quieres?", "que [q"),
        ("\u{00BF}Pero que dices?", "que [q"),
        ("\u{00BF}Y donde esta?", "donde [d"),
    ];

    for (text, fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains(fragment),
            "Debe acentuar interrogativo tras conector en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_diacritics_indirect_interrogative_inside_question() {
    let corrector = create_test_corrector();

    let result = corrector.correct("\u{00BF}Sabes donde vive?");
    assert!(
        result.to_lowercase().contains("donde [d"),
        "Debe acentuar 'donde' en interrogativa indirecta dentro de \u{00BF}...?: {}",
        result
    );

    let result = corrector.correct("\u{00BF}Me dices como se llama?");
    assert!(
        result.to_lowercase().contains("como [c"),
        "Debe acentuar 'como' en interrogativa indirecta dentro de \u{00BF}...?: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_mas_sentence_start_is_corrected() {
    let corrector = create_test_corrector();
    let cases = ["Mas vale tarde que nunca", "Mas alla de eso", "Mas bien no"];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("mas [m"),
            "Debe corregir 'Mas' al inicio de oracion en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_hay_before_finite_verb_to_ahi() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hay viene el tren");

    assert!(
        result.contains("Hay [Ah"),
        "Deberia corregir 'Hay viene' -> 'Ahí viene': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ay_un_gato_to_hay() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ay un gato");

    assert!(
        result.contains("Ay [Hay]"),
        "Deberia corregir 'Ay un gato' -> 'Hay un gato': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ay_que_dolor_not_changed_to_hay() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ay que dolor");

    assert!(
        !result.contains("Ay [Hay]") && !result.contains("ay [hay]"),
        "No deberia corregir 'Ay que dolor' a 'Hay que dolor': {}",
        result
    );
}

#[test]
fn test_integration_homophone_exclamative_ay_que_bonito_not_changed_to_hay() {
    let corrector = create_test_corrector();
    let result = corrector.correct("\u{00A1}Ay qu\u{00E9} bonito!");

    assert!(
        !result.contains("Ay [Hay]") && !result.contains("ay [hay]"),
        "No deberia corregir interjeccion exclamativa 'Ay que...' a 'Hay': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ahi_que_ir_to_hay_que_ir() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ahí que ir");

    assert!(
        result.contains("Ah") && result.contains("[Hay]"),
        "Deberia corregir 'Ahí que ir' -> 'Hay que ir': {}",
        result
    );
}

#[test]
fn test_integration_homophone_vaya_valla_extended_coverage() {
    let corrector = create_test_corrector();
    let cases = [
        ("La vaya del jardin", "vaya [valla]"),
        ("Salto la vaya", "vaya [valla]"),
        ("Es una vaya publicitaria", "vaya [valla]"),
        ("Que le valla bien", "valla [vaya]"),
        ("Valla usted a saber", "Valla [Vaya]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia corregir homofono vaya/valla en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_homophone_valla_que_exclamative() {
    let corrector = create_test_corrector();
    let cases = ["Valla que sorpresa", "Valla que d\u{00ED}a", "Valla hombre"];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("valla [vaya]"),
            "Deberia corregir interjeccion con 'valla' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_nominal_valla_not_changed_to_vaya() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La valla del jardin");

    assert!(
        !result.to_lowercase().contains("valla [vaya]"),
        "No deberia corregir sustantivo 'valla' a verbo 'vaya': {}",
        result
    );
}

#[test]
fn test_integration_homophone_de_ahi_que_not_changed_to_hay() {
    let corrector = create_test_corrector();
    let result = corrector.correct("De ahí que no venga");

    assert!(
        !result.contains("ahí [hay]") && !result.contains("Ahí [Hay]"),
        "No deberia corregir 'de ahí que' a 'de hay que': {}",
        result
    );
}

#[test]
fn test_integration_homophone_no_hay_nada_not_changed_to_ahi() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No hay nada");

    assert!(
        !result.contains("hay [ah"),
        "No deberia corregir 'No hay nada' a 'No ahí nada': {}",
        result
    );
}

#[test]
fn test_integration_homophone_no_hay_nada_que_hacer_not_changed_to_ahi() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No hay nada que hacer");

    assert!(
        !result.contains("hay [ah"),
        "No deberia corregir 'No hay nada que hacer' a 'No ahí nada...': {}",
        result
    );
}

#[test]
fn test_integration_homophone_no_explico_porque_causal_no_change() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No explico porque estoy cansado");

    assert!(
        !result.contains("porque [por qu"),
        "No debe forzar 'por que' interrogativo en uso causal: {}",
        result
    );
}

#[test]
fn test_integration_homophone_sino_como_conditional_split() {
    let corrector = create_test_corrector();
    let result = corrector.correct("sino como me muero");

    assert!(
        result.contains("sino [Si no]") || result.contains("sino [si no]"),
        "Debe corregir 'sino como' -> 'si no como': {}",
        result
    );
}

#[test]
fn test_integration_homophone_sino_como_adversative_not_split() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No como metáfora, sino como estado real");

    assert!(
        !result.contains("sino [si no]") && !result.contains("Sino [Si no]"),
        "No debe corregir 'sino como + SN' adversativo a 'si no': {}",
        result
    );
}

#[test]
fn test_integration_homophone_no_se_porque_indirect_question() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se porque vino");

    assert!(
        result.to_lowercase().contains("se [s"),
        "Deberia corregir 'se' -> 'se' (se con tilde) en interrogativa indirecta: {}",
        result
    );
    assert!(
        result.contains("porque [por qu\u{00E9}]"),
        "Deberia corregir 'porque' -> 'por que' en subordinada interrogativa: {}",
        result
    );
}

#[test]
fn test_integration_homophone_indirect_interrogative_with_imperative_triggers() {
    let corrector = create_test_corrector();
    let cases = [
        ("Cuentame que paso", "que [qué]"),
        ("Dinos donde esta", "donde [dónde]"),
        ("Indicame como llegar", "como [cómo]"),
        ("Muestrame donde vives", "donde [dónde]"),
        ("Enseñame como se hace", "como [cómo]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia acentuar interrogativo indirecto tras imperativo en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_homophone_indirect_interrogative_with_infinitive_triggers() {
    let corrector = create_test_corrector();
    let cases = [
        ("Quiero saber donde esta", "donde [dónde]"),
        ("Necesito saber como hacerlo", "como [cómo]"),
        ("Puedo preguntar cuando viene", "cuando [cuándo]"),
        ("Hay que averiguar quien fue", "quien [quién]"),
        ("Voy a preguntar donde queda", "donde [dónde]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia acentuar interrogativo indirecto tras infinitivo trigger en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_homophone_el_porque_nominal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El porque de todo");

    assert!(
        result.contains("porque [porqu\u{00E9}]"),
        "Deberia corregir sustantivo 'el porque' -> 'el porque': {}",
        result
    );
}

#[test]
fn test_integration_homophone_por_que_relative_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Lucho por que vengas");

    assert!(
        !result.contains("que [qu\u{00E9}]"),
        "No deberia corregir 'por que' no interrogativo: {}",
        result
    );
}

#[test]
fn test_integration_homophone_porque_causal_inside_question_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Te fuiste porque llovia?");

    assert!(
        !result.contains("porque [por qu\u{00E9}]"),
        "No deberia forzar 'por que' cuando 'porque' es causal: {}",
        result
    );
}

#[test]
fn test_integration_homophone_si_no_contrast_should_be_sino() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No quiero ir, si no quedarme");

    assert!(
        result.contains("si [sino]"),
        "Debería corregir 'si no' adversativo a 'sino': {}",
        result
    );
    assert!(
        result.contains("~~no~~"),
        "Debe mostrar el 'no' eliminado al fusionar 'si no' -> 'sino': {}",
        result
    );
}

#[test]
fn test_integration_homophone_si_no_conditional_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Si no vienes, me voy");

    assert!(
        !result.contains("Si [Sino]") && !result.contains("si [sino]"),
        "No debería tocar condicional negativo 'si no + verbo': {}",
        result
    );
}

#[test]
fn test_integration_homophone_sino_conditional_should_be_si_no() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Sino vienes, me voy");

    assert!(
        result.contains("Sino [Si no]") || result.contains("sino [si no]"),
        "Debería corregir 'sino + verbo' a 'si no + verbo': {}",
        result
    );
}

#[test]
fn test_integration_subject_verb_en_todos_los_frentes_not_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El ascenso en todos los frentes ha despertado temor");

    assert!(
        !result.contains("ha [han]"),
        "No debe forzar plural de 'ha' por 'en todos los frentes': {}",
        result
    );
}

#[test]
fn test_integration_noun_adverb_adjective_cada_vez_mas_not_forced() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Las huestes de Vox, cada vez más envalentonadas");

    assert!(
        !result.contains("envalentonadas [envalentonada]"),
        "No debe singularizar adjetivo en expresión adverbial 'cada vez más': {}",
        result
    );
}

#[test]
fn test_integration_spelling_hipotecar_recognized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Va a hipotecar tu futuro");

    assert!(
        !result.contains("hipotecar |"),
        "No debe marcar 'hipotecar' como error ortográfico: {}",
        result
    );
}

#[test]
fn test_integration_predicative_does_not_use_cardinal_direction_as_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La frontera norte estaba integrada");

    assert!(
        !result.contains("integrada [integrado]"),
        "No debe forzar masculino por 'norte' en posicion posnominal: {}",
        result
    );
}

#[test]
fn test_integration_predicative_keeps_head_over_de_phrase_modifier() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Las sesiones de ejercicio supervisadas son utiles");

    assert!(
        !result.contains("supervisadas [supervisado]"),
        "No debe perder el nucleo 'sesiones' por PP intermedia: {}",
        result
    );
}

#[test]
fn test_integration_subject_verb_org_name_with_internal_y_not_plural_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El Grupo de Trabajo Salud y Deporte est\u{00E1} activo");

    assert!(
        !result.contains("est\u{00E1} [est\u{00E1}n]")
            && !result.contains("activo [activos]"),
        "No debe tratar 'Salud y Deporte' como sujeto coordinado del verbo principal: {}",
        result
    );
}

#[test]
fn test_integration_subject_verb_incluyendo_phrase_does_not_override_main_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La descripcion, incluyendo transistores, puede fallar");

    assert!(
        !result.contains("puede [pueden]"),
        "No debe forzar plural por inciso 'incluyendo ...': {}",
        result
    );
}

#[test]
fn test_integration_homophone_a_la_inversa_porque_not_nominal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No se da a la inversa porque llueve");

    assert!(
        !result.contains("porque ["),
        "No debe corregir 'porque' causal tras 'a la inversa': {}",
        result
    );
}

#[test]
fn test_integration_spelling_sentadilla_recognized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La sentadilla fortalece las piernas");

    assert!(
        !result.contains("sentadilla |"),
        "No debe marcar 'sentadilla' como error ortografico: {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_en_base_a() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Tomamos la decision en base a los datos");

    assert!(
        result.contains("en [con]") && result.contains("a [en]"),
        "Debería corregir 'en base a' -> 'con base en': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_de_acuerdo_a() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Actuamos de acuerdo a la norma");

    assert!(
        result.contains("a [con]"),
        "Debería corregir 'de acuerdo a' -> 'de acuerdo con': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_en_relacion_a() {
    let corrector = create_test_corrector();
    let result = corrector.correct("En relación a tu pregunta");

    assert!(
        result.contains("a [con]"),
        "Debería corregir 'en relación a' -> 'en relación con': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_en_relacion_al() {
    let corrector = create_test_corrector();
    let result = corrector.correct("En relación al tema");

    assert!(
        result.contains("al [con el]"),
        "Debería corregir 'en relación al' -> 'en relación con el': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_bajo_punto_de_vista() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Bajo mi punto de vista, eso es correcto");

    assert!(
        result.contains("Bajo [Desde]") || result.contains("bajo [desde]"),
        "Debería corregir 'bajo ... punto de vista' -> 'desde ... punto de vista': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_a_nivel_de_non_technical() {
    let corrector = create_test_corrector();
    let result = corrector.correct("A nivel de educacion, hay avances");

    assert!(
        (result.contains("A [En cuanto a]") || result.contains("A [En cuanto a la]"))
            && result.contains("~~nivel~~")
            && result.contains("~~de~~"),
        "Debería corregir 'a nivel de' no técnico -> 'en cuanto a': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_a_nivel_de_adds_article_when_missing() {
    let corrector = create_test_corrector();
    let result = corrector.correct("A nivel de empresa hay problemas");

    assert!(
        result.contains("A [En cuanto a la]")
            && result.contains("~~nivel~~")
            && result.contains("~~de~~"),
        "Debe sugerir 'en cuanto a la ...' cuando falta artículo: {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_a_nivel_del_mar_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Viviamos a nivel del mar");

    assert!(
        !result.contains("a [en]")
            && !result.contains("nivel [cuanto a]")
            && !result.contains("~~de~~"),
        "No debería tocar uso técnico 'a nivel del mar': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_grosso_modo_marks_redundant_a_without_spelling_noise() {
    let corrector = create_test_corrector();
    let result = corrector.correct("A grosso modo");

    assert!(
        result.contains("~~A~~") || result.contains("~~a~~"),
        "Debería marcar la 'a' redundante en 'a grosso modo': {}",
        result
    );
    assert!(
        !result.contains("grosso |"),
        "No debería marcar 'grosso' como error ortográfico en esta locución: {}",
        result
    );
}

#[test]
fn test_integration_fossilized_grosso_modo_without_a_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Grosso modo, avanzamos");

    assert!(
        !result.contains("grosso |") && !result.contains("Grosso |"),
        "No debería marcar 'grosso' como error ortográfico: {}",
        result
    );
    assert!(
        !result.contains("~~Grosso~~") && !result.contains("~~grosso~~"),
        "No debería tocar la locución ya correcta 'grosso modo': {}",
        result
    );
}

#[test]
fn test_integration_gerund_posteriority_clear_pattern() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Salio de casa, llegando al trabajo a las 9");

    assert!(
        result.contains("llegando [al llegar]"),
        "Deberia marcar gerundio de posterioridad en patron claro: {}",
        result
    );
}

#[test]
fn test_integration_gerund_posteriority_non_arrival_gerund_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Salio de casa, caminando rapido");

    assert!(
        !result.contains("caminando ["),
        "No deberia tocar gerundio no incluido en patron conservador: {}",
        result
    );
}

#[test]
fn test_integration_gerund_posteriority_without_comma_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Salio de casa llegando al trabajo");

    assert!(
        !result.contains("llegando [al llegar]"),
        "Sin coma no deberia aplicar regla de posterioridad: {}",
        result
    );
}

#[test]
fn test_integration_gerund_posteriority_terminando_pattern() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Salio de clase, terminando luego el informe");

    assert!(
        result.contains("terminando [al terminar]"),
        "Deberia detectar gerundio de posterioridad con 'terminando': {}",
        result
    );
}

#[test]
fn test_integration_gerund_posteriority_aprobando_pattern() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Salio del examen, aprobando finalmente la materia");

    assert!(
        result.contains("aprobando [al aprobar]"),
        "Deberia detectar gerundio de posterioridad con 'aprobando': {}",
        result
    );
}

#[test]
fn test_integration_gerund_posteriority_explicit_despues_without_movement_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Estudió medicina, dedicándose después a la investigación");

    assert!(
        result.contains("dedicándose [al dedicarse]"),
        "Con marcador explícito 'después' debería detectar posterioridad: {}",
        result
    );
}

#[test]
fn test_integration_homophone_boy_a_ir() {
    let corrector = create_test_corrector();
    let result = corrector.correct("boy a ir");

    assert!(
        result.contains("boy [Voy]"),
        "Debería corregir 'boy a ir' -> 'Voy a ir': {}",
        result
    );
}

#[test]
fn test_integration_homophone_boy_al_cine() {
    let corrector = create_test_corrector();
    let result = corrector.correct("boy al cine");

    assert!(
        result.contains("boy [Voy]"),
        "Debería corregir 'boy al cine' -> 'Voy al cine': {}",
        result
    );
}

#[test]
fn test_integration_homophone_iva_sentence_start_capitalized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Iva al colegio");

    assert!(
        result.contains("Iva [Iba]"),
        "Debería corregir 'Iva' inicial a 'Iba': {}",
        result
    );
}

#[test]
fn test_integration_homophone_iva_proper_name_no_correction() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Iva Morales vino");

    assert!(
        !result.contains("Iva [Iba]"),
        "No debería forzar corrección cuando 'Iva' es nombre propio: {}",
        result
    );
}

#[test]
fn test_integration_homophone_se_a_ido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Se a ido");

    assert!(
        result.contains("a [ha]"),
        "Debería corregir 'Se a ido' -> 'Se ha ido': {}",
        result
    );
}

#[test]
fn test_integration_homophone_haz_visto_should_be_has() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¿Haz visto eso?");

    assert!(
        result.contains("Haz [Has]") || result.contains("haz [has]"),
        "Debería corregir 'Haz visto' -> 'Has visto': {}",
        result
    );
}

#[test]
fn test_integration_homophone_no_haz_hecho_should_be_has() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No haz hecho nada");

    assert!(
        result.contains("haz [has]") || result.contains("Haz [Has]"),
        "Debería corregir 'No haz hecho' -> 'No has hecho': {}",
        result
    );
}

#[test]
fn test_integration_homophone_no_haz_hecho_after_grosso_modo_should_be_has() {
    let corrector = create_test_corrector();
    let result = corrector.correct("A grosso modo no haz hecho nada");

    assert!(
        result.contains("haz [has]") || result.contains("Haz [Has]"),
        "Debería corregir 'no haz hecho' -> 'no has hecho' tras 'grosso modo': {}",
        result
    );
    assert!(
        !result.contains("haz [ha]") && !result.contains("Haz [Ha]"),
        "No debería sugerir 'ha' para 'haz' tras 'grosso modo': {}",
        result
    );
}

#[test]
fn test_integration_homophone_haz_imperative_no_correction() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Haz la tarea");

    assert!(
        !result.contains("Haz [Has]") && !result.contains("haz [has]"),
        "No debería tocar el imperativo válido 'Haz la tarea': {}",
        result
    );
}

#[test]
fn test_integration_homophone_sentence_start_a_echo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("A echo su tarea");

    assert!(
        result.contains("A [Ha]"),
        "Debería corregir 'A echo' -> 'Ha echo': {}",
        result
    );
    assert!(
        result.contains("echo [hecho]"),
        "Debería corregir 'A echo' -> 'Ha hecho': {}",
        result
    );
}

#[test]
fn test_integration_homophone_yo_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Yo a venido tarde");

    assert!(
        result.contains("a [he]"),
        "Debería corregir 'Yo a venido' -> 'Yo he venido': {}",
        result
    );
}

#[test]
fn test_integration_homophone_tu_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Tu a venido tarde");

    assert!(
        result.contains("Tu [Tú]"),
        "Debería corregir 'Tu' -> 'Tú' en patrón 'Tu a + participio': {}",
        result
    );
    assert!(
        result.contains("a [has]"),
        "Debería corregir 'a' -> 'has' en patrón 'Tu a + participio': {}",
        result
    );
}

#[test]
fn test_integration_homophone_tu_with_accent_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Tú a venido tarde");

    assert!(
        !result.contains("Tú [Tu]"),
        "No debería quitar tilde de 'Tú' en patrón 'Tú a + participio': {}",
        result
    );
    assert!(
        result.contains("a [has]"),
        "Debería corregir 'a' -> 'has' en patrón 'Tú a + participio': {}",
        result
    );
}

#[test]
fn test_integration_homophone_nosotros_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Nosotros a venido tarde");

    assert!(
        result.contains("a [hemos]"),
        "Debería corregir 'Nosotros a venido' -> 'Nosotros hemos venido': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ellos_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ellos a venido tarde");

    assert!(
        result.contains("a [han]"),
        "Debería corregir 'Ellos a venido' -> 'Ellos han venido': {}",
        result
    );
}

#[test]
fn test_integration_homophone_nominal_singular_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La gente a venido tarde");

    assert!(
        result.contains("a [ha]"),
        "Debería corregir 'La gente a venido' -> 'La gente ha venido': {}",
        result
    );
}

#[test]
fn test_integration_homophone_nominal_plural_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los niños a venido tarde");

    assert!(
        result.contains("a [han]"),
        "Debería corregir 'Los niños a venido' -> 'Los niños han venido': {}",
        result
    );
}

#[test]
fn test_integration_homophone_temporal_plural_a_venido_prefers_ha() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Estos días a venido mucha gente");

    assert!(
        result.contains("a [ha]"),
        "Debería corregir 'a' -> 'ha' en complemento temporal: {}",
        result
    );
    assert!(
        !result.contains("a [han]"),
        "No debería forzar 'han' por temporal plural: {}",
        result
    );
}

#[test]
fn test_integration_homophone_a_before_adjective_phrase_not_haber() {
    let corrector = create_test_corrector();
    for text in [
        "He enviado cartas a distintas ciudades",
        "He enviado ejemplares a distintas ciudades",
        "Las cartas a distintas personas llegaron",
        "Lleve los libros a distintas tiendas",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("a [han]") && !result.contains("a [ha]"),
            "No debe reinterpretar preposicion 'a' como haber antes de adjetivo: '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_numeral_unit_abbreviation_not_pluralized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("a 25 W, de mejoras");
    assert!(
        !result.contains("W [Wes]"),
        "No debe pluralizar abreviatura de unidad tras numeral: {}",
        result
    );
}

#[test]
fn test_integration_homophone_proper_name_a_venido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Juan a venido tarde");

    assert!(
        result.contains("a [ha]"),
        "Debería corregir 'Juan a venido' -> 'Juan ha venido': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ya_a_participle() {
    let corrector = create_test_corrector();
    let cases = ["Ya a llegado", "Ya a comido"];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains("a [ha]") || result.contains("A [Ha]"),
            "Deberia corregir auxiliar en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_ya_a_estas_alturas_not_changed_to_ha() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ya a estas alturas da igual");

    assert!(
        !result.contains("a [ha]") && !result.contains("A [Ha]"),
        "No deberia cambiar 'a' por 'ha' en 'a estas alturas': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ya_a_esas_horas_not_changed_to_ha() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ya a esas horas no habia nadie");

    assert!(
        !result.contains("a [ha]") && !result.contains("A [Ha]"),
        "No deberia cambiar 'a' por 'ha' en 'a esas horas': {}",
        result
    );
}

#[test]
fn test_integration_homophone_subject_plus_ya_a_participle() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Yo ya a llegado tarde");

    assert!(
        result.contains("a [he]"),
        "Deberia conjugar auxiliar segun sujeto en 'Yo ya a llegado': {}",
        result
    );
}

#[test]
fn test_integration_homophone_le_a_costado() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Le a costado mucho");

    assert!(
        result.contains("a [ha]") || result.contains("A [Ha]"),
        "Deberia corregir 'Le a costado' -> 'Le ha costado': {}",
        result
    );
}

#[test]
fn test_integration_homophone_a_lado_not_ha() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Estoy a lado de casa");

    assert!(
        !result.contains("a [ha]"),
        "No debería cambiar 'a' por 'ha' en contexto nominal: {}",
        result
    );
}

#[test]
fn test_integration_homophone_de_acuerdo_a_not_ha_before_plural_noun() {
    let corrector = create_test_corrector();
    for text in [
        "Pondria de acuerdo a rusos",
        "Ponga de acuerdo a todos",
        "Pongan de acuerdo a todos",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("a [ha]") && !result.contains("A [Ha]"),
            "No deberia cambiar 'a' por 'ha' en '{}': {}",
            text,
            result
        );
        assert!(
            !result.contains("a [con]") && !result.contains("A [Con]"),
            "No deberia forzar 'de acuerdo a' -> 'de acuerdo con' tras 'poner' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_a_preposition_not_changed_to_ha() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Voy a casa");

    assert!(
        !result.contains("a [ha]"),
        "No debería cambiar preposición 'a' en 'Voy a casa': {}",
        result
    );
}

#[test]
fn test_integration_homophone_voy_ha_comprar() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Voy ha comprar pan");

    assert!(
        result.contains("ha [a]"),
        "Debería corregir 'Voy ha comprar' -> 'Voy a comprar': {}",
        result
    );
}

#[test]
fn test_integration_homophone_voy_ha_comprarlo_enclitic() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Voy ha comprarlo");

    assert!(
        result.contains("ha [a]"),
        "Deberia corregir 'ha' -> 'a' antes de infinitivo con enclitico: {}",
        result
    );
    assert!(
        !result.contains("comprarlo [comprado]"),
        "No deberia forzar participio al corregir 'ha comprarlo': {}",
        result
    );
}

#[test]
fn test_integration_homophone_maria_se_ha_ido_ha_comprar() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Maria se ha ido ha comprar");

    assert!(
        result.contains("ha [a] comprar"),
        "Deberia corregir solo la segunda 'ha' como preposicion: {}",
        result
    );
}

#[test]
fn test_integration_loismo_lo_regalaron_flores() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Lo regalaron flores");

    assert!(
        result.contains("Lo [Le]"),
        "Debería corregir loísmo en 'Lo regalaron flores': {}",
        result
    );
}

#[test]
fn test_integration_loismo_lo_dije_la_verdad() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Lo dije la verdad");

    assert!(
        result.contains("Lo [Le]"),
        "Debería corregir loísmo en 'Lo dije la verdad': {}",
        result
    );
}

#[test]
fn test_integration_laismo_la_contaron_la_verdad() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La contaron la verdad");

    assert!(
        result.contains("La [Le]"),
        "Deberia corregir laismo en 'La contaron la verdad': {}",
        result
    );
}

#[test]
fn test_integration_laismo_la_ensenaron_el_camino() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La ensenaron el camino");

    assert!(
        result.contains("La [Le]"),
        "Deberia corregir laismo en 'La ensenaron el camino': {}",
        result
    );
}

#[test]
fn test_integration_laismo_la_regalaron_flores() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La regalaron flores");

    assert!(
        result.contains("La [Le]"),
        "Deberia corregir laismo en 'La regalaron flores': {}",
        result
    );
}

#[test]
fn test_integration_laismo_extended_ditransitive_verbs() {
    let corrector = create_test_corrector();
    let cases = [
        ("La explicaron el problema", "La [Le]"),
        ("La comunicaron la noticia", "La [Le]"),
        ("La ofrecieron un puesto", "La [Le]"),
        ("La preguntaron su nombre", "La [Le]"),
        ("La robaron el bolso", "La [Le]"),
        ("La ensene a conducir", "La [Le]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result.contains(expected_fragment),
            "Deberia corregir laismo en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_laismo_residual_verbs_and_accented_forms() {
    let corrector = create_test_corrector();
    let cases = [
        ("La traje flores", "La [Le]"),
        ("La regalé flores", "La [Le]"),
        ("La informaron del cambio", "La [Le]"),
        ("La mandé un paquete", "La [Le]"),
        ("La compré un libro", "La [Le]"),
        ("La hice un favor", "La [Le]"),
        ("La entregaron el paquete", "La [Le]"),
        ("La devolvieron el libro", "La [Le]"),
        ("La sirvieron la comida", "La [Le]"),
        ("La quitaron la cartera", "La [Le]"),
        ("La pasaron la pelota", "La [Le]"),
        ("La prestaron dinero", "La [Le]"),
        ("Mi madre la preparó la comida a mi hermana", "la [le]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result.contains(expected_fragment),
            "Deberia corregir laismo residual en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_clitic_inversion_me_se_cayo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Me se cayó");

    assert!(
        result.contains("Me [Se]"),
        "Debería corregir primer clítico en 'Me se cayó': {}",
        result
    );
    assert!(
        result.contains("se [me]"),
        "Debería corregir segundo clítico en 'Me se cayó': {}",
        result
    );
}

#[test]
fn test_integration_clitic_inversion_te_se_olvido() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Te se olvidó");

    assert!(
        result.contains("Te [Se]"),
        "Debería corregir primer clítico en 'Te se olvidó': {}",
        result
    );
    assert!(
        result.contains("se [te]"),
        "Debería corregir segundo clítico en 'Te se olvidó': {}",
        result
    );
}

#[test]
fn test_integration_lo_dije_la_semana_pasada_not_loismo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Lo dije la semana pasada.");

    assert!(
        !result.contains("Lo [Le]") && !result.contains("lo [le]"),
        "No debería corregir 'Lo dije la semana pasada' como loísmo: {}",
        result
    );
}

#[test]
fn test_integration_se_lo_dieron_a_el_not_loismo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Se lo dieron a él.");

    assert!(
        !result.contains("lo [le]") && !result.contains("lo [Le]"),
        "No debería corregir 'se lo' como loísmo: {}",
        result
    );
}

#[test]
fn test_integration_se_lo_regalo_a_maria_not_loismo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Se lo regaló a María.");

    assert!(
        !result.contains("lo [le]") && !result.contains("lo [Le]"),
        "No debería corregir 'se lo' como loísmo: {}",
        result
    );
}

#[test]
fn test_integration_ni_conjunction_not_vocative_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ni come ni deja comer");

    assert!(
        !result.contains("Ni,"),
        "No debería insertar coma vocativa tras conjunción 'Ni': {}",
        result
    );
}

#[test]
fn test_integration_aunque_conjunction_not_vocative_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Aunque come mucho");

    assert!(
        !result.contains("Aunque,"),
        "No deberia insertar coma vocativa tras conjuncion 'Aunque': {}",
        result
    );
}

#[test]
fn test_integration_segun_preposition_not_vocative_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Según come bien");

    assert!(
        !result.contains("Según,"),
        "No deberia insertar coma vocativa tras preposicion 'Según': {}",
        result
    );
}

#[test]
fn test_integration_plural_vocative_with_vosotros_imperative() {
    let corrector = create_test_corrector();
    let cases = [
        ("Chicos venid aquí", "Chicos,"),
        ("Niños callad", "Niños,"),
        ("Amigos escuchad", "Amigos,"),
    ];

    for (input, expected) in cases {
        let result = corrector.correct(input);
        assert!(
            result.contains(expected),
            "Debería insertar coma vocativa en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_topicalized_feminine_object_not_laismo() {
    let corrector = create_test_corrector();

    let result = corrector.correct("La carta la escribió Juan.");
    assert!(
        !result.contains("la [le]") && !result.contains("la [Le]"),
        "No debería corregir laísmo en OD topicalizado: {}",
        result
    );

    let result = corrector.correct("La carta la envié ayer.");
    assert!(
        !result.contains("la [le]") && !result.contains("la [Le]"),
        "No debería corregir laísmo en OD topicalizado: {}",
        result
    );
}

#[test]
fn test_integration_nominal_subject_ministerio_intensifica() {
    // "El Ministerio del Interior intensifica" - NO debe corregir "intensifica"
    // porque "intensifica" es reconocido como forma verbal de "intensificar"
    let corrector = create_test_corrector();
    let result = corrector.correct("El Ministerio del Interior intensifica");

    // No debe sugerir "[intensifico]" ni ninguna otra corrección de concordancia
    assert!(
        !result.contains("[intensifico]"),
        "No debería corregir 'intensifica' (es forma verbal): {}",
        result
    );
}

#[test]
fn test_integration_verb_form_modifica() {
    // "La empresa modifica" - NO debe corregir "modifica"
    // porque "modifica" es reconocido como forma verbal de "modificar"
    let corrector = create_test_corrector();
    let result = corrector.correct("La empresa modifica sus precios");

    assert!(
        !result.contains("[modifico]") && !result.contains("[modifica]"),
        "No debería corregir 'modifica' (es forma verbal): {}",
        result
    );
}

#[test]
fn test_integration_verb_form_unifica() {
    // "El gobierno unifica" - NO debe corregir "unifica"
    let corrector = create_test_corrector();
    let result = corrector.correct("El gobierno unifica los criterios");

    assert!(
        !result.contains("[unifico]"),
        "No debería corregir 'unifica' (es forma verbal): {}",
        result
    );
}

#[test]
fn test_integration_adjective_agreement_still_works() {
    // "El coche roja" - SÍ debe corregir "roja" → "rojo"
    // porque "roja" no es forma verbal sino adjetivo
    let corrector = create_test_corrector();
    let result = corrector.correct("El coche roja");

    assert!(
        result.contains("[rojo]"),
        "Debería corregir 'roja' → 'rojo' (concordancia adjetivo): {}",
        result
    );
}

#[test]
fn test_integration_copulative_predicative_adjective_agreement() {
    let corrector = create_test_corrector();

    let cases = [
        ("La casa es bonito", "bonito [bonita]"),
        ("La casa es muy bonito", "bonito [bonita]"),
        ("Las paredes están sucios", "sucios [sucias]"),
        ("Mi madre está contento", "contento [contenta]"),
        ("La situación es complicado", "complicado [complicada]"),
        ("Estas camisas son rojos", "rojos [rojas]"),
        ("Los niños son traviesas", "traviesas [traviesos]"),
        ("El libro es cara", "cara [caro]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result.contains(expected_fragment),
            "Debería corregir concordancia predicativa en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_copulative_predicative_invariable_plural_and_name_gender() {
    let corrector = create_test_corrector();

    let cases = [
        ("Los ninos son feliz", "feliz [felices]"),
        ("Las mesas son grande", "grande [grandes]"),
        ("Los problemas son grave", "grave [graves]"),
        ("Los coches son veloz", "veloz [veloces]"),
        ("Esas decisiones son importante", "importante [importantes]"),
        ("Juan esta contenta", "contenta [contento]"),
        ("Pedro esta cansada", "cansada [cansado]"),
        ("Carlos esta enfadada", "enfadada [enfadado]"),
        ("Carmen esta cansado", "cansado [cansada]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia corregir concordancia copulativa en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_copulative_predicative_pseudocopulative_verbs() {
    let corrector = create_test_corrector();

    let cases = [
        ("La puerta quedo abierto", "abierto [abierta]"),
        ("Los platos quedaron limpio", "limpio [limpios]"),
        ("Las ventanas quedaron cerrado", "cerrado [cerradas]"),
        ("Ella se siente cansado", "cansado [cansada]"),
        ("Mar\u{00ED}a se puso contento", "contento [contenta]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia corregir concordancia predicativa con pseudocopulativo en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_copulative_predicative_with_coordinated_adjectives() {
    let corrector = create_test_corrector();
    let cases = [
        ("La casa es grande y bonito", "bonito [bonita]"),
        ("Las ninas son altas y delgado", "delgado [delgadas]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe corregir adjetivo coordinado en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_subject_verb_does_not_force_present_when_missing_preterite_accent() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ella llego tarde");

    assert!(
        !result.to_lowercase().contains("llego [llega]"),
        "No deberia forzar presente 3s cuando puede ser preterito sin tilde: {}",
        result
    );
}

#[test]
fn test_integration_copulative_predicative_plural_accentuation_rules() {
    let corrector = create_test_corrector();

    let cases = [
        ("Los chicos son francés", "francés [franceses]"),
        ("Los chicos son inglés", "inglés [ingleses]"),
        ("Los chicos son portugués", "portugués [portugueses]"),
        ("Los chicos son japonés", "japonés [japoneses]"),
        ("Los alumnos son común", "común [comunes]"),
        ("Los chicos son joven", "joven [jóvenes]"),
        ("Las personas son cortés", "cortés [corteses]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia aplicar pluralizacion con acentuacion correcta en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_copulative_predicative_adjective_correct_cases_no_correction() {
    let corrector = create_test_corrector();

    let cases = [
        "La casa es bonita",
        "La casa es muy bonita",
        "Las paredes están sucias",
        "Mi madre está contenta",
        "La situación es complicada",
        "Estas camisas son rojas",
        "La lista de tareas está actualizada",
        "En mi opinion es correcto",
        "En la actualidad es necesario",
        "Sin esta herramienta es complicado",
        "Los ninos son felices",
        "Las mesas son grandes",
        "Los problemas son graves",
        "Juan est\u{00E1} contento",
        "Pedro est\u{00E1} cansado",
        "Carlos est\u{00E1} enfadado",
    ];

    for input in cases {
        let result = corrector.correct(input);
        assert!(
            !result.contains('['),
            "No debería corregir concordancia predicativa ya correcta en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_copulative_predicative_yo_subject_not_forced_to_masculine() {
    let corrector = create_test_corrector();
    let cases = [
        ("Yo soy alta", "alta [alto]"),
        ("Yo estuve cansada", "cansada [cansado]"),
        ("Yo estaba sentada", "sentada [sentado]"),
        ("Yo fui elegida", "elegida [elegido]"),
        ("Yo soy abogada", "abogada [abogado]"),
        ("Yo soy profesora", "profesora [profesoro]"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe forzar genero masculino con sujeto 'yo' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_attributive_noun_adverb_adjective_agreement() {
    let corrector = create_test_corrector();

    let cases = [
        ("Es una persona muy bueno", "bueno [buena]"),
        ("una persona muy bueno", "bueno [buena]"),
        ("persona muy bueno", "bueno [buena]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia corregir concordancia atributiva en '{}': {}",
            input,
            result
        );
    }

    let result = corrector.correct("Es una persona muy buena");
    assert!(
        !result.contains('['),
        "No deberia corregir cuando la concordancia atributiva ya es correcta: {}",
        result
    );
}

#[test]
fn test_integration_copulative_participle_verbal_forms_are_corrected() {
    let corrector = create_test_corrector();
    let cases = [
        ("La carta fue escrito", "escrito [escrita]"),
        ("La carta fue enviado", "enviado [enviada]"),
        ("La carta fue leído", "leído [leída]"),
        ("La carta fue publicado", "publicado [publicada]"),
        ("La carta fue pintado", "pintado [pintada]"),
        ("La carta fue hecho", "hecho [hecha]"),
        ("La carta fue terminado", "terminado [terminada]"),
        ("La carta fue acabado", "acabado [acabada]"),
        ("La carta fue completado", "completado [completada]"),
        ("La carta fue cumplido", "cumplido [cumplida]"),
        ("Los informes fueron escrito", "escrito [escritos]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result.contains(expected_fragment),
            "Debería corregir participio predicativo en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_copulative_predicative_no_false_positive_tanto_como_pronouns() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Tanto él como ella son buenos");

    assert!(
        !result.contains("buenos [buena]"),
        "No debería forzar singular femenino en sujeto coordinado: {}",
        result
    );
}

#[test]
fn test_integration_tanto_como_pronouns_after_comma_clause_no_false_positive() {
    let corrector = create_test_corrector();
    let result =
        corrector.correct("Tanto él como ella son buenos, y tanto yo como tú sabemos la verdad");

    assert!(
        !result.contains("sabemos [sabes]"),
        "No debería forzar 2ª persona singular tras nueva cláusula con 'tanto...como...': {}",
        result
    );
}

#[test]
fn test_integration_copulative_predicative_no_false_positive_relative_temporal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los perros que ladraban toda la noche están dormidos");

    assert!(
        !result.contains("dormidos [dormida]"),
        "No debería tomar 'toda la noche' como sujeto del atributo: {}",
        result
    );

    let result = corrector.correct("Los atletas que corrieron toda la carrera están cansados");
    assert!(
        !result.contains("cansados [cansada]"),
        "No debería tomar 'toda la carrera' como sujeto del atributo: {}",
        result
    );

    let result = corrector.correct("Los estudiantes que leyeron toda la jornada están agotados");
    assert!(
        !result.contains("agotados [agotada]"),
        "No debería tomar 'toda la jornada' como sujeto del atributo: {}",
        result
    );

    let result = corrector
        .correct("El informe que redactaron los técnicos durante toda la jornada parecía confuso");
    assert!(
        !result.contains("confuso [confusa]"),
        "No debería tomar 'toda la jornada' como sujeto del atributo en subordinada relativa: {}",
        result
    );

    let result = corrector.correct("El acta que redactaron los técnicos es correcta");
    assert!(
        !result.contains("correcta [correctos]"),
        "No debería tomar 'los técnicos' (sujeto de la relativa) como sujeto de 'es': {}",
        result
    );

    let result = corrector.correct("La carta que escribieron los técnicos es buena");
    assert!(
        !result.contains("buena [buenos]"),
        "No debería tomar 'los técnicos' (sujeto de la relativa) como sujeto de 'es': {}",
        result
    );

    let result =
        corrector.correct("La serie de cambios que propusieron ayer los técnicos es adecuada");
    assert!(
        !result.contains("adecuada [adecuados]"),
        "No debería tomar 'los técnicos' (sujeto de la relativa) como sujeto de 'es': {}",
        result
    );

    let result = corrector.correct("La lista de tareas está actualizada");
    assert!(
        !result.contains("actualizada [actualizadas]"),
        "No debería tomar el complemento con 'de' como sujeto de la copulativa: {}",
        result
    );

    let result = corrector.correct("La actualización de los módulos es correcta");
    assert!(
        !result.contains("correcta [correctos]"),
        "No debería tomar el complemento 'de los módulos' como sujeto de la copulativa: {}",
        result
    );
}

#[test]
fn test_integration_tanto_como_pronouns_after_previous_clause_without_comma_no_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector
        .correct("Tanto Pedro como Juan vienen temprano y tanto ella como él están listos");

    assert!(
        !result.contains("están [está]"),
        "No debería forzar singular tras nueva coordinación pronominal 'tanto...como...': {}",
        result
    );
}

#[test]
fn test_integration_coordinated_nouns_plural_adjective_no_false_singular() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Una medicina y una nutrición personalizadas");
    assert!(
        !result.contains("personalizadas [personalizada]"),
        "No debería forzar singular en coordinación nominal: {}",
        result
    );

    let result = corrector.correct("Un hombre y una mujer cansados");
    assert!(
        !result.contains("cansados [cansada]"),
        "No debería forzar singular en coordinación nominal mixta: {}",
        result
    );
}

#[test]
fn test_integration_adjective_agreement_with_ambiguous_following_spelling() {
    // "salio" genera varias sugerencias ortográficas; eso no debe bloquear
    // la concordancia "niña bonito" -> "niña bonita".
    let corrector = create_test_corrector();
    let result = corrector.correct("La niña bonito salio");

    assert!(
        result.contains("bonito [bonita]"),
        "Debería corregir concordancia adjetival aunque haya spelling ambiguo después: {}",
        result
    );
}

#[test]
fn test_integration_participle_as_adjective() {
    // "la puerta cerrado" - SÍ debe corregir "cerrado" → "cerrada"
    // porque el participio funciona como adjetivo y necesita concordancia
    let corrector = create_test_corrector();
    let result = corrector.correct("la puerta cerrado");

    assert!(
        result.contains("[cerrada]"),
        "Debería corregir 'cerrado' → 'cerrada' (participio como adjetivo): {}",
        result
    );
}

#[test]
fn test_integration_participle_irregular_as_adjective() {
    // "el libro escrita" - SÍ debe corregir "escrita" → "escrito"
    // porque el participio irregular funciona como adjetivo
    let corrector = create_test_corrector();
    let result = corrector.correct("el libro escrita");

    assert!(
        result.contains("[escrito]"),
        "Debería corregir 'escrita' → 'escrito' (participio irregular): {}",
        result
    );
}

#[test]
fn test_integration_una_vez_absolute_participle_no_false_singular() {
    let corrector = create_test_corrector();
    let samples = [
        ("Una vez obtenidas las credenciales", "obtenidas [obtenida]"),
        ("Una vez firmados los contratos", "firmados [firmada]"),
        ("Una vez revisadas las cuentas", "revisadas [revisada]"),
        ("Una vez cumplidas las condiciones", "cumplidas [cumplida]"),
        ("Una vez resueltos los problemas", "resueltos [resuelta]"),
    ];

    for (input, wrong_pattern) in samples {
        let result = corrector.correct(input);
        assert!(
            !result.contains(wrong_pattern),
            "No debería forzar singular en cláusula absoluta de participio ('{}'): {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_nominal_subject_with_adverb() {
    // "El Ministerio del Interior hoy intensifica" - debe saltar "hoy" y no corregir
    let corrector = create_test_corrector();
    let result = corrector.correct("El Ministerio del Interior hoy intensifica");

    assert!(
        !result.contains("[intensifico]") && !result.contains("[intensifican]"),
        "No debería corregir 'intensifica' cuando hay adverbio entre SN y verbo: {}",
        result
    );
}

#[test]
fn test_integration_temporal_impersonal_llueve_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Todos los días llueve");

    assert!(
        !result.contains("llueve [llueven]"),
        "No debería forzar plural con verbo impersonal meteorológico: {}",
        result
    );
}

#[test]
fn test_integration_nominal_subject_coordination_without_det() {
    // "El ministro y presidente habla" - coordinación sin determinante, plural
    let corrector = create_test_corrector();
    let result = corrector.correct("El ministro y presidente habla");

    assert!(
        result.contains("[hablan]"),
        "Debería corregir 'habla' → 'hablan' (coordinación sin det es plural): {}",
        result
    );
}

#[test]
fn test_integration_nominal_subject_coordination_correct() {
    // "El ministro y presidente hablan" - ya es plural, no debe corregir
    let corrector = create_test_corrector();
    let result = corrector.correct("El ministro y presidente hablan");

    assert!(
        !result.contains("[habla]"),
        "No debería corregir 'hablan' (ya es plural correcto): {}",
        result
    );
}

#[test]
fn test_integration_coordinated_subject_then_comma_new_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La casa y el coche, el niño corrió.");

    assert!(
        !result.contains("[corrieron]"),
        "No debería corregir cuando hay nuevo sujeto tras coma: {}",
        result
    );
}

#[test]
fn test_integration_coordinated_subject_then_comma_new_subject_acronym() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La salida y la ausencia, el CNE autorizó.");

    assert!(
        !result.contains("[autorizaron]"),
        "No debería corregir cuando hay nuevo sujeto tras coma con sigla: {}",
        result
    );
}

#[test]
fn test_integration_coordinated_proper_names_require_plural_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Maria y Pedro sale");

    assert!(
        result.contains("sale [salen]"),
        "Deberia corregir verbo singular con sujeto coordinado de nombres propios: {}",
        result
    );
}

#[test]
fn test_integration_coordinated_possessive_subject_requires_plural_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Mi hermano y mi hermana estudia");

    assert!(
        result.contains("estudia [estudian]"),
        "Deberia corregir verbo singular con sujeto coordinado posesivo: {}",
        result
    );
}

#[test]
fn test_integration_coordinated_possessive_subject_plural_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Mi hermano y mi hermana estudian");

    assert!(
        !result.contains("[estudia]"),
        "No deberia corregir cuando ya esta en plural con sujeto coordinado posesivo: {}",
        result
    );
}

#[test]
fn test_integration_nominal_subject_with_prep_phrase_en_2020() {
    // "El Ministerio en 2020 intensifican" → debe corregir a "intensifica"
    // El sistema debe saltar "en 2020" para encontrar el verbo
    let corrector = create_test_corrector();
    let result = corrector.correct("El Ministerio en 2020 intensifican.");

    assert!(
        result.contains("[intensifica]"),
        "Debería corregir 'intensifican' a 'intensifica': {}",
        result
    );
}

#[test]
fn test_integration_nominal_subject_with_prep_phrase_correct() {
    // "El Ministerio en 2020 intensifica" - ya es correcto
    let corrector = create_test_corrector();
    let result = corrector.correct("El Ministerio en 2020 intensifica.");

    assert!(
        !result.contains("["),
        "No debería hacer correcciones (ya es correcto): {}",
        result
    );
}

#[test]
fn test_integration_object_after_relative_verb_not_treated_as_subject() {
    // "Los estudiantes que aprobaron el examen celebraron"
    // "el examen" es objeto directo de "aprobaron", NO un nuevo sujeto
    // "celebraron" concuerda con "los estudiantes", no con "el examen"
    let corrector = create_test_corrector();
    let result = corrector.correct("Los estudiantes que aprobaron el examen celebraron");

    assert!(
        !result.contains("[celebró]"),
        "No debería corregir 'celebraron' - concuerda con 'los estudiantes': {}",
        result
    );
}

#[test]
fn test_integration_relative_conjunto_de_microorganismos_no_false_plural() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El conjunto de microorganismos que habita en el intestino");

    assert!(
        !result.contains("habita [habitan]"),
        "No debería forzar plural en relativo con núcleo 'conjunto': {}",
        result
    );
}

#[test]
fn test_integration_object_after_relative_verb_singular_subject() {
    // "La mujer que conocí el sábado llamó"
    // "el sábado" es complemento de tiempo, no objeto, pero viene después del verbo
    let corrector = create_test_corrector();
    let result = corrector.correct("La mujer que conocí el sábado llamó");

    assert!(
        !result.contains("[llamaron]"),
        "No debería corregir 'llamó' - concuerda con 'la mujer': {}",
        result
    );
}

#[test]
fn test_cuyo_not_treated_as_verb() {
    // "cuyo/cuya/cuyos/cuyas" son determinantes posesivos relativos
    // No deben tratarse como verbos (cuyo ≠ "yo cuyo" de verbo "cuyar")
    let corrector = create_test_corrector();

    // cuyo con sustantivo masculino poseído
    let result = corrector.correct("El libro cuyo autor es famoso");
    assert!(
        !result.contains("[cuya]"),
        "No debería corregir 'cuyo': {}",
        result
    );

    // cuyo con sustantivo femenino (antecedente femenino, poseído masculino)
    let result2 = corrector.correct("La casa cuyo tejado se rompió");
    assert!(
        !result2.contains("[cuya]"),
        "No debería corregir 'cuyo' (tejado es masculino): {}",
        result2
    );

    // cuya correcto
    let result3 = corrector.correct("El libro cuya portada es roja");
    assert!(
        !result3.contains("[cuyo]"),
        "No debería corregir 'cuya': {}",
        result3
    );
}

#[test]
fn test_integration_uno_de_los_que_vino_is_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es uno de los que vino temprano.");

    assert!(
        result.contains("vino [vinieron]"),
        "Debería corregir 'vino' en 'uno de los que ...': {}",
        result
    );
}

#[test]
fn test_integration_una_de_las_que_vino_is_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es una de las que vino temprano.");

    assert!(
        result.contains("vino [vinieron]"),
        "Debería corregir 'vino' en 'una de las que ...': {}",
        result
    );
}

#[test]
fn test_integration_uno_de_los_que_vinieron_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es uno de los que vinieron temprano.");

    assert!(
        !result.contains("["),
        "No debería corregir cuando ya está en plural: {}",
        result
    );
}

#[test]
fn test_integration_uno_de_los_que_mejor_juega_is_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es uno de los que mejor juega.");

    assert!(
        result.contains("juega [juegan]"),
        "Deberia corregir 'juega' en 'uno de los que mejor ...': {}",
        result
    );
}

#[test]
fn test_integration_uno_de_los_que_mejor_juegan_not_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es uno de los que mejor juegan.");

    assert!(
        !result.contains("["),
        "No deberia corregir cuando ya esta en plural en 'uno de los que mejor ...': {}",
        result
    );
}

#[test]
fn test_integration_subject_verb_de_complement_with_possessive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La hermana de mis amigos trabajan.");

    assert!(
        result.contains("trabajan [trabaja]"),
        "Debería corregir plural con sujeto singular en 'de mis ...': {}",
        result
    );
}

#[test]
fn test_integration_infinitive_imperative_with_opening_exclamation() {
    let corrector = create_test_corrector();
    let result = corrector.correct("¡Callar!");

    assert!(
        result.contains("Callar [Callad]"),
        "Deberia corregir infinitivo imperativo en exclamacion: {}",
        result
    );
}

#[test]
fn test_integration_infinitive_imperative_sentence_start_with_closing_exclamation() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Callar!");

    assert!(
        result.contains("Callar [Callad]"),
        "Deberia corregir infinitivo imperativo al inicio con cierre exclamativo: {}",
        result
    );
}

#[test]
fn test_integration_infinitive_not_corrected_when_not_imperative() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Callar es dificil.");

    assert!(
        !result.contains("["),
        "No deberia corregir infinitivo en uso no imperativo: {}",
        result
    );
}

// ==========================================================================
// Haber impersonal pluralizado
// ==========================================================================

#[test]
fn test_impersonal_habian_muchas_personas() {
    let corrector = create_test_corrector();
    let result = corrector.correct("habían muchas personas");
    assert!(
        result.contains("[había]") || result.contains("[Había]"),
        "Debería corregir 'habían' → 'había': {}",
        result
    );
}

#[test]
fn test_impersonal_hubieron_accidentes() {
    let corrector = create_test_corrector();
    let result = corrector.correct("hubieron accidentes graves");
    assert!(
        result.contains("[hubo]") || result.contains("[Hubo]"),
        "Debería corregir 'hubieron' → 'hubo': {}",
        result
    );
}

#[test]
fn test_impersonal_habran_consecuencias() {
    let corrector = create_test_corrector();
    let result = corrector.correct("habrán consecuencias graves");
    assert!(
        result.contains("[habrá]") || result.contains("[Habrá]"),
        "Debería corregir 'habrán' → 'habrá': {}",
        result
    );
}

#[test]
fn test_impersonal_habrian_problemas() {
    let corrector = create_test_corrector();
    let result = corrector.correct("habrían problemas serios");
    assert!(
        result.contains("[habría]") || result.contains("[Habría]"),
        "Debería corregir 'habrían' → 'habría': {}",
        result
    );
}

#[test]
fn test_impersonal_hayan_motivos() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No creo que hayan motivos");
    assert!(
        result.contains("[haya]"),
        "Debería corregir 'hayan' → 'haya': {}",
        result
    );
}

#[test]
fn test_impersonal_han_habido_quejas() {
    let corrector = create_test_corrector();
    let result = corrector.correct("han habido muchas quejas");
    assert!(
        result.contains("[ha]") || result.contains("[Ha]"),
        "Debería corregir 'han' → 'ha': {}",
        result
    );
}

#[test]
fn test_impersonal_habian_habido_problemas() {
    let corrector = create_test_corrector();
    let result = corrector.correct("habían habido problemas graves");
    assert!(
        result.contains("[había]") || result.contains("[Había]"),
        "Debería corregir 'habían' → 'había': {}",
        result
    );
}

#[test]
fn test_impersonal_hay_el_problema_to_un() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hay el problema de siempre");
    assert!(
        result.contains("el [un]"),
        "Deberia corregir articulo definido tras 'hay': {}",
        result
    );
}

#[test]
fn test_impersonal_hay_la_solucion_to_una() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hay la solucion adecuada");
    assert!(
        result.contains("la [una]"),
        "Deberia corregir articulo definido tras 'hay': {}",
        result
    );
}

#[test]
fn test_impersonal_habia_el_problema_to_un() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Habia el problema de siempre");
    assert!(
        result.contains("el [un]"),
        "Deberia corregir articulo definido tras 'habia': {}",
        result
    );
}

#[test]
fn test_impersonal_hay_un_problema_no_correction() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hay un problema de siempre");
    assert!(
        !result.contains("["),
        "No deberia corregir cuando ya es indefinido: {}",
        result
    );
}

#[test]
fn test_impersonal_hay_la_de_no_correction() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hay la de gente");
    assert!(
        !result.contains("["),
        "No deberia tocar la locucion 'la de': {}",
        result
    );
}

#[test]
fn test_impersonal_no_corrige_auxiliar() {
    let corrector = create_test_corrector();

    // Auxiliar: "habían comido" es correcto
    let result = corrector.correct("Ellos habían comido mucho");
    assert!(
        !result.contains("[había]"),
        "No debería corregir auxiliar 'habían comido': {}",
        result
    );

    // Auxiliar: "han llegado" es correcto
    let result = corrector.correct("Ya han llegado los invitados");
    assert!(
        !result.contains("[ha]"),
        "No debería corregir auxiliar 'han llegado': {}",
        result
    );
}

#[test]
fn test_impersonal_no_corrige_haber_de() {
    let corrector = create_test_corrector();
    let result = corrector.correct("habían de marcharse pronto");
    assert!(
        !result.contains("[había]"),
        "No debería corregir perífrasis 'habían de': {}",
        result
    );
}

// ==========================================================================
// Hacer impersonal temporal
// ==========================================================================

#[test]
fn test_hacer_impersonal_hacen_tres_años() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Eso hacen tres años que no nos vemos");
    assert!(
        result.contains("[hace]"),
        "Debería corregir 'hacen' → 'hace': {}",
        result
    );
}

#[test]
fn test_hacer_impersonal_hacian_muchos_dias() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ya hacían muchos días que no llovía");
    assert!(
        result.contains("[hacía]"),
        "Debería corregir 'hacían' → 'hacía': {}",
        result
    );
}

#[test]
fn test_hacer_impersonal_haran_dos_meses() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Pronto harán dos meses que se fue");
    assert!(
        result.contains("[hará]"),
        "Debería corregir 'harán' → 'hará': {}",
        result
    );
}

#[test]
fn test_hacer_impersonal_hacen_capitalization() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hacen ya varios meses");
    assert!(
        result.contains("[Hace]"),
        "Debería corregir 'Hacen' → 'Hace' con mayúscula: {}",
        result
    );
}

#[test]
fn test_hacer_impersonal_no_corrige_transitivo() {
    let corrector = create_test_corrector();

    // "Ellos hacen deporte cada día" → transitivo, no impersonal
    let result = corrector.correct("Ellos hacen deporte cada día");
    assert!(
        !result.contains("[hace]"),
        "No debería corregir transitivo 'Ellos hacen deporte': {}",
        result
    );
}

#[test]
fn test_hacer_impersonal_no_corrige_objeto_lexico() {
    let corrector = create_test_corrector();

    // "Los niños hacen tres horas de deberes" → objeto léxico
    let result = corrector.correct("Los niños hacen tres horas de deberes");
    assert!(
        !result.contains("[hace]"),
        "No debería corregir objeto léxico 'tres horas de deberes': {}",
        result
    );
}

#[test]
fn test_hacer_impersonal_hicieron_temporal() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ya hicieron dos semanas que se marchó");
    assert!(
        result.contains("[hizo]"),
        "Debería corregir 'hicieron' → 'hizo': {}",
        result
    );
}

// ── Concordancia adjetivo-sustantivo con homógrafos verbales ──

#[test]
fn test_adjective_verb_homograph_plural_noun_larga() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los caminos larga");
    assert!(
        result.contains("[largos]"),
        "Debería corregir 'larga' → 'largos' con sustantivo plural: {}",
        result
    );
}

#[test]
fn test_adjective_verb_homograph_plural_noun_corta() {
    let corrector = create_test_corrector();
    let result = corrector.correct("las calles corta");
    assert!(
        result.contains("[cortas]"),
        "Debería corregir 'corta' → 'cortas' con sustantivo plural: {}",
        result
    );
}

#[test]
fn test_adjective_verb_homograph_plural_noun_seca() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los ríos seca");
    assert!(
        result.contains("[secos]"),
        "Debería corregir 'seca' → 'secos' con sustantivo plural: {}",
        result
    );
}

#[test]
fn test_adjective_verb_homograph_not_corrected_with_object() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los caminos corta el paso");
    assert!(
        !result.contains("[cortos]") && !result.contains("[cortas]"),
        "No debería corregir 'corta' cuando le sigue artículo (contexto verbal): {}",
        result
    );
}

#[test]
fn test_integration_dequeismo_preterite_plural_forms() {
    let corrector = create_test_corrector();
    let cases = [
        "Pensaron de que era fácil",
        "Dijeron de que vendría",
        "Creyeron de que era posible",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains("~~de~~ que"),
            "Debería detectar dequeísmo en pretérito plural: '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_dequeismo_es_adjetivo_de_que() {
    let corrector = create_test_corrector();
    let cases = [
        "Es posible de que llueva",
        "Es probable de que venga",
        "Es necesario de que estudies",
        "Es importante de que vengas",
        "Es cierto de que funcione",
        "Es verdad de que llueva",
        "Es evidente de que ocurre",
        "Es seguro de que viene",
        "Es obvio de que falta",
        "Es claro de que conviene",
        "Es logico de que pase",
        "Es natural de que duela",
        "Es normal de que suceda",
        "Es falso de que exista",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains("~~de~~ que"),
            "Deberia detectar dequeismo en patron 'es + adjetivo + de que': '{}': {}",
            text,
            result
        );
    }

    let result_ok = corrector.correct("Es posible que llueva");
    assert!(
        !result_ok.contains("~~de~~"),
        "No deberia marcar uso correcto sin 'de': {}",
        result_ok
    );
}

#[test]
fn test_integration_queismo_no_cabe_duda_que() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No cabe duda que vendrá");

    assert!(
        result.contains("que [de que]"),
        "Debería detectar queísmo en 'No cabe duda que': {}",
        result
    );
}

#[test]
fn test_integration_queismo_es_hora_que() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es hora que te vayas");

    assert!(
        result.contains("que [de que]"),
        "Deberia detectar queismo en 'Es hora que': {}",
        result
    );
}

#[test]
fn test_integration_queismo_preposition_specific_patterns() {
    let corrector = create_test_corrector();
    let cases = [
        ("Depende que llueva", "que [de que]"),
        ("Consiste que todos participen", "que [en que]"),
        ("Se trata que seamos puntuales", "que [de que]"),
        ("Me fijé que estaba roto", "que [en que]"),
        ("Insisto que vengas", "que [en que]"),
        ("Confio que salga bien", "que [en que]"),
        ("Aspiramos que nos elijan", "que [a que]"),
        ("Se preocupa que no llueva", "que [de que]"),
        ("Me averguenzo que me vean", "que [de que]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia detectar queismo con preposicion adecuada en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_queismo_antes_despues_que_temporal() {
    let corrector = create_test_corrector();
    let cases = [
        ("Antes que llegues", "que [de que]"),
        ("Despues que termino", "que [de que]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Deberia detectar queismo temporal en '{}': {}",
            input,
            result
        );
    }

    let result_ok = corrector.correct("Antes que nada");
    assert!(
        !result_ok.contains("que [de que]"),
        "No deberia forzar 'de que' en comparativo/fijado 'antes que nada': {}",
        result_ok
    );
}

#[test]
fn test_integration_queismo_despues_with_inserted_temporal_phrase_no_false_positive() {
    let corrector = create_test_corrector();
    let cases = [
        "Renato Flores reconocio horas despues que hubo un fallo",
        "Hablo minutos despues que habia un problema",
        "Reconocio dias despues que se equivoco",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.to_lowercase().contains("que [de que]"),
            "No debe forzar 'despues de que' cuando 'que' es completiva en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_compound_participle_invariable_extended_coverage() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Los han vendidos");
    assert!(
        result.contains("vendidos [vendido]"),
        "Debe corregir participio plural tras haber: {}",
        result
    );

    let result = corrector.correct("Las he compradas");
    assert!(
        result.contains("compradas [comprado]"),
        "Debe corregir participio femenino tras haber: {}",
        result
    );

    let result = corrector.correct("Las he vistas");
    assert!(
        result.contains("vistas [visto]") && !result.contains("vistas [vestido]"),
        "Debe corregir 'vistas' a 'visto' (no 'vestido'): {}",
        result
    );

    let result = corrector.correct("Ha vida en ese planeta");
    assert!(
        !result.to_lowercase().contains("vida [vido]"),
        "No debe inventar participio inexistente desde sustantivo 'vida': {}",
        result
    );
}

#[test]
fn test_integration_hubieron_varias_no_compound_conflict() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hubieron varias quejas");

    assert!(
        result.to_lowercase().contains("hubieron [hubo]"),
        "Debe corregir haber impersonal pluralizado: {}",
        result
    );
    assert!(
        !result.to_lowercase().contains("varias [variado]"),
        "No debe reinterpretar 'varias' como participio: {}",
        result
    );
}

#[test]
fn test_integration_homophone_estar_echo_variants() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Esta mal echo");
    assert!(
        result.to_lowercase().contains("echo [hecho]"),
        "Debe corregir 'mal echo' -> 'mal hecho': {}",
        result
    );

    let result = corrector.correct("Bien echo");
    assert!(
        result.to_lowercase().contains("echo [hecho]"),
        "Debe corregir 'bien echo' -> 'bien hecho': {}",
        result
    );

    let result = corrector.correct("La tarea esta echa");
    assert!(
        result.to_lowercase().contains("echa [hecha]"),
        "Debe corregir 'esta echa' -> 'esta hecha': {}",
        result
    );
}

#[test]
fn test_integration_homophone_halla_extended_subjunctive_contexts() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Espero que lo halla");
    assert!(
        result.to_lowercase().contains("halla [haya]"),
        "Debe corregir 'que lo halla' -> 'que lo haya': {}",
        result
    );

    let result = corrector.correct("Ojala halla solucion");
    assert!(
        result.to_lowercase().contains("halla [haya]"),
        "Debe corregir 'ojala halla solucion' -> 'ojala haya solucion': {}",
        result
    );
}

#[test]
fn test_integration_queismo_percatarse_de_que() {
    let corrector = create_test_corrector();
    let cases = ["Se percato que era tarde", "Nos percatamos que faltaba"];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("que [de que]"),
            "Debe detectar queismo con 'percatarse de que' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_pronoun_mismo_agreement() {
    let corrector = create_test_corrector();
    let cases = [
        ("Ella mismo lo dijo", "mismo [misma]"),
        ("Ellas mismos vinieron", "mismos [mismas]"),
        ("Nosotras mismos decidimos", "mismos [mismas]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe corregir concordancia pronombre+mismo en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_pleonasm_mas_comparative() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Es mas mejor asi");
    assert!(
        result.to_lowercase().contains("~~mas~~"),
        "Debe marcar 'mas' redundante en 'mas mejor': {}",
        result
    );

    let result = corrector.correct("Nivel mas superior");
    assert!(
        result.to_lowercase().contains("~~mas~~"),
        "Debe marcar 'mas' redundante en 'mas superior': {}",
        result
    );

    for text in [
        "Es mas mayor que yo",
        "Es el mas mayor de todos",
        "Es mas menor que yo",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("~~mas~~"),
            "Debe marcar 'mas' redundante en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_echos_in_copular_context_prefers_grammar() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los deberes est\u{00E1}n echos");

    assert!(
        result.to_lowercase().contains("echos [hechos]"),
        "Debe corregir 'echos' -> 'hechos' en contexto copulativo: {}",
        result
    );
    assert!(
        !result.contains('|'),
        "No debe degradar a sugerencia ortogr\u{00E1}fica en este caso: {}",
        result
    );
}

#[test]
fn test_integration_noun_adjective_number_agreement_invariable_gender() {
    let corrector = create_test_corrector();
    let cases = [
        ("Problemas grave", "grave [graves]"),
        ("Problema graves", "graves [grave]"),
        ("Cosas importante", "importante [importantes]"),
        ("Libros interesante", "interesante [interesantes]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe corregir concordancia de n\u{00FA}mero en adjetivo invariable para '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_noun_adjective_number_singularization_avoids_truncation() {
    let corrector = create_test_corrector();
    let cases = [
        ("Persona amables", "amables [amable]"),
        ("Tema notables", "notables [notable]"),
        ("Persona posibles", "posibles [posible]"),
        ("Persona vulnerables", "vulnerables [vulnerable]"),
        ("Persona responsables", "responsables [responsable]"),
        ("Problema simples", "simples [simple]"),
        ("Viaje salvajes", "salvajes [salvaje]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe singularizar sin truncar en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_cuyo_agreement_with_possessed_noun() {
    let corrector = create_test_corrector();
    let cases = [
        ("El profesor cuyo clase es dificil", "cuyo [cuya]"),
        ("El pais cuyo fronteras son extensas", "cuyo [cuyas]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe corregir concordancia de 'cuyo' en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_apocope_before_singular_noun() {
    let corrector = create_test_corrector();
    let cases = [
        ("Un bueno día", "bueno [buen]"),
        ("Un malo presagio", "malo [mal]"),
        ("Un grande hombre", "grande [gran]"),
        ("La grande periodista", "grande [gran]"),
        ("Una grande mujer", "grande [gran]"),
        ("El primero piso", "primero [primer]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe aplicar ap\u{00F3}cope en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_apocope_alguno_ninguno_before_masculine_singular_noun() {
    let corrector = create_test_corrector();
    let cases = [
        ("Ninguno hombre vino", "Ninguno [Ningún]"),
        ("Alguno d\u{00ED}a lo har\u{00E9}", "Alguno [Algún]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe aplicar apocope cuantificadora en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_homophone_halla_modal_context_without_participle() {
    let corrector = create_test_corrector();
    let cases = [
        "No creo que halla nadie",
        "Es posible que halla",
        "Dudo que halla tiempo",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("halla [haya]"),
            "Debe corregir 'halla' -> 'haya' en contexto modal de subjuntivo en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_queismo_reflexive_additional_verbs() {
    let corrector = create_test_corrector();
    let cases = [
        "Se convenci\u{00F3} que era cierto",
        "Se aprovech\u{00F3} que nadie miraba",
        "Se encarg\u{00F3} que llegara a tiempo",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("que [de que]"),
            "Debe detectar que\u{00ED}smo en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_desconozco_porque_indirect_question() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Desconozco porque se fue");

    assert!(
        result.to_lowercase().contains("porque [por qu\u{00E9}]"),
        "Debe corregir interrogativo indirecto tras 'desconozco': {}",
        result
    );
}

#[test]
fn test_integration_homophone_haber_comma_discourse_marker() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Haber, ven aqui");

    assert!(
        result.to_lowercase().contains("haber [a ver]"),
        "Debe corregir marcador discursivo 'Haber,' -> 'A ver,': {}",
        result
    );
}

#[test]
fn test_integration_homophone_haber_discourse_marker_without_comma() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Haber ven aqui");

    assert!(
        result.to_lowercase().contains("haber [a ver]"),
        "Debe corregir marcador discursivo 'Haber ven ...' -> 'A ver ven ...': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ha_prepositional_locutions() {
    let corrector = create_test_corrector();
    let cases = [
        ("Ha veces no se", "Ha [A]"),
        ("Ha traves de la calle", "Ha [A]"),
        ("Ha menudo viene", "Ha [A]"),
        ("Ha que hora es", "Ha [A]"),
        ("Ha donde vas", "Ha [A]"),
        ("Ha causa de la lluvia", "Ha [A]"),
    ];

    for (text, expected_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe corregir 'ha' preposicional en '{}': {}",
            text,
            result
        );
    }

    let causal = corrector.correct("Ha causa de la lluvia");
    assert!(
        !causal.contains("causa [causado]"),
        "No debe proponer participio en locucion preposicional 'a causa de': {}",
        causal
    );
}

#[test]
fn test_integration_homophone_hay_exclamative_to_ay() {
    let corrector = create_test_corrector();
    for text in ["\u{00A1}Hay que bonito!", "Hay que bonito es"] {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("hay [ay]"),
            "Debe corregir interjeccion exclamativa en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_uno_de_los_mejores_que_singular_is_corrected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Uno de los mejores que existi\u{00F3}");

    assert!(
        result.to_lowercase().contains("existió [existieron]"),
        "Deberia corregir relativo plural en 'uno de los mejores que ...': {}",
        result
    );
}

#[test]
fn test_integration_subject_verb_with_parenthetical_incise() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El presidente (del partido) anunciaron");

    assert!(
        result.to_lowercase().contains("anunciaron [anunci"),
        "Debe mantener tracking de sujeto a traves de parentesis: {}",
        result
    );
}

#[test]
fn test_integration_capitalization_me_after_sentence_boundary() {
    let corrector = create_test_corrector();

    for text in ["Hola. me llamo Juan", "¡Bien! me alegro"] {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("me [me]"),
            "Debe capitalizar 'me' tras fin de oracion en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_ha_traves_del() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ha traves del cristal");

    assert!(
        result.to_lowercase().contains("ha [a]"),
        "Debe corregir 'ha traves del' -> 'a traves del': {}",
        result
    );
}

#[test]
fn test_integration_homophone_estar_missing_accent_extended_cases() {
    let corrector = create_test_corrector();

    let result = corrector.correct("¿Cómo estas?");
    assert!(
        result.to_lowercase().contains("estas [estás]"),
        "Debe corregir 2ª persona 'estas' -> 'estás': {}",
        result
    );

    let result = corrector.correct("Esta todo bien");
    assert!(
        result.to_lowercase().contains("esta [está]"),
        "Debe corregir 'Esta todo bien' -> 'Está todo bien': {}",
        result
    );

    let result = corrector.correct("¿Cómo esta usted?");
    assert!(
        result.to_lowercase().contains("esta [está]"),
        "Debe corregir inversión interrogativa con 'usted': {}",
        result
    );
}

#[test]
fn test_integration_homophone_ha_a_and_haber_a_ver_extended_cases() {
    let corrector = create_test_corrector();

    let result = corrector.correct("El tren a partido");
    assert!(
        result.to_lowercase().contains("a [ha]"),
        "Debe corregir 'a' -> 'ha' ante participio ambiguo: {}",
        result
    );

    let result = corrector.correct("Voy ha la tienda");
    assert!(
        result.to_lowercase().contains("ha [a]"),
        "Debe corregir 'Voy ha la tienda' -> 'Voy a la tienda': {}",
        result
    );

    let result = corrector.correct("Compraré pan, haber si hay");
    assert!(
        result.to_lowercase().contains("haber [a ver]"),
        "Debe corregir 'haber si' tras coma: {}",
        result
    );
}

#[test]
fn test_integration_ademas_with_accent() {
    let corrector = create_test_corrector();
    let result = corrector.correct("ademas");
    assert!(
        result.to_lowercase().contains("ademas [además]"),
        "Debe corregir 'ademas' -> 'además': {}",
        result
    );
}

#[test]
fn test_integration_ademas_does_not_generate_identity_correction() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Y además seguimos");

    assert!(
        !result.to_lowercase().contains("además [además]"),
        "No debe generar correccion idempotente en 'ademas': {}",
        result
    );
}

#[test]
fn test_integration_valid_verbs_inquietan_and_pronostica_not_spelling_errors() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Ellos inquietan a todos");
    assert!(
        !result.to_lowercase().contains("inquietan |"),
        "No debe marcar 'inquietan' como error ortografico: {}",
        result
    );

    let result = corrector.correct("El modelo pronostica lluvia");
    assert!(
        !result.to_lowercase().contains("pronostica |"),
        "No debe marcar 'pronostica' como error ortografico: {}",
        result
    );
}

#[test]
fn test_integration_proper_names_ukraine_context_not_flagged_as_spelling_errors() {
    let corrector = create_test_corrector();
    let result = corrector
        .correct("Donbass, Kyiv, Sloviansk, Járkiv, Lavrov, Kremlin, Zaporiya y Novarosiya.");

    for name in [
        "donbass",
        "kyiv",
        "sloviansk",
        "járkiv",
        "lavrov",
        "kremlin",
        "zaporiya",
        "novarosiya",
    ] {
        assert!(
            !result.to_lowercase().contains(&format!("{name} |")),
            "No debe sugerir ortografia para nombre propio '{}': {}",
            name,
            result
        );
    }
}

#[test]
fn test_integration_esta_verb_without_accent_and_determiner_context() {
    let corrector = create_test_corrector();

    let result = corrector.correct("La casa esta bien");
    assert!(
        result.to_lowercase().contains("esta [está]"),
        "Debe corregir 'esta' verbal sin tilde: {}",
        result
    );

    let result = corrector.correct("El coche esta semana");
    assert!(
        !result.to_lowercase().contains("esta [está]"),
        "No debe corregir determinante 'esta' en complemento temporal: {}",
        result
    );
}

#[test]
fn test_integration_copulative_predicative_reflexive_volverse_hacerse() {
    let corrector = create_test_corrector();
    let cases = [
        ("Ella se volvio loco", "loco [loca]"),
        ("Ella se hizo rico", "rico [rica]"),
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result
                .to_lowercase()
                .contains(&expected_fragment.to_lowercase()),
            "Debe corregir pseudocopulativo reflexivo en '{}': {}",
            input,
            result
        );
    }
}

#[test]
fn test_integration_copulative_predicative_with_salir() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los estudiantes salieron contento");

    assert!(
        result.to_lowercase().contains("contento [contentos]"),
        "Debe corregir predicativo con verbo de cambio/resultado 'salir': {}",
        result
    );
}

#[test]
fn test_integration_subject_pronoun_indefinite_and_coordination_with_yo() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Nadie vinieron");
    assert!(
        result.to_lowercase().contains("vinieron [vino]"),
        "Debe corregir concordancia singular con 'nadie': {}",
        result
    );

    let result = corrector.correct("Mi hermano y yo va");
    assert!(
        result.to_lowercase().contains("va [vamos]"),
        "Debe corregir 'X y yo' a primera plural: {}",
        result
    );
}

#[test]
fn test_integration_pronoun_ni_correlative_with_yo_requires_first_plural() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Ni tú ni yo sabe");
    assert!(
        result.to_lowercase().contains("sabe [sabemos]"),
        "Debe corregir 'Ni tú ni yo sabe' a primera plural: {}",
        result
    );

    let result = corrector.correct("Ni yo ni tú sabe");
    assert!(
        result.to_lowercase().contains("sabe [sabemos]"),
        "Debe corregir 'Ni yo ni tú sabe' a primera plural: {}",
        result
    );
}

#[test]
fn test_integration_copulative_predicative_subject_with_de_complement() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La gata de Juan está gordo");

    assert!(
        result.to_lowercase().contains("gordo [gorda]"),
        "Debe mantener sujeto nominal a traves de complemento 'de ...': {}",
        result
    );
}

#[test]
fn test_integration_sobretodo_aparte_and_leismo_with_explicit_feminine_referent() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Sobretodo me gusta");
    assert!(
        result.to_lowercase().contains("sobretodo [sobre todo]"),
        "Debe corregir locucion adverbial 'sobre todo': {}",
        result
    );

    let result = corrector.correct("A parte de eso");
    assert!(
        result.to_lowercase().contains("a [aparte]"),
        "Debe corregir 'a parte de' -> 'aparte de': {}",
        result
    );

    let result = corrector.correct("Les vi a ellas");
    assert!(
        result.to_lowercase().contains("les [las]"),
        "Debe sugerir 'las' con referente femenino explicito: {}",
        result
    );
}

#[test]
fn test_integration_predicative_vez_adverbial_expressions_no_false_positive() {
    let corrector = create_test_corrector();
    for text in [
        "Tal vez sea cierto",
        "A veces es complicado",
        "Muchas veces es difícil",
        "Pocas veces fue tan claro",
        "Dos veces fue suficiente",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains('['),
            "No debe forzar concordancia predicativa con 'vez/veces' adverbial en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_hacer_falta_quantified_no_false_positive() {
    let corrector = create_test_corrector();
    for text in [
        "Hacen falta más recursos",
        "Hace falta más dinero",
        "Hace falta más tiempo",
        "Hacen falta más médicos",
        "Hacen falta más camas",
        "Hacen falta tres voluntarios",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains('['),
            "No debe tocar construcción 'hacer falta' cuantificada en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_subject_verb_imperfect_preserved() {
    let corrector = create_test_corrector();
    let cases = [
        ("Los niños jugaba en el parque", "jugaba [jugaban]"),
        ("Los libros costaba mucho", "costaba [costaban]"),
        ("Las calles estaba vacías", "estaba [estaban]"),
        ("Los perros ladraba mucho", "ladraba [ladraban]"),
        ("Los niños comía a las dos", "comía [comían]"),
        ("Los niños tenía hambre", "tenía [tenían]"),
        ("Los niños quería jugar", "quería [querían]"),
        ("Los niños iba al parque", "iba [iban]"),
        ("Las casas era grandes", "era [eran]"),
    ];

    for (text, expected) in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains(&expected.to_lowercase()),
            "Debe preservar imperfecto al corregir concordancia en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_relative_clause_object_not_used_as_main_subject() {
    let corrector = create_test_corrector();
    for text in [
        "La casa que tiene garaje es cara",
        "La mesa que tiene cajón es vieja",
        "El libro que escribió María es bueno",
        "El edificio que necesita reforma es viejo",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains('['),
            "No debe usar objeto de relativa como sujeto de la principal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_homophone_estas_predicative_cases() {
    let corrector = create_test_corrector();
    let cases = [
        ("Estas enfermo", "estas [estás]"),
        ("Estas lista", "estas [estás]"),
        ("Estas seguro de eso", "estas [estás]"),
        ("No estas contento", "estas [estás]"),
    ];

    for (text, expected) in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains(expected),
            "Debe corregir 'estas' verbal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_common_gender_professions_no_false_positive() {
    let corrector = create_test_corrector();
    for text in [
        "El pianista es talentoso",
        "El dentista es cuidadoso",
        "El novelista es brillante",
        "El contratista es responsable",
        "El turista es curioso",
        "El terapeuta es atento",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains('['),
            "No debe forzar femenino en sustantivos de género común en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_predicative_nominal_phrase_internal_agreement_respected() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los niños son buenas personas");
    assert!(
        !result.contains("buenas ["),
        "No debe cambiar adjetivo que ya concuerda con el núcleo de la frase nominal: {}",
        result
    );
}

#[test]
fn test_integration_homophone_ha_before_articles_and_time_phrases() {
    let corrector = create_test_corrector();
    let cases = [
        "Ha la vista",
        "Ha lo lejos",
        "Ha lo mejor",
        "Ha las cinco",
        "Ha lo largo del río",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains("ha [a]"),
            "Debe corregir 'ha' -> 'a' en locución nominal/temporal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_collective_family_and_public_subjects() {
    let corrector = create_test_corrector();

    let result = corrector.correct("La familia se reunieron");
    assert!(
        result.to_lowercase().contains("reunieron [reunió]"),
        "Debe corregir concordancia singular con 'familia': {}",
        result
    );

    let result = corrector.correct("El público aplaudieron");
    assert!(
        result.to_lowercase().contains("aplaudieron [aplaudió]"),
        "Debe corregir concordancia singular con 'público': {}",
        result
    );
}

#[test]
fn test_integration_pronoun_iste_present_not_forced_to_preterite() {
    let corrector = create_test_corrector();
    for text in [
        "Ella consiste en eso",
        "Ella insiste en ir",
        "Ella existe desde hace anos",
        "Ella persiste en el error",
        "Ella resiste bien",
        "Ella asiste a clase",
        "Ella viste ropa elegante",
        "Usted existe para ayudar",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains('['),
            "No debe forzar preterito en forma presente terminada en -iste: '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_noun_adverb_adjective_de_complement_not_crossed() {
    let corrector = create_test_corrector();
    for text in [
        "Un plan de inversiones muy ambicioso",
        "Un grupo de personas realmente grande",
        "La red de carreteras bastante extensa",
        "El nivel de dificultad muy alto",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains('['),
            "No debe cruzar complemento con 'de' para concordancia atributiva en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_initial_infinitive_clause_subject_not_reanalysed() {
    let corrector = create_test_corrector();
    let cases = [
        ("Comer frutas es saludable", "saludable [saludables]"),
        ("Tener hijos es maravilloso", "maravilloso [maravillosos]"),
        ("Preparar la cena es aburrido", "aburrido [aburrida]"),
        ("Lavar platos es aburrido", "aburrido [aburridos]"),
        (
            "Resolver problemas es satisfactorio",
            "satisfactorio [satisfactorios]",
        ),
        ("Pero comprar flores es divertido", "divertido [divertidas]"),
        ("Y comprar flores es divertido", "divertido [divertidas]"),
        (
            "Ademas, comprar flores es divertido",
            "divertido [divertidas]",
        ),
        ("Sin embargo, comprar flores es caro", "caro [caras]"),
        ("No obstante, comprar flores es caro", "caro [caras]"),
        ("De hecho, comprar flores es caro", "caro [caras]"),
        ("En todo caso, comprar flores es caro", "caro [caras]"),
        ("Por otro lado, comprar flores es caro", "caro [caras]"),
        ("Con todo, comprar flores es caro", "caro [caras]"),
    ];
    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe usar objeto interno del infinitivo inicial como sujeto en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_he_hecho_de_menos_prefers_echado() {
    let corrector = create_test_corrector();
    let result = corrector.correct("He hecho de menos a mi familia");
    assert!(
        result.to_lowercase().contains("hecho [echado]"),
        "Debe corregir 'he hecho de menos' a 'he echado de menos': {}",
        result
    );
}

#[test]
fn test_integration_hacer_falta_not_rephrased_to_faltar() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Me hacen falta herramientas");
    assert!(
        !result.contains('['),
        "No debe reescribir construccion valida 'hacen falta': {}",
        result
    );
}

#[test]
fn test_integration_copulative_y_proper_name_not_gender_changed() {
    let corrector = create_test_corrector();
    let cases = [
        "Pedro es alto y Maria baja",
        "Pedro es gordo y Ana delgada",
        "Pedro es feliz y Maria triste",
        "Juan es guapo y Maria guapa",
        "Pedro es simpatico y Ana simpatica",
        "Juan es alto y Maria es baja",
    ];

    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains("Maria [Mario]") && !result.contains("Ana [Ano]"),
            "No debe flexionar nombres propios en copulativas coordinadas: '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_gustar_like_postposed_coordinated_subject_not_singularized() {
    let corrector = create_test_corrector();
    let cases = [
        ("Me gustan el cine y la musica", "gustan [gusta]"),
        ("Me gustan el pan y la leche", "gustan [gusta]"),
        (
            "Le interesan la politica y la economia",
            "interesan [interesa]",
        ),
        (
            "Nos preocupan el clima y la contaminacion",
            "preocupan [preocupa]",
        ),
        ("Me encantan la playa y la montana", "encantan [encanta]"),
        ("Me gustan la pizza y el helado", "gustan [gusta]"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe singularizar verbo tipo gustar con sujeto pospuesto coordinado en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_gustar_like_coordination_stops_before_new_clause() {
    let corrector = create_test_corrector();
    let cases = [
        (
            "Me gusta el chocolate y la vainilla es buena",
            "gusta [gustan]",
            "buena [buenos]",
        ),
        (
            "Me encanta la playa y el mar es precioso",
            "encanta [encantan]",
            "precioso [preciosos]",
        ),
        (
            "Me gusta el verano y el otono es bonito",
            "gusta [gustan]",
            "bonito [bonitos]",
        ),
        (
            "Me gusta la Navidad y el invierno es frio",
            "gusta [gustan]",
            "es [son]",
        ),
    ];

    for (text, wrong_verb, wrong_adj) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_verb) && !result.contains(wrong_adj),
            "No debe cruzar a la clausula siguiente tras 'y + SN + verbo' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_parenthetical_reporting_verb_not_forced_to_main_subject() {
    let corrector = create_test_corrector();
    let cases = [
        ("El ministro, segun dicen, renunciara", "dicen [dice]"),
        ("La ley, segun cuentan, cambiara", "cuentan [cuenta]"),
        ("El asunto, como saben, es grave", "saben [sabe]"),
        (
            "La noticia, segun confirman, es cierta",
            "confirman [confirma]",
        ),
        (
            "El plan, como anunciaron, es ambicioso",
            "anunciaron [anuncio]",
        ),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe forzar concordancia del verbo parentetico con el sujeto principal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_de_infinitive_complement_not_used_as_main_subject() {
    let corrector = create_test_corrector();
    let cases = [
        (
            "La necesidad de implementar nuevas medidas es urgente",
            "urgente [urgentes]",
        ),
        (
            "El deseo de comprar mas productos es comprensible",
            "comprensible [comprensibles]",
        ),
        (
            "La obligacion de pagar los impuestos es clara",
            "clara [claros]",
        ),
        (
            "La costumbre de comer tapas es espanola",
            "espanola [espanolas]",
        ),
        (
            "El placer de leer buenos libros es inmenso",
            "inmenso [inmensos]",
        ),
        ("La posibilidad de ganar el premio es alta", "alta [alto]"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe usar el objeto interno de 'de + infinitivo' como sujeto principal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_vocative_name_plus_er_ir_present_not_forced() {
    let corrector = create_test_corrector();
    let cases = [
        ("Juan come pan todos los dias", "Juan [Juan,]"),
        ("Pedro bebe agua", "Pedro [Pedro,]"),
        ("Ana lee el periodico", "Ana [Ana,]"),
        ("Maria corre por el parque", "Maria [Maria,]"),
        ("Luis escribe cartas", "Luis [Luis,]"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe insertar coma vocativa en sujeto + verbo presente en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_explica_que_completive_not_interrogative() {
    let corrector = create_test_corrector();
    for text in [
        "El experto explica que todo va bien",
        "La profesora explica que la situacion es dificil",
        "Pedro explica que no puede venir",
        "Me explica que no hay problema",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("que [qué]") && !result.contains("que [quÃ©]"),
            "No debe acentuar 'que' completivo tras 'explica' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_el_mismo_plus_specific_nominals_not_pronoun() {
    let corrector = create_test_corrector();
    for text in [
        "Comparten el mismo final",
        "Comparten el mismo ideal",
        "Comparten el mismo origen",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("el [él]") && !result.contains("el [Ã©l]"),
            "No debe corregir 'el mismo + sustantivo' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_caber_irregular_forms_recognized() {
    let corrector = create_test_corrector();
    for text in ["No quepa duda", "Quepa lo que quepa", "Cupieron todos"] {
        let result = corrector.correct(text);
        let lower = result.to_lowercase();
        assert!(
            !lower.contains("quepa |") && !lower.contains("cupieron |"),
            "No debe marcar como spelling formas irregulares de 'caber' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_azuzar_and_resquebrajar_not_flagged() {
    let corrector = create_test_corrector();
    for text in [
        "Los perros azuzan al rebano",
        "La pared se resquebraja con el calor",
    ] {
        let result = corrector.correct(text);
        let lower = result.to_lowercase();
        assert!(
            !lower.contains("azuzan |") && !lower.contains("resquebraja |"),
            "No debe marcar como spelling verbos comunes en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_las_artes_not_forced_to_masculine_article() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Las artes son importantes");
    assert!(
        !result.contains("Las [Los] artes"),
        "'Las artes' debe permanecer femenino plural: {}",
        result
    );
}

#[test]
fn test_integration_vociferantes_not_forced_to_masculine_article() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Las vociferantes amenazas");
    assert!(
        !result.contains("Las [Los]"),
        "No debe forzar articulo masculino con 'vociferantes': {}",
        result
    );
}

#[test]
fn test_integration_dequeismo_does_not_flip_predicative_adjective() {
    let corrector = create_test_corrector();
    let cases = [
        (
            "Pienso de que las calles de Madrid es muy bonitas",
            "bonitas [bonito]",
        ),
        (
            "Opino de que la situacion es complicada",
            "complicada [complicado]",
        ),
        (
            "Pienso de que los ninos son inteligentes",
            "inteligentes [inteligente]",
        ),
        ("Creo de que las mesas son grandes", "grandes [grande]"),
        ("Dijo de que ella es guapa", "guapa [guapo]"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe forzar concordancia predicativa por el verbo inicial en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_initial_infinitive_subject_does_not_force_plural_agreement() {
    let corrector = create_test_corrector();
    let cases = [
        (
            "Subir arriba los paquetes es dificil",
            "es [son]",
            "dificil [dificiles]",
        ),
        (
            "Bajar abajo las cajas es pesado",
            "es [son]",
            "pesado [pesadas]",
        ),
        (
            "Volver a repetir las lecciones es aburrido",
            "es [son]",
            "aburrido [aburridas]",
        ),
        (
            "Subir rapido los paquetes es dificil",
            "es [son]",
            "dificil [dificiles]",
        ),
    ];

    for (text, wrong_verb, wrong_adj) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_verb) && !result.contains(wrong_adj),
            "No debe concordar con el objeto interno de un sujeto infinitivo en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_veintiuna_is_recognized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Son las veintiuna horas");
    assert!(
        !result.contains("|"),
        "'veintiuna' no debe marcarse como error ortografico: {}",
        result
    );
    assert!(
        !result.contains("las [la]"),
        "'las veintiuna horas' no debe forzarse a singular: {}",
        result
    );
}

#[test]
fn test_integration_relative_names_and_pronouns_not_treated_as_verbs() {
    let corrector = create_test_corrector();
    let cases = [
        ("Los libros que Maria compro son buenos", "Maria ["),
        ("El coche que Juan compro es rojo", "Juan ["),
        ("Los cuadros que Ana pinto son famosos", "Ana ["),
        ("Las cartas que Sofia escribio son emotivas", "Sofia ["),
        (
            "Los edificios que Cristina proyecto son modernos",
            "Cristina [",
        ),
        ("Los coches que ella conduce son rapidos", "ella ["),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe tratar nombres/pronombres como verbos tras relativo en '{}': {}",
            text,
            result
        );
        if text.contains("ella conduce") {
            assert!(
                !result.contains("conduce ["),
                "No debe forzar concordancia del verbo con antecedente cuando hay sujeto 'ella': {}",
                result
            );
        }
    }
}

#[test]
fn test_integration_relative_transitive_object_plural_not_forced_to_singular() {
    let corrector = create_test_corrector();
    let cases = [
        ("La valla que pusieron es alta", "pusieron [ponio]"),
        ("La cancion que cantaron fue bonita", "cantaron [canto]"),
        ("La mesa que miraron es cara", "miraron [miro]"),
        ("La orden que dieron es clara", "dieron [dio]"),
        ("La decision que tomaron es buena", "tomaron [tomo]"),
        ("La foto que sacaron es bonita", "sacaron [saco]"),
        (
            "La empresa que fundaron hace veinte anos sigue funcionando",
            "fundaron [fundo]",
        ),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe forzar singular en relativas de objeto transitivo en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_relative_ambiguous_object_cases_not_forced_by_agreement() {
    let corrector = create_test_corrector();
    let cases = [
        ("El problema que tienen es serio", "tienen ["),
        ("Los problemas que tiene son serios", "tiene ["),
        ("La teoria que defendieron fue solida", "defendieron ["),
        ("Los argumentos que respalda son fuertes", "respalda ["),
        ("El hombre que cantaron", "cantaron ["),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe forzar concordancia en relativas ambiguas de objeto en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_subject_verb_skips_de_article_que_relative_bridge() {
    let corrector = create_test_corrector();
    let cases = [
        ("La mujer de la que hablan es simpatica", "hablan [habla]"),
        (
            "La mujer de la que hablaron es simpatica",
            "hablaron [hablo]",
        ),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe cruzar relativas 'de + art + que' para concordancia sujeto-verbo en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_noun_adj_verb_homograph_gender_mismatch_is_corrected() {
    let corrector = create_test_corrector();
    let cases = [
        ("Los niños contentas", "contentas [contentos]"),
        ("Los niños contentas juegan", "contentas [contentos]"),
    ];

    for (text, expected_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            result.contains(expected_fragment),
            "Debe corregir discordancia de genero en adjetivo homografo verbal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_predicative_partitive_plural_is_accepted() {
    let corrector = create_test_corrector();
    let cases = [
        ("La mitad de los votos fueron nulos", "nulos ["),
        ("La mayoria de las casas son antiguas", "antiguas ["),
        ("Un tercio de los alumnos estaban enfermos", "enfermos ["),
        (
            "La totalidad de los edificios fueron destruidos",
            "destruidos [",
        ),
        (
            "Gran parte de los trabajadores estan cansados",
            "cansados [",
        ),
        (
            "El conjunto de las pruebas fueron concluyentes",
            "concluyentes [",
        ),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe forzar singular en predicativa ad sensum partitiva en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_lexicalized_participle_noun_subject_allows_predicative_agreement() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El resultado esta clara");
    assert!(
        result.contains("clara [claro]"),
        "Debe corregir concordancia predicativa con 'resultado' nominalizado: {}",
        result
    );
}

#[test]
fn test_integration_no_de_que_clause_predicative_crossing() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La idea de que el mundo es plano es absurda");
    assert!(
        !result.contains("plano [plana]") && !result.contains("absurda [absurdo]"),
        "No debe cruzar cláusulas 'de que' en concordancia predicativa: {}",
        result
    );

    let result = corrector.correct("No cabe duda de que es cierto");
    assert!(
        !result.contains("cierto [cierta]"),
        "No debe forzar 'cierto' por 'duda' en construcción impersonal: {}",
        result
    );
}

#[test]
fn test_integration_subject_verb_does_not_cross_que_completive_clause() {
    let corrector = create_test_corrector();
    let result = corrector.correct("si viéramos que realmente ya está a la vuelta");
    assert!(
        !result.contains("está [están]")
            && !result.contains("está [están]")
            && !result.contains("esta [estan]"),
        "No debe forzar concordancia de 'está' con sujeto externo tras 'que': {}",
        result
    );
}

#[test]
fn test_integration_no_object_muy_adj_false_agreement() {
    let corrector = create_test_corrector();
    let cases = [
        ("Hicieron los deberes muy rapido", "rápido ["),
        ("Pintaron la casa muy bonito", "bonito ["),
        ("Cantaron las canciones muy alto", "alto ["),
        ("Resolvieron los problemas muy facil", "fácil ["),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe forzar concordancia en patrón V+OD+muy+adj en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_pronoun_after_sino_excepto_salvo_not_subject() {
    let corrector = create_test_corrector();
    let cases = [
        ("Nadie sino tú puede hacerlo", "puede [puedes]"),
        ("Nadie excepto tú sabe la verdad", "sabe [sabes]"),
        ("Todo salvo tú parece estar en orden", "parece [pareces]"),
        ("Nadie más que tú lo sabe", "sabe [sabes]"),
        ("Nadie más que yo lo sabe", "sabe [sé]"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe tratar pronombre tras sino/excepto/salvo como sujeto en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_comma_apposition_not_used_as_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("El director, no los profesores, tomó la decisión");
    assert!(
        !result.contains("tomó [tomaron]"),
        "No debe usar inciso con 'no' como sujeto principal: {}",
        result
    );

    let result = corrector.correct("Mi hermano, al igual que mis primos, estudia medicina");
    assert!(
        !result.contains("estudia [estudian]"),
        "No debe usar inciso 'al igual que' como sujeto principal: {}",
        result
    );

    let result = corrector.correct("Mi hermana, igual que tú, prefiere");
    assert!(
        !result.contains("prefiere [prefieres]"),
        "No debe usar inciso 'igual que' como sujeto principal: {}",
        result
    );

    let result = corrector.correct("Mi hermana, lo mismo que yo, prefiere");
    assert!(
        !result.contains("prefiere [prefiero]"),
        "No debe usar inciso 'lo mismo que' como sujeto principal: {}",
        result
    );
}

#[test]
fn test_integration_exceptive_parenthetical_not_used_as_subject() {
    let corrector = create_test_corrector();
    let cases = [
        (
            "Los jugadores, menos el portero, celebraron",
            "celebraron [celebró]",
        ),
        (
            "Las profesoras, salvo la directora, firmaron",
            "firmaron [firmó]",
        ),
        (
            "Todos los empleados, salvo el director, cobran",
            "cobran [cobra]",
        ),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe usar inciso exceptivo como sujeto principal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_pronoun_after_menos_not_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Todos menos tú vinieron a la fiesta");
    assert!(
        !result.contains("vinieron [viniste]"),
        "No debe tratar pronombre tras 'menos' como sujeto: {}",
        result
    );
}

#[test]
fn test_integration_preferir_sentir_pronoun_agreement() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Yo prefiere el chocolate");
    assert!(
        result.contains("prefiere [prefiero]"),
        "Debe corregir 'Yo prefiere' a 'prefiero': {}",
        result
    );

    let result = corrector.correct("Yo siente dolor");
    assert!(
        result.contains("siente [siento]"),
        "Debe corregir 'Yo siente' a 'siento': {}",
        result
    );

    let result = corrector.correct("Mi hermana, como tú, prefiere el chocolate");
    assert!(
        !result.contains("prefiere [prefieres]")
            && !result.contains("prefiere [prefes]")
            && !result.contains("prefiere [prefo]"),
        "No debe usar 'tú' en inciso como sujeto principal: {}",
        result
    );
}

#[test]
fn test_integration_enclitic_stem_change_gerunds_not_flagged() {
    let corrector = create_test_corrector();
    let cases = [
        ("Ayer estaba sintiéndose bien", "sintiéndose |"),
        ("Ayer estaba vistiéndose", "vistiéndose |"),
        ("Ayer estaba durmiéndose", "durmiéndose |"),
        ("Ayer estaba muriéndose de risa", "muriéndose |"),
        ("Ayer estaba divirtiéndose", "divirtiéndose |"),
        ("Ayer estaba arrepintiéndose", "arrepintiéndose |"),
        ("Ayer estaba mintiéndose", "mintiéndose |"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe marcar gerundio con enclítico como error ortográfico en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_missing_prefixed_infinitives_are_not_flagged() {
    let corrector = create_test_corrector();
    let cases = [
        ("Podemos entreabrir la puerta", "entreabrir |"),
        ("No debes presuponer nada", "presuponer |"),
        ("Van a sobrecargar el sistema", "sobrecargar |"),
        ("No conviene contraponer intereses", "contraponer |"),
        ("Podrían sobreponer una capa", "sobreponer |"),
        ("Quiere sobreproteger a su hijo", "sobreproteger |"),
        ("Suelen entrecruzar cables", "entrecruzar |"),
        ("No debemos sobreentender intenciones", "sobreentender |"),
    ];

    for (text, wrong_fragment) in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains(wrong_fragment),
            "No debe marcar infinitivo prefijado común como error ortográfico en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_missing_verbs_and_homograph_subject_verb() {
    let corrector = create_test_corrector();
    let lexical_cases = [
        "Anochece pronto",
        "Atardece tarde",
        "Las vacas mugen",
        "El gato ronronea",
        "Las ranas croan",
        "El gato maulla",
        "Se nublaba el cielo",
        "Me hipnotizo la musica",
        "No me incordies",
        "La ciudad empalidece",
        "El agua enturbia",
        "El valle reverdece",
        "El avión sobrevolaba la ciudad",
        "Los rios fluyen hacia el mar",
        "Eso desconcierta a todos",
        "Ellos manufacturan piezas",
        "La estratosfera protege la Tierra",
        "Pueden campar a sus anchas",
        "La preventa inicia hoy",
        "El autocorrector ayuda mucho",
        "Esto atañe a todos",
        "Estos asuntos atañen al equipo",
    ];

    for text in lexical_cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar forma verbal válida como ortográfica en '{}': {}",
            text,
            result
        );
    }

    let result = corrector.correct("Los perros ladra");
    assert!(
        result.contains("ladra [ladran]"),
        "Debe corregir homógrafo nominal/verbal con sujeto plural: {}",
        result
    );
    let result = corrector.correct("Los gatos maulla");
    assert!(
        result.contains("maulla ["),
        "Debe corregir homógrafo nominal/verbal con sujeto plural: {}",
        result
    );
}

#[test]
fn test_integration_o_correlative_pronouns_not_forced() {
    let corrector = create_test_corrector();
    let result = corrector.correct("O tú o yo tenemos razón");
    assert!(
        !result.contains("tenemos [tengo]") && !result.contains("tenemos [tienes]"),
        "No debe forzar persona en disyuntiva correlativa con pronombres: {}",
        result
    );
}

#[test]
fn test_integration_infinitive_subject_not_forced_after_long_leading_clause() {
    let corrector = create_test_corrector();
    let result = corrector.correct(
        "China ya está en una situación complicada, pero tirar líneas de Ultra Alta Tensión es carísimo",
    );
    assert!(
        !result.contains("carísimo [carísimas]")
            && !result.contains("carisimo [carisimas]")
            && !result.contains("carísimo [carisimo]"),
        "No debe forzar concordancia con objeto interno de infinitivo: {}",
        result
    );
}

#[test]
fn test_integration_participle_after_de_noun_phrase_keeps_head_noun_agreement() {
    let corrector = create_test_corrector();
    let result = corrector.correct("una subestación de tamaño medio conformada por");
    assert!(
        !result.contains("conformada [conformado]"),
        "No debe forzar 'conformado' en 'subestación ... conformada': {}",
        result
    );
}

#[test]
fn test_integration_coordinated_noun_with_adverb_before_participle_not_singularized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ese contexto y ese proceso arriba descritos no son");
    assert!(
        !result.contains("descritos [descrito]"),
        "No debe singularizar participio coordinado con adverbio intermedio: {}",
        result
    );
}

#[test]
fn test_integration_industrializar_is_not_flagged() {
    let corrector = create_test_corrector();
    let result = corrector.correct("la capacidad para industrializarse");
    assert!(
        !result.contains("industrializarse |"),
        "No debe marcar 'industrializarse' como error ortográfico: {}",
        result
    );
}

#[test]
fn test_integration_name_plus_lleva_not_treated_as_vocative() {
    let corrector = create_test_corrector();
    let cases = ["China lleva décadas", "María lleva tiempo"];
    for text in cases {
        let result = corrector.correct(text);
        assert!(
            !result.contains("China [China,]") && !result.contains("María [María,]"),
            "No debe insertar coma vocativa en sujeto + 'lleva' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_ahora_mismo_not_forced_to_gender_agree() {
    let corrector = create_test_corrector();
    let result = corrector.correct("la Tierra ahora mismo está a salvo");
    assert!(
        !result.contains("mismo [misma]"),
        "No debe forzar 'mismo' en la locución adverbial 'ahora mismo': {}",
        result
    );
}

#[test]
fn test_integration_millon_de_plural_allows_plural_verb() {
    let corrector = create_test_corrector();
    let result = corrector.correct("un millón de ojos pueden ver");
    assert!(
        !result.contains("pueden [puede]"),
        "No debe forzar singular en 'un millón de + plural': {}",
        result
    );
}

#[test]
fn test_integration_relative_with_foreign_tokens_not_singularized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los premios Ig Nobel esos que reconocen logros curiosos");
    assert!(
        !result.contains("reconocen [reconoce]"),
        "No debe singularizar relativo con antecedente plural y tokens extranjeros: {}",
        result
    );
}

#[test]
fn test_integration_completive_que_not_treated_as_relative() {
    let corrector = create_test_corrector();
    let result = corrector.correct("se les dice a cualquier otra persona que son inteligentes");
    assert!(
        !result.contains("son [es]"),
        "No debe tratar 'que son' completiva como relativo de antecedente singular: {}",
        result
    );

    let result = corrector
        .correct("se les dice a los narcisistas o a cualquier otra persona que son inteligentes");
    assert!(
        !result.contains("son [es]"),
        "No debe tratar como relativo el caso coordinado con 'o ... persona que son': {}",
        result
    );
}

#[test]
fn test_integration_intercalar_forms_not_flagged() {
    let corrector = create_test_corrector();
    for text in ["segundos intercalares", "segundo intercalar"] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("intercalares |") && !result.contains("intercalar |"),
            "No debe marcar 'intercalar/intercalares' como error ortográfico en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_de_de_diacritic_contexts() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Ojalá se dé una cosecha");
    assert!(
        !result.contains("dé [de]"),
        "No debe quitar tilde en 'Ojalá se dé': {}",
        result
    );

    let result = corrector.correct("ya que de haber estado");
    assert!(
        !result.contains("de [dé]"),
        "No debe añadir tilde en 'de haber' tras 'ya que': {}",
        result
    );
}

#[test]
fn test_integration_haber_estado_adjective_agrees_with_subject() {
    let corrector = create_test_corrector();

    let result = corrector.correct("La casa había estado vacía");
    assert!(
        !result.contains("vacía [vacío]"),
        "No debe forzar masculino tras 'había estado': {}",
        result
    );

    let result = corrector.correct("haber estado activa");
    assert!(
        !result.contains("activa [activo]"),
        "No debe forzar masculino en 'haber estado + adjetivo': {}",
        result
    );
}

#[test]
fn test_integration_relative_puesto_que_causal_not_singularized() {
    let corrector = create_test_corrector();
    let result =
        corrector.correct("puesto que apuntan a que la humedad convierte la casa en una incubadora");
    assert!(
        !result.contains("apuntan [apunta]"),
        "No debe tratar 'puesto que' como relativo con antecedente singular: {}",
        result
    );
}

#[test]
fn test_integration_un_poco_fixed_adverb_not_forced_to_gender() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Hay que ventilar un poco la vivienda");
    assert!(
        !result.contains("poco [poca]"),
        "No debe concordar 'un poco' con el sustantivo siguiente: {}",
        result
    );

    let result = corrector.correct("Debemos esperar un poco la respuesta");
    assert!(
        !result.contains("poco [poca]"),
        "No debe corregir 'un poco' cuando funciona como cuantificador adverbial: {}",
        result
    );

    let result = corrector.correct("Me alegro de que hayas venido aunque sea un poco tarde");
    assert!(
        !result.contains("poco [poca]"),
        "No debe forzar concordancia en 'un poco tarde' (uso adverbial): {}",
        result
    );
}

#[test]
fn test_integration_pp_coordination_not_treated_as_main_subject() {
    let corrector = create_test_corrector();
    let result = corrector
        .correct("En Mexico y otros paises de America Latina ese beneficio no fue implementado");
    assert!(
        !result.contains("implementado [implementados]"),
        "No debe forzar plural por coordinacion interna en PP inicial: {}",
        result
    );
}

#[test]
fn test_integration_long_pp_chain_keeps_head_noun_for_agreement() {
    let corrector = create_test_corrector();
    let result = corrector.correct("un promedio de horas a la semana mas elevado");
    assert!(
        !result.contains("elevado [elevada]"),
        "No debe concordar con 'semana' dentro de cadena preposicional: {}",
        result
    );
}

#[test]
fn test_integration_comma_new_clause_postposed_subject_not_forced_singular() {
    let corrector = create_test_corrector();
    let result = corrector.correct("se acumula el cansancio, aumentan los indices de error");
    assert!(
        !result.contains("aumentan [aumenta]"),
        "No debe forzar singular cuando hay nueva clausula con sujeto pospuesto plural: {}",
        result
    );
}

#[test]
fn test_integration_agrietar_not_flagged_as_spelling() {
    let corrector = create_test_corrector();
    let result = corrector.correct("puede agrietar las mucosas");
    assert!(
        !result.contains("agrietar |"),
        "No debe marcar 'agrietar' como error ortografico: {}",
        result
    );
}

#[test]
fn test_integration_invariable_adjective_not_inflected_antitabaco() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Las medidas antitabaco");
    assert!(
        !result.contains("antitabaco [antitabacas]"),
        "No debe flexionar adjetivo invariable 'antitabaco': {}",
        result
    );
}

#[test]
fn test_integration_como_phrase_not_treated_as_nominal_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Como un bloque resultan mucho mas faciles de separar");
    assert!(
        !result.contains("resultan [resulta]"),
        "No debe tomar 'como un bloque' como sujeto nominal: {}",
        result
    );
}

#[test]
fn test_integration_numeric_veces_not_used_as_subject_for_predicative_adjective() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Es 200 veces mas resistente que el acero");
    assert!(
        !result.contains("resistente [resistentes]"),
        "No debe pluralizar adjetivo por expresion adverbial '200 veces': {}",
        result
    );
}

#[test]
fn test_integration_de_complement_noun_not_treated_as_verb_demanda() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Los mercados de alta demanda");
    assert!(
        !result.contains("demanda [demandan]"),
        "No debe tratar 'demanda' en 'de alta demanda' como verbo principal: {}",
        result
    );
}

#[test]
fn test_integration_predicate_noun_oro_not_changed_to_adjective() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Esta molecula es oro puro");
    assert!(
        !result.contains("oro [ora]"),
        "No debe tratar 'oro' (nombre predicativo) como adjetivo flexionable: {}",
        result
    );
}

#[test]
fn test_integration_duration_complement_not_singularized() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Encender el interruptor 20 segundos");
    assert!(
        !result.contains("segundos [segundo]"),
        "No debe singularizar complemento de duracion con numero: {}",
        result
    );
}

#[test]
fn test_integration_relative_keeps_head_noun_over_pp_internal_plural() {
    let corrector = create_test_corrector();
    let result = corrector.correct(
        "Un famoso estudio con mas de 100.000 participantes, que ya demostro la relacion, fue publicado.",
    );
    let lower = result.to_lowercase();
    assert!(
        !lower.contains("demostro [demostraron]") && !lower.contains("demostró [demostraron]"),
        "No debe pluralizar el verbo relativo por un nombre dentro de PP: {}",
        result
    );
}

#[test]
fn test_integration_relative_predicative_keeps_clause_subject() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Hubo anos en los que la superficie ya era habitable.");
    assert!(
        !result.contains("habitable [habitables]"),
        "No debe pluralizar adjetivo predicativo de la subordinada por antecedente plural: {}",
        result
    );
}

#[test]
fn test_integration_implicit_nosotros_predicatives_not_forced_to_external_noun() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La etapa entre que estamos despiertos y dormidos es breve.");
    assert!(
        !result.contains("despiertos [despierta]") && !result.contains("dormidos [dormida]"),
        "No debe forzar genero singular externo sobre predicativos con sujeto implicito: {}",
        result
    );
}

#[test]
fn test_integration_coordinated_infinitive_subject_keeps_singular_copula() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Ser un buho nocturno y despertarse tarde tampoco es bueno.");
    assert!(
        !result.contains("es [son]"),
        "No debe pluralizar verbo copulativo con sujeto de infinitivos coordinados: {}",
        result
    );
}

#[test]
fn test_integration_clause_subject_keeps_singular_sigue_siendo() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Que paso con los mayas sigue siendo un tema complejo.");
    assert!(
        !result.contains("sigue [siguen]"),
        "No debe pluralizar 'sigue' cuando el sujeto es una clausula completa: {}",
        result
    );
}

#[test]
fn test_integration_relative_plural_antecedent_not_forced_singular() {
    let corrector = create_test_corrector();
    let result = corrector.correct(
        "A diferencia de otras regiones, las poblaciones mayas que si sufrieron sequias devastadoras migraron.",
    );
    let lower = result.to_lowercase();
    assert!(
        !lower.contains("sufrieron [sufrio]") && !lower.contains("sufrieron [sufrió]"),
        "No debe singularizar verbo relativo con antecedente plural correcto: {}",
        result
    );
}

#[test]
fn test_integration_diacritics_pronoun_si_verb_conditional_not_accented() {
    let corrector = create_test_corrector();
    for text in [
        "ustedes si viven en Madrid, avisen",
        "vosotros si podeis venir, genial",
        "ellos si quieren, participan",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("si [sí]"),
            "No debe acentuar 'si' condicional en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_quien_sentence_start_not_pluralized_from_previous_sentence() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Pidieron disculpas. Quien este libre de pecado, que tire.");
    assert!(
        !result.contains("Quien [Quienes]"),
        "No debe pluralizar 'Quien' por arrastre de la oración anterior: {}",
        result
    );
}

#[test]
fn test_integration_vocative_name_plus_ve_que_not_forced() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Rufian ve que la soberania esta amenazada");
    assert!(
        !result.contains("Rufian [Rufian,]"),
        "No debe insertar coma vocativa en sujeto + 've que': {}",
        result
    );
}

#[test]
fn test_integration_el_me_gusta_nominalized_fragment_no_el_to_el() {
    let corrector = create_test_corrector();
    let result = corrector.correct("el me gusta la fruta de Ayuso");
    assert!(
        !result.contains("el [Él]") && !result.contains("el [él]"),
        "No debe convertir artículo nominalizador en pronombre tónico: {}",
        result
    );
}

#[test]
fn test_integration_el_mitad_tonta_epithet_fragment_not_forced_to_la() {
    let corrector = create_test_corrector();
    let result = corrector.correct("el mitad tonta de Belmonte");
    assert!(
        !result.contains("el [La]") && !result.contains("el [la]"),
        "No debe forzar artículo en fragmento nominalizado con epíteto: {}",
        result
    );
}

#[test]
fn test_integration_new_dictionary_entries_not_flagged() {
    let corrector = create_test_corrector();
    for text in [
        "No podemos truncar la explicación",
        "La idea empezó a cristalizar",
        "El discurso busca aglutinar apoyos",
        "Van a apilar cajas en el almacén",
        "Esa experiencia puede vivificar su obra",
        "Estoy loquísima hoy",
        "Debemos remangarnos para terminar",
        "El Estado va a subvencionar viviendas",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar como ortográfico tras añadir entradas en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round8_diacritics_and_nominalization_regressions() {
    let corrector = create_test_corrector();

    let result_cuando = corrector.correct("Dice que cuando una empresa empieza");
    assert!(
        !result_cuando.to_lowercase().contains("cuando [cu"),
        "No debe acentuar 'cuando' temporal tras completiva: {}",
        result_cuando
    );

    let result_el_inf = corrector.correct("El disponer de al menos dos naves facilita la misión");
    assert!(
        !result_el_inf.contains("El [Él]") && !result_el_inf.contains("el [él]"),
        "No debe convertir artículo nominalizador en pronombre: {}",
        result_el_inf
    );
}

#[test]
fn test_integration_round8_existential_haber_quantifier_no_false_positive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No habría demasiados problemas");
    assert!(
        !result.contains("demasiados [demasiado]"),
        "No debe reinterpretar cuantificador existencial como participio: {}",
        result
    );
}

#[test]
fn test_integration_round8_units_and_comparative_times_no_false_positive() {
    let corrector = create_test_corrector();

    let result_unit = corrector.correct("100 V de tensión");
    assert!(
        !result_unit.contains("V [Ves]"),
        "No debe corregir símbolo de unidad 'V': {}",
        result_unit
    );

    let result_times = corrector.correct("10 veces mayor");
    assert!(
        !result_times.contains("mayor [mayores]"),
        "No debe pluralizar comparativo en 'N veces mayor': {}",
        result_times
    );
}

#[test]
fn test_integration_round8_partitives_and_ambiguous_gender_no_false_positive() {
    let corrector = create_test_corrector();

    let result_partitive = corrector.correct("Un puñado de empresas se pueden coordinar");
    assert!(
        !result_partitive.contains("pueden [puede]"),
        "No debe forzar singular con 'puñado de + plural': {}",
        result_partitive
    );

    let result_uno = corrector.correct("La razón es uno de los principales motivos");
    assert!(
        !result_uno.contains("uno [una]"),
        "No debe forzar género de 'uno de los ...' por el sujeto: {}",
        result_uno
    );

    let result_lentes = corrector.correct("lentes homologadas");
    assert!(
        !result_lentes.contains("homologadas [homologados]"),
        "No debe forzar género en sustantivo ambiguo 'lentes': {}",
        result_lentes
    );
}

#[test]
fn test_integration_round8_pp_internal_and_clause_coordination_no_false_positive() {
    let corrector = create_test_corrector();

    let result_coordinated = corrector.correct("La rama cruje y el pájaro se inclina");
    assert!(
        !result_coordinated.contains("inclina [inclinan]"),
        "No debe interpretar dos cláusulas coordinadas como sujeto compuesto: {}",
        result_coordinated
    );

    let result_pp_head = corrector.correct(
        "Necesitamos estrategias de control que incorporen medidas eficaces",
    );
    assert!(
        !result_pp_head.contains("incorporen [incorpore]"),
        "No debe concordar verbo con sustantivo interno de PP: {}",
        result_pp_head
    );

    let result_pp_participle =
        corrector.correct("Lanzaron acciones de sensibilización dirigidas a jóvenes");
    assert!(
        !result_pp_participle.contains("dirigidas [dirigida]"),
        "No debe forzar participio por sustantivo interno de PP: {}",
        result_pp_participle
    );

    let result_relative =
        corrector.correct("La central, cuyo cierre estaba planeado, seguirá operando");
    assert!(
        !result_relative.contains("planeado [planeadas]"),
        "No debe tomar antecedente incorrecto en predicado participial: {}",
        result_relative
    );

    let result_postposed =
        corrector.correct("En mecánica cuántica, el estado cuántico del sistema es par");
    assert!(
        !result_postposed.contains("par [pares]"),
        "No debe arrastrar sujeto pospuesto de otra estructura: {}",
        result_postposed
    );

    let result_proper_name = corrector.correct("Santander y Donostia son ciudades costeras");
    assert!(
        !result_proper_name.contains("Donostia [Donostios]"),
        "No debe flexionar topónimos como sustantivos comunes: {}",
        result_proper_name
    );
}

#[test]
fn test_integration_round8_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    for text in [
        "Estamos desequilibrando el sistema",
        "La lluvia humedece el suelo",
        "Piden que vuelquen la carga",
        "Por favor, analicémoslo ahora",
        "Eso cuadruplicará el coste",
        "Capital sobreinvertido en ese sector",
        "Publicaron un contrainforme técnico",
        "Ese descuadre contable preocupa",
        "Por ende, seguimos adelante",
        "Tarifa electrointensiva para industria",
        "Arquitecturas hiperescalares modernas",
        "La hiperescala exige automatización",
        "Empresas hiperescaladoras globales",
        "Santander y Donostia colaboran",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round9_diacritics_and_homophone_no_false_positive() {
    let corrector = create_test_corrector();

    let result_como = corrector.correct("La salud se entiende como la valoración personal");
    assert!(
        !result_como.contains("como [cómo]"),
        "No debe acentuar 'como' comparativo/preposicional: {}",
        result_como
    );

    let result_sino = corrector.correct("El problema no es X, sino cómo lo hacemos");
    assert!(
        !result_sino.contains("sino [si no]"),
        "No debe separar 'sino' en contraste adversativo: {}",
        result_sino
    );

    let result_cuando =
        corrector.correct("Es decir, cuando el Sol cruza el ecuador celeste empieza la primavera");
    assert!(
        !result_cuando.to_lowercase().contains("cuando [cu"),
        "No debe acentuar 'cuando' temporal tras 'es decir': {}",
        result_cuando
    );

    let result_mismo_nominal = corrector.correct("Se utilice el mismo pasaporte en ambos controles");
    assert!(
        !result_mismo_nominal.contains("el [él]") && !result_mismo_nominal.contains("El [Él]"),
        "No debe acentuar artículo en 'el mismo + sustantivo': {}",
        result_mismo_nominal
    );

    let result_mismo_prep = corrector.correct("En el mismo a partir de los años 70 se observan cambios");
    assert!(
        !result_mismo_prep.contains("el [él]") && !result_mismo_prep.contains("El [Él]"),
        "No debe acentuar artículo en 'en el mismo ...': {}",
        result_mismo_prep
    );
}

#[test]
fn test_integration_round9_agreement_and_vocative_no_false_positive() {
    let corrector = create_test_corrector();

    let result_vocative = corrector.correct("Eva Madrid investigadora del instituto explicó los resultados");
    assert!(
        !result_vocative.contains("Eva [Eva,]"),
        "No debe insertar coma vocativa en nombre compuesto: {}",
        result_vocative
    );

    let result_pp = corrector.correct("Detectaron tectónica de placas estable en exoplanetas");
    assert!(
        !result_pp.contains("estable [estables]"),
        "No debe forzar concordancia por sustantivo interno de PP: {}",
        result_pp
    );

    let result_caza = corrector.correct("El escuadrón usó un caza bimotor durante la misión");
    assert!(
        !result_caza.contains("un [una]"),
        "No debe forzar género único en 'caza' polisémico: {}",
        result_caza
    );

    let result_titles = corrector.correct("La bioeticista y profesora afirma que hay riesgos");
    assert!(
        !result_titles.contains("afirma [afirman]"),
        "No debe forzar plural en coordinación nominal potencialmente aposicional: {}",
        result_titles
    );

    let result_drone = corrector.correct("Presentaron un drone comercial para reparto urbano");
    assert!(
        !result_drone.contains("comercial [comerciales]"),
        "No debe pluralizar adjetivo por cabeza nominal desconocida: {}",
        result_drone
    );

    let result_decimal = corrector.correct("La mejora fue 2,37 veces superior a la prevista");
    assert!(
        !result_decimal.contains("superior [superiores]"),
        "No debe pluralizar comparativo en 'N,DD veces + comparativo': {}",
        result_decimal
    );
}

#[test]
fn test_integration_round9_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    for text in [
        "El criterio difiere entre regiones",
        "La escuela inculca valores cívicos",
        "Presentaron un drone comercial",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round9_remaining_fps_no_false_positive() {
    let corrector = create_test_corrector();

    let result_estimulan = corrector.correct(
        "Compuestos como la teobromina y la cafeína estimulan el sistema nervioso",
    );
    assert!(
        !result_estimulan.contains("estimulan [estimula]"),
        "No debe forzar singular por ejemplo interno con 'como': {}",
        result_estimulan
    );

    let result_de =
        corrector.correct("Hacen que un pájaro le dé mil vueltas a cualquier drone comercial");
    assert!(
        !result_de.contains("dé [de]"),
        "No debe quitar tilde a subjuntivo 'dé': {}",
        result_de
    );

    let result_siguen =
        corrector.correct("La firma final y la configuración definitiva siguen pendientes");
    assert!(
        !result_siguen.contains("siguen [sigue]"),
        "No debe romper concordancia plural en sujeto coordinado: {}",
        result_siguen
    );

    let result_combinan = corrector.correct(
        "Competiciones de resistencia como Hyrox, que combinan carrera y ejercicios",
    );
    assert!(
        !result_combinan.contains("combinan [combina]"),
        "No debe tomar apposición con 'como' como antecedente singular del relativo: {}",
        result_combinan
    );
}

#[test]
fn test_integration_round10_reported_false_positives() {
    let corrector = create_test_corrector();

    let result_estas = corrector.correct("Estas pequeñas muestras se conviertan en evidencia");
    assert!(
        !result_estas.to_lowercase().contains("estas [estás]"),
        "No debe acentuar demostrativo 'estas' ante adjetivo+sustantivo: {}",
        result_estas
    );
    let result_esta = corrector.correct("Esta pequeña muestra confirma la hipótesis");
    assert!(
        !result_esta.to_lowercase().contains("esta [está]"),
        "No debe acentuar demostrativo 'esta' ante adjetivo+sustantivo: {}",
        result_esta
    );

    let result_el_num = corrector.correct("Se situó en el 14,6% de la población");
    assert!(
        !result_el_num.contains("el [él]") && !result_el_num.contains("El [Él]"),
        "No debe acentuar artículo antes de porcentaje/número: {}",
        result_el_num
    );

    let result_copula =
        corrector.correct("El que acapara las miradas es Gabriel Rufián y el acto siguió");
    assert!(
        !result_copula.contains("es [son]"),
        "No debe pluralizar 'ser' por predicado nominal coordinado posverbal: {}",
        result_copula
    );

    let result_pp_internal =
        corrector.correct("Los centros de estudio tienen más sencillo adaptar el plan");
    assert!(
        !result_pp_internal.contains("tienen [tiene]"),
        "No debe forzar singular por sustantivo interno de PP: {}",
        result_pp_internal
    );

    let result_coord = corrector.correct(
        "La fortaleza, la moderación o el despliegue son algunas de las claves",
    );
    assert!(
        !result_coord.contains("son [es]"),
        "No debe forzar singular en sujeto coordinado enumerativo: {}",
        result_coord
    );
    assert!(
        !result_coord.contains("algunas [algunos]"),
        "No debe romper concordancia de 'algunas' en predicado nominal: {}",
        result_coord
    );

    let result_sobre_todo = corrector.correct("Sobre todo gracias al consumo interno");
    assert!(
        !result_sobre_todo.to_lowercase().contains("todo [tod"),
        "No debe flexionar 'todo' en locución fija 'sobre todo': {}",
        result_sobre_todo
    );

    let result_mediados = corrector.correct("A mediados del año comenzaron los cambios");
    assert!(
        !result_mediados.contains("A [Ha]") && !result_mediados.contains("a [ha]"),
        "No debe corregir preposición en locución 'a mediados de': {}",
        result_mediados
    );
    assert!(
        !result_mediados.to_lowercase().contains("mediados [mediado]"),
        "No debe lematizar 'mediados' como participio en locución fija: {}",
        result_mediados
    );

    let result_relative =
        corrector.correct("La revisión de los datos que arrojaron los sensores fue exhaustiva");
    assert!(
        !result_relative.contains("arrojaron [arrojó]"),
        "No debe perder antecedente plural en relativo con 'de + los + N': {}",
        result_relative
    );
    let result_relative_chain = corrector.correct(
        "La revisión de los datos del estudio que arrojaron los sensores fue exhaustiva",
    );
    assert!(
        !result_relative_chain.contains("arrojaron [arrojó]"),
        "No debe forzar singular por sustantivo singular interno en cadena con 'de': {}",
        result_relative_chain
    );
    let result_relative_temporal_tail = corrector.correct(
        "La revisión de los datos del mercado de trabajo de septiembre, que arrojaron, fue exhaustiva",
    );
    assert!(
        !result_relative_temporal_tail.contains("arrojaron [arrojó]"),
        "No debe tomar el complemento temporal final como antecedente del relativo: {}",
        result_relative_temporal_tail
    );

    let result_porque = corrector.correct("Por otro, porque potenciales desarrollos fallaron");
    assert!(
        !result_porque.to_lowercase().contains("porque [porqué]"),
        "No debe nominalizar 'porque' causal tras inciso: {}",
        result_porque
    );

    let result_ministry = corrector
        .correct("El Ministerio de Inclusión, Seguridad Social y Migraciones atendió la demanda");
    assert!(
        !result_ministry.contains("atendió [atendieron]"),
        "No debe pluralizar verbo por coordinación interna de nombre institucional: {}",
        result_ministry
    );

    let result_quien = corrector.correct("El profesor de Ciencias, quien falleció, dejó legado");
    assert!(
        !result_quien.contains("quien [quienes]"),
        "No debe pluralizar 'quien' por antecedente interno de complemento con 'de': {}",
        result_quien
    );
    assert!(
        !result_quien.contains("falleció [fallecieron]"),
        "No debe pluralizar verbo de relativo por antecedente interno de complemento con 'de': {}",
        result_quien
    );
    let result_quien_compound =
        corrector.correct("El profesor de Ciencias Médicas, quien falleció, dejó legado");
    assert!(
        !result_quien_compound.contains("quien [quienes]"),
        "No debe pluralizar 'quien' en títulos de área compuestos: {}",
        result_quien_compound
    );
    assert!(
        !result_quien_compound.contains("falleció [fallecieron]"),
        "No debe pluralizar 'falleció' en relativos con antecedente singular externo: {}",
        result_quien_compound
    );
    let result_quien_weizmann = corrector
        .correct("El profesor del Instituto Weizmann de Ciencias, quien falleció, dejó legado");
    assert!(
        !result_quien_weizmann.contains("quien [quienes]"),
        "No debe usar el sustantivo interno plural como antecedente de 'quien': {}",
        result_quien_weizmann
    );
    assert!(
        !result_quien_weizmann.contains("falleció [fallecieron]"),
        "No debe pluralizar el verbo del relativo por 'de Ciencias' interno: {}",
        result_quien_weizmann
    );
}

#[test]
fn test_integration_round10_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    for text in [
        "Si desacelere hoy, recuperaré ritmo mañana",
        "Es habitual desacelerarse al final del ciclo",
        "La presión tensiona la estructura",
        "Pidieron que siguieran con cautela",
        "El paciente agoniza en silencio",
        "Hay plantillas precarizadas en varios sectores",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round11_reported_false_positives() {
    let corrector = create_test_corrector();

    let result_mismo = corrector.correct("Observó el mismo eclipse");
    assert!(
        !result_mismo.contains("el [él]") && !result_mismo.contains("El [Él]"),
        "No debe acentuar artículo en 'el mismo + sustantivo': {}",
        result_mismo
    );

    let result_no_va_mas = corrector.correct("Hoy es el no va más");
    assert!(
        !result_no_va_mas.contains("el [él]") && !result_no_va_mas.contains("El [Él]"),
        "No debe acentuar artículo en la locución 'el no va más': {}",
        result_no_va_mas
    );

    let result_nested = corrector.correct("La IA que tú le cuentas es menos útil");
    assert!(
        !result_nested.contains("es [eres]"),
        "No debe tomar 'tú' de la subordinada como sujeto del verbo principal: {}",
        result_nested
    );
    assert!(
        !result_nested.contains("útil [útiles]"),
        "No debe pluralizar atributo por sujeto mal resuelto en subordinada: {}",
        result_nested
    );

    let result_clave =
        corrector.correct("La comunicación y la planificación son clave para reducir riesgos");
    assert!(
        !result_clave.contains("clave [claves]"),
        "No debe forzar flexión de adjetivo invariable 'clave': {}",
        result_clave
    );

    let result_contienen = corrector.correct("Las células tumorales que contienen mutaciones avanzan");
    assert!(
        !result_contienen.contains("contienen [contene]"),
        "No debe degradar forma verbal válida en relativo con antecedente plural: {}",
        result_contienen
    );

    let result_vocative_parenthetical =
        corrector.correct("El sindicato de Madrid (AMYTS) convocó huelga");
    assert!(
        !result_vocative_parenthetical.contains("Madrid [Madrid,]"),
        "No debe insertar coma vocativa antes de paréntesis en topónimo institucional: {}",
        result_vocative_parenthetical
    );

    let result_que_porque = corrector.correct(
        "La pregunta que se hacen es por qué, sobre todo porque los servicios fallan",
    );
    assert!(
        !result_que_porque.to_lowercase().contains("que [qué]"),
        "No debe acentuar relativo 'que' tras núcleo nominal ('la pregunta que...'): {}",
        result_que_porque
    );
    assert!(
        !result_que_porque.to_lowercase().contains("porque [porqué]"),
        "No debe nominalizar 'porque' en la locución causal 'sobre todo porque': {}",
        result_que_porque
    );

    let result_si = corrector.correct("No se da título, pero sí un diploma");
    assert!(
        !result_si.contains("sí [si]"),
        "No debe quitar tilde al 'sí' enfático tras contraste con 'pero': {}",
        result_si
    );

    let result_concejal_1 = corrector.correct("Lo apunta la concejal Irene");
    assert!(
        !result_concejal_1.contains("la [el]"),
        "No debe masculinizar 'la concejal' cuando hay referente femenino: {}",
        result_concejal_1
    );
    let result_concejal_2 = corrector.correct("La concejal Irene habló después");
    assert!(
        !result_concejal_2.contains("La [El]"),
        "No debe corregir artículo femenino en sustantivo de género común 'concejal': {}",
        result_concejal_2
    );

    let result_covid = corrector.correct("Frente a la covid-19 se reforzaron medidas");
    assert!(
        !result_covid.contains("la [el]") && !result_covid.contains("La [El]"),
        "No debe forzar masculino en 'la covid-19': {}",
        result_covid
    );

    let result_seo = corrector.correct("SEO/BirdLife presentó el informe");
    assert!(
        !result_seo.contains("|"),
        "No debe marcar como ortografía una marca/acrónimo en formato con barra: {}",
        result_seo
    );

    let result_vuelquen = corrector.correct("La producción que vuelquen a la red dependerá del clima");
    assert!(
        !result_vuelquen.contains("vuelquen [vuelque]"),
        "No debe forzar singular en relativo de objeto con sujeto implícito plural: {}",
        result_vuelquen
    );

    let result_llevan = corrector.correct("La práctica que llevan a cabo compañías está extendida");
    assert!(
        !result_llevan.contains("llevan [lleva]"),
        "No debe singularizar relativo con sujeto pospuesto plural en 'llevar a cabo': {}",
        result_llevan
    );
    assert!(
        !result_llevan.contains("extendida [extendidas]"),
        "No debe propagar plural espurio al atributo principal: {}",
        result_llevan
    );
}

#[test]
fn test_integration_round11_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    for text in [
        "Los residuos obstruyen el conducto principal",
        "Esto puede repercutir en el coste total",
        "Esperamos que oxigenen mejor el sistema",
        "El forzamiento antropógeno sigue aumentando",
        "Estos bucles se retroalimentan entre sí",
        "El plan busca dinamizar la economía local",
        "Siguen vertebrando la red territorial",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round12_reported_false_positives() {
    let corrector = create_test_corrector();

    let result_tuvo = corrector.correct(
        "Aunque los planes nunca son cosa de uno, este tuvo un rostro",
    );
    assert!(
        !result_tuvo.contains("tuvo [tubo]"),
        "No debe cambiar 'tuvo' por 'tubo' cuando introduce objeto directo: {}",
        result_tuvo
    );

    let result_measure =
        corrector.correct("Es tres mil años más antiguo y cinco mil años anterior");
    assert!(
        !result_measure.contains("antiguo [antiguos]"),
        "No debe pluralizar adjetivo por sustantivo de medida temporal: {}",
        result_measure
    );
    assert!(
        !result_measure.contains("anterior [anteriores]"),
        "No debe pluralizar adjetivo en segunda medida temporal coordinada: {}",
        result_measure
    );

    let result_estas = corrector.correct("Estas narrativas son dañinas");
    assert!(
        !result_estas.contains("Estas [Estás]"),
        "No debe acentuar demostrativo 'Estas' ante sustantivo: {}",
        result_estas
    );

    let result_peste = corrector.correct("Fue responsable de la peste");
    assert!(
        !result_peste.contains("la [el]"),
        "No debe masculinizar 'la peste': {}",
        result_peste
    );

    let result_acunado = corrector.correct("El término sífilis fue acuñado en 1530");
    assert!(
        !result_acunado.contains("acuñado [acuñada]"),
        "No debe concordar participio con aposición interna en lugar del núcleo: {}",
        result_acunado
    );

    let result_las = corrector.correct("A la hora de tomar las muestras");
    assert!(
        !result_las.contains("las [les]"),
        "No debe tratar artículo 'las' como pronombre en infinitivo + SN: {}",
        result_las
    );

    let result_resto = corrector.correct("El resto son gestionados por entidades privadas");
    assert!(
        !result_resto.contains("son [es]"),
        "Debe permitir concordancia plural ad sensum con 'el resto': {}",
        result_resto
    );
    assert!(
        !result_resto.contains("gestionados [gestionado]"),
        "No debe forzar singular en predicativo plural con 'el resto': {}",
        result_resto
    );

    let result_ejecutivo = corrector.correct("El Ejecutivo del PP y Vox mejoró su resultado");
    assert!(
        !result_ejecutivo.contains("mejoró [mejoraron]"),
        "No debe coordinar sujeto con 'y' dentro de complemento preposicional: {}",
        result_ejecutivo
    );

    let result_infinitive = corrector.correct("Subcontratar es bueno");
    assert!(
        !result_infinitive.contains("|"),
        "No debe marcar ortografía en infinitivo válido 'subcontratar': {}",
        result_infinitive
    );
    assert!(
        !result_infinitive.contains("bueno [buena]"),
        "No debe feminizar atributo con sujeto infinitivo: {}",
        result_infinitive
    );

    let result_mismo = corrector.correct("No es el mismo según seamos");
    assert!(
        !result_mismo.contains("el [él]"),
        "No debe acentuar artículo en lectura elíptica de 'el mismo': {}",
        result_mismo
    );

    let result_objeto = corrector.correct("Estos temas son continuamente objeto de disputa");
    assert!(
        !result_objeto.contains("objeto [objetos]"),
        "No debe pluralizar locución lexicalizada 'objeto de': {}",
        result_objeto
    );

    let result_decade = corrector.correct("Los años ochenta marcaron una época");
    assert!(
        !result_decade.contains("ochenta [ochentos]"),
        "No debe flexionar cardinal de década tras 'años': {}",
        result_decade
    );

    let result_quatro = corrector.correct("las quatro reglas");
    assert!(
        !result_quatro.contains("las [El]") && !result_quatro.contains("las [el]"),
        "No debe alterar el artículo en título con grafía histórica: {}",
        result_quatro
    );
}

#[test]
fn test_integration_round12_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    for text in [
        "El laboratorio va a secuenciar nuevas muestras",
        "Conviene acotar el alcance del estudio",
        "Decidieron subcontratar el servicio",
        "Buscan prestigiar la iniciativa pública",
        "Los sobrecostes superaron lo previsto",
        "El pian es una enfermedad tropical",
        "La frambesia afecta a comunidades vulnerables",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round13_reported_false_positives() {
    let corrector = create_test_corrector();

    let result_measure_tail =
        corrector.correct("es tres mil años más antiguo...y cinco mil años anterior");
    assert!(
        !result_measure_tail.contains("anterior [anteriores]"),
        "No debe pluralizar adjetivo comparativo en segundo segmento de medida coordinada: {}",
        result_measure_tail
    );

    let result_cercanias = corrector.correct("Se optó por los cercanías y los trenes");
    assert!(
        !result_cercanias.contains("los [las]"),
        "No debe forzar artículo femenino en uso lexicalizado 'los cercanías': {}",
        result_cercanias
    );

    let result_resfriados = corrector.correct(
        "La idea de que el frío causa resfriados y gripes está arraigada en la cultura popular",
    );
    assert!(
        !result_resfriados.contains("resfriados [resfriada]"),
        "No debe tratar 'resfriados' como adjetivo cuando funciona como sustantivo: {}",
        result_resfriados
    );
    assert!(
        !result_resfriados.contains("arraigada [arraigadas]"),
        "No debe arrastrar concordancia predicativa al interior de subordinada con 'de que': {}",
        result_resfriados
    );

    let result_lisozima = corrector.correct("la actividad de la lisozima");
    assert!(
        !result_lisozima.contains("la [el]"),
        "No debe masculinizar 'lisozima': {}",
        result_lisozima
    );

    let result_copulative = corrector.correct("El perfil mayoritario son mujeres marroquíes");
    assert!(
        !result_copulative.contains("son [es]"),
        "Debe aceptar concordancia plural con atributo nominal postverbal en copulativa: {}",
        result_copulative
    );
}

#[test]
fn test_integration_round13_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    for text in [
        "Las muestras se pueden inactivar antes del análisis",
        "Conviene abrigar bien a la población vulnerable",
        "Tendrán que desclasificar más documentos",
        "Es mejor abrigarse si baja la temperatura",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round14_invariable_forms_reported_false_positives() {
    let corrector = create_test_corrector();

    let result_megalopolis = corrector.correct("Las megalópolis surcadas por autopistas");
    assert!(
        !result_megalopolis.contains("Las [La]"),
        "No debe singularizar artículo con sustantivo invariable: {}",
        result_megalopolis
    );
    assert!(
        !result_megalopolis.contains("surcadas [surcada]"),
        "No debe singularizar adjetivo con sustantivo invariable: {}",
        result_megalopolis
    );

    for text in [
        "Las prótesis nuevas",
        "Las metrópolis antiguas",
        "Las necrópolis famosas",
        "Las diócesis vecinas",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("Las [La]"),
            "No debe singularizar artículo con invariable en -s para '{}': {}",
            text,
            result
        );
    }

    let result_paralisis = corrector.correct("La parálisis fue súbita");
    assert!(
        !result_paralisis.contains("La [El]"),
        "No debe masculinizar 'parálisis': {}",
        result_paralisis
    );
    assert!(
        !result_paralisis.contains("súbita [súbito]"),
        "No debe propagar género incorrecto en predicativo de 'parálisis': {}",
        result_paralisis
    );

    for text in [
        "El concierto es gratis",
        "La casa es gratis",
        "Los libros son gratis",
        "Las entradas son gratis",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("gratis ["),
            "No debe flexionar adjetivo invariable 'gratis' en '{}': {}",
            text,
            result
        );
    }

    let result_dos = corrector.correct("El resultado fueron dos heridos");
    assert!(
        !result_dos.contains("dos [do]"),
        "No debe singularizar cardinal 'dos': {}",
        result_dos
    );

    let result_tres = corrector.correct("El resultado fueron tres muertos");
    assert!(
        !result_tres.contains("tres [tre]"),
        "No debe singularizar cardinal 'tres': {}",
        result_tres
    );

    let result_seis = corrector.correct("El resultado fueron seis muertos");
    assert!(
        !result_seis.contains("seis [seiso]"),
        "No debe singularizar cardinal 'seis': {}",
        result_seis
    );
}

#[test]
fn test_integration_round14_spelling_recognizes_specialized_terms() {
    let corrector = create_test_corrector();

    for text in [
        "La dimetiltriptamina aparece en trazas",
        "La psilocibina se estudia en ensayos clínicos",
        "Los pacientes polisensibilizados requieren seguimiento",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round15_reported_false_positives() {
    let corrector = create_test_corrector();

    for text in [
        "Las dosis son altas",
        "Las dosis fueron administradas",
        "Las prótesis están rotas",
        "Las dosis parecen altas",
        "Las hipótesis resultan falsas",
        "Las dosis que fueron administradas",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("[alta]")
                && !result.contains("[administrada]")
                && !result.contains("[rota]")
                && !result.contains("[falsa]")
                && !result.contains("fueron [fue]"),
            "No debe forzar singular en contextos con sustantivos invariables: '{}': {}",
            text,
            result
        );
    }

    for text in [
        "El tubo que compré está roto",
        "Un tubo que conecta ambas piezas",
        "Cada tubo que fabricamos",
        "Ese tubo que trajiste",
        "Aquel tubo que usamos",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("tubo [tuvo]"),
            "No debe corregir 'tubo que' cuando hay determinante nominal previo: '{}': {}",
            text,
            result
        );
    }

    for text in [
        "La echo de menos",
        "La vaya a buscar",
        "La hierva un poco más",
        "No la vaya a romper",
        "Que la vaya a buscar",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("echo [hecho]")
                && !result.contains("vaya [valla]")
                && !result.contains("hierva [hierba]"),
            "No debe tratar 'la' como artículo en contexto clítico verbal: '{}': {}",
            text,
            result
        );
    }

    for text in [
        "Lave la ropa primero",
        "Hierva el agua primero",
        "Corte la cebolla primero",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("primero [primera]"),
            "No debe forzar concordancia cuando 'primero' es adverbial: '{}': {}",
            text,
            result
        );
    }

    let result_postposed_subject = corrector.correct("Pidió ayuda María");
    assert!(
        !result_postposed_subject.contains("ayuda [ayuda,]"),
        "No debe insertar coma vocativa en sujeto pospuesto: {}",
        result_postposed_subject
    );
}

#[test]
fn test_integration_round15_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    for text in [
        "Hay que vacunar a la población de riesgo",
        "El sistema permite climatizar el edificio",
        "Conviene inmunizar al ganado",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("|"),
            "No debe marcar ortografía en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round16_reported_false_positives_and_negatives() {
    let corrector = create_test_corrector();

    let result_mas = corrector.correct("Es difícil, mas no imposible");
    assert!(
        !result_mas.contains("mas [más]"),
        "No debe acentuar 'mas' adversativo tras coma: {}",
        result_mas
    );

    let result_muestras = corrector.correct("Las muestras revelaron el problema");
    assert!(
        !result_muestras.contains("Las [Les]"),
        "No debe tratar 'Las' como pronombre en 'Las muestras ...': {}",
        result_muestras
    );

    let result_muta = corrector.correct("El virus muta más rápido");
    assert!(
        !result_muta.contains("rápido [rápida]"),
        "No debe concordar con lectura nominal espuria de 'muta': {}",
        result_muta
    );

    let result_que_si = corrector.correct("Dijo que si, que vendría");
    assert!(
        result_que_si.contains("si [sí]"),
        "Debe acentuar 'si' afirmativo en 'que si, ...': {}",
        result_que_si
    );

    for text in ["En si", "Por si solo", "De si depende", "Entre si acordaron"] {
        let result = corrector.correct(text);
        assert!(
            result.contains("si [sí]"),
            "Debe acentuar 'si' reflexivo en '{}': {}",
            text,
            result
        );
    }

    let result_por_si_acaso = corrector.correct("Por si acaso");
    assert!(
        !result_por_si_acaso.contains("si [sí]"),
        "No debe acentuar 'si' condicional en 'Por si acaso': {}",
        result_por_si_acaso
    );
}

#[test]
fn test_integration_round16_spelling_recognizes_reported_forms() {
    let corrector = create_test_corrector();

    let result = corrector.correct("La situación se está agudizando");
    assert!(
        !result.contains("|"),
        "No debe marcar ortografía en 'agudizando': {}",
        result
    );
    assert!(
        !result.contains("agudizando [agudizanda]"),
        "No debe forzar concordancia espuria sobre gerundio verbal: {}",
        result
    );
}

#[test]
fn test_integration_round17_mas_si_coherence_regressions() {
    let corrector = create_test_corrector();

    // "mas" adversativo válido: no forzar "más".
    for text in [
        "Es difícil, mas no imposible",
        "No quiso ir, mas tuvo que hacerlo",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("mas [más]"),
            "No debe forzar tilde en 'mas' adversativo en '{}': {}",
            text,
            result
        );
    }

    // "más" cuantitativo/comparativo tras coma: no desacentuar.
    for text in [
        "Cuanto más investigan, más preguntas surgen",
        "Estudió mucho, más que nadie",
        "El precio subió, más de lo esperado",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("más [mas]"),
            "No debe quitar tilde de 'más' en '{}': {}",
            text,
            result
        );
    }

    // "mas" cuantitativo sin tilde debe corregirse.
    let result_quant = corrector.correct("Necesitamos agua, mas comida y mas tiempo");
    assert!(
        result_quant.contains("mas [más]"),
        "Debe corregir 'mas' cuantitativo sin tilde: {}",
        result_quant
    );

    // "en sí" reflexivo/enfático.
    for text in [
        "Volvió en si después del golpe",
        "El problema en si no es grave",
        "La idea en si resulta interesante",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("si [sí]"),
            "Debe acentuar 'en sí' en '{}': {}",
            text,
            result
        );
    }

    // "en si" como interrogativa indirecta: no acentuar.
    for text in ["Pensó en si debía ir", "No reparó en si estaba listo"] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("si [sí]"),
            "No debe acentuar 'si' conjunción en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round18_high_and_medium_regressions() {
    let corrector = create_test_corrector();

    // 1) No debe cruzar verbos de oración previa al resolver sujeto infinitivo.
    let result_inf = corrector.correct("Juan trabaja bien. Volver a estudiar las lecciones es aburrido");
    assert!(
        !result_inf.contains("aburrido [aburridas]"),
        "No debe forzar concordancia con oración previa: {}",
        result_inf
    );

    // 2) Hacer impersonal: aceptar forma sin tilde en la entrada.
    let result_hacian = corrector.correct("Ayer hacian muchos dias que no llovia");
    assert!(
        result_hacian.contains("[hacía]"),
        "Debe corregir 'hacian' impersonal a 'hacía': {}",
        result_hacian
    );

    // 3) Haber impersonal con sustantivo no debe bloquearse por pseudo-participio.
    let result_piso = corrector.correct("Ayer habían piso libre");
    assert!(
        result_piso.contains("habían [había]"),
        "Debe corregir 'habían piso' en uso existencial: {}",
        result_piso
    );

    // 4) Diacríticos no deben usar contexto de la oración siguiente.
    let result_el_boundary = corrector.correct("Es para el. Juan vino ayer.");
    assert!(
        result_el_boundary.contains("el [él]"),
        "Debe acentuar 'él' al final de oración preposicional: {}",
        result_el_boundary
    );

    // 5) No inferir género por apellido en -o.
    let result_pacheco = corrector.correct("Ayer la periodista Pacheco informó");
    assert!(
        !result_pacheco.contains("la [el]") && !result_pacheco.contains("la [El]"),
        "No debe cambiar artículo por heurística de apellido: {}",
        result_pacheco
    );

    // 6) "a echo" no siempre equivale a auxiliar + participio.
    let result_a_echo = corrector.correct("Voy a echo la basura");
    assert!(
        !result_a_echo.contains("echo [hecho]"),
        "No debe corregir 'echo' a 'hecho' en contexto de preposición: {}",
        result_a_echo
    );

    // 7) Sí enfático tras "eso".
    let result_eso_si = corrector.correct("Eso si para algo sirve");
    assert!(
        result_eso_si.contains("si [sí]"),
        "Debe permitir corrección de 'sí' enfático con 'para': {}",
        result_eso_si
    );

    // 8) En condicional irreal, no bloquear por sufijo -to en verbo finito.
    let result_suelto = corrector.correct("Si suelto la cuerda tendria problemas");
    assert!(
        !result_suelto.contains("tendria [tuviera]"),
        "No debe forzar irrealis cuando la prótasis ya es finita: {}",
        result_suelto
    );

    // 9) Vocativo: evitar falsos positivos en cláusulas indicativas.
    let result_vocative = corrector.correct("Pedro salta la valla");
    assert!(
        !result_vocative.contains("Pedro [Pedro,]"),
        "No debe insertar coma vocativa en enunciado descriptivo: {}",
        result_vocative
    );

    // 10) Vocativo: evitar heurística -ad/-ed/-id sobre nombres propios.
    let result_madrid = corrector.correct("Madrid Juan");
    assert!(
        !result_madrid.contains("Madrid [Madrid,]"),
        "No debe tratar 'Madrid' como imperativo de vosotros: {}",
        result_madrid
    );

    // 11) Haber + habido no debe cruzar límites de oración.
    let result_habido_boundary = corrector.correct("Habían. Habido muchas quejas");
    assert!(
        !result_habido_boundary.contains("Habían [Había]"),
        "No debe buscar 'habido' en otra oración: {}",
        result_habido_boundary
    );
}

#[test]
fn test_integration_mojibake_inverted_question_mark_still_accents_interrogative() {
    let corrector = create_test_corrector();
    let result = corrector.correct("\u{00C3}\u{0082}\u{00C2}\u{00BF}y que quieres?");
    assert!(
        result.contains("que ["),
        "Debe mantener corrección interrogativa aunque haya mojibake en apertura: {}",
        result
    );
}

#[test]
fn test_integration_round19_all_caps_plural_pleonasm_and_email() {
    let corrector = create_test_corrector();

    // 1) En frase ALL-CAPS no se debe saltar ortografia para palabras cortas.
    let result_all_caps = corrector.correct("DEVE COMER MAS FRUTA");
    assert!(
        result_all_caps.contains("DEVE |"),
        "Debe analizar 'DEVE' en ALL-CAPS en lugar de saltarlo: {}",
        result_all_caps
    );

    // 2) No aceptar plurales inexistentes derivados de preteritos mal etiquetados.
    let result_velos = corrector.correct("Es muy vel\u{00F3}s");
    assert!(
        result_velos.contains("vel\u{00F3}s |"),
        "No debe aceptar 'velos' acentuado como plural valido: {}",
        result_velos
    );

    // 3) Pleonasmo con formas adicionales de "salir".
    let result_sal = corrector.correct("Sal afuera a jugar");
    assert!(
        result_sal.contains("~~afuera~~"),
        "Debe marcar pleonasmo en 'Sal afuera': {}",
        result_sal
    );
    let result_salgan = corrector.correct("Salgan afuera todos");
    assert!(
        result_salgan.contains("~~afuera~~"),
        "Debe marcar pleonasmo en 'Salgan afuera': {}",
        result_salgan
    );

    // 4) Los emails deben tratarse como token atomico y no corregirse internamente.
    let result_email = corrector.correct("Escribe a juan@gmail.com");
    assert!(
        !result_email.contains('|') && !result_email.contains('['),
        "No debe proponer cambios dentro de un email: {}",
        result_email
    );
}

#[test]
fn test_integration_round20_diacritics_homophone_and_auxiliary_edge_cases() {
    let corrector = create_test_corrector();

    // 1) No quitar tilde correcta en "tu" cuando el verbo siguiente esta mal escrito o es desconocido.
    for text in [
        "T\u{00FA} dijistes que vendr\u{00ED}as",
        "T\u{00FA} comistes mucho",
        "T\u{00FA} hablastes con Mar\u{00ED}a",
        "T\u{00FA} xyzabc que vendr\u{00ED}as",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("[Tu]") && !result.contains("[tu]"),
            "No debe deacentuar 'tu' en '{}': {}",
            text,
            result
        );
    }

    // 2) "si no" condicional no debe fusionarse a "sino" con verbos homografos sustantivo/verbo.
    for text in [
        "No quiero ir si no me acompa\u{00F1}as",
        "No quiero ir si no me cuentas",
        "No quiero ir si no me plantas",
        "No quiero ir si no me guardas",
        "No quiero ir si no me regalas",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("si [sino]") && !result.contains("~~no~~"),
            "No debe fusionar 'si no' condicional en '{}': {}",
            text,
            result
        );
    }

    // 3) Tras corregir "haiga" -> "haya", no forzar concordancia adjetival del participio.
    for text in ["Haiga llovido mucho", "Haiga nevado mucho", "Haiga helado mucho"] {
        let result = corrector.correct(text);
        assert!(
            result.contains("Haiga [Haya]"),
            "Debe corregir 'haiga' -> 'haya' en '{}': {}",
            text,
            result
        );
        assert!(
            !result.contains("[llovida]")
                && !result.contains("[nevada]")
                && !result.contains("[helada]"),
            "No debe feminizar participio tras auxiliar en '{}': {}",
            text,
            result
        );
    }

    // 4) "pregunte que si/como/donde" no debe convertirse en "que..." interrogativo.
    for text in [
        "Le pregunt\u{00E9} que si quer\u{00ED}a venir",
        "Le pregunt\u{00E9} que c\u{00F3}mo estaba",
        "Le pregunt\u{00E9} que d\u{00F3}nde viv\u{00ED}a",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("que [qu\u{00E9}]"),
            "No debe acentuar 'que' en discurso indirecto con interrogativo explicito en '{}': {}",
            text,
            result
        );
    }

    // 5) En "el por que de ..." no debe cambiar "de" -> "de".
    for text in [
        "El por que de su decisi\u{00F3}n",
        "Busco el por que de todo esto",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("de [d\u{00E9}]"),
            "No debe acentuar 'de' tras 'por que' nominal en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_vulgar_preterite_second_person_extra_s_is_corrected() {
    let corrector = create_test_corrector();

    let samples = [
        ("cantastes muy bien", "cantaste"),
        ("dijistes que vendrías", "dijiste"),
        ("Tú comistes mucho", "comiste"),
    ];

    for (text, expected_form) in samples {
        let result = corrector.correct(text);
        assert!(
            result.contains(&format!(" [{}]", expected_form))
                || result.contains(&format!(" [{}]", capitalize(expected_form))),
            "Debe corregir vulgarismo -stes en '{}': {}",
            text,
            result
        );
        assert!(
            !result.contains('|'),
            "No debe duplicar salida ortográfica cuando ya hay corrección gramatical en '{}': {}",
            text,
            result
        );
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[test]
fn test_integration_leismo_llamar_la_atencion_not_forced() {
    let corrector = create_test_corrector();
    let result_present = corrector.correct("Les llama la atención");
    assert!(
        !result_present.contains("Les [Los]") && !result_present.contains("les [los]"),
        "No debe marcar leísmo en 'Les llama la atención': {}",
        result_present
    );

    let result_preterite = corrector.correct("Les llamó la atención el ruido");
    assert!(
        !result_preterite.contains("Les [Los]") && !result_preterite.contains("les [los]"),
        "No debe marcar leísmo en 'Les llamó la atención ...': {}",
        result_preterite
    );

    let result_direct = corrector.correct("Les llamé ayer");
    assert!(
        result_direct.contains("Les [Los]") || result_direct.contains("les [los]"),
        "Debe seguir detectando leísmo en uso transitivo directo: {}",
        result_direct
    );
}

#[test]
fn test_integration_round21_irregular_dequeismo_and_interaction_regressions() {
    let corrector = create_test_corrector();

    // 1) No aceptar pretéritos/participios regulares para verbos con formas irregulares.
    let result_tenio = corrector.correct("Había tenió problemas");
    assert!(
        result_tenio.contains("tenió |"),
        "Debe marcar forma irregular mal construida en 'tenió': {}",
        result_tenio
    );
    let result_escribido = corrector.correct("He escribido una carta");
    assert!(
        result_escribido.contains("escribido |") || result_escribido.contains("[escrito]"),
        "Debe marcar o corregir forma irregular mal construida en 'escribido': {}",
        result_escribido
    );
    let result_hacieron = corrector.correct("Han hacieron cambios");
    assert!(
        result_hacieron.contains("hacieron |") || result_hacieron.contains("hacieron [hecho]"),
        "Debe marcar o corregir forma irregular mal construida en 'hacieron': {}",
        result_hacieron
    );

    // 2) Dequeísmo: imperfecto/futuro/condicional.
    for text in [
        "Pensaba de que sí",
        "Creía de que sí",
        "Pensaré de que sí",
        "Pensaría de que sí",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("~~de~~"),
            "Debe eliminar 'de' en dequeísmo en '{}': {}",
            text,
            result
        );
    }

    // 3) Queísmo reflexivo: imperfecto/futuro/condicional.
    for text in [
        "Me alegraba que estuvieras bien",
        "Me acordaba que lo dijo",
        "Me alegraré que vengas",
        "Me alegraría que vinieras",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("que [de que]"),
            "Debe insertar 'de' en queísmo reflexivo en '{}': {}",
            text,
            result
        );
    }

    // 4) Formas sin tilde conflictivas en diccionario deben marcarse como ortografía.
    let result_queria = corrector.correct("El queria ir");
    assert!(
        result_queria.contains("queria |"),
        "Debe marcar 'queria' sin tilde: {}",
        result_queria
    );

    // 5) Relativa ambigua con antecedente humano y verbo transivo ("llamar"): no forzar singular.
    for text in [
        "La persona que llamaron",
        "El hombre que llamaron",
        "El profesor que llamaron",
    ] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("[llamó]"),
            "No debe forzar 'llamó' en relativa de objeto ambigua '{}': {}",
            text,
            result
        );
    }

    // 6) Orden de fases: detectar "boy" aunque aparezca "ha" (que se corrige a "a").
    let result_boy_ha = corrector.correct("Boy ha comprar");
    assert!(
        result_boy_ha.contains("Boy [Voy]") && result_boy_ha.contains("ha [a]"),
        "Debe corregir tanto 'Boy' como 'ha' en 'Boy ha comprar': {}",
        result_boy_ha
    );

    // 7) Interacción si no -> sino no debe forzar concordancia de 2ª persona.
    let result_si_no = corrector.correct("Nadie si no tú puede hacerlo");
    assert!(
        result_si_no.contains("si [sino]") && result_si_no.contains("~~no~~")
            && !result_si_no.contains("puede [puedes]"),
        "No debe forzar 'puedes' tras fusionar 'si no' en '{}': {}",
        "Nadie si no tú puede hacerlo",
        result_si_no
    );
}

#[test]
fn test_integration_round22_el_participles_and_enclitics_regressions() {
    let corrector = create_test_corrector();

    // 1) Dequeísmo con raíces irregulares de futuro/condicional.
    for text in [
        "Diría de que es mejor",
        "Dirá de que vendrá",
        "Supondrá de que sí",
        "Supondría de que sí",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("~~de~~"),
            "Debe eliminar 'de' en dequeísmo irregular '{}': {}",
            text,
            result
        );
    }

    // 2) el/él en pretérito y subordinadas.
    for text in [
        "El comió mucho",
        "El habló bien",
        "El dijo que sí",
        "Sé que el viene mañana",
        "Dudo que el pueda venir",
        "Espero que el venga mañana",
        "No creo que el sepa la respuesta",
        "Quiero que el tenga tiempo",
        "Sé que el contesta rápido",
        "Creo que el acepta la propuesta",
        "Dicen que el rechaza la oferta",
        "A el le gusta el café",
        "Según el esto es correcto",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("El [Él]")
                || result.contains("el [él]")
                || result.contains("el [Él]"),
            "Debe acentuar pronombre 'él' en '{}': {}",
            text,
            result
        );
    }

    // 3) Evitar cascada "El tuvo" -> "tubo" en contexto verbal.
    let result_tuvo = corrector.correct("El tuvo razón");
    assert!(
        !result_tuvo.contains("tuvo [tubo]"),
        "No debe corregir 'tuvo' a 'tubo' en contexto verbal: {}",
        result_tuvo
    );

    // 4) Enclíticos imperativos sin tilde deben marcarse.
    for text in ["digame", "llamame", "cuentame", "dimelo", "sirveme", "preparamelo"] {
        let result = corrector.correct(text);
        assert!(
            result.contains(" |") || result.contains('['),
            "Debe marcar o corregir enclítico imperativo sin tilde '{}': {}",
            text,
            result
        );
    }

    // 5) Participios irregulares prefijados regularizados no deben aceptarse.
    for text in [
        "satisfacido",
        "deshacido",
        "prevido",
        "descubrido",
        "encubrido",
        "componido",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains(" |"),
            "Debe marcar participio irregular prefijado mal formado '{}': {}",
            text,
            result
        );
    }

    // 6) En tiempos compuestos, sugerir participio irregular correcto.
    for (text, expected) in [
        ("He abrido", "[abierto]"),
        ("He rompido", "[roto]"),
        ("He escribido", "[escrito]"),
        ("He ponido", "[puesto]"),
        ("He morido", "[muerto]"),
        ("He cubrido", "[cubierto]"),
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains(expected),
            "Debe sugerir participio irregular en '{}': {}",
            text,
            result
        );
    }

    // 7) Entradas espurias de diccionario no deben enmascarar errores.
    let result_dias = corrector.correct("Compré varios dias de vacaciones");
    assert!(
        result_dias.contains("dias [días]") || result_dias.contains("dias |"),
        "Debe marcar/corregir 'dias' sin tilde: {}",
        result_dias
    );

    let result_hiva = corrector.correct("Mi hermano no hiva al colegio");
    assert!(
        result_hiva.contains("hiva |"),
        "Debe marcar 'hiva' como error ortográfico: {}",
        result_hiva
    );
}

#[test]
fn test_integration_round46_dequeismo_asegurar_indirect_object_and_queismo_convencer() {
    let corrector = create_test_corrector();

    let result_aseguro = corrector.correct("Aseguró de que vendría");
    assert!(
        result_aseguro.contains("~~de~~"),
        "Debe eliminar 'de' en dequeísmo con 'asegurar': {}",
        result_aseguro
    );

    let result_se_aseguro = corrector.correct("Se aseguró de que vendría");
    assert!(
        !result_se_aseguro.contains("~~de~~"),
        "No debe romper la construcción pronominal 'asegurarse de que': {}",
        result_se_aseguro
    );

    let result_dijo_np = corrector.correct("El niño le dijo a la profesora de que había suspendido");
    assert!(
        result_dijo_np.contains("~~de~~"),
        "Debe detectar dequeísmo en 'dijo a X de que ...': {}",
        result_dijo_np
    );

    let result_convencio = corrector.correct("Me convenció que era la mejor opción");
    assert!(
        result_convencio.contains("que [de que]"),
        "Debe detectar queísmo en 'convenció que ...': {}",
        result_convencio
    );
}

#[test]
fn test_integration_round23_clause_diacritics_and_homophone_regressions() {
    let corrector = create_test_corrector();

    // 1) "la + homógrafo verbal" al inicio no debe forzarse a artículo.
    for text in ["La cuento un secreto", "La regalo un libro"] {
        let result = corrector.correct(text);
        assert!(
            !result.contains("La [El]") && !result.contains("la [el]"),
            "No debe reinterpretar clítico+verbo como artículo+sustantivo en '{}': {}",
            text,
            result
        );
    }

    // 2) "no se si/que" mantiene lectura de "saber" también con sujeto explícito.
    for text in ["Carlos no se si irá", "Ella no se si podrá"] {
        let result = corrector.correct(text);
        assert!(
            result.contains("se [sé]") || result.contains("Se [Sé]"),
            "Debe sugerir 'sé' en patrón 'no se si' en '{}': {}",
            text,
            result
        );
    }

    // 3) "he echo" con locuciones de echar -> "echado".
    for text in ["He echo de menos a mi familia", "He echo una siesta"] {
        let result = corrector.correct(text);
        assert!(
            result.contains("echo [echado]") || result.contains("Echo [Echado]"),
            "Debe sugerir participio de 'echar' en '{}': {}",
            text,
            result
        );
    }

    // 4) Conjunciones/subordinantes + "el + verbo" deben acentuar pronombre.
    for text in [
        "No fui, pero el sabe",
        "No sé si el quiere",
        "Aunque el quiera, no irá",
        "Cuando el llega, empezamos",
        "Donde el vive hace frío",
        "Porque el trabaja, no vino",
        "Mientras el duerme, leo",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("el [él]") || result.contains("El [Él]"),
            "Debe acentuar pronombre en '{}': {}",
            text,
            result
        );
    }

    // 5) "mas" adversativo sin coma.
    let result_mas = corrector.correct("Lo intentó mas no pudo");
    assert!(
        !result_mas.contains("mas [más]"),
        "No debe forzar acento en 'mas' adversativo sin coma: {}",
        result_mas
    );

    // 6) "que de + artículo" en contexto verbal debe permitir "dé".
    for text in ["Quiero que de la noticia", "No creo que de la talla"] {
        let result = corrector.correct(text);
        assert!(
            result.contains("de [dé]") || result.contains("De [Dé]"),
            "Debe corregir subjuntivo de dar en '{}': {}",
            text,
            result
        );
    }

    // 7) "Aún" inclusivo con con/sin/gerundio debe desacentuarse.
    for text in ["Aún con frío salió", "Aún sin ayuda avanzó", "Aún siendo mayor entrenó"] {
        let result = corrector.correct(text);
        assert!(
            result.contains("Aún [Aun]") || result.contains("aún [aun]"),
            "Debe quitar tilde en uso inclusivo de 'aun' en '{}': {}",
            text,
            result
        );
    }

    // 8) Contextos modales/subjuntivos de "halla" -> "haya".
    for text in [
        "Puede que halla problemas",
        "Para que halla paz",
        "Sin que halla pruebas, no condenes",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("halla [haya]"),
            "Debe corregir 'halla' -> 'haya' en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round24_mi_mi_and_tuvo_without_que() {
    let corrector = create_test_corrector();

    let result_comparative = corrector.correct("Es más fuerte que mi");
    assert!(
        result_comparative.contains("mi [mí]"),
        "Debe acentuar 'mí' en comparativa con 'que': {}",
        result_comparative
    );

    let result_entre = corrector.correct("Entre tu y mi no hay secretos");
    assert!(
        result_entre.contains("tu [tú]") && result_entre.contains("mi [mí]"),
        "Debe acentuar ambos pronombres tónicos en 'entre tú y mí': {}",
        result_entre
    );

    let result_possessive = corrector.correct("Es más fuerte que mi hermano");
    assert!(
        !result_possessive.contains("mi [mí]"),
        "No debe acentuar posesivo en 'que mi + sustantivo': {}",
        result_possessive
    );

    for text in [
        "Juan tubo un accidente",
        "Nunca tubo miedo",
        "Siempre tubo suerte",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("tubo [tuvo]"),
            "Debe corregir 'tubo' verbal en '{}': {}",
            text,
            result
        );
    }

    let result_nominal = corrector.correct("El tubo de cobre está roto");
    assert!(
        !result_nominal.contains("tubo [tuvo]"),
        "No debe corregir uso nominal claro de 'tubo': {}",
        result_nominal
    );
}

#[test]
fn test_integration_round25_pending_pronoun_and_homophone_cases() {
    let corrector = create_test_corrector();

    let result_no_se_si = corrector.correct("Carlos no se si irá");
    assert!(
        result_no_se_si.contains("se [sé]") || result_no_se_si.contains("Se [Sé]"),
        "Debe corregir 'se' -> 'sé' en patrón 'no se si': {}",
        result_no_se_si
    );

    for text in [
        "La dimos un regalo",
        "La daría un consejo",
        "La daremos una oportunidad",
        "La prometí un viaje",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("La [Le]") || result.contains("la [le]"),
            "Debe detectar laísmo ditransitivo en '{}': {}",
            text,
            result
        );
    }

    let result_leismo = corrector.correct("Les oí cantar");
    assert!(
        result_leismo.contains("Les [Los]")
            || result_leismo.contains("Les [Las]")
            || result_leismo.contains("les [los]")
            || result_leismo.contains("les [las]"),
        "Debe detectar leísmo plural en 'Les oí cantar': {}",
        result_leismo
    );

    for text in [
        "Les escuché hablar",
        "Les miré de lejos",
        "Les esperé en casa",
        "Les recogí en la estación",
        "Les llevé al cine",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("Les [Los]")
                || result.contains("Les [Las]")
                || result.contains("les [los]")
                || result.contains("les [las]"),
            "Debe detectar leísmo plural en '{}': {}",
            text,
            result
        );
    }

    let result_leve_ditransitivo = corrector.correct("Les llevé un regalo");
    assert!(
        !result_leve_ditransitivo.contains("Les [Los]")
            && !result_leve_ditransitivo.contains("Les [Las]")
            && !result_leve_ditransitivo.contains("les [los]")
            && !result_leve_ditransitivo.contains("les [las]"),
        "No debe marcar leísmo en uso ditransitivo con tema explícito: {}",
        result_leve_ditransitivo
    );

    let result_loismo = corrector.correct("Lo mandaron un mensaje");
    assert!(
        result_loismo.contains("Lo [Le]") || result_loismo.contains("lo [le]"),
        "Debe detectar loísmo ditransitivo con 'mandar': {}",
        result_loismo
    );

    for text in [
        "Lo enviaron un paquete",
        "Lo compraron un regalo",
        "Lo ofrecieron un puesto",
    ] {
        let result = corrector.correct(text);
        assert!(
            result.contains("Lo [Le]") || result.contains("lo [le]"),
            "Debe detectar loísmo ditransitivo en '{}': {}",
            text,
            result
        );
    }

    let result_quizas = corrector.correct("quizas mañana llueva");
    assert!(
        result_quizas.contains("quizás") || result_quizas.contains("Quizás"),
        "Debe marcar/corregir 'quizas' sin tilde: {}",
        result_quizas
    );

    let result_vello = corrector.correct("Es un vello lugar");
    assert!(
        result_vello.contains("vello [bello]") || result_vello.contains("Vello [Bello]"),
        "Debe corregir 'vello' adjetival por 'bello': {}",
        result_vello
    );

    let result_ahi_esta = corrector.correct("ahi esta el problema");
    assert!(
        result_ahi_esta.contains("ahi [Ahí]"),
        "Debe corregir 'ahi' -> 'ahí': {}",
        result_ahi_esta
    );
    assert!(
        result_ahi_esta.contains("esta [está]") || result_ahi_esta.contains("Esta [Está]"),
        "Debe corregir 'esta' -> 'está' en contexto locativo: {}",
        result_ahi_esta
    );
}

#[test]
fn test_integration_round26_remaining_pending_cases() {
    let corrector = create_test_corrector();

    let result_por_el_bien = corrector.correct("por el bien de todos");
    assert!(
        !result_por_el_bien.contains("el [él]") && !result_por_el_bien.contains("El [Él]"),
        "No debe acentuar articulo en locucion 'por el bien': {}",
        result_por_el_bien
    );

    let result_tubo_nominal = corrector.correct("El tubo se rompió");
    assert!(
        !result_tubo_nominal.contains("tubo [tuvo]") && !result_tubo_nominal.contains("Tubo [Tuvo]"),
        "No debe forzar lectura verbal de 'tubo' con determinante nominal: {}",
        result_tubo_nominal
    );

    let result_segun_el_la = corrector.correct("segun el la culpa es mia");
    assert!(
        result_segun_el_la.contains("el [él]") || result_segun_el_la.contains("El [Él]"),
        "Debe acentuar pronombre en 'segun el la ...': {}",
        result_segun_el_la
    );

    let result_de_por_si = corrector.correct("de por si es dificil");
    assert!(
        result_de_por_si.contains("si [sí]") || result_de_por_si.contains("Si [Sí]"),
        "Debe acentuar la locucion fija 'de por sí': {}",
        result_de_por_si
    );

    let result_governador = corrector.correct("El governador dio un discurso");
    assert!(
        result_governador.contains("governador |") && result_governador.contains("gobernador"),
        "Debe marcar 'governador' y sugerir 'gobernador': {}",
        result_governador
    );

    let result_fue_el = corrector.correct("el que llegó primero fue el");
    assert!(
        result_fue_el.contains("fue el [él]") || result_fue_el.contains("fue El [Él]"),
        "Debe acentuar pronombre final tras cópula en 'fue él': {}",
        result_fue_el
    );

    let result_tu_tambien = corrector.correct("tu tambien lo sabes");
    assert!(
        result_tu_tambien.contains("tu [tú]")
            || result_tu_tambien.contains("tu [Tú]")
            || result_tu_tambien.contains("Tu [Tú]"),
        "Debe acentuar 'tú' en 'tu tambien ...': {}",
        result_tu_tambien
    );
    assert!(
        result_tu_tambien.contains("tambien [también]")
            || result_tu_tambien.contains("tambien [También]")
            || result_tu_tambien.contains("tambien |también"),
        "Debe corregir también el adverbio 'también': {}",
        result_tu_tambien
    );
}

#[test]
fn test_integration_multiline_context_does_not_cross_lines() {
    let corrector = create_test_corrector();
    let input = "el que llegó primero fue el\nsegun el la culpa es mia\npor el bien de todos\nEl tubo se rompió";
    let result = corrector.correct(input);

    assert!(
        result.contains("fue el [él]") || result.contains("fue El [Él]"),
        "Debe acentuar pronombre final en la primera línea: {}",
        result
    );
    assert!(
        result.contains("el [él]") || result.contains("El [Él]"),
        "Debe acentuar 'segun el la ...' en segunda línea: {}",
        result
    );
    assert!(
        !result.contains("todos [todo]"),
        "No debe contaminar concordancia entre líneas ('por el bien de todos'): {}",
        result
    );
    assert!(
        !result.contains("tubo [tuvo]"),
        "No debe forzar lectura verbal nominal en línea independiente: {}",
        result
    );
}

#[test]
fn test_integration_round27_safe_improvements() {
    let corrector = create_test_corrector();

    let result_maria = corrector.correct("Maria esta enferma");
    assert!(
        result_maria.contains("esta [está]") || result_maria.contains("Esta [Está]"),
        "Debe corregir 'esta' -> 'está' tras sujeto nominal: {}",
        result_maria
    );

    let result_cada_uno = corrector.correct("Cada uno de los estudiantes aprobaron");
    assert!(
        result_cada_uno.contains("aprobaron [aprobó]") || result_cada_uno.contains("aprobaron [Aprobó]"),
        "Debe forzar singular con sujeto distributivo 'cada uno': {}",
        result_cada_uno
    );

    let result_havia = corrector.correct("se havia ido");
    assert!(
        result_havia.contains("havia [había]") || result_havia.contains("Havia [Había]"),
        "Debe corregir 'havia' -> forma de 'haber' con b/tilde: {}",
        result_havia
    );

    let result_tonica_plural = corrector.correct("los agua estan sucias");
    assert!(
        result_tonica_plural.contains("los [Las]")
            || result_tonica_plural.contains("los [las]")
            || result_tonica_plural.contains("Los [Las]"),
        "Debe evitar singularizar articulo en plural de femenino con a tónica: {}",
        result_tonica_plural
    );
    assert!(
        result_tonica_plural.contains("agua [aguas]")
            || result_tonica_plural.contains("Agua [Aguas]"),
        "Debe proponer plural del núcleo en 'los agua': {}",
        result_tonica_plural
    );
    assert!(
        !result_tonica_plural.contains("estan [está]")
            && !result_tonica_plural.contains("sucias [sucia]"),
        "No debe arrastrar singularización espuria en verbo/adjetivo: {}",
        result_tonica_plural
    );

    let result_desicion = corrector.correct("desicion");
    assert!(
        result_desicion.to_lowercase().contains("decisión"),
        "Debe incluir 'decisión' entre sugerencias de 'desicion': {}",
        result_desicion
    );
    let suggestions_segment = result_desicion
        .split('|')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    assert!(
        suggestions_segment == "decisión",
        "Para 'desicion', la primera sugerencia debe ser 'decisión': {}",
        result_desicion
    );
}

#[test]
fn test_integration_directional_nominal_adjective_agreement_for_mano() {
    let corrector = create_test_corrector();

    let result_wrong = corrector.correct("La mano derecho");
    assert!(
        result_wrong.contains("derecho [derecha]") || result_wrong.contains("Derecho [Derecha]"),
        "Debe corregir concordancia en 'La mano derecho': {}",
        result_wrong
    );

    let result_ok = corrector.correct("La mano derecha");
    assert!(
        !result_ok.contains("derecha ["),
        "No debe corregir forma ya correcta en 'La mano derecha': {}",
        result_ok
    );
}

#[test]
fn test_integration_tu_que_opinas_markless_interrogative() {
    let corrector = create_test_corrector();

    for text in ["tu que opinas", "tu que piensas", "tu que crees", "tu que dices"] {
        let result = corrector.correct(text);
        assert!(
            result.contains("que [qué]") || result.contains("Que [Qué]"),
            "Debe acentuar 'que' interrogativo en '{}': {}",
            text,
            result
        );
    }

    let declarative = corrector.correct("Creo que tu que opinas siempre te equivocas");
    assert!(
        !declarative.contains("que [qué]"),
        "No debe forzar 'qué' fuera de inicio de cláusula en contexto declarativo: {}",
        declarative
    );
}

#[test]
fn test_integration_attributive_coordinated_adjective_agreement() {
    let corrector = create_test_corrector();

    let singular = corrector.correct("Una casa grande y bonito");
    assert!(
        singular.contains("bonito [bonita]"),
        "Debe corregir segundo adjetivo coordinado en singular: {}",
        singular
    );

    let plural = corrector.correct("Los niños buenos y educada");
    assert!(
        plural.contains("educada [educados]"),
        "Debe corregir segundo adjetivo coordinado en plural: {}",
        plural
    );

    let distributive = corrector.correct("Los sectores público y privado");
    assert!(
        !distributive.contains("público [")
            && !distributive.contains("privado [")
            && !distributive.contains("publico [")
            && !distributive.contains("privado ["),
        "No debe tocar distributivos ('sectores público y privado'): {}",
        distributive
    );
}

#[test]
fn test_integration_round28_spelling_accent_ranking_priority() {
    let corrector = create_test_corrector();

    let result_arbol = corrector.correct("El arbol es muy alto");
    let first_arbol = result_arbol
        .split('|')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    assert_eq!(
        first_arbol,
        "\u{00E1}rbol",
        "La primera sugerencia de 'arbol' debe ser 'árbol': {}",
        result_arbol
    );

    let result_travez = corrector.correct("travez");
    let first_travez = result_travez
        .split('|')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    assert_eq!(
        first_travez,
        "trav\u{00E9}s",
        "La primera sugerencia de 'travez' debe ser 'través': {}",
        result_travez
    );
}

#[test]
fn test_integration_round29_additional_safe_homophone_and_diacritic_cases() {
    let corrector = create_test_corrector();

    let result_vella = corrector.correct("Una vella canción");
    assert!(
        result_vella.contains("vella [bella]") || result_vella.contains("Vella [Bella]"),
        "Debe corregir 'vella' adjetival a 'bella': {}",
        result_vella
    );

    let result_que_si = corrector.correct("Contestó que si con la cabeza");
    assert!(
        result_que_si.contains("si [sí]") || result_que_si.contains("Si [Sí]"),
        "Debe acentuar 'sí' afirmativo en 'que si con la cabeza': {}",
        result_que_si
    );
}

#[test]
fn test_integration_round30_asin_ranking_prefers_asi() {
    let corrector = create_test_corrector();

    for text in ["asin fue", "asín fue"] {
        let result = corrector.correct(text);
        if result.to_lowercase().contains("[así]") {
            continue;
        }
        let first = result
            .split('|')
            .nth(1)
            .unwrap_or_default()
            .split(',')
            .next()
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        assert_eq!(
            first, "así",
            "La primera sugerencia de '{}' debe ser 'así': {}",
            text, result
        );
    }
}

#[test]
fn test_integration_round31_vocative_second_comma_after_name() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Dime María que pasó");
    assert!(
        result.contains("Dime [Dime,]"),
        "Debe mantener coma tras imperativo: {}",
        result
    );
    assert!(
        result.contains("María [María,]"),
        "Debe insertar coma tras vocativo intercalado: {}",
        result
    );
}

#[test]
fn test_integration_round32_quizas_priority_and_que_si_disambiguation() {
    let corrector = create_test_corrector();

    let result_quizas = corrector.correct("quizas mañana");
    let corrected_directly = result_quizas.contains("quizas [Quizás]")
        || result_quizas.contains("quizas [quizás]")
        || result_quizas.contains("Quizas [Quizás]")
        || result_quizas.contains("Quizas [quizás]");
    if !corrected_directly {
        let first_quizas = result_quizas
            .split('|')
            .nth(1)
            .unwrap_or_default()
            .split(',')
            .next()
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        assert_eq!(
            first_quizas, "quizás",
            "La primera sugerencia de 'quizas' debe ser 'quizás': {}",
            result_quizas
        );
    }

    let result_affirmative = corrector.correct("Contestó que si con la cabeza");
    assert!(
        result_affirmative.contains("si [sí]") || result_affirmative.contains("Si [Sí]"),
        "Debe acentuar 'sí' en contexto afirmativo: {}",
        result_affirmative
    );

    let result_indirect_question = corrector.correct("Me preguntó que si con eso bastaba");
    assert!(
        !result_indirect_question.contains("si [sí]")
            && !result_indirect_question.contains("Si [Sí]"),
        "No debe forzar tilde en interrogativa indirecta: {}",
        result_indirect_question
    );
}

#[test]
fn test_integration_round33_el_adverb_bridge_and_coordination() {
    let corrector = create_test_corrector();

    let result_adverb = corrector.correct("el también viene");
    let result_adverb_lower = result_adverb.to_lowercase();
    assert!(
        result_adverb_lower.contains("el [él]"),
        "Debe corregir 'el' a 'él' con adverbio puente + verbo: {}",
        result_adverb
    );

    let result_coord = corrector.correct("el y ella son amigos");
    let result_coord_lower = result_coord.to_lowercase();
    assert!(
        result_coord_lower.contains("el [él]"),
        "Debe corregir 'el' a 'él' en coordinación pronominal: {}",
        result_coord
    );
    assert!(
        !result_coord.contains("amigos [amigas]"),
        "No debe forzar femenino con sujeto mixto 'él y ella': {}",
        result_coord
    );
}

#[test]
fn test_integration_round33_que_si_conditional_de_clitic_and_que_infinitive() {
    let corrector = create_test_corrector();

    let result_que_si = corrector.correct("Ella dijo que si iría a la fiesta");
    let result_que_si_lower = result_que_si.to_lowercase();
    assert!(
        result_que_si_lower.contains("si [sí]"),
        "Debe acentuar 'sí' afirmativo en 'dijo que si iría': {}",
        result_que_si
    );

    let result_clitic_de = corrector.correct("que se lo de a Pedro");
    assert!(
        result_clitic_de.contains("de [dé]") || result_clitic_de.contains("De [Dé]"),
        "Debe corregir 'de' a 'dé' en secuencia clítica 'que se lo de ...': {}",
        result_clitic_de
    );

    let result_que_hacer = corrector.correct("no sabían que hacer");
    assert!(
        result_que_hacer.contains("que [qué]") || result_que_hacer.contains("Que [Qué]"),
        "Debe acentuar 'qué' en interrogativa indirecta 'no sabían que hacer': {}",
        result_que_hacer
    );
}

#[test]
fn test_integration_round33_spelling_initial_h_omission_candidates() {
    let corrector = create_test_corrector();

    for (text, expected) in [("aser", "hacer"), ("acer", "hacer"), ("erida", "herida")] {
        let result = corrector.correct(text);
        let normalized = result.to_lowercase();
        assert!(
            normalized.contains(expected),
            "Las sugerencias de '{}' deben incluir '{}': {}",
            text,
            expected,
            result
        );
    }
}

#[test]
fn test_integration_round34_postposed_subject_vs_gap() {
    let corrector = create_test_corrector();

    let result_salio = corrector.correct("Salió los niños al patio");
    let salio_norm = result_salio.to_lowercase();
    assert!(
        salio_norm.contains("salió [salieron]"),
        "Debe corregir V-S invertido en 'Salió los niños...': {}",
        result_salio
    );

    let result_existe = corrector.correct("Existe muchas razones para ello");
    let existe_norm = result_existe.to_lowercase();
    assert!(
        existe_norm.contains("existe [existen]"),
        "Debe corregir V-S invertido en 'Existe muchas razones...': {}",
        result_existe
    );

    let result_falta = corrector.correct("Falta tres días para la entrega");
    let falta_norm = result_falta.to_lowercase();
    assert!(
        falta_norm.contains("falta [faltan]"),
        "Debe corregir V-S invertido en 'Falta tres días...': {}",
        result_falta
    );

    let result_correct = corrector.correct("Faltan tres días para la entrega");
    assert!(
        !result_correct.contains("Faltan ["),
        "No debe tocar caso ya correcto: {}",
        result_correct
    );
}

#[test]
fn test_integration_round35_main_clause_after_relative_still_corrected() {
    let corrector = create_test_corrector();
    let result =
        corrector.correct("El equipo de rescate que trabajaron toda la noche están agotados");

    assert!(
        result.contains("trabajaron [trabajó]"),
        "Debe seguir corrigiendo el verbo de la relativa: {}",
        result
    );
    assert!(
        result.contains("están [está]") || result.contains("estan [está]"),
        "Debe corregir también el verbo principal tras la relativa: {}",
        result
    );
    assert!(
        result.contains("agotados [agotado]"),
        "Debe corregir también el adjetivo predicativo del sujeto principal: {}",
        result
    );
}

#[test]
fn test_integration_round39_bare_noun_subject_not_misread_as_initial_infinitive() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Primer ministro fue elegida");
    assert!(
        result.contains("elegida [elegido]"),
        "Debe corregir predicativo con sujeto nominal sin artículo: {}",
        result
    );
}

#[test]
fn test_integration_round36_missing_accent_conditional_plural_priority() {
    let corrector = create_test_corrector();
    let result = corrector.correct("tomarian");
    let first = result
        .split('|')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    assert_eq!(
        first, "tomarían",
        "La primera sugerencia de 'tomarian' debe ser 'tomarían': {}",
        result
    );
}

#[test]
fn test_integration_round39_enclitic_missing_accent_prefers_lexical_form() {
    let corrector = create_test_corrector();
    let result = corrector.correct("escuchame");
    if result.to_lowercase().contains("[escúchame]") {
        return;
    }
    let first = result
        .split('|')
        .nth(1)
        .unwrap_or_default()
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    assert_eq!(
        first, "escúchame",
        "La primera sugerencia de 'escuchame' debe ser 'escúchame': {}",
        result
    );
}

#[test]
fn test_integration_round40_enclitic_missing_accent_promoted_to_direct_correction() {
    let corrector = create_test_corrector();
    for (text, expected) in [
        ("digame", "dígame"),
        ("llamame", "llámame"),
        ("cuentame", "cuéntame"),
        ("escuchame", "escúchame"),
        ("sirveme", "sírveme"),
        ("preparamelo", "prepáramelo"),
    ] {
        let result = corrector.correct(text);
        let result_lower = result.to_lowercase();
        assert!(
            result_lower.contains(&format!("[{}]", expected)),
            "Debe promover corrección directa de enclítico sin tilde en '{}': {}",
            text,
            result
        );
        assert!(
            !result.contains('|'),
            "No debe duplicar con listado ortográfico cuando ya promovió enclítico en '{}': {}",
            text,
            result
        );
    }
}

#[test]
fn test_integration_round37_run_together_locutions_are_split() {
    let corrector = create_test_corrector();
    let cases = [
        ("Aveces llueve", "a veces"),
        ("Enserio no lo sé", "en serio"),
        ("Osea que vienes", "o sea"),
        ("Almenos vino", "al menos"),
        ("Amenudo pasa", "a menudo"),
        ("Sinembargo seguimos", "sin embargo"),
        ("Porlomenos vino", "por lo menos"),
        ("Devez en cuando", "de vez"),
    ];

    for (text, expected) in cases {
        let result = corrector.correct(text);
        assert!(
            result.to_lowercase().contains(expected),
            "Debe separar locución '{}' -> '{}': {}",
            text,
            expected,
            result
        );
    }
}

#[test]
fn test_integration_round37_mas_sin_embargo_stays_adversative() {
    let corrector = create_test_corrector();
    let result = corrector.correct("mas sin embargo lo intentó");
    let lower = result.to_lowercase();
    assert!(
        !lower.contains("mas [más]"),
        "No debe acentuar 'mas' adversativo en 'mas sin embargo': {}",
        result
    );
}

#[test]
fn test_integration_round37_compound_participle_keeps_grammar_target() {
    let corrector = create_test_corrector();
    let result = corrector.correct("He escribido una carta");
    assert!(
        result.contains("[escrito]"),
        "Debe mantener corrección gramatical del participio irregular: {}",
        result
    );
    assert!(
        !result.contains("escribido |"),
        "No debe duplicar ortografía cuando ya corrigió el participio irregular: {}",
        result
    );
}

#[test]
fn test_integration_round38_sentence_start_article_not_swallowed_as_clitic() {
    let corrector = create_test_corrector();

    let result_crisis = corrector.correct("el crisis economica");
    let crisis_lower = result_crisis.to_lowercase();
    assert!(
        crisis_lower.contains("el [la]"),
        "Debe corregir artículo en 'el crisis ...': {}",
        result_crisis
    );

    let result_virus = corrector.correct("la virus se propagó");
    let virus_lower = result_virus.to_lowercase();
    assert!(
        virus_lower.contains("la [el]"),
        "Debe corregir artículo en 'la virus ...': {}",
        result_virus
    );

    let result_tesis = corrector.correct("el tesis doctoral");
    let tesis_lower = result_tesis.to_lowercase();
    assert!(
        tesis_lower.contains("el [la]"),
        "Debe corregir artículo en 'el tesis ...': {}",
        result_tesis
    );

    let result_laismo_guard = corrector.correct("la cuento un secreto");
    assert!(
        result_laismo_guard.to_lowercase().contains("la [le]"),
        "No debe romper guardas de laísmo en clítico inicial: {}",
        result_laismo_guard
    );
}

#[test]
fn test_integration_round38_ambiguous_noun_adjective_uses_left_determiner_gender() {
    let corrector = create_test_corrector();
    let result = corrector.correct("La radio antiguo");
    let lower = result.to_lowercase();
    assert!(
        lower.contains("antiguo [antigua]"),
        "Con determinante femenino explícito, 'radio' debe forzar adjetivo femenino: {}",
        result
    );
}

#[test]
fn test_integration_round41_temporal_cuando_not_forced_to_interrogative() {
    let corrector = create_test_corrector();
    let result_temporal = corrector.correct("Se lo explicaré cuando llegue a casa");
    assert!(
        !result_temporal.contains("cuando [cuándo]") && !result_temporal.contains("Cuando [Cuándo]"),
        "No debe acentuar 'cuando' temporal en subordinada: {}",
        result_temporal
    );

    let result_temporal_decir = corrector.correct("Se lo dije cuando pude");
    assert!(
        !result_temporal_decir.contains("cuando [cuándo]")
            && !result_temporal_decir.contains("Cuando [Cuándo]"),
        "No debe acentuar 'cuando' temporal tras 'se lo dije': {}",
        result_temporal_decir
    );

    let result_interrogative = corrector.correct("Dime cuando vienes");
    assert!(
        result_interrogative.contains("cuando [cuándo]")
            || result_interrogative.contains("Cuando [Cuándo]"),
        "Debe mantener acento interrogativo en 'dime cuándo ...': {}",
        result_interrogative
    );
}

#[test]
fn test_integration_round42_tambien_already_accented_no_identity_correction() {
    let corrector = create_test_corrector();
    let result = corrector.correct("No solo es inteligente sino que también es amable");
    assert!(
        !result.contains("también [también]") && !result.contains("También [También]"),
        "No debe emitir corrección idéntica para 'también': {}",
        result
    );
}

#[test]
fn test_integration_round43_tu_dijistes_promoted_to_tu_dijiste_without_nominal_fp() {
    let corrector = create_test_corrector();

    let result = corrector.correct("Tu dijistes que vendrías");
    assert!(
        result.contains("Tu [Tú]") || result.contains("tu [tú]"),
        "Debe acentuar pronombre 'tú' en contexto verbal: {}",
        result
    );
    assert!(
        result.contains("dijistes [dijiste]") || result.contains("Dijistes [Dijiste]"),
        "Debe corregir vulgarismo verbal 'dijistes' como gramática: {}",
        result
    );
    assert!(
        !result.contains("dijistes |"),
        "No debe dejar ruido ortográfico para 'dijistes' cuando hay corrección gramatical: {}",
        result
    );

    let nominal = corrector.correct("tu chistes son graciosos");
    assert!(
        !nominal.contains("chistes [chiste]") && !nominal.contains("Chistes [Chiste]"),
        "No debe forzar lectura verbal en nominal plural ('tu chistes ...'): {}",
        nominal
    );
}

#[test]
fn test_integration_round44_pronoun_detection_with_adverbs_and_missing_forms() {
    let corrector = create_test_corrector();

    let laismo_subj = corrector.correct("No la digas nada");
    assert!(
        laismo_subj.contains("la [le]") || laismo_subj.contains("La [Le]"),
        "Debe detectar laísmo con subjuntivo de 'decir': {}",
        laismo_subj
    );

    let laismo_ayer = corrector.correct("Ayer la regalé flores");
    assert!(
        laismo_ayer.contains("la [le]") || laismo_ayer.contains("La [Le]"),
        "No debe bloquear laísmo por falso infinitivo en 'ayer': {}",
        laismo_ayer
    );

    let laismo_hablar = corrector.correct("La hablé por teléfono");
    assert!(
        laismo_hablar.contains("La [Le]") || laismo_hablar.contains("la [le]"),
        "Debe detectar laísmo con 'hablar': {}",
        laismo_hablar
    );

    let laismo_ofrecer = corrector.correct("La ofrecí mi ayuda");
    assert!(
        laismo_ofrecer.contains("La [Le]") || laismo_ofrecer.contains("la [le]"),
        "Debe detectar laísmo con pretérito de 'ofrecer': {}",
        laismo_ofrecer
    );

    let leismo_invitar = corrector.correct("Mañana les invitamos a cenar");
    assert!(
        leismo_invitar.contains("les [los]") || leismo_invitar.contains("les [las]"),
        "Debe detectar leísmo plural con 'invitar': {}",
        leismo_invitar
    );

    let leismo_dejar = corrector.correct("Mañana les dejamos en casa");
    assert!(
        leismo_dejar.contains("les [los]") || leismo_dejar.contains("les [las]"),
        "Debe detectar leísmo plural con 'dejar': {}",
        leismo_dejar
    );
}

#[test]
fn test_integration_round45_loismo_negated_contar_nada() {
    let corrector = create_test_corrector();

    let loismo = corrector.correct("No lo cuentes nada");
    assert!(
        loismo.contains("lo [le]") || loismo.contains("Lo [Le]"),
        "Debe detectar loísmo en patrón negado con 'contar + nada': {}",
        loismo
    );

    let direct_object = corrector.correct("Lo contó todo");
    assert!(
        !direct_object.contains("lo [le]") && !direct_object.contains("Lo [Le]"),
        "No debe forzar loísmo en uso posible de CD ('Lo contó todo'): {}",
        direct_object
    );
}

#[test]
fn test_integration_round46_e_as_auxiliary_after_clitic() {
    let corrector = create_test_corrector();

    let wrong_aux = corrector.correct("No lo e visto");
    assert!(
        wrong_aux.contains("e [he]") || wrong_aux.contains("E [He]"),
        "Debe corregir 'e' -> 'he' en patrón clítico + participio: {}",
        wrong_aux
    );

    let copulative = corrector.correct("Padre e hija llegaron temprano");
    assert!(
        !copulative.contains("e [he]") && !copulative.contains("E [He]"),
        "No debe tocar la conjunción copulativa 'e': {}",
        copulative
    );
}

#[test]
fn test_integration_round47_depronto_should_split() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Depronto se fue");
    assert!(
        result.contains("Depronto [De pronto]") || result.contains("depronto [de pronto]"),
        "Debe separar 'depronto' como locución: {}",
        result
    );
}

#[test]
fn test_integration_round48_tu_and_el_pronominal_contexts() {
    let corrector = create_test_corrector();

    let tu_clitic = corrector.correct("si tu lo acompañabas");
    assert!(
        tu_clitic.contains("tu [tú]") || tu_clitic.contains("Tu [Tú]"),
        "Debe acentuar 'tú' en patrón clítico + verbo: {}",
        tu_clitic
    );

    let tu_comparative = corrector.correct("más listo que tu");
    assert!(
        tu_comparative.contains("tu [tú]") || tu_comparative.contains("Tu [Tú]"),
        "Debe acentuar 'tú' al cierre de comparativa: {}",
        tu_comparative
    );

    let tu_clause = corrector.correct("que tu si irás");
    assert!(
        tu_clause.contains("tu [tú]") || tu_clause.contains("Tu [Tú]"),
        "Debe acentuar 'tú' en 'que tú sí/no + verbo': {}",
        tu_clause
    );
    assert!(
        tu_clause.contains("si [sí]") || tu_clause.contains("Si [Sí]"),
        "Debe acentuar 'sí' en 'que tú sí/no + verbo': {}",
        tu_clause
    );

    let el_after_tonic = corrector.correct("Para mí el es más listo que tú");
    assert!(
        el_after_tonic.contains("el [él]") || el_after_tonic.contains("El [Él]"),
        "Debe acentuar 'él' tras pronombre tónico previo: {}",
        el_after_tonic
    );
}

#[test]
fn test_integration_round49_grabe_grave_contextual() {
    let corrector = create_test_corrector();

    let adjectival = corrector.correct("La situación es grabe");
    assert!(
        adjectival.contains("grabe [grave]") || adjectival.contains("Grabe [Grave]"),
        "Debe corregir 'grabe' -> 'grave' tras cópula: {}",
        adjectival
    );

    let verbal = corrector.correct("Quiero que grabe el audio");
    assert!(
        !verbal.contains("grabe [grave]") && !verbal.contains("Grabe [Grave]"),
        "No debe corregir subjuntivo verbal 'grabe': {}",
        verbal
    );
}

#[test]
fn test_integration_round50_ernia_prioritizes_hernia() {
    let corrector = create_test_corrector();
    let result = corrector.correct("ernia");
    assert!(
        result.contains("ernia |hernia,") || result.contains("Ernia |Hernia,"),
        "Debe priorizar 'hernia' como primera sugerencia para 'ernia': {}",
        result
    );
}

#[test]
fn test_integration_round51_invariable_noun_gender_keeps_noun_over_wrong_article() {
    let corrector = create_test_corrector();

    let already_agreeing_adj = corrector.correct("El tesis está lista");
    assert!(
        already_agreeing_adj.contains("El [La]") || already_agreeing_adj.contains("el [la]"),
        "Debe corregir solo el artículo en 'El tesis está lista': {}",
        already_agreeing_adj
    );
    assert!(
        !already_agreeing_adj.contains("lista [listo]")
            && !already_agreeing_adj.contains("Lista [Listo]"),
        "No debe forzar el adjetivo al género del artículo errado: {}",
        already_agreeing_adj
    );

    let mismatching_adj = corrector.correct("El tesis está listo");
    assert!(
        mismatching_adj.contains("El [La]") || mismatching_adj.contains("el [la]"),
        "Debe corregir el artículo en 'El tesis está listo': {}",
        mismatching_adj
    );
    assert!(
        mismatching_adj.contains("listo [lista]") || mismatching_adj.contains("Listo [Lista]"),
        "Debe corregir el adjetivo al género del sustantivo: {}",
        mismatching_adj
    );

    let plural_invariable = corrector.correct("Las tesis están listas");
    assert!(
        !plural_invariable.contains('['),
        "No debe introducir correcciones en plural correcto con sustantivo invariable: {}",
        plural_invariable
    );
}

#[test]
fn test_integration_round52_ser_plural_with_adjectival_attribute_is_not_accepted() {
    let corrector = create_test_corrector();

    let singular_subject = corrector.correct("El análisis fueron correctos");
    assert!(
        singular_subject.contains("fueron [fue]") || singular_subject.contains("Fueron [Fue]"),
        "Debe corregir verbo singular en cópula con atributo adjetival: {}",
        singular_subject
    );

    let nominal_plural_attribute = corrector.correct("El problema fueron las lluvias");
    assert!(
        !nominal_plural_attribute.contains("fueron [fue]")
            && !nominal_plural_attribute.contains("Fueron [Fue]"),
        "No debe romper la excepción nominal válida con atributo plural: {}",
        nominal_plural_attribute
    );
}

#[test]
fn test_integration_round53_preposition_el_before_capitalized_noun_not_promoted_to_pronoun() {
    let corrector = create_test_corrector();

    let nominal_capitalized = corrector.correct("DMS en el Data Center");
    assert!(
        !nominal_capitalized.contains("el [él]") && !nominal_capitalized.contains("El [Él]"),
        "No debe acentuar artículo en sintagma nominal capitalizado: {}",
        nominal_capitalized
    );

    let pronominal_control = corrector.correct("De el depende todo");
    assert!(
        pronominal_control.contains("el [él]") || pronominal_control.contains("El [Él]"),
        "Debe mantener corrección pronominal en contexto verbal claro: {}",
        pronominal_control
    );

    let adverb_bridge = corrector.correct("Para el también es difícil");
    assert!(
        adverb_bridge.contains("el [él]") || adverb_bridge.contains("El [Él]"),
        "Debe corregir pronombre tónico con puente adverbial: {}",
        adverb_bridge
    );

    let adverb_negated_bridge = corrector.correct("Para el ya no es posible");
    assert!(
        adverb_negated_bridge.contains("el [él]")
            || adverb_negated_bridge.contains("El [Él]"),
        "Debe corregir pronombre tónico con adverbio + negación: {}",
        adverb_negated_bridge
    );
}
