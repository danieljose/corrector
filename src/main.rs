use std::fs;
use std::process;

use corrector::{Config, Corrector};

fn main() {
    let config = match Config::from_args(std::env::args().collect()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            Config::print_help();
            process::exit(1);
        }
    };

    if config.show_help {
        Config::print_help();
        return;
    }

    let mut corrector = match Corrector::new(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error inicializando corrector: {}", e);
            process::exit(1);
        }
    };

    // Añadir palabra al diccionario custom si se especificó
    if let Some(ref word) = config.add_word {
        if let Err(e) = corrector.add_custom_word(word) {
            eprintln!("Error añadiendo palabra: {}", e);
            process::exit(1);
        }
        println!("Palabra '{}' añadida al diccionario personalizado.", word);
        return;
    }

    // Obtener texto a corregir
    let text = if let Some(ref input_file) = config.input_file {
        match fs::read_to_string(input_file) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error leyendo archivo '{}': {}", input_file, e);
                process::exit(1);
            }
        }
    } else if let Some(ref text) = config.text {
        text.clone()
    } else {
        eprintln!("Error: No se proporcionó texto para corregir.");
        eprintln!();
        Config::print_help();
        process::exit(1);
    };

    // Corregir texto
    let result = corrector.correct(&text);

    // Escribir resultado
    if let Some(ref output_file) = config.output_file {
        if let Err(e) = fs::write(output_file, &result) {
            eprintln!("Error escribiendo archivo '{}': {}", output_file, e);
            process::exit(1);
        }
    } else {
        println!("{}", result);
    }
}
