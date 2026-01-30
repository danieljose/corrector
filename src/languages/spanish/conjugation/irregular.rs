//! Formas irregulares de verbos españoles comunes
//!
//! Este módulo contiene un mapeo de formas conjugadas irregulares
//! a sus infinitivos correspondientes.

use std::collections::HashMap;

/// Crea el HashMap con todas las formas irregulares
pub fn get_irregular_forms() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();

    // SER
    add_ser(&mut map);

    // ESTAR
    add_estar(&mut map);

    // IR
    add_ir(&mut map);

    // HABER
    add_haber(&mut map);

    // TENER
    add_tener(&mut map);

    // HACER
    add_hacer(&mut map);

    // PODER
    add_poder(&mut map);

    // QUERER
    add_querer(&mut map);

    // DECIR
    add_decir(&mut map);

    // VER
    add_ver(&mut map);

    // DAR
    add_dar(&mut map);

    // SABER
    add_saber(&mut map);

    // VENIR
    add_venir(&mut map);

    // PONER
    add_poner(&mut map);

    // SALIR
    add_salir(&mut map);

    // TRAER
    add_traer(&mut map);

    // OÍR
    add_oir(&mut map);

    // CAER
    add_caer(&mut map);

    // Verbos -UCIR (conducir, traducir, producir, etc.)
    add_ucir_verbs(&mut map);

    map
}

fn add_ser(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("soy", "ser");
    map.insert("eres", "ser");
    map.insert("es", "ser");
    map.insert("somos", "ser");
    map.insert("sois", "ser");
    map.insert("son", "ser");

    // Pretérito indefinido
    map.insert("fui", "ser"); // también de "ir"
    map.insert("fuiste", "ser");
    map.insert("fue", "ser");
    map.insert("fuimos", "ser");
    map.insert("fuisteis", "ser");
    map.insert("fueron", "ser");

    // Imperfecto
    map.insert("era", "ser");
    map.insert("eras", "ser");
    // "era" ya está
    map.insert("éramos", "ser");
    map.insert("erais", "ser");
    map.insert("eran", "ser");

    // Futuro
    map.insert("seré", "ser");
    map.insert("serás", "ser");
    map.insert("será", "ser");
    map.insert("seremos", "ser");
    map.insert("seréis", "ser");
    map.insert("serán", "ser");

    // Condicional
    map.insert("sería", "ser");
    map.insert("serías", "ser");
    // "sería" ya está
    map.insert("seríamos", "ser");
    map.insert("seríais", "ser");
    map.insert("serían", "ser");

    // Subjuntivo presente
    map.insert("sea", "ser");
    map.insert("seas", "ser");
    // "sea" ya está
    map.insert("seamos", "ser");
    map.insert("seáis", "ser");
    map.insert("sean", "ser");

    // Subjuntivo imperfecto
    map.insert("fuera", "ser");
    map.insert("fueras", "ser");
    // "fuera" ya está
    map.insert("fuéramos", "ser");
    map.insert("fuerais", "ser");
    map.insert("fueran", "ser");
    map.insert("fuese", "ser");
    map.insert("fueses", "ser");
    // "fuese" ya está
    map.insert("fuésemos", "ser");
    map.insert("fueseis", "ser");
    map.insert("fuesen", "ser");

    // Participio
    map.insert("sido", "ser");

    // Gerundio
    map.insert("siendo", "ser");
}

fn add_estar(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("estoy", "estar");
    map.insert("estás", "estar");
    map.insert("está", "estar");
    map.insert("estamos", "estar");
    map.insert("estáis", "estar");
    map.insert("están", "estar");

    // Pretérito indefinido
    map.insert("estuve", "estar");
    map.insert("estuviste", "estar");
    map.insert("estuvo", "estar");
    map.insert("estuvimos", "estar");
    map.insert("estuvisteis", "estar");
    map.insert("estuvieron", "estar");

    // Subjuntivo presente
    map.insert("esté", "estar");
    map.insert("estés", "estar");
    // "esté" ya está
    map.insert("estemos", "estar");
    map.insert("estéis", "estar");
    map.insert("estén", "estar");

    // Subjuntivo imperfecto
    map.insert("estuviera", "estar");
    map.insert("estuvieras", "estar");
    map.insert("estuviéramos", "estar");
    map.insert("estuvierais", "estar");
    map.insert("estuvieran", "estar");
    map.insert("estuviese", "estar");
    map.insert("estuvieses", "estar");
    map.insert("estuviésemos", "estar");
    map.insert("estuvieseis", "estar");
    map.insert("estuviesen", "estar");

    // Gerundio
    map.insert("estando", "estar");

    // Participio
    map.insert("estado", "estar");
}

