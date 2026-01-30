//! Corrección de tiempos compuestos
//!
//! Detecta errores en construcciones con el auxiliar "haber" + participio.
//!
//! Ejemplos de errores:
//! - "he comí" → "he comido" (pretérito en lugar de participio)
//! - "ha fue" → "ha ido" (pretérito irregular)
//! - "hemos cantamos" → "hemos cantado"
//! - "había vino" → "había venido"

use crate::grammar::{Token, TokenType};
use std::collections::{HashMap, HashSet};

/// Corrección de tiempo compuesto sugerida
#[derive(Debug, Clone)]
pub struct CompoundVerbCorrection {
    pub token_index: usize,
    pub original: String,
    pub suggestion: String,
    pub reason: String,
}

/// Analizador de tiempos compuestos
pub struct CompoundVerbAnalyzer {
    /// Formas conjugadas del verbo "haber" como auxiliar
    haber_forms: HashSet<&'static str>,
    /// Participios irregulares: infinitivo → participio
    irregular_participles: HashMap<&'static str, &'static str>,
    /// Formas conjugadas que NO son participios (pretéritos, presentes, etc.)
    /// Mapeadas a su infinitivo para generar el participio correcto
    non_participle_forms: HashMap<&'static str, &'static str>,
}

impl CompoundVerbAnalyzer {
    pub fn new() -> Self {
        let mut haber_forms = HashSet::new();
        // Presente indicativo
        haber_forms.insert("he");
        haber_forms.insert("has");
        haber_forms.insert("ha");
        haber_forms.insert("hemos");
        haber_forms.insert("habéis");
        haber_forms.insert("han");
        // Imperfecto
        haber_forms.insert("había");
        haber_forms.insert("habías");
        haber_forms.insert("habíamos");
        haber_forms.insert("habíais");
        haber_forms.insert("habían");
        // Pretérito indefinido (tiempos compuestos menos comunes pero válidos)
        haber_forms.insert("hube");
        haber_forms.insert("hubiste");
        haber_forms.insert("hubo");
        haber_forms.insert("hubimos");
        haber_forms.insert("hubisteis");
        haber_forms.insert("hubieron");
        // Futuro
        haber_forms.insert("habré");
        haber_forms.insert("habrás");
        haber_forms.insert("habrá");
        haber_forms.insert("habremos");
        haber_forms.insert("habréis");
        haber_forms.insert("habrán");
        // Condicional
        haber_forms.insert("habría");
        haber_forms.insert("habrías");
        haber_forms.insert("habríamos");
        haber_forms.insert("habríais");
        haber_forms.insert("habrían");
        // Subjuntivo presente
        haber_forms.insert("haya");
        haber_forms.insert("hayas");
        haber_forms.insert("hayamos");
        haber_forms.insert("hayáis");
        haber_forms.insert("hayan");
        // Subjuntivo imperfecto
        haber_forms.insert("hubiera");
        haber_forms.insert("hubieras");
        haber_forms.insert("hubiéramos");
        haber_forms.insert("hubierais");
        haber_forms.insert("hubieran");
        haber_forms.insert("hubiese");
        haber_forms.insert("hubieses");
        haber_forms.insert("hubiésemos");
        haber_forms.insert("hubieseis");
        haber_forms.insert("hubiesen");

        let mut irregular_participles = HashMap::new();
        // Participios irregulares comunes
        irregular_participles.insert("hacer", "hecho");
        irregular_participles.insert("decir", "dicho");
        irregular_participles.insert("ver", "visto");
        irregular_participles.insert("poner", "puesto");
        irregular_participles.insert("escribir", "escrito");
        irregular_participles.insert("abrir", "abierto");
        irregular_participles.insert("volver", "vuelto");
        irregular_participles.insert("romper", "roto");
        irregular_participles.insert("morir", "muerto");
        irregular_participles.insert("cubrir", "cubierto");
        irregular_participles.insert("freír", "frito");
        irregular_participles.insert("imprimir", "impreso");
        irregular_participles.insert("resolver", "resuelto");
        irregular_participles.insert("satisfacer", "satisfecho");
        irregular_participles.insert("ir", "ido");
        irregular_participles.insert("ser", "sido");
        irregular_participles.insert("estar", "estado");
        irregular_participles.insert("haber", "habido");
        // Compuestos
        irregular_participles.insert("deshacer", "deshecho");
        irregular_participles.insert("rehacer", "rehecho");
        irregular_participles.insert("predecir", "predicho");
        irregular_participles.insert("contradecir", "contradicho");
        irregular_participles.insert("descubrir", "descubierto");
        irregular_participles.insert("devolver", "devuelto");
        irregular_participles.insert("envolver", "envuelto");
        irregular_participles.insert("revolver", "revuelto");
        irregular_participles.insert("componer", "compuesto");
        irregular_participles.insert("disponer", "dispuesto");
        irregular_participles.insert("exponer", "expuesto");
        irregular_participles.insert("imponer", "impuesto");
        irregular_participles.insert("oponer", "opuesto");
        irregular_participles.insert("proponer", "propuesto");
        irregular_participles.insert("suponer", "supuesto");
        irregular_participles.insert("prever", "previsto");
        irregular_participles.insert("entrever", "entrevisto");
        irregular_participles.insert("describir", "descrito");
        irregular_participles.insert("inscribir", "inscrito");
        irregular_participles.insert("prescribir", "prescrito");
        irregular_participles.insert("suscribir", "suscrito");
        irregular_participles.insert("transcribir", "transcrito");
        // Verbos en -aer con participio irregular
        irregular_participles.insert("traer", "traído");
        irregular_participles.insert("atraer", "atraído");
        irregular_participles.insert("contraer", "contraído");
        irregular_participles.insert("distraer", "distraído");
        irregular_participles.insert("extraer", "extraído");
        irregular_participles.insert("retraer", "retraído");
        irregular_participles.insert("sustraer", "sustraído");
        // Otros verbos con participio acentuado
        irregular_participles.insert("caer", "caído");
        irregular_participles.insert("leer", "leído");
        irregular_participles.insert("creer", "creído");
        irregular_participles.insert("oír", "oído");
        irregular_participles.insert("poseer", "poseído");
        irregular_participles.insert("proveer", "proveído");

        let mut non_participle_forms = HashMap::new();
        // Formas de verbos irregulares que pueden confundirse
        // IR - pretéritos y otras formas
        non_participle_forms.insert("fui", "ir");
        non_participle_forms.insert("fuiste", "ir");
        non_participle_forms.insert("fue", "ir");
        non_participle_forms.insert("fuimos", "ir");
        non_participle_forms.insert("fuisteis", "ir");
        non_participle_forms.insert("fueron", "ir");
        non_participle_forms.insert("iba", "ir");
        non_participle_forms.insert("ibas", "ir");
        non_participle_forms.insert("íbamos", "ir");
        non_participle_forms.insert("iban", "ir");
        non_participle_forms.insert("voy", "ir");
        non_participle_forms.insert("vas", "ir");
        non_participle_forms.insert("va", "ir");
        non_participle_forms.insert("vamos", "ir");
        non_participle_forms.insert("van", "ir");
        // SER - pretéritos
        // (fui/fue también son de ser, pero ya están mapeados a ir - el participio es diferente)
        // ESTAR
        non_participle_forms.insert("estuve", "estar");
        non_participle_forms.insert("estuviste", "estar");
        non_participle_forms.insert("estuvo", "estar");
        non_participle_forms.insert("estuvimos", "estar");
        non_participle_forms.insert("estuvieron", "estar");
        non_participle_forms.insert("estoy", "estar");
        non_participle_forms.insert("estás", "estar");
        non_participle_forms.insert("está", "estar");
        non_participle_forms.insert("estamos", "estar");
        non_participle_forms.insert("están", "estar");
        // HACER
        non_participle_forms.insert("hice", "hacer");
        non_participle_forms.insert("hiciste", "hacer");
        non_participle_forms.insert("hizo", "hacer");
        non_participle_forms.insert("hicimos", "hacer");
        non_participle_forms.insert("hicieron", "hacer");
        non_participle_forms.insert("hago", "hacer");
        non_participle_forms.insert("haces", "hacer");
        non_participle_forms.insert("hace", "hacer");
        non_participle_forms.insert("hacemos", "hacer");
        non_participle_forms.insert("hacen", "hacer");
        // DECIR
        non_participle_forms.insert("dije", "decir");
        non_participle_forms.insert("dijiste", "decir");
        non_participle_forms.insert("dijo", "decir");
        non_participle_forms.insert("dijimos", "decir");
        non_participle_forms.insert("dijeron", "decir");
        non_participle_forms.insert("digo", "decir");
        non_participle_forms.insert("dices", "decir");
        non_participle_forms.insert("dice", "decir");
        non_participle_forms.insert("decimos", "decir");
        non_participle_forms.insert("dicen", "decir");
        // VER
        non_participle_forms.insert("vi", "ver");
        non_participle_forms.insert("viste", "ver");
        non_participle_forms.insert("vio", "ver");
        non_participle_forms.insert("vimos", "ver");
        non_participle_forms.insert("vieron", "ver");
        non_participle_forms.insert("veo", "ver");
        non_participle_forms.insert("ves", "ver");
        non_participle_forms.insert("ve", "ver");
        non_participle_forms.insert("vemos", "ver");
        non_participle_forms.insert("ven", "ver");
        // PONER
        non_participle_forms.insert("puse", "poner");
        non_participle_forms.insert("pusiste", "poner");
        non_participle_forms.insert("puso", "poner");
        non_participle_forms.insert("pusimos", "poner");
        non_participle_forms.insert("pusieron", "poner");
        non_participle_forms.insert("pongo", "poner");
        non_participle_forms.insert("pones", "poner");
        non_participle_forms.insert("pone", "poner");
        non_participle_forms.insert("ponemos", "poner");
        non_participle_forms.insert("ponen", "poner");
        // VENIR
        non_participle_forms.insert("vine", "venir");
        non_participle_forms.insert("viniste", "venir");
        non_participle_forms.insert("vino", "venir");
        non_participle_forms.insert("vinimos", "venir");
        non_participle_forms.insert("vinieron", "venir");
        non_participle_forms.insert("vengo", "venir");
        non_participle_forms.insert("vienes", "venir");
        non_participle_forms.insert("viene", "venir");
        non_participle_forms.insert("venimos", "venir");
        non_participle_forms.insert("vienen", "venir");
        // TENER
        non_participle_forms.insert("tuve", "tener");
        non_participle_forms.insert("tuviste", "tener");
        non_participle_forms.insert("tuvo", "tener");
        non_participle_forms.insert("tuvimos", "tener");
        non_participle_forms.insert("tuvieron", "tener");
        non_participle_forms.insert("tengo", "tener");
        non_participle_forms.insert("tienes", "tener");
        non_participle_forms.insert("tiene", "tener");
        non_participle_forms.insert("tenemos", "tener");
        non_participle_forms.insert("tienen", "tener");
        // PODER
        non_participle_forms.insert("pude", "poder");
        non_participle_forms.insert("pudiste", "poder");
        non_participle_forms.insert("pudo", "poder");
        non_participle_forms.insert("pudimos", "poder");
        non_participle_forms.insert("pudieron", "poder");
        non_participle_forms.insert("puedo", "poder");
        non_participle_forms.insert("puedes", "poder");
        non_participle_forms.insert("puede", "poder");
        non_participle_forms.insert("podemos", "poder");
        non_participle_forms.insert("pueden", "poder");
        // SABER
        non_participle_forms.insert("supe", "saber");
        non_participle_forms.insert("supiste", "saber");
        non_participle_forms.insert("supo", "saber");
        non_participle_forms.insert("supimos", "saber");
        non_participle_forms.insert("supieron", "saber");
        non_participle_forms.insert("sé", "saber");
        non_participle_forms.insert("sabes", "saber");
        non_participle_forms.insert("sabe", "saber");
        non_participle_forms.insert("sabemos", "saber");
        non_participle_forms.insert("saben", "saber");
        // QUERER
        non_participle_forms.insert("quise", "querer");
        non_participle_forms.insert("quisiste", "querer");
        non_participle_forms.insert("quiso", "querer");
        non_participle_forms.insert("quisimos", "querer");
        non_participle_forms.insert("quisieron", "querer");
        non_participle_forms.insert("quiero", "querer");
        non_participle_forms.insert("quieres", "querer");
        non_participle_forms.insert("quiere", "querer");
        non_participle_forms.insert("queremos", "querer");
        non_participle_forms.insert("quieren", "querer");
        // ABRIR
        non_participle_forms.insert("abrí", "abrir");
        non_participle_forms.insert("abriste", "abrir");
        non_participle_forms.insert("abrió", "abrir");
        non_participle_forms.insert("abrimos", "abrir");
        non_participle_forms.insert("abrieron", "abrir");
        // ESCRIBIR
        non_participle_forms.insert("escribí", "escribir");
        non_participle_forms.insert("escribiste", "escribir");
        non_participle_forms.insert("escribió", "escribir");
        non_participle_forms.insert("escribimos", "escribir");
        non_participle_forms.insert("escribieron", "escribir");
        // ROMPER
        non_participle_forms.insert("rompí", "romper");
        non_participle_forms.insert("rompiste", "romper");
        non_participle_forms.insert("rompió", "romper");
        non_participle_forms.insert("rompimos", "romper");
        non_participle_forms.insert("rompieron", "romper");
        // VOLVER
        non_participle_forms.insert("volví", "volver");
        non_participle_forms.insert("volviste", "volver");
        non_participle_forms.insert("volvió", "volver");
        non_participle_forms.insert("volvimos", "volver");
        non_participle_forms.insert("volvieron", "volver");
        // TRAER
        non_participle_forms.insert("traje", "traer");
        non_participle_forms.insert("trajiste", "traer");
        non_participle_forms.insert("trajo", "traer");
        non_participle_forms.insert("trajimos", "traer");
        non_participle_forms.insert("trajeron", "traer");
        non_participle_forms.insert("traigo", "traer");
        non_participle_forms.insert("traes", "traer");
        non_participle_forms.insert("trae", "traer");
        non_participle_forms.insert("traemos", "traer");
        non_participle_forms.insert("traen", "traer");
        // CAER
        non_participle_forms.insert("caí", "caer");
        non_participle_forms.insert("caíste", "caer");
        non_participle_forms.insert("cayó", "caer");
        non_participle_forms.insert("caímos", "caer");
        non_participle_forms.insert("cayeron", "caer");
        non_participle_forms.insert("caigo", "caer");
        non_participle_forms.insert("caes", "caer");
        non_participle_forms.insert("cae", "caer");
        non_participle_forms.insert("caemos", "caer");
        non_participle_forms.insert("caen", "caer");
        // OÍR
        non_participle_forms.insert("oí", "oír");
        non_participle_forms.insert("oíste", "oír");
        non_participle_forms.insert("oyó", "oír");
        non_participle_forms.insert("oímos", "oír");
        non_participle_forms.insert("oyeron", "oír");
        non_participle_forms.insert("oigo", "oír");
        non_participle_forms.insert("oyes", "oír");
        non_participle_forms.insert("oye", "oír");
        non_participle_forms.insert("oímos", "oír");
        non_participle_forms.insert("oyen", "oír");
        // LEER
        non_participle_forms.insert("leí", "leer");
        non_participle_forms.insert("leíste", "leer");
        non_participle_forms.insert("leyó", "leer");
        non_participle_forms.insert("leímos", "leer");
        non_participle_forms.insert("leyeron", "leer");
        non_participle_forms.insert("leo", "leer");
        non_participle_forms.insert("lees", "leer");
        non_participle_forms.insert("lee", "leer");
        non_participle_forms.insert("leemos", "leer");
        non_participle_forms.insert("leen", "leer");
        // CREER
        non_participle_forms.insert("creí", "creer");
        non_participle_forms.insert("creíste", "creer");
        non_participle_forms.insert("creyó", "creer");
        non_participle_forms.insert("creímos", "creer");
        non_participle_forms.insert("creyeron", "creer");
        // MORIR
        non_participle_forms.insert("morí", "morir");
        non_participle_forms.insert("moriste", "morir");
        non_participle_forms.insert("murió", "morir");
        non_participle_forms.insert("morimos", "morir");
        non_participle_forms.insert("murieron", "morir");
        // COMER
        non_participle_forms.insert("comí", "comer");
        non_participle_forms.insert("comiste", "comer");
        non_participle_forms.insert("comió", "comer");
        non_participle_forms.insert("comimos", "comer");
        non_participle_forms.insert("comieron", "comer");
        non_participle_forms.insert("como", "comer");
        non_participle_forms.insert("comes", "comer");
        non_participle_forms.insert("come", "comer");
        non_participle_forms.insert("comemos", "comer");
        non_participle_forms.insert("comen", "comer");
        // VIVIR
        non_participle_forms.insert("viví", "vivir");
        non_participle_forms.insert("viviste", "vivir");
        non_participle_forms.insert("vivió", "vivir");
        non_participle_forms.insert("vivimos", "vivir");
        non_participle_forms.insert("vivieron", "vivir");
        non_participle_forms.insert("vivo", "vivir");
        non_participle_forms.insert("vives", "vivir");
        non_participle_forms.insert("vive", "vivir");
        non_participle_forms.insert("viven", "vivir");
        // CANTAR
        non_participle_forms.insert("canté", "cantar");
        non_participle_forms.insert("cantaste", "cantar");
        non_participle_forms.insert("cantó", "cantar");
        non_participle_forms.insert("cantamos", "cantar");
        non_participle_forms.insert("cantaron", "cantar");
        non_participle_forms.insert("canto", "cantar");
        non_participle_forms.insert("cantas", "cantar");
        non_participle_forms.insert("canta", "cantar");
        non_participle_forms.insert("cantan", "cantar");
        // HABLAR
        non_participle_forms.insert("hablé", "hablar");
        non_participle_forms.insert("hablaste", "hablar");
        non_participle_forms.insert("habló", "hablar");
        non_participle_forms.insert("hablamos", "hablar");
        non_participle_forms.insert("hablaron", "hablar");
        non_participle_forms.insert("hablo", "hablar");
        non_participle_forms.insert("hablas", "hablar");
        non_participle_forms.insert("habla", "hablar");
        non_participle_forms.insert("hablan", "hablar");
        // SALIR
        non_participle_forms.insert("salí", "salir");
        non_participle_forms.insert("saliste", "salir");
        non_participle_forms.insert("salió", "salir");
        non_participle_forms.insert("salimos", "salir");
        non_participle_forms.insert("salieron", "salir");
        non_participle_forms.insert("salgo", "salir");
        non_participle_forms.insert("sales", "salir");
        non_participle_forms.insert("sale", "salir");
        non_participle_forms.insert("salen", "salir");
        // LLEGAR
        non_participle_forms.insert("llegué", "llegar");
        non_participle_forms.insert("llegaste", "llegar");
        non_participle_forms.insert("llegó", "llegar");
        non_participle_forms.insert("llegamos", "llegar");
        non_participle_forms.insert("llegaron", "llegar");
        non_participle_forms.insert("llego", "llegar");
        non_participle_forms.insert("llegas", "llegar");
        non_participle_forms.insert("llega", "llegar");
        non_participle_forms.insert("llegan", "llegar");
        // EMPEZAR
        non_participle_forms.insert("empecé", "empezar");
        non_participle_forms.insert("empezaste", "empezar");
        non_participle_forms.insert("empezó", "empezar");
        non_participle_forms.insert("empezamos", "empezar");
        non_participle_forms.insert("empezaron", "empezar");
        non_participle_forms.insert("empiezo", "empezar");
        non_participle_forms.insert("empiezas", "empezar");
        non_participle_forms.insert("empieza", "empezar");
        non_participle_forms.insert("empiezan", "empezar");
        // TERMINAR
        non_participle_forms.insert("terminé", "terminar");
        non_participle_forms.insert("terminaste", "terminar");
        non_participle_forms.insert("terminó", "terminar");
        non_participle_forms.insert("terminamos", "terminar");
        non_participle_forms.insert("terminaron", "terminar");

        Self {
            haber_forms,
            irregular_participles,
            non_participle_forms,
        }
    }

