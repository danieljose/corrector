//! Implementación del idioma español

pub mod capitalization;
pub mod common_gender;
pub mod compound;
pub mod conjugation;
pub mod dequeismo;
pub mod diacritics;
pub mod exceptions;
pub mod homophone;
pub mod impersonal;
pub mod names_gender;
pub mod pipeline;
pub mod pleonasm;
pub mod plurals;
pub mod pronoun;
pub mod punctuation;
pub mod relative;
pub mod rules;
pub mod subject_verb;
pub mod vocative;

pub use capitalization::CapitalizationAnalyzer;
pub use common_gender::CommonGenderAnalyzer;
pub use compound::CompoundVerbAnalyzer;
pub use conjugation::VerbRecognizer;
pub use dequeismo::DequeismoAnalyzer;
pub use diacritics::DiacriticAnalyzer;
pub use homophone::HomophoneAnalyzer;
pub use impersonal::ImpersonalAnalyzer;
pub use names_gender::get_name_gender;
pub use pleonasm::PleonasmAnalyzer;
pub use pronoun::PronounAnalyzer;
pub use punctuation::PunctuationAnalyzer;
pub use relative::RelativeAnalyzer;
pub use subject_verb::SubjectVerbAnalyzer;
pub use vocative::VocativeAnalyzer;

use crate::dictionary::{Gender, Number, ProperNames, Trie};
use crate::grammar::{GrammarRule, Token};
use crate::languages::{Language, VerbFormRecognizer};

pub struct Spanish {
    exceptions: std::collections::HashSet<String>,
}

impl Spanish {
    pub fn new() -> Self {
        Self {
            exceptions: exceptions::get_exceptions(),
        }
    }
}

impl Language for Spanish {
    fn code(&self) -> &str {
        "es"
    }

    fn name(&self) -> &str {
        "Español"
    }

    fn grammar_rules(&self) -> Vec<GrammarRule> {
        rules::get_spanish_rules()
    }

    fn configure_dictionary(&self, dictionary: &mut Trie) {
        dictionary.set_depluralize_fn(plurals::depluralize_candidates);
    }

    fn build_verb_recognizer(&self, dictionary: &Trie) -> Option<Box<dyn VerbFormRecognizer>> {
        Some(Box::new(VerbRecognizer::from_dictionary(dictionary)))
    }

    fn apply_language_specific_corrections(
        &self,
        tokens: &mut [Token],
        dictionary: &Trie,
        proper_names: &ProperNames,
        verb_recognizer: Option<&dyn VerbFormRecognizer>,
    ) {
        pipeline::apply_spanish_corrections(tokens, dictionary, proper_names, verb_recognizer);
    }

    fn check_gender_agreement(&self, token1: &Token, token2: &Token) -> bool {
        match (&token1.word_info, &token2.word_info) {
            (Some(info1), Some(info2)) => {
                // Si alguno no tiene género definido, asumimos concordancia
                if info1.gender == Gender::None || info2.gender == Gender::None {
                    return true;
                }

                // Sustantivos de género ambiguo por significado (ej. cólera):
                // aceptar "el/un" y "la/una" para evitar falsas correcciones de artículo.
                if exceptions::allows_both_gender_articles(&token2.text) {
                    let article_lower = token1.text.to_lowercase();
                    if matches!(
                        article_lower.as_str(),
                        "el" | "la" | "un" | "una" | "los" | "las" | "unos" | "unas"
                    ) {
                        return true;
                    }
                }

                // Excepción especial: sustantivos femeninos que usan "el" (agua, alma, etc.)
                if exceptions::uses_el_with_feminine(&token2.text) {
                    let article_lower = token1.text.to_lowercase();
                    // "el/un" + agua = correcto
                    if article_lower == "el" || article_lower == "un" {
                        return true;
                    }
                    // "la/una" + agua = incorrecto (debe ser "el/un")
                    if article_lower == "la" || article_lower == "una" {
                        return false;
                    }
                }

                info1.gender == info2.gender
            }
            _ => true, // Sin información, asumimos correcto
        }
    }

