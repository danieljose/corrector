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
        "Debe corregir ambos 'se' como 'sÃ©' en 'Solo se que no se nada': {}",
        result
    );
    assert!(
        lower.contains("solo se [s"),
        "Debe corregir tambiÃ©n el primer 'se' tras adverbio: {}",
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
    let infinitive = corrector::languages::VerbFormRecognizer::get_infinitive(
        &recognizer,
        "tendr\u{00ED}amos",
    );
    let knows_tener = corrector::languages::VerbFormRecognizer::knows_infinitive(
        &recognizer,
        "tener",
    );

    assert!(
        corrections.iter().any(|c| c.suggestion == "tuvi\u{00E9}ramos"),
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
    let result = corrector.correct("Ningún personas");

    assert!(
        result.contains("Ningún ["),
        "Deberia corregir cuantificador indefinido en 'Ningún personas': {}",
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
    let result = corrector.correct("El resumen que presentaron en la reunión pasada quedó aprobado");

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
        "Debería corregir 'Haber cuando' -> 'A ver cuándo': {}",
        result
    );
    assert!(
        result.contains("cuando [cuándo]"),
        "Debería acentuar interrogativo en 'A ver cuándo': {}",
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
        "Debe acentuar 'donde' en interrogativa indirecta dentro de Â¿...?: {}",
        result
    );

    let result = corrector.correct("\u{00BF}Me dices como se llama?");
    assert!(
        result.to_lowercase().contains("como [c"),
        "Debe acentuar 'como' en interrogativa indirecta dentro de Â¿...?: {}",
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
        "Debería marcar el 'no' sobrante al fusionar 'sino': {}",
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
        result.contains("A [En]")
            && result.contains("nivel [cuanto]")
            && result.contains("de [a]"),
        "Debería corregir 'a nivel de' no técnico -> 'en cuanto a': {}",
        result
    );
}

#[test]
fn test_integration_fossilized_preposition_a_nivel_del_mar_not_changed() {
    let corrector = create_test_corrector();
    let result = corrector.correct("Viviamos a nivel del mar");

    assert!(
        !result.contains("a [en]")
            && !result.contains("nivel [cuanto]")
            && !result.contains("de [a]"),
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
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result.to_lowercase().contains(&expected_fragment.to_lowercase()),
            "Deberia corregir concordancia copulativa en '{}': {}",
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
        "Juan esta contento",
        "Pedro esta cansado",
        "Carlos esta enfadado",
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
            result.to_lowercase().contains(&expected_fragment.to_lowercase()),
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
    let result = corrector.correct("Tanto él como ella son buenos, y tanto yo como tú sabemos la verdad");

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
    let result = corrector.correct("Tanto Pedro como Juan vienen temprano y tanto ella como él están listos");

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
    ];

    for (input, expected_fragment) in cases {
        let result = corrector.correct(input);
        assert!(
            result.to_lowercase().contains(&expected_fragment.to_lowercase()),
            "Deberia detectar queismo con preposicion adecuada en '{}': {}",
            input,
            result
        );
    }
}
