//! El idioma del juego: espanol o ingles.
//!
//! Todo lo que se lee en pantalla sale de aqui, de dos sitios:
//!
//! - **`Texto`**: los textos del codigo, cada uno con su espanol y su ingles en
//!   la misma linea de la tabla de abajo. La tabla la convierte en un `enum`
//!   la macro `textos!`, asi que un texto que no exista no compila, y uno sin
//!   traduccion tampoco: la macro pide los dos. Los huecos que se rellenan van
//!   como `{}` y se rellenan en orden con `txf`.
//! - **`nombre`**: los nombres que vienen de los datos —bailes y figuras de
//!   los RON de los jefes, paseos, piezas de musica—, buscados por su clave.
//!   Esa clave es tambien la del guardado (y el texto de los RON entra en el
//!   replay dorado), y por eso no se toca: se traduce al ensenarlo, no al
//!   guardarlo. Hasta en espanol: la clave va sin tildes, como el codigo, y
//!   `en_espanol` le pone las suyas.
//!
//! El idioma de ahora es un global, como las fuentes: lo leen cincuenta
//! funciones de dibujo, y pasarlo por todas no explicaria nada que no se sepa
//! ya (macroquad es de un solo hilo). Se pone desde las opciones cada frame.
//!
//! El codigo va sin tildes ni enes, pero lo que se lee en pantalla no: el
//! espanol de la tabla lleva su ortografia entera (tildes, enes, aperturas de
//! interrogacion y exclamacion). Las dos fuentes traen esos glifos, y
//! `fuentes` mide y dibuja por caracteres, no por bytes.
//!
//! Anadir un texto: una linea en `textos!` con la clave, el espanol y el
//! ingles, y `tx(Texto::Clave)` donde se dibuje. El test de abajo comprueba
//! que ninguno este vacio y que los dos lleven los mismos huecos.

use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Idioma {
    #[default]
    Es,
    En,
}

impl Idioma {
    /// Como se guarda.
    pub fn clave(self) -> &'static str {
        match self {
            Idioma::Es => "es",
            Idioma::En => "en",
        }
    }

    pub fn por_clave(c: &str) -> Option<Self> {
        match c {
            "es" => Some(Idioma::Es),
            "en" => Some(Idioma::En),
            _ => None,
        }
    }

    /// Como se llama en pantalla: cada uno en el suyo, que es como lo busca
    /// quien no entiende el otro.
    pub fn nombre(self) -> &'static str {
        match self {
            Idioma::Es => "Español",
            Idioma::En => "English",
        }
    }

    pub fn otro(self) -> Self {
        match self {
            Idioma::Es => Idioma::En,
            Idioma::En => Idioma::Es,
        }
    }
}

static ACTUAL: AtomicU8 = AtomicU8::new(0);

pub fn actual() -> Idioma {
    if ACTUAL.load(Ordering::Relaxed) == 1 {
        Idioma::En
    } else {
        Idioma::Es
    }
}

pub fn poner(i: Idioma) {
    ACTUAL.store(i as u8, Ordering::Relaxed);
}

/// Un texto en el idioma de ahora.
pub fn tx(t: Texto) -> &'static str {
    t.en(actual())
}

/// Un texto con huecos, rellenos en orden.
pub fn txf(t: Texto, args: &[&dyn Display]) -> String {
    rellenar(tx(t), args)
}

fn rellenar(plantilla: &str, args: &[&dyn Display]) -> String {
    let mut out = String::new();
    let mut args = args.iter();
    let mut trozos = plantilla.split("{}");
    if let Some(t) = trozos.next() {
        out.push_str(t);
    }
    for t in trozos {
        if let Some(a) = args.next() {
            out.push_str(&a.to_string());
        }
        out.push_str(t);
    }
    out
}

/// Un nombre de los datos, en el idioma de ahora. Uno que no este en la tabla
/// sale tal cual: un baile nuevo se ve en espanol antes que no verse.
pub fn nombre(es: &str) -> &str {
    match actual() {
        Idioma::Es => en_espanol(es).unwrap_or(es),
        Idioma::En => en_ingles(es).unwrap_or(es),
    }
}