    fn check_number_agreement(&self, token1: &Token, token2: &Token) -> bool {
        // Sustantivos invariables (virus, crisis, análisis, etc.) no generan error de número
        // En concordancia art-sust: token1=artículo, token2=sustantivo
        // En concordancia sust-adj: token1=sustantivo, token2=adjetivo
        // Verificamos ambos por si acaso
        if exceptions::is_invariable_noun(&token1.text)
            || exceptions::is_invariable_noun(&token2.text)
        {
            return true;
        }

        match (&token1.word_info, &token2.word_info) {
            (Some(info1), Some(info2)) => {
                if info1.number == Number::None || info2.number == Number::None {
                    return true;
                }
                info1.number == info2.number
            }
            _ => true,
        }
    }

    fn get_correct_article(&self, gender: Gender, number: Number, definite: bool) -> &str {
        match (definite, gender, number) {
            (true, Gender::Masculine, Number::Singular) => "el",
            (true, Gender::Masculine, Number::Plural) => "los",
            (true, Gender::Feminine, Number::Singular) => "la",
            (true, Gender::Feminine, Number::Plural) => "las",
            (false, Gender::Masculine, Number::Singular) => "un",
            (false, Gender::Masculine, Number::Plural) => "unos",
            (false, Gender::Feminine, Number::Singular) => "una",
            (false, Gender::Feminine, Number::Plural) => "unas",
            _ => "",
        }
    }

    fn get_correct_article_for_noun(
        &self,
        noun: &str,
        gender: Gender,
        number: Number,
        definite: bool,
    ) -> String {
        // Excepción: sustantivos femeninos con "a" tónica usan "el/un" en singular
        // Ejemplos: el agua, el águila, el alma, un hacha, un hada
        if gender == Gender::Feminine
            && number == Number::Singular
            && exceptions::uses_el_with_feminine(noun)
        {
            return if definite {
                "el".to_string()
            } else {
                "un".to_string()
            };
        }

        // Caso normal
        self.get_correct_article(gender, number, definite)
            .to_string()
    }