fn add_ir(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("voy", "ir");
    map.insert("vas", "ir");
    map.insert("va", "ir");
    map.insert("vamos", "ir");
    map.insert("vais", "ir");
    map.insert("van", "ir");

    // Pretérito (compartido con ser, ya añadido)

    // Imperfecto
    map.insert("iba", "ir");
    map.insert("ibas", "ir");
    // "iba" ya está
    map.insert("íbamos", "ir");
    map.insert("ibais", "ir");
    map.insert("iban", "ir");

    // Futuro
    map.insert("iré", "ir");
    map.insert("irás", "ir");
    map.insert("irá", "ir");
    map.insert("iremos", "ir");
    map.insert("iréis", "ir");
    map.insert("irán", "ir");

    // Condicional
    map.insert("iría", "ir");
    map.insert("irías", "ir");
    map.insert("iríamos", "ir");
    map.insert("iríais", "ir");
    map.insert("irían", "ir");

    // Subjuntivo presente
    map.insert("vaya", "ir");
    map.insert("vayas", "ir");
    // "vaya" ya está
    map.insert("vayamos", "ir");
    map.insert("vayáis", "ir");
    map.insert("vayan", "ir");

    // Gerundio
    map.insert("yendo", "ir");

    // Participio
    map.insert("ido", "ir");
}

fn add_haber(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("he", "haber");
    map.insert("has", "haber");
    map.insert("ha", "haber");
    map.insert("hay", "haber"); // impersonal
    map.insert("hemos", "haber");
    map.insert("habéis", "haber");
    map.insert("han", "haber");

    // Pretérito indefinido
    map.insert("hube", "haber");
    map.insert("hubiste", "haber");
    map.insert("hubo", "haber");
    map.insert("hubimos", "haber");
    map.insert("hubisteis", "haber");
    map.insert("hubieron", "haber");

    // Imperfecto
    map.insert("había", "haber");
    map.insert("habías", "haber");
    map.insert("habíamos", "haber");
    map.insert("habíais", "haber");
    map.insert("habían", "haber");

    // Futuro
    map.insert("habré", "haber");
    map.insert("habrás", "haber");
    map.insert("habrá", "haber");
    map.insert("habremos", "haber");
    map.insert("habréis", "haber");
    map.insert("habrán", "haber");

    // Condicional
    map.insert("habría", "haber");
    map.insert("habrías", "haber");
    map.insert("habríamos", "haber");
    map.insert("habríais", "haber");
    map.insert("habrían", "haber");

    // Subjuntivo presente
    map.insert("haya", "haber");
    map.insert("hayas", "haber");
    map.insert("hayamos", "haber");
    map.insert("hayáis", "haber");
    map.insert("hayan", "haber");

    // Subjuntivo imperfecto
    map.insert("hubiera", "haber");
    map.insert("hubieras", "haber");
    map.insert("hubiéramos", "haber");
    map.insert("hubierais", "haber");
    map.insert("hubieran", "haber");
    map.insert("hubiese", "haber");
    map.insert("hubieses", "haber");
    map.insert("hubiésemos", "haber");
    map.insert("hubieseis", "haber");
    map.insert("hubiesen", "haber");

    // Participio
    map.insert("habido", "haber");

    // Gerundio
    map.insert("habiendo", "haber");
}

fn add_tener(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("tengo", "tener");
    map.insert("tienes", "tener");
    map.insert("tiene", "tener");
    map.insert("tenemos", "tener");
    map.insert("tenéis", "tener");
    map.insert("tienen", "tener");

    // Pretérito indefinido
    map.insert("tuve", "tener");
    map.insert("tuviste", "tener");
    map.insert("tuvo", "tener");
    map.insert("tuvimos", "tener");
    map.insert("tuvisteis", "tener");
    map.insert("tuvieron", "tener");

    // Futuro
    map.insert("tendré", "tener");
    map.insert("tendrás", "tener");
    map.insert("tendrá", "tener");
    map.insert("tendremos", "tener");
    map.insert("tendréis", "tener");
    map.insert("tendrán", "tener");

    // Condicional
    map.insert("tendría", "tener");
    map.insert("tendrías", "tener");
    map.insert("tendríamos", "tener");
    map.insert("tendríais", "tener");
    map.insert("tendrían", "tener");

    // Subjuntivo presente
    map.insert("tenga", "tener");
    map.insert("tengas", "tener");
    map.insert("tengamos", "tener");
    map.insert("tengáis", "tener");
    map.insert("tengan", "tener");

    // Subjuntivo imperfecto
    map.insert("tuviera", "tener");
    map.insert("tuvieras", "tener");
    map.insert("tuviéramos", "tener");
    map.insert("tuvierais", "tener");
    map.insert("tuvieran", "tener");
    map.insert("tuviese", "tener");
    map.insert("tuvieses", "tener");
    map.insert("tuviésemos", "tener");
    map.insert("tuvieseis", "tener");
    map.insert("tuviesen", "tener");

    // Participio
    map.insert("tenido", "tener");

    // Gerundio
    map.insert("teniendo", "tener");

    // Imperativo
    map.insert("ten", "tener");
    map.insert("tened", "tener");
}

