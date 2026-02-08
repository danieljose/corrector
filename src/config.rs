//! Configuración y argumentos CLI

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// Idioma a usar (default: "es")
    pub language: String,
    /// Separador para errores ortográficos (default: "|")
    pub spelling_separator: String,
    /// Separador para errores gramaticales (default: "[]")
    pub grammar_separator: (String, String),
    /// Archivo de entrada
    pub input_file: Option<String>,
    /// Archivo de salida
    pub output_file: Option<String>,
    /// Diccionario personalizado adicional
    pub custom_dict: Option<String>,
    /// Palabra a añadir al diccionario custom
    pub add_word: Option<String>,
    /// Texto a corregir (argumento posicional)
    pub text: Option<String>,
    /// Mostrar ayuda
    pub show_help: bool,
    /// Directorio de datos
    pub data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "es".to_string(),
            spelling_separator: "|".to_string(),
            grammar_separator: ("[".to_string(), "]".to_string()),
            input_file: None,
            output_file: None,
            custom_dict: None,
            add_word: None,
            text: None,
            show_help: false,
            data_dir: PathBuf::from("data"),
        }
    }
}

impl Config {
    pub fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut config = Config::default();
        let mut args_iter = args.into_iter().skip(1); // Skip program name

        while let Some(arg) = args_iter.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    config.show_help = true;
                    return Ok(config);
                }
                "-l" | "--lang" => {
                    config.language = args_iter.next().ok_or("--lang requiere un valor")?;
                }
                "-s" | "--separator" => {
                    config.spelling_separator =
                        args_iter.next().ok_or("--separator requiere un valor")?;
                }
                "-g" | "--grammar-separator" => {
                    let sep = args_iter
                        .next()
                        .ok_or("--grammar-separator requiere un valor")?;
                    if sep.len() < 2 {
                        return Err(
                            "--grammar-separator debe tener al menos 2 caracteres".to_string()
                        );
                    }
                    let mid = sep.len() / 2;
                    config.grammar_separator = (sep[..mid].to_string(), sep[mid..].to_string());
                }
                "-i" | "--input" => {
                    config.input_file = Some(args_iter.next().ok_or("--input requiere un valor")?);
                }
                "-o" | "--output" => {
                    config.output_file =
                        Some(args_iter.next().ok_or("--output requiere un valor")?);
                }
                "-d" | "--custom-dict" => {
                    config.custom_dict =
                        Some(args_iter.next().ok_or("--custom-dict requiere un valor")?);
                }
                "-a" | "--add-word" => {
                    config.add_word = Some(args_iter.next().ok_or("--add-word requiere un valor")?);
                }
                "--data-dir" => {
                    config.data_dir =
                        PathBuf::from(args_iter.next().ok_or("--data-dir requiere un valor")?);
                }
                _ => {
                    if arg.starts_with('-') {
                        return Err(format!("Opción desconocida: {}", arg));
                    }
                    // Argumento posicional = texto a corregir
                    config.text = Some(arg);
                }
            }
        }

        config.language = Self::canonicalize_language(&config.language);
        Ok(config)
    }

    fn canonicalize_language(language: &str) -> String {
        let normalized = language.trim().to_lowercase();
        match normalized.as_str() {
            "es" | "spanish" | "espanol" | "español" => "es".to_string(),
            "ca" | "catalan" | "catala" | "català" => "ca".to_string(),
            _ => normalized,
        }
    }

    pub fn print_help() {
        println!(
            r#"Corrector - Corrector ortográfico y gramatical

USO:
    corrector [OPCIONES] [TEXTO]

ARGUMENTOS:
    [TEXTO]    Texto a corregir

OPCIONES:
    -h, --help                  Muestra esta ayuda
    -l, --lang <IDIOMA>         Idioma a usar (default: es)
    -s, --separator <SEP>       Separador ortográfico (default: |)
    -g, --grammar-separator <SEP>  Separador gramatical (default: [])
    -i, --input <ARCHIVO>       Archivo de entrada
    -o, --output <ARCHIVO>      Archivo de salida
    -d, --custom-dict <ARCHIVO> Diccionario adicional
    -a, --add-word <PALABRA>    Añadir palabra al diccionario custom
    --data-dir <DIR>            Directorio de datos (default: data)

EJEMPLOS:
    corrector "Hola, tengo un probelma"
    corrector --lang es --input archivo.txt
    corrector -s "||" "texto a corregir"
    corrector --add-word "programación""#
        );
    }
}
