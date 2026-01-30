//! Módulo de conjugación verbal para español
//!
//! Permite reconocer formas verbales conjugadas dinámicamente
//! sin necesidad de añadirlas al diccionario.

pub mod enclitics;
pub mod irregular;
pub mod prefixes;
pub mod recognizer;
pub mod regular;
pub mod stem_changing;

/// Tiempo verbal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tense {
    Presente,
    Preterito,
    Imperfecto,
    Futuro,
    Condicional,
}

/// Modo verbal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mood {
    Indicativo,
    Subjuntivo,
    Imperativo,
}

/// Persona gramatical
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Person {
    First,
    Second,
    Third,
}

/// Número gramatical (para conjugación)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerbNumber {
    Singular,
    Plural,
}

/// Clase de verbo según su terminación
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerbClass {
    Ar,
    Er,
    Ir,
}

impl VerbClass {
    /// Determina la clase de verbo a partir del infinitivo
    pub fn from_infinitive(infinitive: &str) -> Option<Self> {
        let lower = infinitive.to_lowercase();
        if lower.ends_with("ar") {
            Some(VerbClass::Ar)
        } else if lower.ends_with("er") {
            Some(VerbClass::Er)
        } else if lower.ends_with("ir") {
            Some(VerbClass::Ir)
        } else {
            None
        }
    }

    /// Obtiene la terminación del infinitivo
    pub fn infinitive_ending(&self) -> &'static str {
        match self {
            VerbClass::Ar => "ar",
            VerbClass::Er => "er",
            VerbClass::Ir => "ir",
        }
    }
}

/// Resultado del reconocimiento de un verbo
#[derive(Debug, Clone)]
pub struct VerbRecognitionResult {
    /// Infinitivo del verbo (si se pudo determinar)
    pub infinitive: String,
    /// Si la forma verbal es válida
    pub is_valid: bool,
}

pub use recognizer::VerbRecognizer;