fn add_hacer(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("hago", "hacer");
    map.insert("haces", "hacer");
    map.insert("hace", "hacer");
    map.insert("hacemos", "hacer");
    map.insert("hacéis", "hacer");
    map.insert("hacen", "hacer");

    // Pretérito indefinido
    map.insert("hice", "hacer");
    map.insert("hiciste", "hacer");
    map.insert("hizo", "hacer");
    map.insert("hicimos", "hacer");
    map.insert("hicisteis", "hacer");
    map.insert("hicieron", "hacer");

    // Futuro
    map.insert("haré", "hacer");
    map.insert("harás", "hacer");
    map.insert("hará", "hacer");
    map.insert("haremos", "hacer");
    map.insert("haréis", "hacer");
    map.insert("harán", "hacer");

    // Condicional
    map.insert("haría", "hacer");
    map.insert("harías", "hacer");
    map.insert("haríamos", "hacer");
    map.insert("haríais", "hacer");
    map.insert("harían", "hacer");

    // Subjuntivo presente
    map.insert("haga", "hacer");
    map.insert("hagas", "hacer");
    map.insert("hagamos", "hacer");
    map.insert("hagáis", "hacer");
    map.insert("hagan", "hacer");

    // Subjuntivo imperfecto
    map.insert("hiciera", "hacer");
    map.insert("hicieras", "hacer");
    map.insert("hiciéramos", "hacer");
    map.insert("hicierais", "hacer");
    map.insert("hicieran", "hacer");
    map.insert("hiciese", "hacer");
    map.insert("hicieses", "hacer");
    map.insert("hiciésemos", "hacer");
    map.insert("hicieseis", "hacer");
    map.insert("hiciesen", "hacer");

    // Participio
    map.insert("hecho", "hacer");

    // Gerundio
    map.insert("haciendo", "hacer");

    // Imperativo
    map.insert("haz", "hacer");
    map.insert("haced", "hacer");
}

fn add_poder(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("puedo", "poder");
    map.insert("puedes", "poder");
    map.insert("puede", "poder");
    map.insert("podemos", "poder");
    map.insert("podéis", "poder");
    map.insert("pueden", "poder");

    // Pretérito indefinido
    map.insert("pude", "poder");
    map.insert("pudiste", "poder");
    map.insert("pudo", "poder");
    map.insert("pudimos", "poder");
    map.insert("pudisteis", "poder");
    map.insert("pudieron", "poder");

    // Futuro
    map.insert("podré", "poder");
    map.insert("podrás", "poder");
    map.insert("podrá", "poder");
    map.insert("podremos", "poder");
    map.insert("podréis", "poder");
    map.insert("podrán", "poder");

    // Condicional
    map.insert("podría", "poder");
    map.insert("podrías", "poder");
    map.insert("podríamos", "poder");
    map.insert("podríais", "poder");
    map.insert("podrían", "poder");

    // Subjuntivo presente
    map.insert("pueda", "poder");
    map.insert("puedas", "poder");
    map.insert("podamos", "poder");
    map.insert("podáis", "poder");
    map.insert("puedan", "poder");

    // Subjuntivo imperfecto
    map.insert("pudiera", "poder");
    map.insert("pudieras", "poder");
    map.insert("pudiéramos", "poder");
    map.insert("pudierais", "poder");
    map.insert("pudieran", "poder");
    map.insert("pudiese", "poder");
    map.insert("pudieses", "poder");
    map.insert("pudiésemos", "poder");
    map.insert("pudieseis", "poder");
    map.insert("pudiesen", "poder");

    // Participio
    map.insert("podido", "poder");

    // Gerundio
    map.insert("pudiendo", "poder");
}

fn add_querer(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("quiero", "querer");
    map.insert("quieres", "querer");
    map.insert("quiere", "querer");
    map.insert("queremos", "querer");
    map.insert("queréis", "querer");
    map.insert("quieren", "querer");

    // Pretérito indefinido
    map.insert("quise", "querer");
    map.insert("quisiste", "querer");
    map.insert("quiso", "querer");
    map.insert("quisimos", "querer");
    map.insert("quisisteis", "querer");
    map.insert("quisieron", "querer");

    // Futuro
    map.insert("querré", "querer");
    map.insert("querrás", "querer");
    map.insert("querrá", "querer");
    map.insert("querremos", "querer");
    map.insert("querréis", "querer");
    map.insert("querrán", "querer");

    // Condicional
    map.insert("querría", "querer");
    map.insert("querrías", "querer");
    map.insert("querríamos", "querer");
    map.insert("querríais", "querer");
    map.insert("querrían", "querer");

    // Subjuntivo presente
    map.insert("quiera", "querer");
    map.insert("quieras", "querer");
    map.insert("queramos", "querer");
    map.insert("queráis", "querer");
    map.insert("quieran", "querer");

    // Subjuntivo imperfecto
    map.insert("quisiera", "querer");
    map.insert("quisieras", "querer");
    map.insert("quisiéramos", "querer");
    map.insert("quisierais", "querer");
    map.insert("quisieran", "querer");
    map.insert("quisiese", "querer");
    map.insert("quisieses", "querer");
    map.insert("quisiésemos", "querer");
    map.insert("quisieseis", "querer");
    map.insert("quisiesen", "querer");

    // Participio
    map.insert("querido", "querer");

    // Gerundio
    map.insert("queriendo", "querer");

    // Imperativo
    map.insert("quiere", "querer");
    map.insert("quered", "querer");
}