/// Los nombres de los datos, de la clave al espanol de pantalla. Casi todos
/// salen igual; estan todos para que el test pueda pedir que no falte
/// ninguno, y un baile nuevo no se quede sin sus tildes sin que nadie lo vea.
fn en_espanol(clave: &str) -> Option<&'static str> {
    Some(match clave {
        // Los bailes y sus figuras (assets/patterns).
        "El Vals" => "El Vals",
        "El paso de cambio" => "El paso de cambio",
        "El giro natural" => "El giro natural",
        "El giro inverso" => "El giro inverso",
        "El fleckerl" => "El fleckerl",
        "El Tango" => "El Tango",
        "La caminata" => "La caminata",
        "Los ochos" => "Los ochos",
        "El molinete" => "El molinete",
        "El Charleston" => "El Charleston",
        "El basico" => "El básico",
        "Bee's knees" => "Bee's knees",
        "El cambio de lado" => "El cambio de lado",
        "El Cancan" => "El Cancán",
        "El battement" => "El battement",
        "El port d'armes" => "El port d'armes",
        "La rueda y el grand ecart" => "La rueda y el grand écart",
        // Los paseos (assets/paseos).
        "El paseo de Viena" => "El paseo de Viena",
        "Camino del baile" => "Camino del baile",
        "El paseo del arrabal" => "El paseo del arrabal",
        "Por la Vuelta de Rocha" => "Por la Vuelta de Rocha",
        "El paseo de Chicago" => "El paseo de Chicago",
        "Por encima de la ciudad" => "Por encima de la ciudad",
        "El paseo de Montmartre" => "El paseo de Montmartre",
        "Cuesta arriba, hasta el molino" => "Cuesta arriba, hasta el molino",
        // Las piezas (music.rs).
        "la sala" => "la sala",
        "El Danubio azul - Johann Strauss II, 1866" => "El Danubio azul - Johann Strauss II, 1866",
        "El Danubio azul, reflejado" => "El Danubio azul, reflejado",
        "Sobre las olas - Juventino Rosas, 1888" => "Sobre las olas - Juventino Rosas, 1888",
        "El Danubio azul, coda" => "El Danubio azul, coda",
        "La Cumparsita - Matos Rodriguez, 1916" => "La Cumparsita - Matos Rodríguez, 1916",
        "La Cumparsita, segunda parte" => "La Cumparsita, segunda parte",
        "La Cumparsita, quebrada" => "La Cumparsita, quebrada",
        "Maple Leaf Rag - Scott Joplin, 1899" => "Maple Leaf Rag - Scott Joplin, 1899",
        "Maple Leaf Rag, segunda parte" => "Maple Leaf Rag, segunda parte",
        "Maple Leaf Rag, al galope" => "Maple Leaf Rag, al galope",
        "El galop infernal - Offenbach, 1858" => "El galop infernal - Offenbach, 1858",
        "El galop infernal, segunda parte" => "El galop infernal, segunda parte",
        "El galop infernal, desbocado" => "El galop infernal, desbocado",
        // Las ciudades del prologo.
        "VIENA" => "VIENA",
        "BUENOS AIRES" => "BUENOS AIRES",
        "CHICAGO" => "CHICAGO",
        "PARIS" => "PARÍS",
        _ => return None,
    })
}

