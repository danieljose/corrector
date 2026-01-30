//! Módulo de diccionario
//!
//! Proporciona estructuras para almacenamiento y búsqueda eficiente de palabras.

pub mod loader;
pub mod names;
pub mod trie;

pub use loader::DictionaryLoader;
pub use names::ProperNames;
pub use trie::{Gender, Number, Trie, WordCategory, WordInfo};