fn add_decir(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("digo", "decir");
    map.insert("dices", "decir");
    map.insert("dice", "decir");
    map.insert("decimos", "decir");
    map.insert("decís", "decir");
    map.insert("dicen", "decir");

    // Pretérito indefinido
    map.insert("dije", "decir");
    map.insert("dijiste", "decir");
    map.insert("dijo", "decir");
    map.insert("dijimos", "decir");
    map.insert("dijisteis", "decir");
    map.insert("dijeron", "decir");

    // Futuro
    map.insert("diré", "decir");
    map.insert("dirás", "decir");
    map.insert("dirá", "decir");
    map.insert("diremos", "decir");
    map.insert("diréis", "decir");
    map.insert("dirán", "decir");

    // Condicional
    map.insert("diría", "decir");
    map.insert("dirías", "decir");
    map.insert("diríamos", "decir");
    map.insert("diríais", "decir");
    map.insert("dirían", "decir");

    // Subjuntivo presente
    map.insert("diga", "decir");
    map.insert("digas", "decir");
    map.insert("digamos", "decir");
    map.insert("digáis", "decir");
    map.insert("digan", "decir");

    // Subjuntivo imperfecto
    map.insert("dijera", "decir");
    map.insert("dijeras", "decir");
    map.insert("dijéramos", "decir");
    map.insert("dijerais", "decir");
    map.insert("dijeran", "decir");
    map.insert("dijese", "decir");
    map.insert("dijeses", "decir");
    map.insert("dijésemos", "decir");
    map.insert("dijeseis", "decir");
    map.insert("dijesen", "decir");

    // Participio
    map.insert("dicho", "decir");

    // Gerundio
    map.insert("diciendo", "decir");

    // Imperativo
    map.insert("di", "decir");
    map.insert("decid", "decir");
}

fn add_ver(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("veo", "ver");
    map.insert("ves", "ver");
    map.insert("ve", "ver");
    map.insert("vemos", "ver");
    map.insert("veis", "ver");
    map.insert("ven", "ver");

    // Pretérito indefinido
    map.insert("vi", "ver");
    map.insert("viste", "ver");
    map.insert("vio", "ver");
    map.insert("vimos", "ver");
    map.insert("visteis", "ver");
    map.insert("vieron", "ver");

    // Imperfecto
    map.insert("veía", "ver");
    map.insert("veías", "ver");
    map.insert("veíamos", "ver");
    map.insert("veíais", "ver");
    map.insert("veían", "ver");

    // Subjuntivo presente
    map.insert("vea", "ver");
    map.insert("veas", "ver");
    map.insert("veamos", "ver");
    map.insert("veáis", "ver");
    map.insert("vean", "ver");

    // Participio
    map.insert("visto", "ver");

    // Gerundio
    map.insert("viendo", "ver");
}

fn add_dar(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("doy", "dar");
    map.insert("das", "dar");
    map.insert("da", "dar");
    map.insert("damos", "dar");
    map.insert("dais", "dar");
    map.insert("dan", "dar");

    // Pretérito indefinido
    map.insert("di", "dar");
    map.insert("diste", "dar");
    map.insert("dio", "dar");
    map.insert("dimos", "dar");
    map.insert("disteis", "dar");
    map.insert("dieron", "dar");

    // Subjuntivo presente
    map.insert("dé", "dar");
    map.insert("des", "dar");
    map.insert("demos", "dar");
    map.insert("deis", "dar");
    map.insert("den", "dar");

    // Subjuntivo imperfecto
    map.insert("diera", "dar");
    map.insert("dieras", "dar");
    map.insert("diéramos", "dar");
    map.insert("dierais", "dar");
    map.insert("dieran", "dar");
    map.insert("diese", "dar");
    map.insert("dieses", "dar");
    map.insert("diésemos", "dar");
    map.insert("dieseis", "dar");
    map.insert("diesen", "dar");

    // Participio
    map.insert("dado", "dar");

    // Gerundio
    map.insert("dando", "dar");
}