/// Los nombres de los datos, del espanol al ingles. El vocabulario de baile
/// se queda (fleckerl, ochos, molinete...): es como se llaman los pasos en
/// cualquier idioma.
fn en_ingles(es: &str) -> Option<&'static str> {
    Some(match es {
        // Los bailes y sus figuras (assets/patterns).
        "El Vals" => "The Waltz",
        "El paso de cambio" => "The Change Step",
        "El giro natural" => "Natural Turn",
        "El giro inverso" => "Reverse Turn",
        "El fleckerl" => "The Fleckerl",
        "El Tango" => "The Tango",
        "La caminata" => "The Walk",
        "Los ochos" => "The Ochos",
        "El molinete" => "The Molinete",
        "El Charleston" => "The Charleston",
        "El basico" => "The Basic",
        "Bee's knees" => "Bee's Knees",
        "El cambio de lado" => "Switching Sides",
        "El Cancan" => "The Can-can",
        "El battement" => "The Battement",
        "El port d'armes" => "The Port d'Armes",
        "La rueda y el grand ecart" => "The Cartwheel and the Grand Ecart",
        // Los paseos (assets/paseos).
        "El paseo de Viena" => "The Vienna Promenade",
        "Camino del baile" => "On the way to the ball",
        "El paseo del arrabal" => "The Arrabal Promenade",
        "Por la Vuelta de Rocha" => "Along the Vuelta de Rocha",
        "El paseo de Chicago" => "The Chicago Promenade",
        "Por encima de la ciudad" => "High above the city",
        "El paseo de Montmartre" => "The Montmartre Promenade",
        "Cuesta arriba, hasta el molino" => "Uphill, all the way to the mill",
        // Las piezas (music.rs).
        "la sala" => "the ballroom",
        "El Danubio azul - Johann Strauss II, 1866" => "The Blue Danube - Johann Strauss II, 1866",
        "El Danubio azul, reflejado" => "The Blue Danube, mirrored",
        "Sobre las olas - Juventino Rosas, 1888" => "Over the Waves - Juventino Rosas, 1888",
        "El Danubio azul, coda" => "The Blue Danube, coda",
        "La Cumparsita - Matos Rodriguez, 1916" => "La Cumparsita - Matos Rodriguez, 1916",
        "La Cumparsita, segunda parte" => "La Cumparsita, second part",
        "La Cumparsita, quebrada" => "La Cumparsita, broken",
        "Maple Leaf Rag - Scott Joplin, 1899" => "Maple Leaf Rag - Scott Joplin, 1899",
        "Maple Leaf Rag, segunda parte" => "Maple Leaf Rag, second part",
        "Maple Leaf Rag, al galope" => "Maple Leaf Rag, at a gallop",
        "El galop infernal - Offenbach, 1858" => "The Infernal Galop - Offenbach, 1858",
        "El galop infernal, segunda parte" => "The Infernal Galop, second part",
        "El galop infernal, desbocado" => "The Infernal Galop, runaway",
        // Las ciudades del prologo.
        "VIENA" => "VIENNA",
        "BUENOS AIRES" => "BUENOS AIRES",
        "CHICAGO" => "CHICAGO",
        "PARIS" => "PARIS",
        _ => return None,
    })
}

/// El idioma de la primera vez, sin guardado. En web, el del navegador: el
/// ingles salvo que diga espanol, que fuera de Espana y America el juego lo
/// abre quien no lo habla. En nativo, el espanol salvo que el sistema diga
/// ingles en `LANG` y compania; Windows no los pone, y se queda en espanol.
pub fn de_primeras() -> Idioma {
    #[cfg(target_arch = "wasm32")]
    {
        de_la_web(&web::idioma())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let lang = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .find(|v| !v.is_empty())
            .unwrap_or_default();
        del_sistema(&lang)
    }
}

/// `navigator.language`: "es", "es-AR", "en-US"...
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn de_la_web(lang: &str) -> Idioma {
    if lang.to_ascii_lowercase().starts_with("es") {
        Idioma::Es
    } else {
        Idioma::En
    }
}