    fn get_adjective_form(
        &self,
        adjective: &str,
        gender: Gender,
        number: Number,
    ) -> Option<String> {
        let adj_lower = adjective.to_lowercase();

        // Detectar tipo de adjetivo por su terminación
        let last_char = adj_lower.chars().last()?;

        // Adjetivos invariables en género (terminan en -e, -es, o consonante)
        // Solo cambian en número: interesante/interesantes, amable/amables, fácil/fáciles
        // También: adicional/adicionales, especial/especiales
        let is_invariable_gender = last_char == 'e'
            || !matches!(last_char, 'a' | 'o' | 's')
            || (adj_lower.ends_with("es")
                && !adj_lower.ends_with("os")
                && !adj_lower.ends_with("as")
                && !adj_lower.ends_with("eses")); // Excluir casos como "intereses"

        if is_invariable_gender {
            // Obtener base singular del adjetivo
            // "interesantes" -> "interesante" (quitar solo 's')
            // "adicionales" -> "adicional" (quitar 'es')
            // "fáciles" -> "fácil" (quitar 'es')
            // "capaces" -> "capaz" (cambio ortográfico c->z)
            let base = if adj_lower.ends_with("ces") {
                // Cambio ortográfico: capaces -> capaz, felices -> feliz
                let without_ces = &adj_lower[..adj_lower.len() - 3];
                format!("{}z", without_ces)
            } else if adj_lower.ends_with("es") {
                let without_es = &adj_lower[..adj_lower.len() - 2];
                // Si la raíz sin "es" termina en vocal, el singular debería terminar en 'e'
                // Ejemplo: "interesant" + "e" = "interesante"
                let last = without_es.chars().last();
                if last
                    .map(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
                    .unwrap_or(false)
                {
                    // La raíz termina en vocal, añadir 'e' para el singular
                    format!("{}e", without_es)
                } else {
                    // La raíz termina en consonante, usar tal cual
                    without_es.to_string()
                }
            } else if adj_lower.ends_with('s') {
                adj_lower.trim_end_matches('s').to_string()
            } else {
                adj_lower.clone()
            };

            // Estos adjetivos no cambian de género, solo de número
            return match number {
                Number::Singular => Some(base),
                Number::Plural => {
                    // Añadir 's' si termina en vocal, 'es' si termina en consonante
                    // Cambio ortográfico z->c antes de 'es': capaz -> capaces
                    if base.ends_with('z') {
                        let without_z = &base[..base.len() - 1];
                        Some(format!("{}ces", without_z))
                    } else {
                        let last = base.chars().last();
                        if last
                            .map(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
                            .unwrap_or(false)
                        {
                            Some(format!("{}s", base))
                        } else {
                            Some(format!("{}es", base))
                        }
                    }
                }
                _ => None,
            };
        }

        // Adjetivos regulares que cambian en género y número (bueno/buena/buenos/buenas)
        // Quitar terminación o/a/os/as para obtener la base
        let base = if adj_lower.ends_with("os") || adj_lower.ends_with("as") {
            &adj_lower[..adj_lower.len() - 2]
        } else if adj_lower.ends_with('o') || adj_lower.ends_with('a') {
            &adj_lower[..adj_lower.len() - 1]
        } else {
            &adj_lower
        };

        let suffix = match (gender, number) {
            (Gender::Masculine, Number::Singular) => "o",
            (Gender::Masculine, Number::Plural) => "os",
            (Gender::Feminine, Number::Singular) => "a",
            (Gender::Feminine, Number::Plural) => "as",
            _ => return None,
        };

        Some(format!("{}{}", base, suffix))
    }

    fn get_correct_determiner(
        &self,
        determiner: &str,
        gender: Gender,
        number: Number,
    ) -> Option<String> {
        let det_lower = determiner.to_lowercase();

        // Identificar el tipo de determinante y devolver la forma correcta

        // Determinantes demostrativos - este/esta/estos/estas
        if det_lower == "este"
            || det_lower == "esta"
            || det_lower == "estos"
            || det_lower == "estas"
        {
            return Some(
                match (gender, number) {
                    (Gender::Masculine, Number::Singular) => "este",
                    (Gender::Feminine, Number::Singular) => "esta",
                    (Gender::Masculine, Number::Plural) => "estos",
                    (Gender::Feminine, Number::Plural) => "estas",
                    _ => return None,
                }
                .to_string(),
            );
        }

        // Determinantes demostrativos - ese/esa/esos/esas
        if det_lower == "ese" || det_lower == "esa" || det_lower == "esos" || det_lower == "esas" {
            return Some(
                match (gender, number) {
                    (Gender::Masculine, Number::Singular) => "ese",
                    (Gender::Feminine, Number::Singular) => "esa",
                    (Gender::Masculine, Number::Plural) => "esos",
                    (Gender::Feminine, Number::Plural) => "esas",
                    _ => return None,
                }
                .to_string(),
            );
        }

        // Determinantes demostrativos - aquel/aquella/aquellos/aquellas
        if det_lower == "aquel"
            || det_lower == "aquella"
            || det_lower == "aquellos"
            || det_lower == "aquellas"
        {
            return Some(
                match (gender, number) {
                    (Gender::Masculine, Number::Singular) => "aquel",
                    (Gender::Feminine, Number::Singular) => "aquella",
                    (Gender::Masculine, Number::Plural) => "aquellos",
                    (Gender::Feminine, Number::Plural) => "aquellas",
                    _ => return None,
                }
                .to_string(),
            );
        }

        // Determinantes posesivos - nuestro/nuestra/nuestros/nuestras
        if det_lower == "nuestro"
            || det_lower == "nuestra"
            || det_lower == "nuestros"
            || det_lower == "nuestras"
        {
            return Some(
                match (gender, number) {
                    (Gender::Masculine, Number::Singular) => "nuestro",
                    (Gender::Feminine, Number::Singular) => "nuestra",
                    (Gender::Masculine, Number::Plural) => "nuestros",
                    (Gender::Feminine, Number::Plural) => "nuestras",
                    _ => return None,
                }
                .to_string(),
            );
        }

        // Determinantes posesivos - vuestro/vuestra/vuestros/vuestras
        if det_lower == "vuestro"
            || det_lower == "vuestra"
            || det_lower == "vuestros"
            || det_lower == "vuestras"
        {
            return Some(
                match (gender, number) {
                    (Gender::Masculine, Number::Singular) => "vuestro",
                    (Gender::Feminine, Number::Singular) => "vuestra",
                    (Gender::Masculine, Number::Plural) => "vuestros",
                    (Gender::Feminine, Number::Plural) => "vuestras",
                    _ => return None,
                }
                .to_string(),
            );
        }

        // Determinantes invariables en género (mi, tu, su, etc.) - no se corrigen aquí
        None
    }

    fn is_exception(&self, word: &str) -> bool {
        self.exceptions.contains(&word.to_lowercase())
    }

    fn is_likely_verb_form_in_context(&self, word: &str, tokens: &[Token], index: usize) -> bool {
        Self::is_likely_verb_form_no_dict(word) && Self::is_verbal_context(tokens, index)
    }

    fn is_known_abbreviation(&self, word: &str) -> bool {
        let w = word.to_lowercase();
        w == "n.º" || w == "n.ª"
    }

    fn article_features(&self, article: &str) -> Option<(bool, Number, Gender)> {
        match article {
            "el" => Some((true, Number::Singular, Gender::Masculine)),
            "la" => Some((true, Number::Singular, Gender::Feminine)),
            "los" => Some((true, Number::Plural, Gender::Masculine)),
            "las" => Some((true, Number::Plural, Gender::Feminine)),
            "un" => Some((false, Number::Singular, Gender::Masculine)),
            "una" => Some((false, Number::Singular, Gender::Feminine)),
            "unos" => Some((false, Number::Plural, Gender::Masculine)),
            "unas" => Some((false, Number::Plural, Gender::Feminine)),
            _ => None,
        }
    }

    fn determiner_features(&self, determiner: &str) -> Option<(&str, Number, Gender)> {
        match determiner {
            "el" => Some(("art_def", Number::Singular, Gender::Masculine)),
            "la" => Some(("art_def", Number::Singular, Gender::Feminine)),
            "los" => Some(("art_def", Number::Plural, Gender::Masculine)),
            "las" => Some(("art_def", Number::Plural, Gender::Feminine)),
            "un" => Some(("art_indef", Number::Singular, Gender::Masculine)),
            "una" => Some(("art_indef", Number::Singular, Gender::Feminine)),
            "unos" => Some(("art_indef", Number::Plural, Gender::Masculine)),
            "unas" => Some(("art_indef", Number::Plural, Gender::Feminine)),
            "este" => Some(("dem_este", Number::Singular, Gender::Masculine)),
            "esta" => Some(("dem_este", Number::Singular, Gender::Feminine)),
            "estos" => Some(("dem_este", Number::Plural, Gender::Masculine)),
            "estas" => Some(("dem_este", Number::Plural, Gender::Feminine)),
            "ese" => Some(("dem_ese", Number::Singular, Gender::Masculine)),
            "esa" => Some(("dem_ese", Number::Singular, Gender::Feminine)),
            "esos" => Some(("dem_ese", Number::Plural, Gender::Masculine)),
            "esas" => Some(("dem_ese", Number::Plural, Gender::Feminine)),
            "aquel" => Some(("dem_aquel", Number::Singular, Gender::Masculine)),
            "aquella" => Some(("dem_aquel", Number::Singular, Gender::Feminine)),
            "aquellos" => Some(("dem_aquel", Number::Plural, Gender::Masculine)),
            "aquellas" => Some(("dem_aquel", Number::Plural, Gender::Feminine)),
            "nuestro" => Some(("pos_nuestro", Number::Singular, Gender::Masculine)),
            "nuestra" => Some(("pos_nuestro", Number::Singular, Gender::Feminine)),
            "nuestros" => Some(("pos_nuestro", Number::Plural, Gender::Masculine)),
            "nuestras" => Some(("pos_nuestro", Number::Plural, Gender::Feminine)),
            "vuestro" => Some(("pos_vuestro", Number::Singular, Gender::Masculine)),
            "vuestra" => Some(("pos_vuestro", Number::Singular, Gender::Feminine)),
            "vuestros" => Some(("pos_vuestro", Number::Plural, Gender::Masculine)),
            "vuestras" => Some(("pos_vuestro", Number::Plural, Gender::Feminine)),
            _ => None,
        }
    }

    fn is_preposition(&self, word: &str) -> bool {
        matches!(
            word,
            "de" | "del"
                | "con"
                | "contra"
                | "sobre"
                | "sin"
                | "entre"
                | "para"
                | "por"
                | "bajo"
                | "ante"
                | "tras"
                | "hacia"
                | "hasta"
                | "desde"
                | "durante"
                | "mediante"
                | "según"
                | "segun"
                | "en"
        )
    }

    fn is_participle_form(&self, word: &str) -> bool {
        // Participios regulares
        if word.ends_with("ado")
            || word.ends_with("ada")
            || word.ends_with("ados")
            || word.ends_with("adas")
            || word.ends_with("ido")
            || word.ends_with("ida")
            || word.ends_with("idos")
            || word.ends_with("idas")
        {
            return true;
        }

        // Participios con tilde (verbos en -aer, -eer, -oír, -eír)
        if word.ends_with("ído")
            || word.ends_with("ída")
            || word.ends_with("ídos")
            || word.ends_with("ídas")
        {
            return true;
        }

        // Participios irregulares (-to, -cho, -so con variaciones de género/número)
        if word.ends_with("to")
            || word.ends_with("ta")
            || word.ends_with("tos")
            || word.ends_with("tas")
            || word.ends_with("cho")
            || word.ends_with("cha")
            || word.ends_with("chos")
            || word.ends_with("chas")
            || word.ends_with("so")
            || word.ends_with("sa")
            || word.ends_with("sos")
            || word.ends_with("sas")
        {
            let irregular_participle_stems = [
                "escrit",
                "abiert",
                "rot",
                "muert",
                "puest",
                "vist",
                "vuelt",
                "cubiert",
                "descubiert",
                "devuelt",
                "envuelt",
                "resuelv",
                "resuelt",
                "disuelv",
                "disuelt",
                "revuelt",
                "compuest",
                "dispuest",
                "expuest",
                "impuest",
                "opuest",
                "propuest",
                "supuest",
                "frit",
                "inscrit",
                "proscrit",
                "suscrit",
                "descript",
                "prescrit",
                "hech",
                "dich",
                "satisfech",
                "contradicho",
                "maldich",
                "bendich",
                "impres",
                "confes",
                "expres",
                "compres",
                "supres",
            ];

            for stem in irregular_participle_stems {
                if word.starts_with(stem) {
                    return true;
                }
            }
        }

        false
    }

    fn is_common_gender_noun_form(&self, noun: &str) -> bool {
        let noun_lower = noun.to_lowercase();
        if exceptions::is_common_gender_noun(&noun_lower) {
            return true;
        }
        if let Some(stem) = noun_lower.strip_suffix("es") {
            if exceptions::is_common_gender_noun(stem) {
                return true;
            }
        }
        if let Some(stem) = noun_lower.strip_suffix('s') {
            if exceptions::is_common_gender_noun(stem) {
                return true;
            }
        }
        false
    }

    fn allows_both_gender_articles(&self, word: &str) -> bool {
        exceptions::allows_both_gender_articles(word)
    }

    fn is_conjunction(&self, word: &str) -> bool {
        matches!(word, "y" | "e" | "o" | "u" | "ni")
    }

    fn is_time_noun(&self, word: &str) -> bool {
        matches!(
            word,
            "segundo"
                | "segundos"
                | "minuto"
                | "minutos"
                | "hora"
                | "horas"
                | "día"
                | "días"
                | "semana"
                | "semanas"
                | "mes"
                | "meses"
                | "año"
                | "años"
                | "rato"
                | "momento"
                | "instante"
        )
    }

    fn is_predicative_adjective(&self, word: &str) -> bool {
        let w = word.to_lowercase();
        matches!(
            w.as_str(),
            "juntos"
                | "juntas"
                | "junto"
                | "junta"
                | "solos"
                | "solas"
                | "solo"
                | "sola"
                | "presentes"
                | "presente"
                | "ausentes"
                | "ausente"
                | "contentos"
                | "contentas"
                | "contento"
                | "contenta"
                | "satisfechos"
                | "satisfechas"
                | "satisfecho"
                | "satisfecha"
                | "dispuestos"
                | "dispuestas"
                | "dispuesto"
                | "dispuesta"
                | "seguros"
                | "seguras"
                | "seguro"
                | "segura"
                | "listos"
                | "listas"
                | "listo"
                | "lista"
                | "muertos"
                | "muertas"
                | "muerto"
                | "muerta"
                | "vivos"
                | "vivas"
                | "vivo"
                | "viva"
                | "sometidos"
                | "sometidas"
                | "sometido"
                | "sometida"
                | "expuestos"
                | "expuestas"
                | "expuesto"
                | "expuesta"
                | "obligados"
                | "obligadas"
                | "obligado"
                | "obligada"
                | "destinados"
                | "destinadas"
                | "destinado"
                | "destinada"
                | "condenados"
                | "condenadas"
                | "condenado"
                | "condenada"
                | "llamados"
                | "llamadas"
                | "llamado"
                | "llamada"
                | "considerados"
                | "consideradas"
                | "considerado"
                | "considerada"
                | "recogidos"
                | "recogidas"
                | "recogido"
                | "recogida"
                | "publicados"
                | "publicadas"
                | "publicado"
                | "publicada"
                | "citados"
                | "citadas"
                | "citado"
                | "citada"
                | "mencionados"
                | "mencionadas"
                | "mencionado"
                | "mencionada"
                | "debido"
                | "gracias"
                | "apoyados"
                | "apoyadas"
                | "apoyado"
                | "apoyada"
                | "impulsados"
                | "impulsadas"
                | "impulsado"
                | "impulsada"
                | "afectados"
                | "afectadas"
                | "afectado"
                | "afectada"
                | "motivados"
                | "motivadas"
                | "motivado"
                | "motivada"
                | "acompañados"
                | "acompañadas"
                | "acompañado"
                | "acompañada"
                | "seguidos"
                | "seguidas"
                | "seguido"
                | "seguida"
                | "precedidos"
                | "precedidas"
                | "precedido"
                | "precedida"
                | "liderados"
                | "lideradas"
                | "liderado"
                | "liderada"
                | "encabezados"
                | "encabezadas"
                | "encabezado"
                | "encabezada"
                | "respaldados"
                | "respaldadas"
                | "respaldado"
                | "respaldada"
                | "marcados"
                | "marcadas"
                | "marcado"
                | "marcada"
                | "caracterizados"
                | "caracterizadas"
                | "caracterizado"
                | "caracterizada"
                | "cubiertos"
                | "cubiertas"
                | "cubierto"
                | "cubierta"
                | "incluidos"
                | "incluidas"
                | "incluido"
                | "incluida"
                | "excluidos"
                | "excluidas"
                | "excluido"
                | "excluida"
                | "protegidos"
                | "protegidas"
                | "protegido"
                | "protegida"
                | "relacionados"
                | "relacionadas"
                | "relacionado"
                | "relacionada"
                | "situados"
                | "situadas"
                | "situado"
                | "situada"
                | "ubicados"
                | "ubicadas"
                | "ubicado"
                | "ubicada"
                | "ingresados"
                | "ingresadas"
                | "ingresado"
                | "ingresada"
                | "internados"
                | "internadas"
                | "internado"
                | "internada"
                | "hospitalizado"
                | "hospitalizada"
                | "hospitalizados"
                | "hospitalizadas"
                | "conectados"
                | "conectadas"
                | "conectado"
                | "conectada"
                | "dormidos"
                | "dormidas"
                | "dormido"
                | "dormida"
                | "despiertos"
                | "despiertas"
                | "despierto"
                | "despierta"
                | "sentados"
                | "sentadas"
                | "sentado"
                | "sentada"
                | "parados"
                | "paradas"
                | "parado"
                | "parada"
                | "acostados"
                | "acostadas"
                | "acostado"
                | "acostada"
                | "absorbidos"
                | "absorbidas"
                | "absorbido"
                | "absorbida"
                | "reclamados"
                | "reclamadas"
                | "reclamado"
                | "reclamada"
                | "asociados"
                | "asociadas"
                | "asociado"
                | "asociada"
                | "completados"
                | "completadas"
                | "completado"
                | "completada"
                | "terminados"
                | "terminadas"
                | "terminado"
                | "terminada"
                | "finalizados"
                | "finalizadas"
                | "finalizado"
                | "finalizada"
                | "aprobados"
                | "aprobadas"
                | "aprobado"
                | "aprobada"
                | "confirmados"
                | "confirmadas"
                | "confirmado"
                | "confirmada"
                | "verificados"
                | "verificadas"
                | "verificado"
                | "verificada"
                | "validados"
                | "validadas"
                | "validado"
                | "validada"
                | "aceptados"
                | "aceptadas"
                | "aceptado"
                | "aceptada"
                | "rechazados"
                | "rechazadas"
                | "rechazado"
                | "rechazada"
        )
    }
}

impl Spanish {
    /// Heurística: ¿la palabra parece forma verbal por sus terminaciones?
    /// (fallback cuando el infinitivo no está en diccionario)
    fn is_likely_verb_form_no_dict(word: &str) -> bool {
        let word_lower = word.to_lowercase();
        let len = word_lower.len();

        // Mínimo 5 caracteres para evitar falsos positivos
        if len < 5 {
            return false;
        }

        // Terminaciones muy específicas de verbos (ordenadas por longitud descendente)

        // 5+ caracteres
        if word_lower.ends_with("ieron") // comieron, vivieron
            || word_lower.ends_with("arían") // hablarían
            || word_lower.ends_with("erían") // comerían
            || word_lower.ends_with("irían") // vivirían
            || word_lower.ends_with("ieran") // comieran
            || word_lower.ends_with("iesen") // comiesen
            || word_lower.ends_with("iendo")
        // comiendo (gerundio)
        {
            return true;
        }

        // 4 caracteres
        if word_lower.ends_with("aron") // hablaron
            || word_lower.ends_with("aban") // hablaban
            || word_lower.ends_with("ando") // hablando (gerundio)
            || word_lower.ends_with("aste") // hablaste
            || word_lower.ends_with("iste") // comiste
            || word_lower.ends_with("amos") // hablamos (cuidado: sustantivos como "ramos")
            || word_lower.ends_with("emos") // comemos
            || word_lower.ends_with("imos") // vivimos
            || word_lower.ends_with("arán") // hablarán
            || word_lower.ends_with("erán") // comerán
            || word_lower.ends_with("irán") // vivirán
            || word_lower.ends_with("aran") // hablaran
            || word_lower.ends_with("asen") // hablasen
            || word_lower.ends_with("aría") // hablaría
            || word_lower.ends_with("ería") // comería
            || word_lower.ends_with("iría") // viviría
            || word_lower.ends_with("iera") // comiera
            || word_lower.ends_with("iese")
        // comiese
        {
            // Excluir palabras conocidas que no son verbos
            let non_verbs = [
                "abecedario",
                "acuario",
                "calendario",
                "canario",
                "diario",
                "escenario",
                "horario",
                "salario",
                "vocabulario",
                "matadero",
                "panadero",
                "soltero",
            ];
            if non_verbs.iter().any(|&nv| word_lower == nv) {
                return false;
            }
            return true;
        }

        // 3 caracteres - muy conservador
        if word_lower.ends_with("ían") && len >= 6 {
            // comían, vivían
            return true;
        }

        false
    }

    /// Verifica si el contexto indica que la palabra es probablemente un verbo
    fn is_verbal_context(tokens: &[Token], current_idx: usize) -> bool {
        use crate::grammar::tokenizer::TokenType;

        // Buscar palabra anterior (saltando whitespace)
        let mut prev_word_idx = None;
        for i in (0..current_idx).rev() {
            if tokens[i].token_type == TokenType::Word {
                prev_word_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = prev_word_idx {
            let prev = tokens[idx].text.to_lowercase();

            // Auxiliar "haber" (tiempos compuestos)
            let haber_forms = [
                "he",
                "has",
                "ha",
                "hemos",
                "habéis",
                "habeis",
                "han",
                "había",
                "habias",
                "habías",
                "habíamos",
                "habiamos",
                "habíais",
                "habiais",
                "habían",
                "habian",
                "hube",
                "hubiste",
                "hubo",
                "hubimos",
                "hubisteis",
                "hubieron",
                "habré",
                "habre",
                "habrás",
                "habras",
                "habrá",
                "habra",
                "habremos",
                "habréis",
                "habreis",
                "habrán",
                "habran",
                "habría",
                "habria",
                "habrías",
                "habrias",
                "habríamos",
                "habriamos",
                "habríais",
                "habriais",
                "habrían",
                "habrian",
                "haya",
                "hayas",
                "hayamos",
                "hayáis",
                "hayais",
                "hayan",
                "hubiera",
                "hubieras",
                "hubiéramos",
                "hubieramos",
                "hubierais",
                "hubieran",
                "hubiese",
                "hubieses",
                "hubiésemos",
                "hubiesemos",
                "hubieseis",
                "hubiesen",
            ];
            if haber_forms.contains(&prev.as_str()) {
                return true;
            }

            // Pronombres sujeto
            let subject_pronouns = [
                "yo", "tú", "él", "ella", "usted", "nosotros", "nosotras", "vosotros", "vosotras",
                "ellos", "ellas", "ustedes",
            ];
            if subject_pronouns.contains(&prev.as_str()) {
                return true;
            }

            // Relativos e interrogativos que introducen cláusulas verbales
            let verbal_introducers = ["que", "quien", "quienes", "donde", "cuando", "como"];
            if verbal_introducers.contains(&prev.as_str()) {
                return true;
            }

            // Pronombres reflexivos/objeto que preceden verbos
            let object_pronouns = [
                "se", "me", "te", "nos", "os", "le", "les", "lo", "la", "los", "las",
            ];
            if object_pronouns.contains(&prev.as_str()) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::WordInfo;
    use crate::grammar::tokenizer::TokenType;

    fn token_with_info(text: &str, gender: Gender, number: Number) -> Token {
        let mut token = Token::new(text.to_string(), TokenType::Word, 0, text.len());
        token.word_info = Some(WordInfo {
            gender,
            number,
            ..WordInfo::default()
        });
        token
    }

    #[test]
    fn test_get_correct_determiner_este_to_esta() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("este", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("esta".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_esta_to_este() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("esta", Gender::Masculine, Number::Singular);
        assert_eq!(result, Some("este".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_este_to_estos() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("este", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("estos".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_este_to_estas() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("este", Gender::Feminine, Number::Plural);
        assert_eq!(result, Some("estas".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_ese_to_esa() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("ese", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("esa".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_esa_to_esos() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("esa", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("esos".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_aquel_to_aquella() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("aquel", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("aquella".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_aquella_to_aquellos() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("aquella", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("aquellos".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_nuestro_to_nuestra() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("nuestro", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("nuestra".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_nuestra_to_nuestros() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("nuestra", Gender::Masculine, Number::Plural);
        assert_eq!(result, Some("nuestros".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_vuestro_to_vuestra() {
        let spanish = Spanish::new();
        let result = spanish.get_correct_determiner("vuestro", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("vuestra".to_string()));
    }

    #[test]
    fn test_get_correct_determiner_invariable_returns_none() {
        let spanish = Spanish::new();
        // Determinantes invariables como "mi", "tu", "su" no se corrigen
        let result = spanish.get_correct_determiner("mi", Gender::Feminine, Number::Singular);
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_correct_determiner_preserves_when_correct() {
        let spanish = Spanish::new();
        // "esta" con género femenino singular debería devolver "esta"
        let result = spanish.get_correct_determiner("esta", Gender::Feminine, Number::Singular);
        assert_eq!(result, Some("esta".to_string()));
    }

    #[test]
    fn test_check_gender_agreement_accepts_el_colera() {
        let spanish = Spanish::new();
        let article = token_with_info("El", Gender::Masculine, Number::Singular);
        let noun = token_with_info("c\u{00f3}lera", Gender::Feminine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_colera() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("c\u{00f3}lera", Gender::Feminine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_el_cometa() {
        let spanish = Spanish::new();
        let article = token_with_info("El", Gender::Masculine, Number::Singular);
        let noun = token_with_info("cometa", Gender::Feminine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_cometa() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("cometa", Gender::Masculine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_los_capitales() {
        let spanish = Spanish::new();
        let article = token_with_info("Los", Gender::Masculine, Number::Plural);
        let noun = token_with_info("capitales", Gender::Feminine, Number::Plural);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_las_ordenes() {
        let spanish = Spanish::new();
        let article = token_with_info("Las", Gender::Feminine, Number::Plural);
        let noun = token_with_info("ordenes", Gender::Masculine, Number::Plural);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_radio() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("radio", Gender::Masculine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_internet() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("internet", Gender::Masculine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_sarten() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("sart\u{00e9}n", Gender::Masculine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_azucar() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("az\u{00fa}car", Gender::Masculine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_calor() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("calor", Gender::Masculine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }

    #[test]
    fn test_check_gender_agreement_accepts_la_maraton() {
        let spanish = Spanish::new();
        let article = token_with_info("La", Gender::Feminine, Number::Singular);
        let noun = token_with_info("marat\u{00f3}n", Gender::Masculine, Number::Singular);

        assert!(spanish.check_gender_agreement(&article, &noun));
    }
}