fn add_saber(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("sé", "saber");
    map.insert("sabes", "saber");
    map.insert("sabe", "saber");
    map.insert("sabemos", "saber");
    map.insert("sabéis", "saber");
    map.insert("saben", "saber");

    // Pretérito indefinido
    map.insert("supe", "saber");
    map.insert("supiste", "saber");
    map.insert("supo", "saber");
    map.insert("supimos", "saber");
    map.insert("supisteis", "saber");
    map.insert("supieron", "saber");

    // Futuro
    map.insert("sabré", "saber");
    map.insert("sabrás", "saber");
    map.insert("sabrá", "saber");
    map.insert("sabremos", "saber");
    map.insert("sabréis", "saber");
    map.insert("sabrán", "saber");

    // Condicional
    map.insert("sabría", "saber");
    map.insert("sabrías", "saber");
    map.insert("sabríamos", "saber");
    map.insert("sabríais", "saber");
    map.insert("sabrían", "saber");

    // Subjuntivo presente
    map.insert("sepa", "saber");
    map.insert("sepas", "saber");
    map.insert("sepamos", "saber");
    map.insert("sepáis", "saber");
    map.insert("sepan", "saber");

    // Subjuntivo imperfecto
    map.insert("supiera", "saber");
    map.insert("supieras", "saber");
    map.insert("supiéramos", "saber");
    map.insert("supierais", "saber");
    map.insert("supieran", "saber");
    map.insert("supiese", "saber");
    map.insert("supieses", "saber");
    map.insert("supiésemos", "saber");
    map.insert("supieseis", "saber");
    map.insert("supiesen", "saber");

    // Participio
    map.insert("sabido", "saber");

    // Gerundio
    map.insert("sabiendo", "saber");
}

fn add_venir(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("vengo", "venir");
    map.insert("vienes", "venir");
    map.insert("viene", "venir");
    map.insert("venimos", "venir");
    map.insert("venís", "venir");
    map.insert("vienen", "venir");

    // Pretérito indefinido
    map.insert("vine", "venir");
    map.insert("viniste", "venir");
    map.insert("vino", "venir");
    map.insert("vinimos", "venir");
    map.insert("vinisteis", "venir");
    map.insert("vinieron", "venir");

    // Futuro
    map.insert("vendré", "venir");
    map.insert("vendrás", "venir");
    map.insert("vendrá", "venir");
    map.insert("vendremos", "venir");
    map.insert("vendréis", "venir");
    map.insert("vendrán", "venir");

    // Condicional
    map.insert("vendría", "venir");
    map.insert("vendrías", "venir");
    map.insert("vendríamos", "venir");
    map.insert("vendríais", "venir");
    map.insert("vendrían", "venir");

    // Subjuntivo presente
    map.insert("venga", "venir");
    map.insert("vengas", "venir");
    map.insert("vengamos", "venir");
    map.insert("vengáis", "venir");
    map.insert("vengan", "venir");

    // Subjuntivo imperfecto
    map.insert("viniera", "venir");
    map.insert("vinieras", "venir");
    map.insert("viniéramos", "venir");
    map.insert("vinierais", "venir");
    map.insert("vinieran", "venir");
    map.insert("viniese", "venir");
    map.insert("vinieses", "venir");
    map.insert("viniésemos", "venir");
    map.insert("vinieseis", "venir");
    map.insert("viniesen", "venir");

    // Participio
    map.insert("venido", "venir");

    // Gerundio
    map.insert("viniendo", "venir");

    // Imperativo
    map.insert("ven", "venir");
    map.insert("venid", "venir");
}

fn add_poner(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("pongo", "poner");
    map.insert("pones", "poner");
    map.insert("pone", "poner");
    map.insert("ponemos", "poner");
    map.insert("ponéis", "poner");
    map.insert("ponen", "poner");

    // Pretérito indefinido
    map.insert("puse", "poner");
    map.insert("pusiste", "poner");
    map.insert("puso", "poner");
    map.insert("pusimos", "poner");
    map.insert("pusisteis", "poner");
    map.insert("pusieron", "poner");

    // Futuro
    map.insert("pondré", "poner");
    map.insert("pondrás", "poner");
    map.insert("pondrá", "poner");
    map.insert("pondremos", "poner");
    map.insert("pondréis", "poner");
    map.insert("pondrán", "poner");

    // Condicional
    map.insert("pondría", "poner");
    map.insert("pondrías", "poner");
    map.insert("pondríamos", "poner");
    map.insert("pondríais", "poner");
    map.insert("pondrían", "poner");

    // Subjuntivo presente
    map.insert("ponga", "poner");
    map.insert("pongas", "poner");
    map.insert("pongamos", "poner");
    map.insert("pongáis", "poner");
    map.insert("pongan", "poner");

    // Subjuntivo imperfecto
    map.insert("pusiera", "poner");
    map.insert("pusieras", "poner");
    map.insert("pusiéramos", "poner");
    map.insert("pusierais", "poner");
    map.insert("pusieran", "poner");
    map.insert("pusiese", "poner");
    map.insert("pusieses", "poner");
    map.insert("pusiésemos", "poner");
    map.insert("pusieseis", "poner");
    map.insert("pusiesen", "poner");

    // Participio
    map.insert("puesto", "poner");

    // Gerundio
    map.insert("poniendo", "poner");

    // Imperativo
    map.insert("pon", "poner");
    map.insert("poned", "poner");
}