    /// Analiza los tokens y detecta errores en tiempos compuestos
    pub fn analyze(&self, tokens: &[Token]) -> Vec<CompoundVerbCorrection> {
        let mut corrections = Vec::new();

        let word_tokens: Vec<(usize, &Token)> = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| t.token_type == TokenType::Word)
            .collect();

        // Buscar patrones: forma de haber + palabra
        for i in 0..word_tokens.len().saturating_sub(1) {
            let (idx1, token1) = word_tokens[i];
            let (idx2, token2) = word_tokens[i + 1];

            // Verificar que no hay limite de oracion entre las palabras
            if Self::has_sentence_boundary(tokens, idx1, idx2) {
                continue;
            }

            // Usar effective_text() para ver correcciones de fases anteriores
            let word1_lower = token1.effective_text().to_lowercase();
            let word2_lower = token2.effective_text().to_lowercase();

            // Verificar si el primer token es una forma de "haber"
            if !self.haber_forms.contains(word1_lower.as_str()) {
                continue;
            }

            // Verificar si el segundo token ya es un participio válido
            if self.is_valid_participle(&word2_lower) {
                continue;
            }

            // Verificar si es una forma verbal incorrecta (no participio)
            if let Some(infinitive) = self.non_participle_forms.get(word2_lower.as_str()) {
                // Obtener el participio correcto
                let participle = self.get_participle(infinitive);
                corrections.push(CompoundVerbCorrection {
                    token_index: idx2,
                    original: token2.text.clone(),
                    suggestion: participle.clone(),
                    reason: format!(
                        "Tiempo compuesto requiere participio: '{}' → '{}'",
                        token2.text, participle
                    ),
                });
                continue;
            }

            // Detectar formas regulares incorrectas
            // Ejemplo: "he cantamos" → "he cantado"
            // Pero si la palabra es un sustantivo conocido, no sugerir corrección
            // (evita falsos positivos como "había carisma" → "había carismado")
            if let Some(ref info) = token2.word_info {
                if info.category == crate::dictionary::WordCategory::Sustantivo {
                    continue;
                }
            }
            if let Some(correction) = self.check_regular_verb_error(&word2_lower, idx2, &token2.text) {
                corrections.push(correction);
            }
        }

