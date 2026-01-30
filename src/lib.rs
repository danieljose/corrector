//! Corrector - Biblioteca de corrección ortográfica y gramatical
//!
//! Proporciona funcionalidades para corrección de texto en múltiples idiomas.

pub mod config;
pub mod corrector;
pub mod dictionary;
pub mod grammar;
pub mod languages;
pub mod spelling;

pub use config::Config;
pub use corrector::Corrector;