/// `LANG` y compania: "en_US.UTF-8", "es_ES.UTF-8", "C"...
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn del_sistema(lang: &str) -> Idioma {
    if lang.to_ascii_lowercase().starts_with("en") {
        Idioma::En
    } else {
        Idioma::Es
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    unsafe extern "C" {
        /// La pone `web/index.html`, en el plugin del guardado: copia
        /// `navigator.language` y devuelve cuantos bytes ha escrito.
        fn vals_idioma(ptr: *mut u8, cap: u32) -> i32;
    }

    pub fn idioma() -> String {
        let mut buf = [0u8; 32];
        // SAFETY: buffer propio y su capacidad; JS no escribe mas de `cap`.
        let n = unsafe { vals_idioma(buf.as_mut_ptr(), buf.len() as u32) };
        let n = n.clamp(0, buf.len() as i32) as usize;
        String::from_utf8_lossy(&buf[..n]).into_owned()
    }
}

/// La tabla de abajo, convertida en un `enum` con su `match`.
macro_rules! textos {
    ($($clave:ident: $es:literal, $en:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Texto {
            $($clave,)*
        }

        impl Texto {
            #[cfg(test)]
            const TODOS: &[Texto] = &[$(Texto::$clave,)*];

            pub fn en(self, idioma: Idioma) -> &'static str {
                match (self, idioma) {
                    $(
                        (Texto::$clave, Idioma::Es) => $es,
                        (Texto::$clave, Idioma::En) => $en,
                    )*
                }
            }
        }
    };
}

textos! {
    // --- El titulo ---
    Subtitulo: "cuatro bailes, cuatro jefes", "four dances, four bosses";
    MenuEntrar: "{}   entrar a la pista{}", "{}   step onto the dance floor{}";
    MenuIntentos: "   ({} intentos)", "   ({} attempts)";
    MenuMover: "{} mover    {} disparar    {} dash", "{} move    {} shoot    {} dash";
    MenuParry: "{} parry    {} saltar    {} focus", "{} parry    {} jump    {} focus";
    MenuRosas: "Parriar las balas ROSAS llena la barra SUPER;", "Parrying PINK bullets fills the SUPER meter;";
    MenuSuper: "llena, {} limpia la pantalla y hace mucho daño.", "when full, {} clears the screen and hits hard.";
    MenuPausa: "{} pausa y opciones    {} reintentar", "{} pause and options    {} retry";
    LaHistoria: "la historia", "the story";
    AbrirLibro: "{}  el libro de baile", "{}  the dance card";

    // --- El combate ---
    ABailar: "¡A BAILAR!", "LET'S DANCE!";
    EnMarcha: "¡EN MARCHA!", "OFF WE GO!";
    Programa: "PROGRAMA", "PROGRAMME";
    Figura1: "FIGURA PRIMERA", "FIRST FIGURE";
    Figura2: "FIGURA SEGUNDA", "SECOND FIGURE";
    Figura3: "FIGURA TERCERA", "THIRD FIGURE";
    Figura4: "FIGURA CUARTA", "FOURTH FIGURE";
    Prologo: "PRÓLOGO", "PROLOGUE";
    Ayuda: "{} disparar  {} saltar  {} dash  {} parry  {} super  {} focus  {} pausa  {} reintentar  {} mudo  {}  {} pasar",
        "{} shoot  {} jump  {} dash  {} parry  {} super  {} focus  {} pause  {} retry  {} mute  {}  {} skip";
    Bravo: "¡BRAVO!", "BRAVO!";
    // El rotulo del super, uno por baile (ver `ovacion.rs`): gritos de
    // patio de butacas de los de siempre, sin dobles sentidos.
    SuperVals: "¡BIS!", "ENCORE!";
    SuperTango: "¡BRAVÍSIMO!", "BRAVISSIMO!";
    SuperCharleston: "¡HURRA!", "HOORAY!";
    SuperCancan: "¡ALLEZ HOP!", "ALLEZ HOP!";
    BailadoEntero: "{}, bailado entero", "{}, danced to the end";
    VictoriaAyuda: "{} otra vez    {} volver a la pista", "{} again    {} back to the dance floor";
    SeAcabo: "SE ACABÓ", "CURTAINS";
    LlegasteBaile: "llegaste al {}% del baile", "you made it {}% through the dance";
    ReintentarBaile: "{} o {}  reintentar este baile", "{} or {}  retry this dance";
    VolverPista: "{}  volver a la pista", "{}  back to the dance floor";

    // --- Avisos ---
    TrazoVivo: "tinta temblorosa", "wobbly ink";
    TrazoQuieto: "tinta quieta", "steady ink";
    FichasTuyas: "{} de {} fichas del paseo son tuyas", "{} of {} dance tokens from the promenade are yours";
    PaseoSaltado: "paseo saltado", "promenade skipped";
    BaileSaltado: "baile saltado", "dance skipped";

    // --- El paseo ---
    Plantar: "{} te planta  -  plantada, {} apunta arriba", "{} plants your feet  -  planted, {} aims up";
    EnLaCalle: "te quedaste en la calle", "you were left out in the street";
    LlegasteCamino: "llegaste al {}% del camino a la puerta", "you made it {}% of the way to the door";
    ReintentarPaseo: "{} o {}  reintentar el paseo", "{} or {}  retry the promenade";
    EstaNoche: "ESTA NOCHE", "TONIGHT";

    // --- La pista ---
    LaPista: "LA PISTA", "THE DANCE FLOOR";
    CuentaBailes: "{} / {} bailes", "{} / {} dances";
    Bailado: "bailado", "danced";
    EnElSuelo: "en el suelo", "on the ground";
    Volando: "volando", "in the air";
    Bailar: "{}    bailar {}, {}", "{}    dance {}, {}";
    Pasear: "{}     {}    pasear ({}/{} fichas)", "{}     {}    promenade ({}/{} tokens)";
    TodaviaNo: "{} todavía no: antes hay que sacar lo anterior", "{} not yet: win the dances before it first";
    EntrarModista: "{}    entrar a la modista", "{}    visit the Dressmaker";
    Andar: "{} andar    {} menú", "{} walk    {} menu";

    // --- La modista ---
    PlacaModista: "La modista", "The Dressmaker";
    RotuloModista: "LA MODISTA", "THE DRESSMAKER";
    AltaCostura: "ALTA COSTURA  -  SE ADMITEN FICHAS DE BAILE", "HAUTE COUTURE  -  DANCE TOKENS ACCEPTED";
    LosTiros: "LOS TIROS", "THE SHOTS";
    LosAmuletos: "LOS AMULETOS", "THE CHARMS";
    Aguja: "La aguja", "The Needle";
    Abanico: "El abanico", "The Fan";
    Castanuela: "La castañuela", "The Castanet";
    Serpentina: "La serpentina", "The Streamer";
    SinAmuleto: "Sin amuleto", "No Charm";
    Relicario: "El relicario", "The Locket";
    Guante: "El guante", "The Glove";
    Corse: "El corsé", "The Corset";
    AgujaA: "Dos chorros rectos.", "Two straight streams.";
    AgujaB: "La de siempre.", "The usual.";
    AbanicoA: "Tres a la vez, en abanico.", "Three at once, fanned out.";
    AbanicoB: "De lejos entra una.", "From afar, one lands.";
    CastanuelaA: "Pega más fuerte, pero", "Hits harder, but";
    CastanuelaB: "no llega lejos.", "doesn't reach far.";
    SerpentinaA: "Busca sola al blanco,", "Finds the target itself,";
    SerpentinaB: "y pega la mitad.", "and hits for half.";
    SinAmuletoA: "Nada colgado.", "Nothing hanging.";
    SinAmuletoB: "El cuello libre.", "A bare neck.";
    RelicarioA: "Una vida más, en el", "One more life, on the";
    RelicarioB: "baile y en la calle.", "floor and in the street.";
    GuanteA: "El parry se queda", "The parry stays open";
    GuanteB: "abierto casi el doble.", "almost twice as long.";
    CorseA: "Un talle más estrecho:", "A slimmer waist:";
    CorseB: "te da menos.", "harder to hit.";
    YaPuesto: "{} ya lo llevas puesto", "{}: you're already wearing it";
    TePones: "Te pones {}", "You put on {}";
    ParaTi: "{}, para ti", "{}, all yours";
    TeFalta: "Te falta {} ficha", "You need {} more dance token";
    TeFaltan: "Te faltan {} fichas", "You need {} more dance tokens";
    Bolsillo: "{} ficha en el bolsillo", "{} dance token in your pocket";
    Bolsillos: "{} fichas en el bolsillo", "{} dance tokens in your pocket";
    AyudaModista: "{} comprar o ponerse    {} salir", "{} buy or wear    {} leave";
    Puesto: "puesto", "worn";
    EsTuyo: "es tuyo", "it's yours";
    Ficha: "{} ficha", "{} token";
    Fichas: "{} fichas", "{} tokens";
    SelloPuesto: "PUESTO", "WORN";
    SelloTuyo: "TUYO", "YOURS";

    // --- El libro de baile y las tarjetas ---
    Tiempo: "Tiempo", "Time";
    Golpes: "Golpes", "Hits";
    Parries: "Parries", "Parries";
    Supers: "Supers", "Supers";
    FichasFila: "Fichas", "Tokens";
    Record: "nuevo récord", "new record";
    TarjetaDeBaile: "TARJETA DE BAILE", "DANCE CARD";
    HastaLaPuerta: "hasta la puerta", "to the door";
    ElLibro: "El libro de baile", "The Dance Card";
    Temporada: "la temporada, apuntada a pluma", "the season, written down in ink";
    LosBailes: "LOS BAILES", "THE DANCES";
    LosPaseos: "LOS PASEOS", "THE PROMENADES";
    SinBailar: "sin bailar todavía", "not danced yet";
    SinAndar: "sin andar todavía", "not walked yet";
    SinCaidas: "ni un golpe, de momento", "not a single hit, so far";
    TarjetaVolver: "{}  volver a la pista       {}  otra vez", "{}  back to the dance floor       {}  again";
    Seguir: "{}  seguir", "{}  continue";
    Intento: "{} intento", "{} attempt";
    Intentos: "{} intentos", "{} attempts";
    Mejor: "mejor {}", "best {}";
    DondeCaes: "donde más caes: {} ({})", "where you fall most: {} ({})";
    Jugado: "en la pista: {}", "on the floor: {}";
    CerrarLibro: "{}  cerrar el libro", "{}  close the dance card";
    Ovacion: "¡ovación!", "standing ovation!";
    Repitan: "¡que lo repitan!", "encore!";
    Aplausos: "aplausos", "applause";
    Palmas: "palmas de cortesía", "polite clapping";
    Silbidos: "algún silbido", "some booing";

    // --- El entreacto y las opciones ---
    Entreacto: "ENTREACTO", "INTERMISSION";
    Opciones: "OPCIONES", "OPTIONS";
    ElTeclado: "EL TECLADO", "THE KEYBOARD";
    ElMando: "EL MANDO", "THE GAMEPAD";
    FilaSeguir: "Seguir", "Continue";
    FilaReintentar: "Reintentar", "Retry";
    FilaMapa: "Volver a la pista", "Back to the dance floor";
    FilaOpciones: "Opciones", "Options";
    FilaIdioma: "Idioma / Language", "Idioma / Language";
    Musica: "Música", "Music";
    Efectos: "Efectos", "Sound effects";
    Hervor: "Tinta temblorosa", "Wobbly ink";
    Sacudida: "Sacudida", "Screen shake";
    PantallaCompleta: "Pantalla completa", "Fullscreen";
    ControlesTeclado: "Controles del teclado", "Keyboard controls";
    ControlesMando: "Controles del mando", "Gamepad controls";
    Restaurar: "Restaurar los de serie", "Restore defaults";
    Volver: "Volver", "Back";
    Si: "sí", "on";
    No: "no", "off";
    Pie: "{} elegir    {} volver", "{} select    {} back";
    PieCambiar: "    {} cambiar", "    {} change";
    PulsaTecla: "pulsa la tecla nueva para {}", "press the new key for {}";
    PulsaBoton: "pulsa el botón nuevo para {}", "press the new button for {}";
    Cancelar: "{} deja la que estaba", "{} keeps the old one";
    NoSePuede: "esa no: es de moverse, pausar o reintentar", "not that one: it moves, pauses or retries";
    Restaurado: "vuelven los de serie", "defaults restored";

    // --- Las acciones y sus teclas ---
    Disparar: "Disparar", "Shoot";
    Saltar: "Saltar", "Jump";
    Dash: "Dash", "Dash";
    Parry: "Parry", "Parry";
    Super: "Super", "Super";
    Focus: "Focus", "Focus";
    ArribaO: "{} o {}", "{} or {}";
    Espacio: "ESPACIO", "SPACE";
    ShiftDcho: "SHIFT DCHO", "RIGHT SHIFT";
    CtrlDcho: "CTRL DCHO", "RIGHT CTRL";
    Intro: "INTRO", "ENTER";
    Borrar: "BORRAR", "BACKSPACE";

    // --- El prologo ---
    Siguiente: "siguiente", "next";
    SaltarHistoria: "saltar", "skip";
    // La historia, contada para que se entienda a la primera: quien es ella,
    // que pasa, que roban, adonde huyen y que tiene que hacer. Una idea por
    // lamina y nada que haya que deducir.
    Lamina1: "EL GRAN SALÓN", "THE GRAND BALLROOM";
    Lamina1A: "Corre el año 1900. Esta noche hay gran baile en el salón,",
        "The year is 1900. Tonight is the night of the grand ball,";
    Lamina1B: "y ella va a bailar por primera vez.",
        "and she is about to dance for the very first time.";
    Lamina2: "A MEDIANOCHE", "AT THE STROKE OF MIDNIGHT";
    Lamina2A: "Al dar las doce, cuatro cosas del salón cobran vida:",
        "As the clock strikes twelve, four things in the ballroom come alive:";
    Lamina2B: "una caja de música, un bandoneón, un gramófono y una fila de coristas.",
        "a music box, a bandoneon, a gramophone and a line of chorus girls.";
    Lamina3: "Y ROBAN LA MÚSICA", "AND THEY STEAL THE MUSIC";
    Lamina3A: "Cada uno se lleva la música de su baile y huye a su ciudad:",
        "Each one takes the music of its dance and flees to its own city:";
    Lamina3B: "Viena, Buenos Aires, Chicago y París. El salón se queda en silencio.",
        "Vienna, Buenos Aires, Chicago and Paris. The ballroom falls silent.";
    Lamina4: "EL RETO", "THE CHALLENGE";
    Lamina4A: "Ella sale tras ellos, de ciudad en ciudad.",
        "She sets off after them, from city to city.";
    Lamina4B: "Para recuperar cada música tendrá que ganarles bailando.",
        "To win each song back, she will have to out-dance its thief.";
    Lamina5: "PROGRAMA DE LA NOCHE", "TONIGHT'S PROGRAMME";
    Lamina5A: "Cuatro ciudades, cuatro bailes, cuatro rivales.",
        "Four cities, four dances, four rivals.";
    Lamina5B: "Que empiece la función.", "Let the show begin.";
    LemaVals: "Se llevó el vals a Viena, y gira sin que nadie le dé cuerda.",
        "It took the waltz to Vienna, and spins with no one to wind it.";
    LemaTango: "Se llevó el tango al arrabal, y no lo suelta ni un compás.",
        "It took the tango to the arrabal, and won't let go of a single bar.";
    LemaCharleston: "Se llevó el jazz a Chicago, y lo pone una y otra vez.",
        "It took the jazz to Chicago, and plays it over and over again.";
    LemaCancan: "Se llevaron el galop a París, y patean sin parar.",
        "They took the galop to Paris, and kick without end.";
    LemaViena: "Las calles de Viena todavía silban el vals que se llevaron.",
        "The streets of Vienna still whistle the stolen waltz.";
    LemaArrabal: "En La Boca hasta los compadritos buscan al bandoneón.",
        "In La Boca, even the compadritos are out hunting the bandoneon.";
    LemaChicago: "En Chicago la música suena por los tejados: hay que subir.",
        "In Chicago the music plays over the rooftops: up you go.";
    LemaMontmartre: "Cuesta arriba, hasta el molino rojo donde patean las coristas.",
        "Uphill, to the red mill where the chorus girls kick.";

    // --- El final ---
    // Igual que el prologo: una idea por lamina, dicha tal cual.
    ElFinal: "el final", "the ending";
    Final1: "DE VUELTA EN EL SALÓN", "BACK AT THE BALLROOM";
    Final1A: "Con las cuatro músicas recuperadas, ella vuelve al gran salón.",
        "With all four songs won back, she returns to the grand ballroom.";
    Final1B: "Los cuatro ladrones vuelven con ella.",
        "The four thieves come back with her.";
    Final2: "LA MÚSICA VUELVE", "THE MUSIC RETURNS";
    Final2A: "Cada uno vuelve a su sitio y toca su música otra vez.",
        "Each one goes back to its place and plays its music again.";
    Final2B: "El vals, el tango, el charleston y el cancán suenan en el salón.",
        "The waltz, the tango, the charleston and the can-can fill the ballroom.";
    Final3: "SU PRIMER BAILE", "HER FIRST DANCE";
    Final3A: "Por fin, ella baila por primera vez en el gran salón.",
        "At last, she dances in the grand ballroom for the very first time.";
    Final3B: "Y el baile sigue hasta el amanecer.", "And the ball goes on until dawn.";
    Final4: "FIN", "THE END";
    Final4A: "El gran salón ya no está en silencio.", "The grand ballroom is silent no more.";
    Final4B: "Gracias por jugar.", "Thank you for playing.";
    // Los creditos. Las piezas con su autor, como en el programa de un
    // concierto.
    LaMusica: "LA MÚSICA", "THE MUSIC";
    CreditoDanubio: "El Danubio azul - Johann Strauss II", "The Blue Danube - Johann Strauss II";
    CreditoOlas: "Sobre las olas - Juventino Rosas", "Over the Waves - Juventino Rosas";
    CreditoCumparsita: "La Cumparsita - Gerardo Matos Rodríguez",
        "La Cumparsita - Gerardo Matos Rodriguez";
    CreditoRag: "Maple Leaf Rag - Scott Joplin", "Maple Leaf Rag - Scott Joplin";
    CreditoGalop: "El galop infernal - Jacques Offenbach", "The Infernal Galop - Jacques Offenbach";
    LasTipografias: "LAS TIPOGRAFÍAS (SIL OFL)", "THE TYPEFACES (SIL OFL)";
    CreditoPoiret: "Poiret One - The Poiret One Project Authors",
        "Poiret One - The Poiret One Project Authors";
    CreditoBarlow: "Barlow - The Barlow Project Authors", "Barlow - The Barlow Project Authors";
    HechoEnRust: "Hecho en Rust con macroquad", "Made in Rust with macroquad";
    TusBailes: "TUS BAILES", "YOUR DANCES";
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::boss::BossDef;
    use vals_core::paseo::PaseoDef;

    #[test]
    fn cada_texto_esta_en_los_dos_idiomas_con_los_mismos_huecos() {
        for t in Texto::TODOS {
            let (es, en) = (t.en(Idioma::Es), t.en(Idioma::En));
            assert!(!es.trim().is_empty() && !en.trim().is_empty(), "{t:?}");
            assert_eq!(es.matches("{}").count(), en.matches("{}").count(), "{t:?}");
        }
    }

    /// Todas las claves de los datos que salen en pantalla.
    fn nombres_de_los_datos() -> Vec<String> {
        let mut nombres = Vec::new();
        for b in BossDef::default_bosses() {
            nombres.push(b.name.clone());
            nombres.extend(b.phases.iter().map(|p| p.name.clone()));
        }
        for p in PaseoDef::de_serie() {
            nombres.extend([p.nombre, p.subtitulo]);
        }
        nombres.extend(crate::music::TEMAS.iter().map(|t| t.titulo().to_owned()));
        nombres.extend(["VIENA", "BUENOS AIRES", "CHICAGO", "PARIS"].map(String::from));
        nombres
    }

    #[test]
    fn cada_nombre_de_los_datos_tiene_traduccion() {
        for n in nombres_de_los_datos() {
            assert!(en_ingles(&n).is_some_and(|en| !en.is_empty()), "{n}");
            assert!(en_espanol(&n).is_some_and(|es| !es.is_empty()), "{n}");
        }
    }

    /// Las faltas de siempre al escribir sin tildes: si una vuelve a colarse en
    /// el espanol de pantalla, salta. Por palabras enteras, que "mas" esta mal
    /// y "mascara" no.
    #[test]
    fn el_espanol_de_pantalla_lleva_sus_tildes() {
        const FALTAS: &str = "musica salon funcion castanuela bandoneon gramofono tendra mas \
            dano ano todavia algun boton circulo triangulo record prologo corse basico cancan \
            paris ecart compas ovacion menu espanol rodriguez cortesia tipografias";
        let mut textos: Vec<&str> = Texto::TODOS.iter().map(|t| t.en(Idioma::Es)).collect();
        textos.push(Idioma::Es.nombre());
        textos.extend(nombres_de_los_datos().iter().filter_map(|n| en_espanol(n)));
        for t in textos {
            let minus = t.to_lowercase();
            for p in minus.split(|c: char| !c.is_alphabetic()) {
                assert!(
                    !FALTAS.split_whitespace().any(|f| f == p),
                    "\"{p}\" sin tilde en \"{t}\""
                );
            }
        }
    }

    #[test]
    fn los_huecos_se_rellenan_en_orden() {
        let s = rellenar("{} de {} fichas", &[&3, &"cinco"]);
        assert_eq!(s, "3 de cinco fichas");
        // Si faltan, se quedan vacios en vez de reventar.
        assert_eq!(rellenar("a {} b {}", &[&1]), "a 1 b ");
    }

    #[test]
    fn el_idioma_de_la_primera_vez() {
        // Web: ingles salvo que el navegador diga espanol.
        assert_eq!(de_la_web("es"), Idioma::Es);
        assert_eq!(de_la_web("es-AR"), Idioma::Es);
        assert_eq!(de_la_web("en-US"), Idioma::En);
        assert_eq!(de_la_web("fr-FR"), Idioma::En);
        assert_eq!(de_la_web(""), Idioma::En);
        // Nativo: espanol salvo que el sistema diga ingles.
        assert_eq!(del_sistema("en_US.UTF-8"), Idioma::En);
        assert_eq!(del_sistema("es_ES.UTF-8"), Idioma::Es);
        assert_eq!(del_sistema("C"), Idioma::Es);
        assert_eq!(del_sistema(""), Idioma::Es);
    }
}