fn add_salir(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("salgo", "salir");
    map.insert("sales", "salir");
    map.insert("sale", "salir");
    map.insert("salimos", "salir");
    map.insert("salís", "salir");
    map.insert("salen", "salir");

    // Futuro
    map.insert("saldré", "salir");
    map.insert("saldrás", "salir");
    map.insert("saldrá", "salir");
    map.insert("saldremos", "salir");
    map.insert("saldréis", "salir");
    map.insert("saldrán", "salir");

    // Condicional
    map.insert("saldría", "salir");
    map.insert("saldrías", "salir");
    map.insert("saldríamos", "salir");
    map.insert("saldríais", "salir");
    map.insert("saldrían", "salir");

    // Subjuntivo presente
    map.insert("salga", "salir");
    map.insert("salgas", "salir");
    map.insert("salgamos", "salir");
    map.insert("salgáis", "salir");
    map.insert("salgan", "salir");

    // Participio
    map.insert("salido", "salir");

    // Gerundio
    map.insert("saliendo", "salir");

    // Imperativo
    map.insert("sal", "salir");
    map.insert("salid", "salir");
}

fn add_traer(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("traigo", "traer");
    map.insert("traes", "traer");
    map.insert("trae", "traer");
    map.insert("traemos", "traer");
    map.insert("traéis", "traer");
    map.insert("traen", "traer");

    // Pretérito indefinido
    map.insert("traje", "traer");
    map.insert("trajiste", "traer");
    map.insert("trajo", "traer");
    map.insert("trajimos", "traer");
    map.insert("trajisteis", "traer");
    map.insert("trajeron", "traer");

    // Subjuntivo presente
    map.insert("traiga", "traer");
    map.insert("traigas", "traer");
    map.insert("traigamos", "traer");
    map.insert("traigáis", "traer");
    map.insert("traigan", "traer");

    // Subjuntivo imperfecto
    map.insert("trajera", "traer");
    map.insert("trajeras", "traer");
    map.insert("trajéramos", "traer");
    map.insert("trajerais", "traer");
    map.insert("trajeran", "traer");
    map.insert("trajese", "traer");
    map.insert("trajeses", "traer");
    map.insert("trajésemos", "traer");
    map.insert("trajeseis", "traer");
    map.insert("trajesen", "traer");

    // Participio
    map.insert("traído", "traer");

    // Gerundio
    map.insert("trayendo", "traer");

    // Imperativo
    map.insert("trae", "traer");
    map.insert("traed", "traer");
}

fn add_oir(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("oigo", "oír");
    map.insert("oyes", "oír");
    map.insert("oye", "oír");
    map.insert("oímos", "oír");
    map.insert("oís", "oír");
    map.insert("oyen", "oír");

    // Pretérito indefinido
    map.insert("oí", "oír");
    map.insert("oíste", "oír");
    map.insert("oyó", "oír");
    map.insert("oímos", "oír");
    map.insert("oísteis", "oír");
    map.insert("oyeron", "oír");

    // Subjuntivo presente
    map.insert("oiga", "oír");
    map.insert("oigas", "oír");
    map.insert("oigamos", "oír");
    map.insert("oigáis", "oír");
    map.insert("oigan", "oír");

    // Participio
    map.insert("oído", "oír");

    // Gerundio
    map.insert("oyendo", "oír");

    // Imperativo
    map.insert("oye", "oír");
    map.insert("oíd", "oír");
}

fn add_caer(map: &mut HashMap<&'static str, &'static str>) {
    // Presente indicativo
    map.insert("caigo", "caer");
    map.insert("caes", "caer");
    map.insert("cae", "caer");
    map.insert("caemos", "caer");
    map.insert("caéis", "caer");
    map.insert("caen", "caer");

    // Pretérito indefinido
    map.insert("caí", "caer");
    map.insert("caíste", "caer");
    map.insert("cayó", "caer");
    map.insert("caímos", "caer");
    map.insert("caísteis", "caer");
    map.insert("cayeron", "caer");

    // Subjuntivo presente
    map.insert("caiga", "caer");
    map.insert("caigas", "caer");
    map.insert("caigamos", "caer");
    map.insert("caigáis", "caer");
    map.insert("caigan", "caer");

    // Subjuntivo imperfecto
    map.insert("cayera", "caer");
    map.insert("cayeras", "caer");
    map.insert("cayéramos", "caer");
    map.insert("cayerais", "caer");
    map.insert("cayeran", "caer");
    map.insert("cayese", "caer");
    map.insert("cayeses", "caer");
    map.insert("cayésemos", "caer");
    map.insert("cayeseis", "caer");
    map.insert("cayesen", "caer");

    // Participio
    map.insert("caído", "caer");

    // Gerundio
    map.insert("cayendo", "caer");

    // ============================================================
    // VERBOS -UIR (i→y pattern: contribuir, construir, destruir, etc.)
    // ============================================================
    add_uir_verb_forms(map, "contribuir");
    add_uir_verb_forms(map, "construir");
    add_uir_verb_forms(map, "destruir");
    add_uir_verb_forms(map, "distribuir");
    add_uir_verb_forms(map, "huir");
    add_uir_verb_forms(map, "incluir");
    add_uir_verb_forms(map, "concluir");
    add_uir_verb_forms(map, "excluir");
    add_uir_verb_forms(map, "influir");
    add_uir_verb_forms(map, "sustituir");
    add_uir_verb_forms(map, "constituir");
    add_uir_verb_forms(map, "instruir");
    add_uir_verb_forms(map, "atribuir");
    add_uir_verb_forms(map, "disminuir");
}

