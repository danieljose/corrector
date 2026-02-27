# Corrector

> **Offline spelling and grammar checker for Spanish**, written in pure Rust (zero dependencies). Extensible to multiple languages — includes Catalan spell-checking. 28 grammar rules, dynamic verb conjugation, ~280K Spanish words, ~874K Catalan forms, ~1975 tests.

Corrector ortográfico y gramatical **offline** para español, escrito en Rust puro (sin dependencias externas). Extensible a múltiples idiomas — incluye soporte ortográfico para catalán.

## Características

- **100% offline** — no requiere conexión a internet ni APIs externas
- **Sin dependencias** — solo biblioteca estándar de Rust
- **Rápido** — binario de ~2 MB, respuesta instantánea
- **Español completo** — ortografía + gramática (28 reglas gramaticales)
- **Catalán** — corrección ortográfica (diccionario de ~874K formas, punt volat, elisiones)
- **Diccionario extensible** — añade palabras propias con `--add-word`
- **Conjugador dinámico** — reconoce todas las formas verbales a partir del infinitivo, incluyendo irregulares, prefijos y enclíticos
- **~1975 tests** automatizados

## Instalación

```bash
git clone https://github.com/danieljose/corrector.git
cd corrector
cargo build --release
```

El binario queda en `./target/release/corrector`.

## Uso

```bash
# Texto directo
corrector "El casa es muy bonito"

# Desde archivo
corrector -i entrada.txt -o salida.txt

# Catalán
corrector --lang ca "L'home va anar a l'escola"

# Añadir palabra al diccionario personalizado
corrector --add-word "chatbot"
```

### Opciones

| Opción | Descripción |
|--------|-------------|
| `-l, --lang <IDIOMA>` | Idioma: `es` (default), `ca` |
| `-i, --input <ARCHIVO>` | Archivo de entrada |
| `-o, --output <ARCHIVO>` | Archivo de salida |
| `-d, --custom-dict <ARCHIVO>` | Diccionario adicional |
| `-a, --add-word <PALABRA>` | Añadir palabra al diccionario custom |
| `--data-dir <DIR>` | Directorio de datos (default: `data`) |

## Ejemplos

### Concordancia

```
» corrector "El casa es muy bonito"
  El [La] casa es muy bonito [bonita]
```

### Tildes diacríticas y homófonos

```
» corrector "Yo no se porque el no vino con migo"
  Yo no se [sé] porque [por qué] el [él] no vino con migo [conmigo]
```

### Dequeísmo

```
» corrector "Pienso de que tu deberías ir con el"
  Pienso ~~de~~ que tu [tú] deberías ir con el [él]
```

### Haber impersonal

```
» corrector "Habían muchas personas que no sabian que hacer"
  Habían [Había] muchas personas que no sabian [sabían] que [qué] hacer
```

### Condicional irreal

```
» corrector "Si tendría dinero viajaría por el mundo"
  Si tendría [tuviera] dinero viajaría por el mundo
```

### Pleonasmos

```
» corrector "Vamos a subir arriba y luego bajar abajo"
  Vamos a subir ~~arriba~~ y luego bajar ~~abajo~~
```

### Tiempos compuestos (participios irregulares)

```
» corrector "He escribido la carta y la he ponido en el correo"
  He escribido [escrito] la carta y la he ponido [puesto] en el correo
```

### Sino / si no

```
» corrector "No es blanco si no negro"
  No es blanco ~~si no~~ [sino] negro
```

### Vocativos

```
» corrector "Hola Juan cómo estás"
  Hola [Hola,] Juan cómo estás
```

## Formato de salida

| Notación | Significado | Ejemplo |
|----------|-------------|---------|
| `palabra \|sug1,sug2\|` | Error ortográfico con sugerencias | `probelma \|problema\|` |
| `palabra [corrección]` | Error gramatical | `bonito [bonita]` |
| `~~palabra~~` | Palabra que sobra | `~~arriba~~` |

## Reglas gramaticales (español)

1. Concordancia artículo-sustantivo — *el casa* → *la casa*
2. Concordancia sustantivo-adjetivo — *casa bonito* → *casa bonita*
3. Concordancia determinante-sustantivo — *este casa* → *esta casa*
4. Concordancia sujeto-verbo — *ellos dijo* → *ellos dijeron*
5. Tildes diacríticas — *tu cantas* → *tú cantas*, *para mi* → *para mí*
6. Mayúsculas — inicio de oración, nombres propios tras títulos
7. Puntuación — emparejamiento de ¿? ¡!
8. Homófonos — *he echo* → *he hecho*, *tubo que* → *tuvo que*
9. Porque / por qué / porqué — *¿Porque vienes?* → *¿Por qué vienes?*
10. Sino / si no — *no es blanco si no negro* → *sino negro*
11. Dequeísmo / queísmo — *pienso de que* → *pienso que*
12. Laísmo / leísmo / loísmo — *la dije* → *le dije*
13. Comas vocativas — *Hola Juan* → *Hola, Juan*
14. Tiempos compuestos — *he escribido* → *he escrito*
15. Haber impersonal — *habían personas* → *había personas*
16. Hacer impersonal temporal — *hacen tres años* → *hace tres años*
17. Haber existencial + artículo definido — *hay el problema* → *hay un problema*
18. Condicional irreal — *si tendría* → *si tuviera*
19. Concordancia con colectivos — *la gente vinieron* → *la gente vino*
20. Concordancia de relativos — *la persona que vinieron* → *vino*
21. Uno de los que — *uno de los que vino* → *vinieron*
22. Pleonasmos — *subir arriba* → *subir*
23. Preposiciones fosilizadas — *en base a* → *con base en*
24. Ha / a ante infinitivo — *voy ha comprar* → *voy a comprar*
25. Gerundio de posterioridad
26. Infinitivo por imperativo — *¡Callar!* → *¡Callad!*
27. Género común con referente — *el periodista María* → *la periodista María*
28. Sujetos coordinados (ni...ni, tanto...como) — concordancia plural