        corrections
    }

    /// Verifica si una palabra es un participio válido
    fn is_valid_participle(&self, word: &str) -> bool {
        // Participios regulares terminan en -ado o -ido
        if word.ends_with("ado") || word.ends_with("ido") {
            return true;
        }

        // Participios con acento en -ído (traído, caído, leído, oído, etc.)
        if word.ends_with("ído") {
            return true;
        }

        // Verificar participios irregulares
        self.irregular_participles.values().any(|&p| p == word)
    }

    /// Obtiene el participio de un infinitivo
    fn get_participle(&self, infinitive: &str) -> String {
        // Primero buscar en participios irregulares
        if let Some(&participle) = self.irregular_participles.get(infinitive) {
            return participle.to_string();
        }

        // Participio regular
        if infinitive.ends_with("ar") {
            format!("{}ado", &infinitive[..infinitive.len() - 2])
        } else if infinitive.ends_with("er") || infinitive.ends_with("ir") {
            format!("{}ido", &infinitive[..infinitive.len() - 2])
        } else {
            infinitive.to_string()
        }
    }

    /// Detecta errores en verbos regulares
    fn check_regular_verb_error(
        &self,
        word: &str,
        idx: usize,
        original: &str,
    ) -> Option<CompoundVerbCorrection> {
        // Excluir palabras comunes que no son verbos pero terminan en sufijos verbales
        // "ha mucho tiempo" es válido (arcaico, "hace mucho tiempo")
        // "había que" = construcción impersonal (it was necessary to)
        // "habrá elecciones" = haber existencial (there will be elections)
        let non_verbs = [
            // Cuantificadores e indefinidos (todas las formas)
            "mucho", "mucha", "muchos", "muchas",
            "poco", "poca", "pocos", "pocas",
            "tanto", "tanta", "tantos", "tantas",
            "cuanto", "cuanta", "cuantos", "cuantas",
            "varios", "varias", "bastante", "bastantes",
            "suficiente", "suficientes", "demasiado", "demasiada", "demasiados", "demasiadas",
            "alguno", "alguna", "algunos", "algunas", "algo", "alguien",
            "ninguno", "ninguna", "ningunos", "ningunas", "nada", "nadie",
            "todo", "toda", "todos", "todas",
            "otro", "otra", "otros", "otras",
            "mismo", "misma", "mismos", "mismas",
            "cierto", "cierta", "ciertos", "ciertas",
            // Demostrativos
            "esto", "eso", "aquello", "este", "esta", "estos", "estas",
            "ese", "esa", "esos", "esas", "aquel", "aquella", "aquellos", "aquellas",
            // Números
            "uno", "una", "dos", "tres", "cuatro", "cinco",
            "seis", "siete", "ocho", "nueve", "diez",
            "primero", "primera", "segundo", "segunda",
            // Adjetivos comunes
            "bueno", "buena", "buenos", "buenas", "malo", "mala", "malos", "malas",
            "nuevo", "nueva", "nuevos", "nuevas", "viejo", "vieja", "viejos", "viejas",
            "grande", "grandes", "pequeño", "pequeña", "pequeños", "pequeñas",
            "largo", "larga", "largos", "largas", "corto", "corta", "cortos", "cortas",
            "alto", "alta", "altos", "altas", "bajo", "baja", "bajos", "bajas",
            // Palabras temporales
            "tiempo", "momento", "año", "día", "mes", "semana", "hora", "minuto",
            // Conjunciones y palabras gramaticales
            "que", "quien", "quienes", "cual", "cuales", "cuyo", "cuya",
            "donde", "cuando", "como", "porque", "aunque", "mientras",
            "si", "no", "sí", "ya", "aún", "todavía", "tampoco", "también",
            // Sustantivos comunes (haber existencial)
            "gente", "persona", "personas", "problema", "problemas", "cosa", "cosas",
            "elección", "elecciones", "cambio", "cambios", "reunión", "reuniones",
            "fiesta", "fiestas", "evento", "eventos", "accidente", "accidentes",
            "error", "errores", "duda", "dudas", "pregunta", "preguntas",
            "respuesta", "respuestas", "noticia", "noticias",
            "comida", "agua", "dinero", "trabajo", "lugar", "sitio",
            "reforma", "reformas", "guerra", "guerras", "paz", "crisis",
            "ley", "leyes", "regla", "reglas", "norma", "normas",
            "clase", "clases", "examen", "exámenes", "prueba", "pruebas",
            "señal", "señales", "aviso", "avisos", "peligro", "peligros",
        ];

        if non_verbs.contains(&word) {
            return None;
        }

        // Detectar formas conjugadas que no son participios
        // Presente: -o, -as, -a, -amos, -áis, -an (verbos -ar)
        // Presente: -o, -es, -e, -emos, -éis, -en (verbos -er/-ir)
        // Pretérito: -é, -aste, -ó, -amos, -asteis, -aron (verbos -ar)
        // Pretérito: -í, -iste, -ió, -imos, -isteis, -ieron (verbos -er/-ir)

        // Verbos -AR
        let ar_present_endings = ["o", "as", "a", "amos", "áis", "an"];
        let ar_preterite_endings = ["é", "aste", "ó", "asteis", "aron"];

        for ending in ar_present_endings.iter().chain(ar_preterite_endings.iter()) {
            if let Some(stem) = word.strip_suffix(ending) {
                if !stem.is_empty() && stem.len() >= 2 {
                    let participle = format!("{}ado", stem);
                    // Verificar que el participio tenga sentido (no crear participios de 2 letras)
                    if participle.len() >= 5 {
                        return Some(CompoundVerbCorrection {
                            token_index: idx,
                            original: original.to_string(),
                            suggestion: participle.clone(),
                            reason: format!(
                                "Tiempo compuesto requiere participio: '{}' → '{}'",
                                original, participle
                            ),
                        });
                    }
                }
            }
        }

        // Verbos -ER/-IR
        let er_ir_present_endings = ["o", "es", "e", "emos", "éis", "en", "imos", "ís"];
        let er_ir_preterite_endings = ["í", "iste", "ió", "isteis", "ieron"];

        for ending in er_ir_present_endings.iter().chain(er_ir_preterite_endings.iter()) {
            if let Some(stem) = word.strip_suffix(ending) {
                if !stem.is_empty() && stem.len() >= 2 {
                    let participle = format!("{}ido", stem);
                    if participle.len() >= 5 {
                        return Some(CompoundVerbCorrection {
                            token_index: idx,
                            original: original.to_string(),
                            suggestion: participle.clone(),
                            reason: format!(
                                "Tiempo compuesto requiere participio: '{}' → '{}'",
                                original, participle
                            ),
                        });
                    }
                }
            }
        }

        None
    }

    /// Verifica si hay un limite de oracion entre dos indices de tokens
    fn has_sentence_boundary(tokens: &[Token], start_idx: usize, end_idx: usize) -> bool {
        for idx in (start_idx + 1)..end_idx {
            if idx < tokens.len() {
                let token = &tokens[idx];
                if token.token_type == TokenType::Punctuation {
                    let text = token.text.as_str();
                    if matches!(text, "." | "!" | "?" | ";" | ":" | "\"" | "\u{201D}" | "\u{BB}") {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Default for CompoundVerbAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Tokenizer;

    fn analyze_text(text: &str) -> Vec<CompoundVerbCorrection> {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(text);
        let analyzer = CompoundVerbAnalyzer::new();
        analyzer.analyze(&tokens)
    }

    // Tests para errores con verbos irregulares

    #[test]
    fn test_he_comido_correcto() {
        let corrections = analyze_text("he comido");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_ha_ido_correcto() {
        let corrections = analyze_text("ha ido");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_hemos_hecho_correcto() {
        let corrections = analyze_text("hemos hecho");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_habia_dicho_correcto() {
        let corrections = analyze_text("había dicho");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_he_fue_incorrecto() {
        let corrections = analyze_text("he fue");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "fue");
        assert_eq!(corrections[0].suggestion, "ido");
    }

    #[test]
    fn test_ha_hizo_incorrecto() {
        let corrections = analyze_text("ha hizo");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "hizo");
        assert_eq!(corrections[0].suggestion, "hecho");
    }

    #[test]
    fn test_habia_dijo_incorrecto() {
        let corrections = analyze_text("había dijo");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "dijo");
        assert_eq!(corrections[0].suggestion, "dicho");
    }

    #[test]
    fn test_hemos_vio_incorrecto() {
        let corrections = analyze_text("hemos vio");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "vio");
        assert_eq!(corrections[0].suggestion, "visto");
    }

    #[test]
    fn test_han_puso_incorrecto() {
        let corrections = analyze_text("han puso");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "puso");
        assert_eq!(corrections[0].suggestion, "puesto");
    }

    #[test]
    fn test_ha_vino_incorrecto() {
        let corrections = analyze_text("ha vino");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "vino");
        assert_eq!(corrections[0].suggestion, "venido");
    }

    // Tests para errores con verbos regulares

    #[test]
    fn test_he_cantado_correcto() {
        let corrections = analyze_text("he cantado");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_hemos_comido_correcto() {
        let corrections = analyze_text("hemos comido");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_han_vivido_correcto() {
        let corrections = analyze_text("han vivido");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_he_canto_incorrecto() {
        let corrections = analyze_text("he canto");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "canto");
        assert_eq!(corrections[0].suggestion, "cantado");
    }

    #[test]
    fn test_ha_comio_incorrecto() {
        let corrections = analyze_text("ha comió");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "comió");
        assert_eq!(corrections[0].suggestion, "comido");
    }

    #[test]
    fn test_hemos_cantamos_incorrecto() {
        let corrections = analyze_text("hemos cantamos");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].original, "cantamos");
        assert_eq!(corrections[0].suggestion, "cantado");
    }

    // Tests con diferentes formas de haber

    #[test]
    fn test_habia_fue_incorrecto() {
        let corrections = analyze_text("había fue");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ido");
    }

    #[test]
    fn test_hayan_hecho_correcto() {
        let corrections = analyze_text("hayan hecho");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_hubiera_visto_correcto() {
        let corrections = analyze_text("hubiera visto");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_habria_sido_correcto() {
        let corrections = analyze_text("habría sido");
        assert!(corrections.is_empty());
    }

    // Tests para participios irregulares compuestos

    #[test]
    fn test_ha_deshecho_correcto() {
        let corrections = analyze_text("ha deshecho");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_ha_devuelto_correcto() {
        let corrections = analyze_text("ha devuelto");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_ha_descubierto_correcto() {
        let corrections = analyze_text("ha descubierto");
        assert!(corrections.is_empty());
    }

    // Tests negativos - frases que NO deben corregirse

    #[test]
    fn test_no_corrige_sin_haber() {
        let corrections = analyze_text("fue a casa");
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_no_corrige_hay_impersonal() {
        // "hay" es forma de haber pero impersonal, no seguida de participio normalmente
        // Este caso es especial y no debería detectarse como error
        let corrections = analyze_text("hay comida");
        // "comida" podría parecer participio femenino, pero está bien
        assert!(corrections.is_empty());
    }

    #[test]
    fn test_oracion_completa() {
        let corrections = analyze_text("Yo he fue al mercado");
        assert_eq!(corrections.len(), 1);
        assert_eq!(corrections[0].suggestion, "ido");
    }

    // Test de limite de oracion
    #[test]
    fn test_sentence_boundary_no_false_positive() {
        // "he" y "fue" estan separados por punto, no debe detectar error de tiempo compuesto
        let corrections = analyze_text("Lo he. Fue ayer cuando paso");
        let compound_corrections: Vec<_> = corrections.iter()
            .filter(|c| c.suggestion == "ido")
            .collect();
        assert!(compound_corrections.is_empty(), "No debe detectar error de tiempo compuesto cuando hay limite de oracion");
    }
}