/// Añade las formas especiales de verbos -uir (donde i→y en ciertas formas)
fn add_uir_verb_forms(map: &mut HashMap<&'static str, &'static str>, infinitive: &'static str) {
    // Use static forms based on the infinitive
    match infinitive {
        "contribuir" => {
            map.insert("contribuyo", "contribuir");
            map.insert("contribuyes", "contribuir");
            map.insert("contribuye", "contribuir");
            map.insert("contribuyen", "contribuir");
            map.insert("contribuyó", "contribuir");
            map.insert("contribuyeron", "contribuir");
            map.insert("contribuyendo", "contribuir");
            map.insert("contribuya", "contribuir");
            map.insert("contribuyas", "contribuir");
            map.insert("contribuyamos", "contribuir");
            map.insert("contribuyan", "contribuir");
            map.insert("contribuyera", "contribuir");
            map.insert("contribuyeras", "contribuir");
            map.insert("contribuyeran", "contribuir");
            map.insert("contribuyese", "contribuir");
            map.insert("contribuyesen", "contribuir");
        }
        "construir" => {
            map.insert("construyo", "construir");
            map.insert("construyes", "construir");
            map.insert("construye", "construir");
            map.insert("construyen", "construir");
            map.insert("construyó", "construir");
            map.insert("construyeron", "construir");
            map.insert("construyendo", "construir");
            map.insert("construya", "construir");
            map.insert("construyas", "construir");
            map.insert("construyamos", "construir");
            map.insert("construyan", "construir");
            map.insert("construyera", "construir");
            map.insert("construyeras", "construir");
            map.insert("construyeran", "construir");
            map.insert("construyese", "construir");
            map.insert("construyesen", "construir");
        }
        "destruir" => {
            map.insert("destruyo", "destruir");
            map.insert("destruyes", "destruir");
            map.insert("destruye", "destruir");
            map.insert("destruyen", "destruir");
            map.insert("destruyó", "destruir");
            map.insert("destruyeron", "destruir");
            map.insert("destruyendo", "destruir");
            map.insert("destruya", "destruir");
            map.insert("destruyas", "destruir");
            map.insert("destruyamos", "destruir");
            map.insert("destruyan", "destruir");
        }
        "distribuir" => {
            map.insert("distribuyo", "distribuir");
            map.insert("distribuyes", "distribuir");
            map.insert("distribuye", "distribuir");
            map.insert("distribuyen", "distribuir");
            map.insert("distribuyó", "distribuir");
            map.insert("distribuyeron", "distribuir");
            map.insert("distribuyendo", "distribuir");
        }
        "huir" => {
            map.insert("huyo", "huir");
            map.insert("huyes", "huir");
            map.insert("huye", "huir");
            map.insert("huyen", "huir");
            map.insert("huyó", "huir");
            map.insert("huyeron", "huir");
            map.insert("huyendo", "huir");
            map.insert("huya", "huir");
            map.insert("huyas", "huir");
            map.insert("huyamos", "huir");
            map.insert("huyan", "huir");
        }
        "incluir" => {
            map.insert("incluyo", "incluir");
            map.insert("incluyes", "incluir");
            map.insert("incluye", "incluir");
            map.insert("incluyen", "incluir");
            map.insert("incluyó", "incluir");
            map.insert("incluyeron", "incluir");
            map.insert("incluyendo", "incluir");
            map.insert("incluya", "incluir");
            map.insert("incluyas", "incluir");
            map.insert("incluyamos", "incluir");
            map.insert("incluyan", "incluir");
        }
        "concluir" => {
            map.insert("concluyo", "concluir");
            map.insert("concluyes", "concluir");
            map.insert("concluye", "concluir");
            map.insert("concluyen", "concluir");
            map.insert("concluyó", "concluir");
            map.insert("concluyeron", "concluir");
            map.insert("concluyendo", "concluir");
        }
        "excluir" => {
            map.insert("excluyo", "excluir");
            map.insert("excluyes", "excluir");
            map.insert("excluye", "excluir");
            map.insert("excluyen", "excluir");
            map.insert("excluyó", "excluir");
            map.insert("excluyeron", "excluir");
            map.insert("excluyendo", "excluir");
        }
        "influir" => {
            map.insert("influyo", "influir");
            map.insert("influyes", "influir");
            map.insert("influye", "influir");
            map.insert("influyen", "influir");
            map.insert("influyó", "influir");
            map.insert("influyeron", "influir");
            map.insert("influyendo", "influir");
        }
        "sustituir" => {
            map.insert("sustituyo", "sustituir");
            map.insert("sustituyes", "sustituir");
            map.insert("sustituye", "sustituir");
            map.insert("sustituyen", "sustituir");
            map.insert("sustituyó", "sustituir");
            map.insert("sustituyeron", "sustituir");
            map.insert("sustituyendo", "sustituir");
        }
        "constituir" => {
            map.insert("constituyo", "constituir");
            map.insert("constituyes", "constituir");
            map.insert("constituye", "constituir");
            map.insert("constituyen", "constituir");
            map.insert("constituyó", "constituir");
            map.insert("constituyeron", "constituir");
            map.insert("constituyendo", "constituir");
        }
        "instruir" => {
            map.insert("instruyo", "instruir");
            map.insert("instruyes", "instruir");
            map.insert("instruye", "instruir");
            map.insert("instruyen", "instruir");
            map.insert("instruyó", "instruir");
            map.insert("instruyeron", "instruir");
            map.insert("instruyendo", "instruir");
        }
        "atribuir" => {
            map.insert("atribuyo", "atribuir");
            map.insert("atribuyes", "atribuir");
            map.insert("atribuye", "atribuir");
            map.insert("atribuyen", "atribuir");
            map.insert("atribuyó", "atribuir");
            map.insert("atribuyeron", "atribuir");
            map.insert("atribuyendo", "atribuir");
        }
        "disminuir" => {
            map.insert("disminuyo", "disminuir");
            map.insert("disminuyes", "disminuir");
            map.insert("disminuye", "disminuir");
            map.insert("disminuyen", "disminuir");
            map.insert("disminuyó", "disminuir");
            map.insert("disminuyeron", "disminuir");
            map.insert("disminuyendo", "disminuir");
        }
        _ => {}
    }
}