## Conjugador dinámico

El corrector reconoce formas verbales automáticamente a partir del infinitivo:

- **Regulares**: todas las conjugaciones de -ar, -er, -ir
- **Irregulares**: ser, estar, ir, haber, tener, hacer, poder, querer, decir, saber, venir, poner, salir, traer, oír, caer, caber, roer y más
- **Cambio de raíz**: e→ie, o→ue, e→i, u→ue, c→zc (~130 verbos)
- **Pronominales**: sentirse, acostarse, convertirse...
- **Prefijos**: des-, re-, pre-, contra-, sobre- (22 prefijos) — *reintroducir*, *desconfinar* se reconocen automáticamente
- **Enclíticos**: *dármelo* → dar + me + lo
- **Derivación de plurales**: ~55K plurales reconocidos sin entradas explícitas

Al añadir un verbo regular al diccionario (ej. `rapear|verbo|_|_||100`), todas sus conjugaciones se reconocen automáticamente.

## Diccionarios

| Archivo | Entradas | Descripción |
|---------|----------|-------------|
| `data/es/words.txt` | ~280K | Español (sustantivos, verbos, adjetivos...) |
| `data/ca/words.txt` | ~874K | Catalán (todas las formas flexionadas) |
| `data/names.txt` | ~69K | Nombres propios (compartido) |

### Formato del diccionario español

```
palabra|categoría|género|número|extra|frecuencia
```

Categorías: `sustantivo`, `verbo`, `adjetivo`, `adverbio`, `articulo`, `preposicion`, `conjuncion`, `pronombre`, `determinante`, `otro`.

## Arquitectura

```
corrector/
├── src/
│   ├── main.rs              # CLI
│   ├── lib.rs, config.rs    # Configuración
│   ├── corrector.rs         # Motor principal (pipeline genérico)
│   ├── dictionary/          # Trie para búsqueda O(m) + derivación de plurales
│   ├── spelling/            # Sugerencias por distancia de Levenshtein
│   ├── grammar/             # Tokenización y análisis
│   └── languages/
│       ├── mod.rs           # Traits: Language, VerbFormRecognizer
│       ├── spanish/         # 22 fases gramaticales
│       └── catalan/         # Ortografía (punt volat, elisiones)
├── data/                    # Diccionarios
└── tests/                   # Tests de integración
```

### Pipeline

El motor ejecuta fases genéricas y delega al idioma:

1. **Ortografía** — diccionario + distancia de edición + reconocimiento verbal
2. **Gramática base** — concordancia artículo/sustantivo/adjetivo/determinante
3. **Fases específicas del idioma** — el trait `Language` permite que cada idioma defina su propio pipeline

### Extensibilidad

Para añadir un nuevo idioma, se implementa el trait `Language`:

```rust
pub trait Language {
    fn code(&self) -> &str;
    fn configure_dictionary(&self, trie: &mut Trie);
    fn build_verb_recognizer(&self, trie: &Trie) -> Option<Box<dyn VerbFormRecognizer>>;
    fn apply_language_specific_corrections(&self, tokens: &mut Vec<Token>, ...);
    fn word_internal_chars(&self) -> &'static [char]; // ej. · para catalán
    // ... métodos de análisis con defaults neutros
}
```

## Tests

```bash
cargo test                          # Todos (~1975 tests)
cargo test --lib                    # Unit tests (~1292)
cargo test --test spanish_corrector # Integración español (~671)
cargo test --test catalan           # Integración catalán (11)
```

## Limitaciones conocidas

- **Ambigüedad posesivo/pronombre** — *tu canto* puede ser "your song" o "you sing"
- **Frases en otros idiomas** — *American Airlines* se marca como error
- **Tildes generales** — *arbol*, *cafe* se detectan como ortografía, no como regla de acentuación
- **Sujetos sin determinante** — *María trabajan* no se corrige (solo detecta det+sustantivo o pronombres)
- **Indicativo por subjuntivo** — *quiero que vienes* no se corrige a *vengas*
- **Catalán** — solo ortografía, sin reglas gramaticales por ahora

## Licencia

El código fuente está bajo licencia **MIT OR Apache-2.0** (a tu elección).

Los datos lingüísticos tienen sus propias licencias:

| Datos | Licencia | Fuente |
|-------|----------|--------|
| Diccionario catalán | LGPL-2.1 | [Softcatalà](https://huggingface.co/datasets/softcatala/catalan-dictionary) |
| Diccionario español | LGPLLR + CC-BY-SA | [FreeLing](https://github.com/TALP-UPC/FreeLing) / [doozan](https://github.com/doozan/spanish_data) |
| Nombres propios | MIT | Fuentes públicas |

Ver `data/THIRD-PARTY-LICENSES.md` para detalles completos.

---

Hecho con Rust, [Claude](https://claude.ai) y [Codex](https://chatgpt.com). Sin dependencias. Sin conexión. Sin excusas.
