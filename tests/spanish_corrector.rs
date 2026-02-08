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