/// Formas irregulares de verbos terminados en -ucir (conducir, traducir, producir, etc.)
/// Estos verbos tienen pretérito indefinido irregular con "j" y subjuntivo imperfecto especial
fn add_ucir_verbs(map: &mut HashMap<&'static str, &'static str>) {
    // Lista de verbos -ucir comunes
    let ucir_verbs = [
        "conducir", "traducir", "producir", "reducir", "deducir",
        "inducir", "introducir", "reproducir", "seducir",
    ];

    for verb in ucir_verbs {
        // Obtener la raíz (sin -ucir)
        let stem = &verb[..verb.len() - 4]; // "conduc" de "conducir"

        // Pretérito indefinido (raíz + uj + terminaciones)
        map.insert(
            Box::leak(format!("{}uje", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujiste", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujo", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujimos", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujisteis", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujeron", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );

        // Subjuntivo imperfecto -ra (raíz + uj + era/eras/etc)
        map.insert(
            Box::leak(format!("{}ujera", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujeras", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujéramos", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujerais", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujeran", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );

        // Subjuntivo imperfecto -se
        map.insert(
            Box::leak(format!("{}ujese", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujeses", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujésemos", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujeseis", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujesen", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );

        // Futuro subjuntivo (raro, pero existe)
        map.insert(
            Box::leak(format!("{}ujere", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujeres", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujéremos", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujereis", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
        map.insert(
            Box::leak(format!("{}ujeren", stem).into_boxed_str()),
            Box::leak(verb.to_string().into_boxed_str()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irregular_forms() {
        let forms = get_irregular_forms();

        // ser
        assert_eq!(forms.get("soy"), Some(&"ser"));
        assert_eq!(forms.get("fue"), Some(&"ser"));
        assert_eq!(forms.get("sido"), Some(&"ser"));

        // estar
        assert_eq!(forms.get("estoy"), Some(&"estar"));
        assert_eq!(forms.get("estuvo"), Some(&"estar"));

        // ir
        assert_eq!(forms.get("voy"), Some(&"ir"));
        assert_eq!(forms.get("iba"), Some(&"ir"));
        assert_eq!(forms.get("yendo"), Some(&"ir"));

        // haber
        assert_eq!(forms.get("he"), Some(&"haber"));
        assert_eq!(forms.get("hay"), Some(&"haber"));
        assert_eq!(forms.get("había"), Some(&"haber"));

        // tener
        assert_eq!(forms.get("tengo"), Some(&"tener"));
        assert_eq!(forms.get("tuvo"), Some(&"tener"));

        // hacer
        assert_eq!(forms.get("hago"), Some(&"hacer"));
        assert_eq!(forms.get("hizo"), Some(&"hacer"));
        assert_eq!(forms.get("hecho"), Some(&"hacer"));

        // decir
        assert_eq!(forms.get("digo"), Some(&"decir"));
        assert_eq!(forms.get("dijo"), Some(&"decir"));
        assert_eq!(forms.get("dicho"), Some(&"decir"));
    }
}
