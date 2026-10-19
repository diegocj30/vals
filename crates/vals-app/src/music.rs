//! La musica, tambien sintetizada. Ni un fichero de audio en el repo.
//!
//! Sin musica, "cada jefe es un baile" no se entiende: el compas es la mitad
//! de la identidad de un baile, y se oye antes que se ve.
//!
//! Un tema aqui no es un fichero ni un secuenciador: es una **lista de voces
//! con retardo**, exactamente las mismas que hacen los efectos de sonido. Poner
//! una nota en el compas 3, tiempo 2, es una `Voz` con `delay` calculado. Todo
//! el motor de audio ya estaba escrito; esto solo lo usa para mas cosas.
//!
//! # Por que valses de verdad
//!
//! La primera version generaba los valses: bajo al uno, triada al dos y al
//! tres, y una melodia que subia y bajaba por el acorde. Reconocias el
//! *compas*, no la *pieza*, y a los dos bucles cansaba. Ahora las melodias son
//! transcripciones de valses reales de dominio publico:
//!
//! - **El Danubio azul**, Johann Strauss II (1866). El primer tema completo,
//!   32 compases, con su armonia.
//! - **Sobre las olas**, Juventino Rosas (1888). Parte A, 16 compases.
//!
//! Ambas salen de transcripciones publicas en notacion ABC, que es texto y por
//! tanto se pasa a esta tabla sin ambiguedad. Las dos piezas son de dominio
//! publico (1866 y 1888, autores muertos en 1899 y 1894).
//!
//! El tercero no es una tercera pieza: **es el Danubio reflejado**. El Espejo
//! toca la segunda mitad del Danubio con cada intervalo invertido alrededor de
//! la tonica y leido en menor. Lo que subia, baja. El ritmo es identico, asi
//! que se reconoce, y todo lo demas esta del reves, que es exactamente lo que
//! tiene que sonar en el jefe espejo.
//!
//! El bucle se renderiza entero una vez al arrancar y se reproduce en bucle.
//!
//! # La sala
//!
//! La pista no toca ningun baile: toca su propia cancion, un vals original
//! escrito para una caja de musica. Es lo unico que les queda a los salones
//! cuando los objetos se llevan la musica, y cada baile ganado le devuelve su
//! instrumento: el vals, el pah-pah que la hace mecer; el tango, un bandoneon
//! por debajo; el charleston, un piano a contratiempo; el cancan, la banda de
//! metales; y el flamenco, una guitarra arpegiando. Con los cinco, el salon
//! entero.

use crate::audio::{Voz, Wave};

/// Cada trozo de musica del juego.
///
/// Se llama `Tema` y no `Baile` porque **la sala no es un baile**: la pista
/// tiene su propia musica, y hacerla pasar por una figura de vals era mentir
/// en el nombre.
///
/// Un baile es un jefe y sus fases son figuras suyas, asi que la musica se
/// busca por (jefe, fase). Antes iba solo por fase, que funcionaba mientras
/// hubo un unico baile y habria hecho que la segunda figura del tango sonase a
/// vals en cuanto entrara el segundo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tema {
    /// La pista. No es ningun baile: es la sala antes de que empiece nada.
    ///
    /// Lleva los bailes ganados, un bit por jefe en el orden de `Tema::de`:
    /// cuanto mas has bailado, mas instrumentos vuelven a la sala.
    Sala(u8),

    // --- El Vals ---
    /// El Danubio azul entero. 3/4, oom-pah-pah, el vals de toda la vida.
    PasoBase,
    /// El mismo Danubio, reflejado y en menor. Mas lento y mas oscuro.
    Espejo,
    /// Sobre las olas: corriendo en corcheas, donde el Danubio sostenia.
    Molinete,
    /// El tema del principio otra vez, un tono mas arriba y a toda velocidad.
    /// Una coda, en musica, es exactamente eso.
    Coda,

    // --- El Tango ---
    /// La Cumparsita, primera parte. 4/4 y marcato: ni rastro de oom-pah-pah.
    Caminata,
    /// Su segunda parte, mas lenta y mas grave.
    Corte,
    /// Y vuelta a la primera, disparada. Un tango tambien vuelve al tema.
    Quebrada,

    // --- El Charleston ---
    /// Maple Leaf Rag, primera parte. 4/4, stride, y las corcheas swingadas.
    CharlestonA,
    /// Su segunda parte.
    CharlestonB,
    /// Y la primera otra vez, un tono arriba y corriendo.
    CharlestonFin,

    // --- El Cancan ---
    /// El galop infernal, primera parte. 2/4, y el unico de dos tiempos.
    Cancan,
    /// Su segunda parte, en sol.
    Galop,
    /// Y la primera otra vez, un tono arriba y desbocada.
    Infernal,

    // --- El Flamenco ---
    /// Asturias, el principio: el si del pedal y el canto entre medias.
    Llamada,
    /// La parte fuerte: el acorde golpeado al uno y el pedal en octavas.
    Rasgueo,
    /// La llamada otra vez, en octavas y mas deprisa: los pies corriendo.
    Zapateado,
    /// Y la bajada y la ultima vuelta de la parte fuerte, para acabar.
    Remate,
}

/// Todos, en el orden en que se sintetizan y se guardan.
///
/// De la sala solo esta la vacia: las demas se sintetizan cuando hacen falta,
/// y comparten su hueco.
pub const TEMAS: [Tema; 18] = [
    Tema::Sala(0),
    Tema::PasoBase,
    Tema::Espejo,
    Tema::Molinete,
    Tema::Coda,
    Tema::Caminata,
    Tema::Corte,
    Tema::Quebrada,
    Tema::CharlestonA,
    Tema::CharlestonB,
    Tema::CharlestonFin,
    Tema::Cancan,
    Tema::Galop,
    Tema::Infernal,
    Tema::Llamada,
    Tema::Rasgueo,
    Tema::Zapateado,
    Tema::Remate,
];

impl Tema {
    /// La musica de una figura, buscada por el baile al que pertenece.
    ///
    /// Un jefe o una fase que no existan caen en la ultima figura de ese
    /// baile, que es lo que menos sorprende: un indice raro suena a final, no
    /// a silencio.
    pub fn de(jefe: usize, fase: usize) -> Self {
        match (jefe, fase) {
            (0, 0) => Tema::PasoBase,
            (0, 1) => Tema::Espejo,
            (0, 2) => Tema::Molinete,
            (0, _) => Tema::Coda,
            (1, 0) => Tema::Caminata,
            (1, 1) => Tema::Corte,
            (1, _) => Tema::Quebrada,
            (2, 0) => Tema::CharlestonA,
            (2, 1) => Tema::CharlestonB,
            (2, _) => Tema::CharlestonFin,
            (3, 0) => Tema::Cancan,
            (3, 1) => Tema::Galop,
            (3, _) => Tema::Infernal,
            (_, 0) => Tema::Llamada,
            (_, 1) => Tema::Rasgueo,
            (_, 2) => Tema::Zapateado,
            (_, _) => Tema::Remate,
        }
    }

    pub fn indice(self) -> usize {
        if let Tema::Sala(_) = self {
            return 0;
        }
        TEMAS.iter().position(|t| *t == self).unwrap_or(0)
    }

    /// Que suena, para poder decirlo en pantalla.
    pub fn titulo(self) -> &'static str {
        partitura(self).titulo
    }
}

/// En que se mide la duracion de una nota. Cuatro por tiempo: da para negras,
/// corcheas, semicorcheas y el puntillo del ultimo compas del Danubio.
///
/// Los tiempos por compas ya no son una constante del modulo: los pone cada
/// partitura, porque un vals tiene tres y un tango cuatro. Fue el unico cambio
/// estructural que pidio meter otro baile.
const UNIDADES_POR_TIEMPO: u32 = 4;

/// Una nota de la melodia: semitonos sobre la tonica y duracion en unidades.
#[derive(Clone, Copy)]
struct Nota {
    tono: i32,
    unidades: u32,
}

/// Tono reservado para el silencio.
const SILENCIO: i32 = i32::MIN;

const fn n(tono: i32, unidades: u32) -> Nota {
    Nota { tono, unidades }
}
const fn z(unidades: u32) -> Nota {
    Nota {
        tono: SILENCIO,
        unidades,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Triada {
    Mayor,
    Menor,
    /// De dominante: mayor con la septima menor. Es la que tira hacia casa.
    Septima,
}

/// Un acorde por compas. La raiz va en semitonos sobre la tonica.
#[derive(Clone, Copy)]
struct Acorde {
    raiz: i32,
    triada: Triada,
}

const fn may(raiz: i32) -> Acorde {
    Acorde {
        raiz,
        triada: Triada::Mayor,
    }
}
const fn men(raiz: i32) -> Acorde {
    Acorde {
        raiz,
        triada: Triada::Menor,
    }
}
const fn sep(raiz: i32) -> Acorde {
    Acorde {
        raiz,
        triada: Triada::Septima,
    }
}

/// Como se toca el acorde debajo de la melodia.
///
/// Es la mitad de la identidad de un baile, y se oye antes que ninguna nota.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Acompanamiento {
    /// Grave al uno, acorde al dos y al tres. **Esto es un vals.**
    OomPahPah,
    /// Grave en los tiempos fuertes y golpe seco en los debiles. Esto es un
    /// tango: no mece, marca.
    Marcato,
    /// Bajo que **anda**, alternando fundamental y quinta en los tiempos
    /// fuertes, y acorde en los debiles. Es el stride del ragtime: se parece al
    /// marcato en donde caen los golpes y no se parece en nada al oirlo,
    /// porque el bajo se mueve en vez de repetirse.
    Stride,
    /// La guitarra de Asturias: entre nota y nota del canto, el pedal, y nada
    /// mas. Semicorcheas alternando: canto, pedal, canto, pedal.
    Toque,
    /// Lo mismo en *fff*: el pedal en octavas, un bajo al uno y el acorde
    /// rasgueado en cada tiempo.
    Rasgueo,
    /// El pedal en octavas y un golpe seco de acorde en el dos y el tres, que
    /// son los tacones.
    Taconeo,
    /// La caja de musica, con una pua grave por compas, y encima lo que
    /// hayan devuelto los bailes ganados (los bits de `Tema::Sala`).
    Sala(u8),
}

/// Que bit de la sala devuelve cada baile. Mismo orden que `Tema::de`.
const VALS: u8 = 1;
const TANGO: u8 = 1 << 1;
const CHARLESTON: u8 = 1 << 2;
const CANCAN: u8 = 1 << 3;
const FLAMENCO: u8 = 1 << 4;

impl Acorde {
    fn intervalos(self) -> &'static [i32] {
        match self.triada {
            Triada::Mayor => &[0, 4, 7],
            Triada::Menor => &[0, 3, 7],
            Triada::Septima => &[0, 4, 7, 10],
        }
    }
}

// ---------------------------------------------------------------------------
// Partituras
// ---------------------------------------------------------------------------

/// **El Danubio azul**, Johann Strauss II (1866), primer tema completo.
///
/// En do mayor: los tonos son semitonos sobre el do central. Un compas son
/// doce unidades, asi que cada linea de aqui abajo suma doce (salvo las notas
/// ligadas, que valen por dos compases).
///
/// Lo que hace que se reconozca a la primera son los dos primeros compases:
/// el arpegio 1-3-5 y la quinta sostenida. Los dos golpes de acorde que
/// contestan —el "pam, pam"— no estan aqui: los pone el acompanamiento, que
/// toca precisamente en los tiempos dos y tres.
#[rustfmt::skip]
const DANUBIO: [Nota; 70] = [
    n(0, 4), n(4, 4), n(7, 4),                          // 1  do mi sol
    n(7, 8), n(19, 4),                                  // 2  sol -- sol'
    n(19, 8), n(16, 4),                                 // 3
    n(16, 8), n(0, 4),                                  // 4
    n(0, 4), n(4, 4), n(7, 4),                          // 5
    n(7, 8), n(19, 4),                                  // 6
    n(19, 8), n(17, 4),                                 // 7
    n(17, 8), n(-1, 4),                                 // 8
    n(-1, 4), n(2, 4), n(9, 4),                         // 9
    n(9, 8), n(21, 4),                                  // 10
    n(21, 8), n(17, 4),                                 // 11
    n(17, 8), n(-1, 4),                                 // 12
    n(-1, 4), n(2, 4), n(9, 4),                         // 13
    n(9, 8), n(21, 4),                                  // 14
    n(21, 8), n(16, 4),                                 // 15
    n(16, 8), n(0, 4),                                  // 16
    // --- MITAD: de aqui en adelante es lo que toca El Espejo, reflejado.
    n(0, 4), n(4, 4), n(7, 4),                          // 17
    n(12, 8), n(24, 4),                                 // 18
    n(24, 8), n(19, 4),                                 // 19
    n(19, 8), n(0, 4),                                  // 20
    n(0, 4), n(4, 4), n(7, 4),                          // 21
    n(12, 8), n(24, 4),                                 // 22
    n(24, 8), n(21, 4),                                 // 23
    n(21, 8), n(2, 4),                                  // 24
    n(2, 4), n(5, 4), n(9, 4),                          // 25
    n(9, 16), n(6, 4), n(7, 4),                         // 26-27 (la ligada)
    n(16, 16), n(12, 4), n(4, 4),                       // 28-29 (la otra)
    n(4, 8), n(2, 4),                                   // 30
    n(9, 8), n(7, 4),                                   // 31
    n(0, 6), n(0, 2), n(0, 4),                          // 32 (el puntillo)
];

/// Donde empieza la segunda mitad del Danubio, la que refleja El Espejo.
const DANUBIO_MITAD: usize = 36;

#[rustfmt::skip]
const DANUBIO_ACORDES: [Acorde; 32] = [
    may(0), may(0), may(0), may(0), may(0),                     // 1-5    do
    sep(7), sep(7), sep(7), sep(7),                             // 6-9    sol7
    sep(7), sep(7), sep(7), sep(7),                             // 10-13
    may(0), may(0), may(0), may(0),                             // 14-17  do
    may(0), may(0), may(0), may(0),                             // 18-21
    may(5), may(5), may(5), may(5),                             // 22-25  fa
    sep(7), sep(7),                                             // 26-27  sol7
    may(0), may(0),                                             // 28-29  do
    men(2),                                                     // 30     re menor
    sep(7),                                                     // 31     sol7
    may(0),                                                     // 32     do
];

/// **Sobre las olas**, Juventino Rosas (1888), parte A.
///
/// En sol mayor: los tonos son semitonos sobre el sol. Es el otro extremo del
/// genero respecto al Danubio —donde aquel sostiene notas largas, este corre
/// en corcheas y cromatismos— y por eso le toca al ultimo jefe.
#[rustfmt::skip]
const OLAS: [Nota; 60] = [
    n(-8, 6), n(-9, 2), n(-8, 2), n(-5, 2),                     // 1
    n(0, 8), n(-1, 2), n(0, 2),                                 // 2
    n(2, 2), n(0, 2), n(-1, 2), n(0, 2), n(-8, 2), n(-5, 2),    // 3
    n(-1, 6), z(6),                                             // 4
    n(-7, 6), n(-8, 2), n(-7, 2), n(-5, 2),                     // 5
    n(-1, 8), n(-2, 2), n(-1, 2),                               // 6
    n(0, 2), n(-1, 2), n(-2, 2), n(-1, 2), n(-7, 2), n(-1, 2),  // 7
    n(-8, 6), z(6),                                             // 8
    n(-8, 6), n(-9, 2), n(-8, 2), n(-5, 2),                     // 9
    n(0, 8), n(-1, 2), n(0, 2),                                 // 10
    n(2, 2), n(0, 2), n(-1, 2), n(0, 2), n(-8, 2), n(-5, 2),    // 11
    n(-3, 6), z(6),                                             // 12
    n(-3, 6), n(2, 2), n(5, 2), n(9, 2),                        // 13
    n(7, 8), n(5, 2), n(4, 2),                                  // 14
    n(2, 2), n(0, 2), n(-1, 2), n(-3, 2), n(-1, 2), n(2, 2),    // 15
    n(0, 6), z(6),                                              // 16
];

#[rustfmt::skip]
const OLAS_ACORDES: [Acorde; 16] = [
    may(0), may(0), may(0), sep(7),                             // 1-4
    men(2), sep(7), sep(7), may(0),                             // 5-8
    may(0), may(0), may(0), may(5),                             // 9-12
    men(2), may(0), sep(7), may(0),                             // 13-16
];

/// **La Cumparsita**, Matos Rodriguez (1916), primera parte.
///
/// No hay facsimil de la primera edicion (Breyer Hermanos, 1917) que se pueda
/// usar: lo que hay en IMSLP son arreglos modernos. La tabla sale de la
/// transcripcion publica en ABC de John Chambers (trillian.mit.edu), y el
/// arranque esta comprobado con el incipit que cita la Wikipedia, que es el
/// de la partitura: sol menor, 2/4, re-do-la-fa# en corcheas picadas, en ese
/// mismo registro.
///
/// Aqui va un tono arriba, en la menor, y no por gusto: en sol, el final de
/// la segunda parte baja a 185 Hz, que es donde vive el acompanamiento. Los
/// tonos son semitonos sobre el la4. El 2/4 del original va con las figuras
/// al doble —un compas suyo es uno de cuatro tiempos de aqui, con la corchea
/// en negra—, que es como lo escribe la transcripcion y lo mismo al oido.
///
/// La figura que lo hace reconocible son los cuatro golpes secos del primer
/// compas: mi, re, si, sol sostenido, **picados**, cada uno con su silencio
/// detras. Y los dos mi repetidos que les sirven de anacrusa.
#[rustfmt::skip]
const CUMPARSITA_A: [Nota; 64] = [
    n(-5, 2), z(2), n(5, 2), z(2), n(2, 2), z(2), n(-1, 2), z(2),           // 1  mi re si sol#
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 2), n(-5, 1), n(-5, 1), // 2
    n(-5, 2), z(2), n(7, 2), z(2), n(3, 2), z(2), n(0, 2), z(2),            // 3  mi mi' do la
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 2), n(-5, 1), n(-5, 1), // 4
    n(-5, 2), z(2), n(5, 2), z(2), n(2, 2), z(2), n(-1, 2), z(2),           // 5
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 2), n(-5, 1), n(-5, 1), // 6
    n(-5, 2), z(2), n(7, 2), z(2), n(3, 2), z(2), n(0, 2), z(2),            // 7
    // En la partitura el 8 acaba en negra y sigue la segunda parte; aqui el
    // tema vuelve al 1, y vuelve con su anacrusa, como en los compases pares.
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 2), n(-5, 1), n(-5, 1), // 8
];

#[rustfmt::skip]
const CUMPARSITA_A_ACORDES: [Acorde; 8] = [
    sep(7), sep(7), men(0), men(0),
    sep(7), sep(7), men(0), men(0),
];

/// La segunda mitad de la primera parte: la melodia baja y baja hasta el
/// suelo, por cromatismos (mi, re, re bemol, do; re, re bemol, do, si).
///
/// El si de esa bajada es natural: es la quinta del mi7 que suena debajo. La
/// tabla vieja lo tenia bemol, un tritono contra el acorde, y empezaba el 9 en
/// el re de arriba en vez de en el de abajo. Y el final sube a la octava: la
/// grave, sol# y la agudos.
#[rustfmt::skip]
const CUMPARSITA_B: [Nota; 40] = [
    n(-7, 4), n(0, 4), n(-1, 4), n(0, 4),                                   // 9  re la sol# la
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-4, 4), n(-1, 4),                 // 10
    n(-5, 6), n(-7, 1), n(-8, 1), n(-9, 8),                                 // 11 mi re reb do
    z(2), n(-6, 2), n(-5, 2), n(-7, 2), n(-5, 4), n(-4, 4),                 // 12
    n(-7, 6), n(-8, 1), n(-9, 1), n(-10, 8),                                // 13 re reb do si
    z(2), n(-7, 2), n(-9, 2), n(-10, 2), n(-12, 4), n(-13, 4),              // 14
    n(-12, 6), n(-4, 2), n(-5, 2), n(-7, 2), n(-9, 2), n(-10, 2),           // 15
    n(-12, 6), n(-1, 2), n(0, 2), z(6),                                     // 16
];

#[rustfmt::skip]
const CUMPARSITA_B_ACORDES: [Acorde; 8] = [
    men(5), men(5), men(0), men(0),
    sep(7), sep(7), men(0), men(0),
];

/// La sala: un vals original para caja de musica, en sol mayor.
///
/// No es de ningun baile a proposito. En la pista no se esta bailando: se
/// esta eligiendo, y poner ahi el Danubio decia que el vals era el juego. Es
/// la cancion del salon, y suena igual sin nada encima que con los cuatro
/// bailes devueltos: lo que cambia es quien la acompana.
///
/// Dos frases de ocho compases. La primera sube por el acorde y vuelve; la
/// segunda repite su arranque con el mi bajado a mi bemol (do mayor, do menor),
/// que es lo que le da la nostalgia, y cierra en sol con una vuelta que lleva
/// otra vez al principio sin que se note la costura.
#[rustfmt::skip]
const SALA: [Nota; 48] = [
    n(7, 8), n(4, 2), n(7, 2),                          // 1  sol
    n(12, 8), n(11, 2), n(9, 2),                        // 2  mi menor
    n(9, 4), n(5, 4), n(2, 4),                          // 3  la menor
    n(-1, 4), n(2, 4), n(5, 4),                         // 4  re7
    n(4, 8), n(2, 2), n(4, 2),                          // 5  sol
    n(13, 8), n(11, 2), n(9, 2),                        // 6  mi7: el compas 2 con sol sostenido
    n(14, 4), n(9, 4), n(5, 4),                         // 7  la menor
    n(7, 4), n(11, 4), n(14, 4),                        // 8  re7
    n(12, 8), n(9, 4),                                  // 9  do
    n(12, 8), n(8, 4),                                  // 10 do menor
    n(7, 8), n(4, 2), n(7, 2),                          // 11 sol
    n(11, 4), n(13, 4), n(16, 4),                       // 12 mi7
    n(14, 6), n(12, 2), n(9, 2), n(5, 2),               // 13 la menor
    n(2, 4), n(7, 4), n(5, 4),                          // 14 re7
    n(4, 4), n(2, 2), n(-1, 2), n(0, 4),                // 15 sol
    z(4), n(2, 4), n(5, 4),                             // 16 re7, y vuelta al 1
];

#[rustfmt::skip]
const SALA_ACORDES: [Acorde; 16] = [
    may(0), men(9), men(2), sep(7),
    may(0), sep(9), men(2), sep(7),
    may(5), men(5), may(0), sep(9),
    men(2), sep(7), may(0), sep(7),
];

/// El bandoneon que devuelve el tango: una voz interior, una nota larga por
/// compas, siempre del acorde y casi siempre a un paso de la anterior. Va al
/// reves que la caja: cuando ella sube, el baja.
#[rustfmt::skip]
const SALA_BANDONEON: [Nota; 16] = [
    n(-5, 12), n(-8, 12), n(-7, 12), n(-10, 12),        // 1-4   re si do la
    n(-8, 12), n(-5, 12), n(-7, 12), n(-10, 12),        // 5-8   si re do la
    n(-7, 24), n(-8, 12), n(-11, 12),                   // 9-12  do (ligada) si sol#
    n(-10, 12), n(-7, 12), n(-8, 12),                   // 13-15 la do si
    n(-10, 6), n(-7, 6),                                // 16    la do, y al re
];

/// **Maple Leaf Rag**, Scott Joplin (1899), primera parte.
///
/// En fa mayor: los tonos son semitonos sobre el fa. Compas de 4/4, dieciseis
/// unidades por compas, igual que el tango.
///
/// Es ragtime y no swing —el swing llega veinte anos despues—, pero es la raiz
/// del jazz, es de dominio publico sin discusion (Joplin murio en 1917) y se
/// reconoce a la primera. Lo que lo convierte en jazz al sonar es que sus
/// corcheas van **swingadas**, que es cosa del sintetizador y no de la tabla.
#[rustfmt::skip]
const RAG_A: [Nota; 97] = [
    z(2), n(0, 2), n(7, 2), n(0, 2), n(4, 2), n(7, 4), n(0, 2),              // 1
    n(7, 2), n(-1, 2), n(2, 2), n(7, 10),                                   // 2
    z(2), n(0, 2), n(7, 2), n(0, 2), n(4, 2), n(7, 4), n(0, 2),              // 3
    n(7, 2), n(-1, 2), n(2, 2), n(7, 10),                                   // 4
    z(2), n(0, 2), n(3, 2), n(8, 2), z(2), n(7, 4), n(7, 2),                 // 5
    z(2), n(0, 2), n(3, 2), n(8, 2), z(2), n(7, 6),                          // 6
    n(0, 2), n(3, 2), n(7, 2), n(12, 2), n(3, 2), n(7, 2), n(12, 2), n(15, 2), // 7
    n(7, 2), n(12, 2), n(15, 2), n(19, 2), n(12, 2), n(15, 2), n(19, 2), n(24, 2), // 8
    n(12, 4), n(12, 4), n(12, 4), n(12, 2), n(12, 2),                        // 9
    n(12, 2), n(7, 2), n(9, 2), n(4, 2), n(7, 2), n(9, 6),                   // 10
    n(0, 2), n(2, 2), n(3, 2), n(0, 2), n(2, 2), n(4, 4), n(0, 2),           // 11
    n(4, 2), n(0, 2), n(2, 4), n(0, 4), z(4),                                // 12
    n(12, 4), n(12, 4), n(12, 4), n(12, 2), n(12, 2),                        // 13
    n(12, 2), n(7, 2), n(9, 2), n(4, 2), n(7, 2), n(9, 6),                   // 14
    n(0, 2), n(2, 2), n(3, 2), n(0, 2), n(2, 2), n(4, 4), n(0, 2),           // 15
    n(4, 2), n(0, 2), n(2, 4), n(0, 4), z(4),                                // 16
];

#[rustfmt::skip]
const RAG_A_ACORDES: [Acorde; 16] = [
    may(0), sep(7), may(0), sep(7),
    may(8), may(8), may(0), may(0),
    may(8), may(0), may(8), sep(7),
    may(8), may(0), may(8), sep(7),
];

/// Su segunda parte, la que sube al agudo.
#[rustfmt::skip]
const RAG_B: [Nota; 57] = [
    z(2), n(11, 2), n(19, 2), n(11, 2), n(14, 2), n(18, 4), n(11, 2),        // 1
    n(17, 2), n(11, 2), n(14, 2), n(16, 2), n(16, 2), n(7, 2), n(14, 2), n(7, 2), // 2
    z(2), n(4, 2), n(12, 2), n(4, 2), n(7, 2), n(9, 4), n(7, 2),             // 3
    n(12, 2), n(4, 2), n(7, 2), n(9, 2), n(9, 2), n(4, 2), n(9, 4),          // 4
    z(2), n(7, 2), n(11, 2), n(2, 2), n(5, 2), n(9, 4), n(7, 2),             // 5
    n(11, 2), n(2, 2), n(5, 2), n(9, 2), n(9, 2), n(5, 2), n(9, 4),          // 6
    z(2), n(4, 2), n(12, 2), n(4, 2), n(7, 2), n(9, 4), n(7, 2),             // 7
    n(12, 2), n(4, 2), n(7, 2), n(9, 2), n(9, 2), n(4, 2), n(9, 4),          // 8
];

#[rustfmt::skip]
const RAG_B_ACORDES: [Acorde; 8] = [
    sep(7), sep(7), may(0), may(0),
    sep(7), sep(7), may(0), may(0),
];

/// **El galop infernal** de Offenbach, de "Orfeo en los infiernos" (1858). Lo
/// que todo el mundo llama el cancan.
///
/// Aqui hubo un dembow escrito a mano, y se quito: no casaba con el salon. Lo
/// que se le pedia —un baile que rompiera el programa educado de la sala— ya
/// existia dentro de este mundo y ademas es de dominio publico. El cancan
/// escandalizo a Paris, se bailaba en music-halls y llego a estar prohibido:
/// hace el mismo papel sin salirse de epoca.
///
/// Leido de la primera edicion de la reduccion para canto y piano (Paris,
/// Heugel, sin fecha [1858]), el galop del final del segundo acto. Offenbach
/// murio en 1880.
///
/// Esto es el coro, "Ce bal est original, d'un galop infernal": el tema que se
/// reconoce. En 2/4, y es la unica pieza del juego **en dos tiempos**: un
/// compas son ocho unidades, cada corchea dos. Las cuatro corcheas que bajan y
/// las dos negras repetidas que contestan, y otra vez. El galop va en re mayor,
/// pero el coro esta en sol (todos los do llevan becuadro), y en sol va aqui.
///
/// Dieciseis compases, la vuelta entera con la primera casilla: el ultimo
/// compas acaba en el re de "ce", que es la anacrusa de "bal" y cierra el bucle.
#[rustfmt::skip]
const GALOP_A: [Nota; 48] = [
    n(0, 8),                                                                // 1  bal
    n(2, 2), n(5, 2), n(4, 2), n(2, 2),                                     // 2  est o-ri-gi
    n(7, 4), n(7, 4),                                                       // 3  nal, d'un
    n(7, 2), n(9, 2), n(4, 2), n(5, 2),                                     // 4  ga-lop in-fer
    n(2, 4), n(2, 4),                                                       // 5  nal, don-
    n(2, 2), n(5, 2), n(4, 2), n(2, 2),                                     // 6  nons tous le si
    n(0, 2), n(12, 2), n(11, 2), n(9, 2),                                   // 7  gnal, vi-ve le
    n(7, 2), n(5, 2), n(4, 2), n(2, 2),                                     // 8  ga-lop in-fer
    n(0, 8),                                                                // 9  nal!
    n(2, 2), n(5, 2), n(4, 2), n(2, 2),                                     // 10 Don-nons le si
    n(7, 4), n(7, 4),                                                       // 11 gnal d'un
    n(7, 2), n(9, 2), n(4, 2), n(5, 2),                                     // 12 ga-lop in-fer
    n(2, 4), n(2, 4),                                                       // 13 nal! A-
    n(2, 2), n(5, 2), n(4, 2), n(2, 2),                                     // 14 mis vi-ve le
    n(0, 2), n(7, 2), n(2, 2), n(4, 2),                                     // 15 bal! vi-ve le
    n(0, 4), n(-5, 4),                                                      // 16 bal! ce
];

/// Bajo en octavas al uno y el acorde al dos: el oom-pah de la partitura, que
/// es el `OomPahPah` metido en dos tiempos. Solo sol y re7.
///
/// En el 15 el piano cambia a re7 en la segunda parte; aqui va sol entero,
/// que es lo que hace el bajo en el uno.
#[rustfmt::skip]
const GALOP_A_ACORDES: [Acorde; 16] = [
    may(0), sep(7), may(0), may(0), sep(7), sep(7), may(0), sep(7),
    may(0), sep(7), may(0), may(0), sep(7), sep(7), may(0), may(0),
];

/// La entrada del galop, la de la orquesta antes del coro ("Tutti", *p*), de
/// la misma edicion: en re mayor y en el agudo de flautas y violines.
///
/// Corcheas de punta a punta: el la, y el mi repetido, y el arpegio de sol
/// que sube hasta el re de arriba. Ocho compases con la segunda casilla, que
/// cae en el re para volver a empezar.
#[rustfmt::skip]
const GALOP_B: [Nota; 30] = [
    n(7, 2), n(14, 2), n(14, 2), n(16, 2),                                  // 1
    n(14, 2), n(12, 2), n(12, 2), n(16, 2),                                 // 2
    n(17, 2), n(21, 2), n(24, 2), n(21, 2),                                 // 3
    n(21, 2), n(19, 2), n(19, 4),                                           // 4
    n(21, 2), n(11, 2), n(11, 2), n(21, 2),                                 // 5
    n(19, 2), n(12, 2), n(12, 2), n(16, 2),                                 // 6
    n(16, 2), n(14, 2), n(16, 2), n(14, 2),                                 // 7
    n(14, 2), n(12, 2), n(12, 4),                                           // 8
];

#[rustfmt::skip]
const GALOP_B_ACORDES: [Acorde; 8] = [
    sep(7), may(0), may(5), may(0),
    sep(7), may(0), sep(7), may(0),
];

/// **Asturias (Leyenda)**, Isaac Albeniz (1892), el principio.
///
/// Leida de la primera edicion (Madrid, Casa Dotesio, 1901), que es de piano y
/// esta en sol menor; aqui va una tercera menor abajo, en el mi menor de la
/// guitarra, que es como la tiene todo el mundo en la cabeza. Albeniz murio en
/// 1909.
///
/// En 3/4 y todo en semicorcheas: **una del canto y una del pedal**, alternando
/// sin parar (en el piano, una mano y la otra). El canto es esta tabla, en
/// corcheas, y el pedal lo pone el acompanamiento en la semicorchea de en
/// medio. La primera del compas es un bajo, marcado: mi, y luego si do la si
/// sol, con el si del pedal entre nota y nota. Eso es lo que se reconoce.
///
/// Los tonos son semitonos sobre el mi3.
#[rustfmt::skip]
const ASTURIAS_A: [Nota; 96] = [
    n(0, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 1  mi, y el pulgar: si do la si sol
    n(0, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 2
    n(0, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 3
    n(5, 2), n(2, 2), n(3, 2), n(5, 2), n(2, 2), n(3, 2),// 4  la fa# sol, dos veces
    n(0, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 5
    n(0, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 6
    n(0, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 7
    n(5, 2), n(2, 2), n(3, 2), n(0, 2), n(-2, 2), n(-4, 2),// 8  y baja hasta el do
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 9  sobre el si grave
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 10
    n(-5, 2), n(2, 2), n(5, 2), n(7, 2), n(8, 2), n(10, 2),// 11 sube hasta el re
    n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2), n(2, 2),// 12
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 13
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 14
    n(-5, 2), n(5, 2), n(7, 2), n(8, 2), n(10, 2), n(7, 2),// 15
    n(8, 2), n(5, 2), n(7, 2), n(3, 2), n(7, 2), n(2, 2),// 16 y vuelta al mi
];

#[rustfmt::skip]
const ASTURIAS_A_ACORDES: [Acorde; 16] = [
    men(0), men(0), men(0), men(0),
    men(0), men(0), men(0), men(0),
    may(7), may(7), may(7), may(7),
    may(7), may(7), may(7), may(7),
];

/// La parte fuerte, en *fff*: el mismo canto con el acorde golpeado al uno, un
/// compas sobre el si y otro sobre la menor. En el de la menor el pedal sube
/// del si al do, y ese medio tono que va y viene es el dramatismo de la pieza.
#[rustfmt::skip]
const ASTURIAS_B: [Nota; 96] = [
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 1  si
    n(-7, 2), n(8, 2), n(10, 2), n(7, 2), n(8, 2), n(3, 2),// 2  la menor, el pedal sube al do
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 3
    n(-7, 2), n(8, 2), n(10, 2), n(7, 2), n(8, 2), n(3, 2),// 4
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 5
    n(-7, 2), n(8, 2), n(10, 2), n(7, 2), n(8, 2), n(3, 2),// 6
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 7
    n(5, 2), n(2, 2), n(3, 2), n(5, 2), n(2, 2), n(3, 2),// 8  mi menor
    n(0, 2), n(2, 2), n(3, 2), n(5, 2), n(2, 2), n(3, 2),// 9  la bajada
    n(5, 2), n(2, 2), n(3, 2), n(0, 2), n(-2, 2), n(-4, 2),// 10
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 11 si
    n(-7, 2), n(8, 2), n(10, 2), n(7, 2), n(8, 2), n(3, 2),// 12 la menor
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(2, 2),// 13
    n(-7, 2), n(8, 2), n(10, 2), n(7, 2), n(8, 2), n(3, 2),// 14
    n(-5, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 15
    n(0, 2), n(7, 2), n(8, 2), n(5, 2), n(7, 2), n(3, 2),// 16 y el mi del principio
];

/// Donde empieza la bajada del rasgueo, que es lo que toca el remate.
const ASTURIAS_B_MITAD: usize = 48;

#[rustfmt::skip]
const ASTURIAS_B_ACORDES: [Acorde; 16] = [
    may(7), men(5), may(7), men(5),
    may(7), men(5), may(7), men(0),
    men(0), men(0), may(7), men(5),
    may(7), men(5), may(7), men(0),
];

/// Todo lo que hace falta para sonar.
struct Partitura {
    titulo: &'static str,
    bpm: f32,
    /// Tiempos por compas: tres en un vals, cuatro en un tango.
    tiempos: usize,
    acompanamiento: Acompanamiento,
    /// Semitonos de la tonica sobre La2 (110 Hz).
    tonica: i32,
    melodia: &'static [Nota],
    acordes: &'static [Acorde],
    /// Si se refleja la melodia y la armonia antes de sonar.
    espejo: bool,
    /// Si las corcheas van swingadas: la de a contratiempo llega tarde, a dos
    /// tercios del tiempo en vez de a la mitad. **Es lo unico que separa un
    /// ritmo de jazz de uno que no lo es**, y son tres lineas.
    swing: bool,
}

fn partitura(t: Tema) -> Partitura {
    // El vals: tres tiempos y oom-pah-pah en las cuatro figuras.
    let vals = |titulo, bpm, tonica, melodia, acordes, espejo| Partitura {
        titulo,
        bpm,
        tiempos: 3,
        acompanamiento: Acompanamiento::OomPahPah,
        tonica,
        melodia,
        acordes,
        espejo,
        swing: false,
    };
    // El tango: cuatro tiempos y marcato. Mismo modulo, otra gramatica.
    let tango = |titulo, bpm, tonica, melodia, acordes| Partitura {
        titulo,
        bpm,
        tiempos: 4,
        acompanamiento: Acompanamiento::Marcato,
        tonica,
        melodia,
        acordes,
        espejo: false,
        swing: false,
    };
    // El charleston: cuatro tiempos, stride, y corcheas que llegan tarde.
    let rag = |titulo, bpm, tonica, melodia, acordes| Partitura {
        titulo,
        bpm,
        tiempos: 4,
        acompanamiento: Acompanamiento::Stride,
        tonica,
        melodia,
        acordes,
        espejo: false,
        swing: true,
    };
    // El flamenco: tres tiempos, como el vals, y una guitarra en vez de banda.
    let guitarra = |titulo, bpm, acompanamiento, tonica, melodia, acordes| Partitura {
        titulo,
        bpm,
        tiempos: 3,
        acompanamiento,
        tonica,
        melodia,
        acordes,
        espejo: false,
        swing: false,
    };

    match t {
        // Cada baile ganado le da un poco mas de prisa: la sala vacia va a
        // paso de caja de musica, y la llena, a paso de baile. Poco: es la
        // musica de descansar entre bailes, no la de uno mas.
        Tema::Sala(bailes) => Partitura {
            titulo: "la sala",
            bpm: 88.0 + 5.0 * bailes.count_ones() as f32,
            tiempos: 3,
            acompanamiento: Acompanamiento::Sala(bailes),
            tonica: 22, // sol
            melodia: &SALA,
            acordes: &SALA_ACORDES,
            espejo: false,
            swing: false,
        },

        Tema::PasoBase => vals(
            "El Danubio azul - Johann Strauss II, 1866",
            174.0,
            15, // do central
            &DANUBIO,
            &DANUBIO_ACORDES,
            false,
        ),
        Tema::Espejo => vals(
            "El Danubio azul, reflejado",
            132.0,
            12, // la
            &DANUBIO[DANUBIO_MITAD..],
            &DANUBIO_ACORDES[16..],
            true,
        ),
        Tema::Molinete => vals(
            "Sobre las olas - Juventino Rosas, 1888",
            192.0,
            22, // sol
            &OLAS,
            &OLAS_ACORDES,
            false,
        ),
        // La coda no es una pieza nueva: es la segunda mitad del Danubio otra
        // vez, un tono mas arriba y disparada. En musica una coda es eso —el
        // tema del principio, acelerado, para cerrar—, y aqui ademas hace que
        // la ultima figura suene a que ya has estado ahi antes.
        Tema::Coda => vals(
            "El Danubio azul, coda",
            232.0,
            17, // re: un tono por encima del principio
            &DANUBIO[DANUBIO_MITAD..],
            &DANUBIO_ACORDES[16..],
            false,
        ),

        Tema::Caminata => tango(
            "La Cumparsita - Matos Rodriguez, 1916",
            120.0,
            24, // la
            &CUMPARSITA_A,
            &CUMPARSITA_A_ACORDES,
        ),
        Tema::Corte => tango(
            "La Cumparsita, segunda parte",
            100.0,
            24,
            &CUMPARSITA_B,
            &CUMPARSITA_B_ACORDES,
        ),
        // Mismo truco que la coda del vals, y por el mismo motivo: un tango
        // tambien vuelve al tema del principio para cerrar.
        Tema::Quebrada => tango(
            "La Cumparsita, quebrada",
            152.0,
            26,
            &CUMPARSITA_A,
            &CUMPARSITA_A_ACORDES,
        ),

        Tema::CharlestonA => rag(
            "Maple Leaf Rag - Scott Joplin, 1899",
            100.0,
            20, // fa
            &RAG_A,
            &RAG_A_ACORDES,
        ),
        Tema::CharlestonB => rag(
            "Maple Leaf Rag, segunda parte",
            92.0,
            20,
            &RAG_B,
            &RAG_B_ACORDES,
        ),
        // Y otra vez la primera, un tono arriba y corriendo. Los tres bailes
        // cierran igual, y no es pereza: volver al tema del principio para
        // acabar es lo que hacen las tres musicas de verdad.
        // El cancan. `OomPahPah` en DOS tiempos es exactamente el oom-pah de
        // un galop: grave al uno, acorde al dos. El mismo acompanamiento del
        // vals metido en un compas de dos suena a otra cosa entera, que es lo
        // que hace que no haga falta inventarse uno nuevo.
        Tema::Cancan => Partitura {
            titulo: "El galop infernal - Offenbach, 1858",
            bpm: 150.0,
            tiempos: 2,
            acompanamiento: Acompanamiento::OomPahPah,
            tonica: 22, // sol, sol4: el coro en su sitio
            melodia: &GALOP_A,
            acordes: &GALOP_A_ACORDES,
            espejo: false,
            swing: false,
        },
        Tema::Galop => Partitura {
            titulo: "El galop infernal, segunda parte",
            bpm: 150.0,
            tiempos: 2,
            acompanamiento: Acompanamiento::OomPahPah,
            tonica: 17, // re: la orquesta toca en el tono del galop
            melodia: &GALOP_B,
            acordes: &GALOP_B_ACORDES,
            espejo: false,
            swing: false,
        },
        // Los cuatro bailes cierran igual: el tema del principio, un tono
        // arriba y mas rapido. Aqui ademas es lo que hace un galop de verdad,
        // que es una carrera hasta que alguien se cae.
        Tema::Infernal => Partitura {
            titulo: "El galop infernal, desbocado",
            bpm: 180.0,
            tiempos: 2,
            acompanamiento: Acompanamiento::OomPahPah,
            tonica: 24, // la: un tono por encima del coro
            melodia: &GALOP_A,
            acordes: &GALOP_A_ACORDES,
            espejo: false,
            swing: false,
        },

        Tema::CharlestonFin => rag(
            "Maple Leaf Rag, al galope",
            128.0,
            22,
            &RAG_A,
            &RAG_A_ACORDES,
        ),

        // A 150, veinticuatro ticks por tiempo: el compas del quiebro y el
        // del paseo de Triana (`tiempo: 24`), que suena con ella. Es tambien
        // el aire al que se toca de verdad, negra a 140 o 150; mas deprisa
        // el pulgar deja de cantar y se oye solo el si.
        Tema::Llamada => guitarra(
            "Asturias (Leyenda) - Isaac Albeniz, 1892",
            150.0,
            Acompanamiento::Toque,
            7, // mi3, el de la guitarra
            &ASTURIAS_A,
            &ASTURIAS_A_ACORDES,
        ),
        Tema::Rasgueo => guitarra(
            "Asturias, rasgueado",
            150.0,
            Acompanamiento::Rasgueo,
            7,
            &ASTURIAS_B,
            &ASTURIAS_B_ACORDES,
        ),
        // La llamada otra vez, con el pedal en octavas como en el crescendo
        // de la partitura, una octava arriba y mas deprisa, con los tacones en
        // el dos y el tres: ya no llama, taconea.
        Tema::Zapateado => guitarra(
            "Asturias, zapateado",
            184.0,
            Acompanamiento::Taconeo,
            19, // mi4
            &ASTURIAS_A,
            &ASTURIAS_A_ACORDES,
        ),
        // Y se cierra como los otros cuatro, volviendo a lo que ya ha sonado,
        // un tono arriba y mas rapido: la bajada y la ultima vuelta de la
        // parte fuerte.
        Tema::Remate => guitarra(
            "Asturias, remate",
            176.0,
            Acompanamiento::Rasgueo,
            9, // fa#
            &ASTURIAS_B[ASTURIAS_B_MITAD..],
            &ASTURIAS_B_ACORDES[8..],
        ),
    }
}

// ---------------------------------------------------------------------------
// El reflejo
// ---------------------------------------------------------------------------

/// Semitonos de cada grado de la escala mayor.
const MAYOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
/// Y de la menor natural.
const MENOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

/// A que grado de la escala mayor corresponde un semitono.
///
/// Lo que no cae en la escala baja al grado inmediatamente inferior. En el
/// Danubio eso solo le pasa a un fa sostenido de paso, en el compas 27.
fn grado_mayor(semitonos: i32) -> i32 {
    let dentro = semitonos.rem_euclid(12);
    let mut g = 0;
    for (i, s) in MAYOR.iter().enumerate() {
        if *s <= dentro {
            g = i as i32;
        }
    }
    semitonos.div_euclid(12) * 7 + g
}

/// El camino de vuelta, pero por la escala menor.
fn semitono_menor(grado: i32) -> i32 {
    MENOR[grado.rem_euclid(7) as usize] + grado.div_euclid(7) * 12
}

/// Refleja una nota alrededor de la tonica: lo que subia, baja.
///
/// Se hace por grados y no por semitonos a proposito. Invertir semitonos da
/// una melodia atonal, que suena a error; invertir grados y leerlos en menor
/// da una melodia que sigue teniendo tonalidad, solo que del reves.
///
/// Las dos octavas de mas son para que el reflejo no acabe metido en el
/// registro del bajo: al invertir, lo agudo se vuelve grave.
fn reflejar(semitonos: i32) -> i32 {
    semitono_menor(-grado_mayor(semitonos)) + 24
}

/// Lo mismo con un acorde.
///
/// La calidad no se conserva: la manda el modo. En menor natural, i, iv y v
/// son menores, y bIII, bVI y bVII mayores. Reflejando el Danubio salen
/// i - v - iv - bVII, que es una progresion menor de las de toda la vida.
fn reflejar_acorde(a: Acorde) -> Acorde {
    let raiz = semitono_menor(-grado_mayor(a.raiz)).rem_euclid(12);
    let triada = match raiz {
        3 | 8 | 10 => Triada::Mayor,
        _ => Triada::Menor,
    };
    Acorde { raiz, triada }
}

// ---------------------------------------------------------------------------
// De partitura a voces
// ---------------------------------------------------------------------------

/// Frecuencia de una nota en semitonos sobre La2 (110 Hz).
pub(crate) fn frecuencia(semitonos: i32) -> f32 {
    // 2^(n/12), sin `powf`: se hace con multiplicaciones para no depender de
    // la libm, igual que el resto del proyecto.
    let mut f = 110.0f32;
    let (mut cuantos, paso) = if semitonos >= 0 {
        (semitonos, 1.059_463_1f32)
    } else {
        (-semitonos, 0.943_874_3f32)
    };
    while cuantos > 0 {
        f *= paso;
        cuantos -= 1;
    }
    f
}

/// Baja el bajo a su octava. Sea cual sea el tono de la pieza, el "oom" suena
/// entre 69 y 130 Hz: si no, al transportar se sube al registro de la melodia.
fn bajo(mut s: i32) -> i32 {
    while s > 3 {
        s -= 12;
    }
    while s < -8 {
        s += 12;
    }
    s
}

/// Y coloca la triada siempre en la misma octava, para que el acompanamiento
/// no pegue saltos de registro al cambiar de acorde.
fn registro(mut s: i32) -> i32 {
    while s < 12 {
        s += 12;
    }
    while s >= 24 {
        s -= 12;
    }
    s
}

/// Unidades que dura un compas de esta partitura.
/// El pedal de Asturias sobre un acorde: la nota suya mas cerca del si (el
/// quinto grado), en semitonos sobre la tonica. Es el si en mi menor y en si
/// mayor, y el do en la menor.
fn pedal(a: Acorde) -> i32 {
    a.intervalos()
        .iter()
        .map(|iv| (a.raiz + iv).rem_euclid(12))
        .min_by_key(|n| ((n - 7).abs(), -n))
        .unwrap_or(7)
}

fn unidades_por_compas(p: &Partitura) -> u32 {
    UNIDADES_POR_TIEMPO * p.tiempos as u32
}

/// El compas de un tema: **ticks por tiempo y tiempos por compas**.
///
/// Lo necesita la escenografia para latir con la musica. Sale de la misma
/// partitura que suena, asi que la sala respira con lo que se oye sin analizar
/// una sola muestra de audio: los dos numeros vienen del mismo sitio.
pub fn compas(t: Tema) -> (f32, usize) {
    let p = partitura(t);
    (60.0 * vals_core::TICK_HZ as f32 / p.bpm, p.tiempos)
}

/// La triada de la tonica de un tema, en semitonos sobre La2: tonica,
/// tercera y quinta.
///
/// Es lo que necesitan los efectos que suenan **dentro** de la musica (la
/// fase, la ficha, la victoria) para sonar en el tono de lo que se oye. El
/// modo no esta apuntado en ninguna parte y no hace falta: lo dice el primer
/// acorde sobre la tonica que toca la pieza, ya reflejado si es el espejo.
pub fn triada_tonica(t: Tema) -> [i32; 3] {
    let p = partitura(t);
    let menor = p
        .acordes
        .iter()
        .map(|a| if p.espejo { reflejar_acorde(*a) } else { *a })
        .find(|a| a.raiz == 0)
        .is_some_and(|a| a.triada == Triada::Menor);
    [p.tonica, p.tonica + if menor { 3 } else { 4 }, p.tonica + 7]
}

/// Duracion del bucle completo.
pub fn duracion(t: Tema) -> f32 {
    let p = partitura(t);
    let unidades = p.acordes.len() as u32 * unidades_por_compas(&p);
    unidades as f32 * (60.0 / p.bpm) / UNIDADES_POR_TIEMPO as f32
}

/// Construye un tema como lista de voces.
pub fn tema(t: Tema) -> Vec<Voz> {
    let p = partitura(t);
    let tiempo = 60.0 / p.bpm;
    let u = tiempo / UNIDADES_POR_TIEMPO as f32;
    let mut v = Vec::with_capacity(512);

    acompanar(&mut v, &p, tiempo);
    let sala = match p.acompanamiento {
        Acompanamiento::Sala(bailes) => Some(bailes),
        _ => None,
    };
    let guitarra = matches!(
        p.acompanamiento,
        Acompanamiento::Toque | Acompanamiento::Rasgueo | Acompanamiento::Taconeo
    );
    if sala.is_some_and(|b| b & TANGO != 0) {
        bandoneon(&mut v, &p, u);
    }

    // --- Melodia.
    let mut unidad = 0u32;
    for nota in p.melodia {
        if nota.tono != SILENCIO {
            let tono = if p.espejo {
                reflejar(nota.tono)
            } else {
                nota.tono
            };
            // Las notas largas caen mas despacio; con la caida corta de las
            // breves se apagarian antes de tiempo y la frase se rompe.
            let decay = if nota.unidades > 4 { 1.2 } else { 2.4 };
            // El swing. La corchea de a contratiempo no cae en la mitad del
            // tiempo: cae a dos tercios. Con cuatro unidades por tiempo, la
            // corchea recta empieza en la 2 y la swingada en la 2,67.
            //
            // Son tres lineas y es **lo unico** que separa un ritmo de jazz de
            // uno que no lo es. La tabla de notas es la misma.
            let atraso = if p.swing && unidad % UNIDADES_POR_TIEMPO == UNIDADES_POR_TIEMPO / 2 {
                2.0 / 3.0
            } else {
                0.0
            };
            let dur = (nota.unidades as f32 - atraso) * u;
            let f = frecuencia(p.tonica + tono);
            let cuando = (unidad as f32 + atraso) * u;
            match sala {
                // El canto va acentuado por encima del pedal, que es lo que
                // hace que se oiga una melodia y no un redoble, y doblado a la
                // octava: solo, en el registro de la guitarra, se perdia
                // debajo del pedal y en un altavoz pequeno no se oia.
                None if guitarra => {
                    pulsar(&mut v, f, dur * 0.92, 0.56, 3.0, cuando);
                    pulsar(&mut v, f * 2.0, dur * 0.7, 0.20, 4.0, cuando);
                }
                // El tango, mas fuerte: con la melodia picada, la mitad de
                // cada golpe es silencio, y a 0,22 se quedaba por debajo de
                // los demas bailes.
                None if p.acompanamiento == Acompanamiento::Marcato => {
                    v.push(Voz::nota(Wave::Sine, f, dur * 0.92, 0.34, decay).tras(cuando));
                }
                None => v.push(Voz::nota(Wave::Sine, f, dur * 0.92, 0.22, decay).tras(cuando)),
                // La caja de musica: el mismo seno, y su octava apagandose
                // enseguida, que es la pua.
                Some(bailes) => {
                    v.push(Voz::nota(Wave::Sine, f, dur * 0.92, 0.29, decay).tras(cuando));
                    v.push(Voz::nota(Wave::Sine, f * 2.0, dur * 0.4, 0.07, 5.0).tras(cuando));
                    // Y la trompeta del cancan doblandola: con la banda, la
                    // cancion deja de ser una nana.
                    if bailes & CANCAN != 0 {
                        v.push(Voz::nota(Wave::Square, f, dur * 0.85, 0.03, decay).tras(cuando));
                    }
                }
            }
        }
        unidad += nota.unidades;
    }

    v
}

/// Una cuerda pulsada: el seno, y una sierra por debajo que se apaga mucho
/// antes, que es el chasquido del dedo contra la cuerda. Sin ella, la guitarra
/// es la caja de musica.
fn pulsar(v: &mut Vec<Voz>, f: f32, dur: f32, vol: f32, decay: f32, cuando: f32) {
    v.push(Voz::nota(Wave::Sine, f, dur, vol, decay).tras(cuando));
    v.push(Voz::nota(Wave::Saw, f, dur * 0.3, vol * 0.2, 4.0).tras(cuando));
}

/// El bandoneon que el tango devuelve a la sala. Dos sierras casi a la par:
/// la lengueta y su batido, que es por lo que un fuelle no suena a organo.
fn bandoneon(v: &mut Vec<Voz>, p: &Partitura, u: f32) {
    let mut unidad = 0u32;
    for nota in &SALA_BANDONEON {
        let f = frecuencia(p.tonica + nota.tono);
        let dur = nota.unidades as f32 * u * 0.95;
        for f in [f, f * 1.006] {
            v.push(Voz::nota(Wave::Saw, f, dur, 0.05, 0.7).tras(unidad as f32 * u));
        }
        unidad += nota.unidades;
    }
}

/// El acompanamiento, compas a compas.
///
/// Es donde vive la diferencia entre un vals y un tango. La melodia se toca
/// igual en los dos; lo que cambia —y lo que se reconoce antes de oir ninguna
/// nota principal— es donde caen el grave y el acorde.
fn acompanar(v: &mut Vec<Voz>, p: &Partitura, t: f32) {
    for (compas, acorde) in p.acordes.iter().enumerate() {
        let a = if p.espejo {
            reflejar_acorde(*acorde)
        } else {
            *acorde
        };
        let t0 = compas as f32 * p.tiempos as f32 * t;
        let raiz = p.tonica + a.raiz;
        let grave = frecuencia(bajo(raiz));
        let triada = || {
            a.intervalos()
                .iter()
                .map(move |iv| frecuencia(registro(raiz) + iv))
        };

        match p.acompanamiento {
            // Grave al uno, acorde al dos y al tres. Esto *es* un vals: es lo
            // que hace que se reconozca el genero antes de la melodia, y lo
            // que contesta al arpegio del Danubio con su "pam, pam".
            Acompanamiento::OomPahPah => {
                v.push(Voz::nota(Wave::Sine, grave, t * 0.9, 0.30, 1.6).tras(t0));
                for paso in 1..p.tiempos {
                    let cuando = t0 + paso as f32 * t;
                    for f in triada() {
                        v.push(Voz::nota(Wave::Square, f, t * 0.35, 0.055, 3.2).tras(cuando));
                    }
                }
            }
            // Grave corto en los tiempos fuertes —uno y tres— y golpe seco de
            // acorde en los debiles. No mece: marca. Las caidas son mas
            // bruscas que en el vals a proposito: un tango es staccato.
            Acompanamiento::Marcato => {
                for paso in (0..p.tiempos).step_by(2) {
                    let cuando = t0 + paso as f32 * t;
                    v.push(Voz::nota(Wave::Sine, grave, t * 0.45, 0.38, 3.0).tras(cuando));
                }
                for paso in (1..p.tiempos).step_by(2) {
                    let cuando = t0 + paso as f32 * t;
                    for f in triada() {
                        v.push(Voz::nota(Wave::Square, f, t * 0.22, 0.09, 4.5).tras(cuando));
                    }
                }
            }
            // El bajo anda: fundamental en el uno, quinta en el tres, y el
            // acorde contestando en los tiempos debiles. Cae donde el marcato
            // del tango, pero no suena igual, porque un bajo que se mueve no
            // es lo mismo que uno que se repite.
            Acompanamiento::Stride => {
                for paso in (0..p.tiempos).step_by(2) {
                    let cuando = t0 + paso as f32 * t;
                    let nota_baja = if paso == 0 { raiz } else { raiz + 7 };
                    let f = frecuencia(bajo(nota_baja));
                    v.push(Voz::nota(Wave::Sine, f, t * 0.6, 0.30, 2.2).tras(cuando));
                }
                for paso in (1..p.tiempos).step_by(2) {
                    let cuando = t0 + paso as f32 * t;
                    for f in triada() {
                        v.push(Voz::nota(Wave::Square, f, t * 0.4, 0.05, 2.6).tras(cuando));
                    }
                }
            }
            // El pedal de Asturias: la semicorchea de en medio, entre nota y
            // nota del canto, de principio a fin. Es el si, la dominante,
            // menos en los compases de la menor de la parte fuerte, donde sube
            // al do: la nota del acorde mas cerca del si (`pedal`).
            //
            // Sin grave: el bajo lo pone el canto en la primera del compas.
            // El pedal va bajito y se apaga enseguida, que es como suena una
            // cuerda punteada.
            Acompanamiento::Toque | Acompanamiento::Rasgueo | Acompanamiento::Taconeo => {
                let u = t / UNIDADES_POR_TIEMPO as f32;
                let fuerte = p.acompanamiento != Acompanamiento::Toque;
                let ped = p.tonica + pedal(a);
                for k in (0..unidades_por_compas(p)).filter(|k| !k.is_multiple_of(2)) {
                    let cuando = t0 + k as f32 * u;
                    pulsar(v, frecuencia(ped), u * 0.9, 0.12, 4.0, cuando);
                    // En *fff*, en octavas.
                    if fuerte {
                        pulsar(v, frecuencia(ped + 12), u * 0.9, 0.06, 4.0, cuando);
                    }
                }
                match p.acompanamiento {
                    // El acorde en cada tiempo, mas fuerte en el uno, y cuerda
                    // a cuerda con unos milisegundos entre una y otra: un
                    // rasgueado no cae de golpe como el acorde de un piano, se
                    // arrastra. Y un bajo al uno, que es el fff de la partitura.
                    Acompanamiento::Rasgueo => {
                        v.push(Voz::nota(Wave::Sine, grave, t * 1.5, 0.20, 2.0).tras(t0));
                        for paso in 0..p.tiempos {
                            let cuando = t0 + paso as f32 * t;
                            let vol = if paso == 0 { 0.09 } else { 0.06 };
                            for (i, f) in triada().enumerate() {
                                pulsar(v, f, t * 0.7, vol, 2.0, cuando + i as f32 * 0.012);
                            }
                        }
                    }
                    // Golpes cortos en el dos y el tres, todas las cuerdas a la
                    // vez: no se arrastra, taconea.
                    Acompanamiento::Taconeo => {
                        for paso in 1..p.tiempos {
                            let cuando = t0 + paso as f32 * t;
                            for f in triada() {
                                pulsar(v, f, t * 0.22, 0.10, 6.0, cuando);
                            }
                        }
                    }
                    _ => {}
                }
            }
            // La caja sola no marca el compas: una pua grave al uno y nada
            // mas. Lo demas lo traen los bailes, cada uno en su sitio del
            // compas, para que se sumen sin pisarse.
            Acompanamiento::Sala(bailes) => {
                let largo = p.tiempos as f32 * t;
                v.push(Voz::nota(Wave::Sine, grave, largo * 0.8, 0.14, 2.5).tras(t0));
                // El vals: el pah-pah del dos y el tres. Es lo que convierte la
                // cancion de la caja en un vals.
                if bailes & VALS != 0 {
                    for paso in 1..p.tiempos {
                        let cuando = t0 + paso as f32 * t;
                        for f in triada() {
                            v.push(Voz::nota(Wave::Sine, f, t * 0.4, 0.09, 3.0).tras(cuando));
                        }
                    }
                }
                // El charleston: un piano desafinado entre golpe y golpe, y
                // swingado, a dos tercios del tiempo. Las dos sierras casi
                // iguales son el piano de bar.
                if bailes & CHARLESTON != 0 {
                    for paso in 0..p.tiempos - 1 {
                        let cuando = t0 + (paso as f32 + 2.0 / 3.0) * t;
                        for f in triada() {
                            for f in [f, f * 1.008] {
                                v.push(Voz::nota(Wave::Saw, f, t * 0.4, 0.025, 3.0).tras(cuando));
                            }
                        }
                    }
                }
                // El cancan: el golpe de metales al uno, encima de la pua.
                //
                // Las ondas cuadradas y las sierras de la sala van a la mitad
                // que en los bailes: a su volumen de baile, la sala llena se
                // oia estridente justo al salir de ganar uno.
                if bailes & CANCAN != 0 {
                    for f in triada() {
                        v.push(Voz::nota(Wave::Square, f, t * 0.6, 0.03, 2.5).tras(t0));
                    }
                }
                // El flamenco: la guitarra arpegiando el acorde hacia arriba,
                // las tres cuerdas en cada tiempo, en tresillo, como el pulgar
                // y los dos dedos de Asturias. Punteada y al volumen del
                // pah-pah del vals: se tiene que notar que ha vuelto, pero
                // desde la otra punta del patio, no desde el tablao.
                if bailes & FLAMENCO != 0 {
                    for paso in 0..p.tiempos {
                        for (i, f) in triada().take(3).enumerate() {
                            let cuando = t0 + (paso as f32 + i as f32 / 3.0) * t;
                            pulsar(v, f, t * (1.0 - i as f32 / 3.0) * 0.95, 0.09, 2.5, cuando);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Todos los temas, y la sala con cada combinacion de bailes ganados.
    fn todos() -> Vec<Tema> {
        TEMAS.into_iter().chain((1..32).map(Tema::Sala)).collect()
    }

    #[test]
    fn la_melodia_y_la_armonia_cuadran() {
        // Un acorde por compas y doce unidades por compas. Si una transcripcion
        // se descuadra por una corchea, la melodia se va desplazando del
        // acompanamiento y el tema se deshace a la mitad. Aqui salta antes.
        for b in todos() {
            let p = partitura(b);
            let unidades: u32 = p.melodia.iter().map(|n| n.unidades).sum();
            assert_eq!(
                unidades,
                p.acordes.len() as u32 * unidades_por_compas(&p),
                "{b:?}: la melodia no dura los compases que dice la armonia"
            );
        }
    }

    #[test]
    fn el_danubio_empieza_por_donde_tiene_que_empezar() {
        // El arpegio 1-3-5 y la quinta sostenida. Es *la* figura reconocible
        // del Danubio; si alguien la toca al editar la tabla, esto avisa.
        assert_eq!(DANUBIO[0].tono, 0);
        assert_eq!(DANUBIO[1].tono, 4);
        assert_eq!(DANUBIO[2].tono, 7);
        assert_eq!(DANUBIO[3].tono, 7);
        assert_eq!(DANUBIO[3].unidades, 2 * UNIDADES_POR_TIEMPO);
    }

    #[test]
    fn la_mitad_del_danubio_cae_en_un_compas() {
        let unidades: u32 = DANUBIO[..DANUBIO_MITAD].iter().map(|n| n.unidades).sum();
        assert_eq!(unidades, 16 * 3 * UNIDADES_POR_TIEMPO);
    }

    #[test]
    fn el_reflejo_da_la_vuelta_a_la_melodia() {
        // Lo que sube, baja: el reflejo invierte el orden de los tonos.
        let arpegio = [0, 4, 7, 12, 24];
        let reflejado: Vec<i32> = arpegio.iter().map(|s| reflejar(*s)).collect();
        for par in reflejado.windows(2) {
            assert!(par[1] < par[0], "el reflejo no baja: {reflejado:?}");
        }
        // Y la tonica sigue siendo la tonica.
        assert_eq!(reflejar(0).rem_euclid(12), 0);
        assert_eq!(reflejar(24).rem_euclid(12), 0);
    }

    #[test]
    fn el_reflejo_del_danubio_da_una_progresion_menor() {
        // do - sol7 - fa - re menor, reflejado, sale i - iv - v - bVII.
        let raices: Vec<i32> = [may(0), sep(7), may(5), men(2)]
            .iter()
            .map(|a| reflejar_acorde(*a).raiz)
            .collect();
        assert_eq!(raices, vec![0, 5, 7, 10]);
        // Y las calidades las manda el modo, no el acorde de partida.
        assert!(reflejar_acorde(sep(7)).triada == Triada::Menor);
        assert!(reflejar_acorde(men(2)).triada == Triada::Mayor);
    }

    #[test]
    fn ninguna_nota_se_sale_del_bucle() {
        // Si una nota terminase despues del bucle, el bucle daria un salto.
        for b in todos() {
            let fin = duracion(b);
            for voz in tema(b) {
                assert!(
                    voz.fin() <= fin + 0.001,
                    "{b:?}: una nota acaba en {} y el bucle dura {fin}",
                    voz.fin()
                );
            }
        }
    }

    #[test]
    fn las_frecuencias_son_audibles() {
        for b in todos() {
            for voz in tema(b) {
                let f = voz.frecuencia();
                assert!(
                    (40.0..2000.0).contains(&f),
                    "{b:?}: frecuencia rara, {f} Hz"
                );
            }
        }
    }

    #[test]
    fn el_bajo_y_la_melodia_no_se_pisan() {
        // El bajo vive por debajo de 150 Hz y la melodia por encima de 200.
        // Es lo que deja sitio en medio para los acordes.
        //
        // Menos en Asturias: el canto de la guitarra baja por la cuerda de re
        // hasta el mi3, que es justo ese hueco, y el bajo lo pone el propio
        // canto. Subirlo una octava para salvarlo era lo que la hacia
        // irreconocible.
        for b in todos() {
            if matches!(
                partitura(b).acompanamiento,
                Acompanamiento::Toque | Acompanamiento::Rasgueo | Acompanamiento::Taconeo
            ) {
                continue;
            }
            for voz in tema(b) {
                let f = voz.frecuencia();
                assert!(
                    !(150.0..200.0).contains(&f),
                    "{b:?}: {f} Hz se mete en tierra de nadie"
                );
            }
        }
    }

    /// En que tiempo de su compas cae cada grave.
    ///
    /// Se redondea al tiempo mas cercano en vez de comparar segundos: los
    /// graves caen justo en los tiempos, y en el compas 30 el error de coma
    /// flotante ya es bastante para que un cero se lea como un "casi tres".
    fn graves_por_compas(t: Tema) -> Vec<u32> {
        let p = partitura(t);
        let tiempo = 60.0 / p.bpm;
        tema(t)
            .iter()
            .filter(|v| v.frecuencia() < 150.0)
            .map(|v| (v.inicio() / tiempo).round() as u32 % p.tiempos as u32)
            .collect()
    }

    #[test]
    fn el_vals_va_de_tres_y_el_tango_de_cuatro() {
        assert_eq!(partitura(Tema::PasoBase).tiempos, 3);
        assert_eq!(partitura(Tema::Caminata).tiempos, 4);
    }

    #[test]
    fn en_el_vals_el_grave_cae_solo_en_el_uno() {
        // Un grave por compas y siempre en el primer tiempo: es lo que hace
        // que se oiga como un vals y no como cualquier otra cosa.
        for t in [Tema::PasoBase, Tema::Espejo, Tema::Molinete, Tema::Coda] {
            let p = partitura(t);
            let graves = graves_por_compas(t);
            assert_eq!(graves.len(), p.acordes.len(), "{t:?}");
            for (i, sitio) in graves.iter().enumerate() {
                assert_eq!(*sitio, 0, "{t:?}: el grave {i} no cae en el uno");
            }
        }
    }

    #[test]
    fn en_el_tango_el_grave_cae_en_el_uno_y_en_el_tres() {
        // El marcato. Es lo que separa un tango de un vals antes de que entre
        // ninguna melodia: no mece en tres, marca en cuatro.
        //
        // El charleston cae en los mismos tiempos —el stride tambien va a uno
        // y tres— y por eso lo que lo distingue no es donde, es que el bajo se
        // mueve. Ver `el_bajo_del_charleston_anda`.
        for t in [Tema::Caminata, Tema::Corte, Tema::Quebrada] {
            let p = partitura(t);
            let graves = graves_por_compas(t);
            assert_eq!(graves.len(), p.acordes.len() * 2, "{t:?}: dos por compas");
            for (i, sitio) in graves.iter().enumerate() {
                let esperado = if i % 2 == 0 { 0 } else { 2 };
                assert_eq!(*sitio, esperado, "{t:?}: el grave {i} esta fuera de sitio");
            }
        }
    }

    #[test]
    fn el_bajo_del_charleston_anda() {
        // Stride: fundamental en el uno, quinta en el tres. Cae donde el
        // marcato del tango, pero un bajo que se mueve no suena como uno que
        // se repite, y esa es toda la diferencia al oido.
        for t in [Tema::CharlestonA, Tema::CharlestonB, Tema::CharlestonFin] {
            let p = partitura(t);
            assert!(p.acompanamiento == Acompanamiento::Stride);
            assert_eq!(graves_por_compas(t), {
                let mut v = Vec::new();
                for _ in 0..p.acordes.len() {
                    v.push(0);
                    v.push(2);
                }
                v
            });
            // Y los dos graves de un compas no son la misma nota.
            let tiempo = 60.0 / p.bpm;
            let graves: Vec<f32> = tema(t)
                .iter()
                .filter(|v| v.frecuencia() < 150.0)
                .map(|v| v.frecuencia())
                .collect();
            assert!(
                graves
                    .chunks(2)
                    .any(|par| par.len() == 2 && par[0] != par[1]),
                "el bajo no se mueve: no es stride, es marcato"
            );
            assert!(tiempo > 0.0);
        }
    }

    #[test]
    fn las_corcheas_del_charleston_llegan_tarde() {
        // Lo que convierte el ragtime en jazz. La corchea de a contratiempo
        // tiene que caer a dos tercios del tiempo, no a la mitad: si cayera en
        // la mitad, esto seria una pianola.
        let p = partitura(Tema::CharlestonA);
        assert!(p.swing);
        let tiempo = 60.0 / p.bpm;
        let corchea = tiempo / 2.0;

        let notas: Vec<f32> = tema(Tema::CharlestonA)
            .iter()
            .filter(|v| v.frecuencia() > 200.0)
            .map(|v| (v.inicio() / corchea) % 2.0)
            .collect();
        // Las que caen a contratiempo estan en 1,33 corcheas y no en 1.
        let tarde = notas.iter().filter(|x| (**x - 1.333).abs() < 0.02).count();
        let rectas = notas.iter().filter(|x| (**x - 1.0).abs() < 0.02).count();
        assert!(tarde > 10, "casi ninguna corchea va swingada: {tarde}");
        assert_eq!(rectas, 0, "hay corcheas cayendo en la mitad del tiempo");

        // Y el vals no swinga: sus corcheas caen donde toca.
        assert!(!partitura(Tema::PasoBase).swing);
    }

    #[test]
    fn el_cancan_es_el_unico_de_dos_tiempos() {
        // Es lo que le da su aire, y no cuesta una linea de sintesis: el mismo
        // `OomPahPah` del vals metido en un compas de dos es el oom-pah de un
        // galop. Si esto se va a cuatro, el cancan pasa a sonar a marcha.
        for t in TEMAS {
            let esperado = match t {
                Tema::Cancan | Tema::Galop | Tema::Infernal => 2,
                Tema::Sala(_) | Tema::PasoBase | Tema::Espejo | Tema::Molinete | Tema::Coda => 3,
                Tema::Llamada | Tema::Rasgueo | Tema::Zapateado | Tema::Remate => 3,
                _ => 4,
            };
            assert_eq!(partitura(t).tiempos, esperado, "{t:?}");
        }
    }

    #[test]
    fn no_hay_percusion_en_ninguna_pieza() {
        // Hubo un dembow con bombo y caja de ruido, y se fue con el. Los cuatro
        // bailes que quedan son de banda de salon: meter ruido aqui seria
        // ponerle una bateria a un vals. El `Wave::Noise` sigue siendo de los
        // efectos, que es de donde salio.
        for t in TEMAS {
            assert!(
                !tema(t).iter().any(|v| matches!(v.forma(), Wave::Noise)),
                "{t:?}: percusion en la musica"
            );
        }
    }

    #[test]
    fn la_sala_vacia_no_suena_a_ningun_baile() {
        // Sin bailes ganados es la caja de musica sola: ni golpes secos, ni
        // sierras, ni pah-pah. Si algun dia alguien le pone un compas, la
        // pista vuelve a decir que el juego es ese baile.
        let voces = tema(Tema::Sala(0));
        assert!(
            voces.iter().all(|v| matches!(v.forma(), Wave::Sine)),
            "la sala vacia es una caja de musica"
        );
        let caja = partitura(Tema::Sala(0)).melodia.iter();
        let notas = caja.filter(|n| n.tono != SILENCIO).count();
        let graves = graves_por_compas(Tema::Sala(0)).len();
        assert_eq!(voces.len(), graves + 2 * notas, "algo suena de mas");
        // Y lo que se le sume despues no le cambia el bajo: una pua por
        // compas, con los bailes que sean.
        for bailes in 0..32 {
            let p = partitura(Tema::Sala(bailes));
            assert_eq!(graves_por_compas(Tema::Sala(bailes)).len(), p.acordes.len());
        }
        let bandoneon: u32 = SALA_BANDONEON.iter().map(|n| n.unidades).sum();
        assert_eq!(bandoneon, 16 * 3 * UNIDADES_POR_TIEMPO);
    }

    #[test]
    fn la_sala_se_llena_con_cada_baile_ganado() {
        // Cuanto suena por segundo: la suma de volumen por duracion de cada
        // voz. Cualquier baile que se gane, se gane cuando se gane, tiene que
        // dejar la sala mas llena y mas deprisa que antes.
        let lleno = |b: u8| {
            let t = Tema::Sala(b);
            let voces = tema(t);
            let energia: f32 = voces
                .iter()
                .map(|v| v.volumen() * (v.fin() - v.inicio()))
                .sum();
            (voces.len() as f32 / duracion(t), energia / duracion(t))
        };
        for antes in 0..32u8 {
            for baile in [VALS, TANGO, CHARLESTON, CANCAN, FLAMENCO] {
                if antes & baile != 0 {
                    continue;
                }
                let (n0, e0) = lleno(antes);
                let (n1, e1) = lleno(antes | baile);
                assert!(n1 > n0, "{antes:05b} + {baile:05b}: no hay mas notas");
                assert!(e1 > e0, "{antes:05b} + {baile:05b}: no suena mas");
                assert!(
                    partitura(Tema::Sala(antes | baile)).bpm > partitura(Tema::Sala(antes)).bpm
                );
            }
        }
    }

    #[test]
    fn el_bucle_dura_los_compases_que_dice() {
        for b in todos() {
            let p = partitura(b);
            let esperado = 60.0 / p.bpm * (p.acordes.len() * p.tiempos) as f32;
            assert!((duracion(b) - esperado).abs() < 0.001, "{b:?}");
        }
    }

    #[test]
    fn cada_baile_tiene_su_musica_y_no_la_del_otro() {
        // El fallo que esto evita es el que habria aparecido solo al entrar el
        // segundo baile: buscar la musica por fase y no por (jefe, fase) hacia
        // que la segunda figura del tango sonase a Danubio reflejado.
        assert_eq!(Tema::de(0, 1), Tema::Espejo);
        assert_eq!(Tema::de(1, 1), Tema::Corte);
        assert_ne!(Tema::de(0, 0), Tema::de(1, 0));
        // Una fase que no existe cae en la ultima figura de SU baile.
        assert_eq!(Tema::de(2, 1), Tema::CharlestonB);
        assert_eq!(Tema::de(3, 1), Tema::Galop);
        assert_eq!(Tema::de(3, 99), Tema::Infernal);
        assert_eq!(Tema::de(0, 99), Tema::Coda);
        assert_eq!(Tema::de(1, 99), Tema::Quebrada);
        assert_eq!(Tema::de(2, 99), Tema::CharlestonFin);
        assert_eq!(Tema::de(4, 0), Tema::Llamada);
        assert_eq!(Tema::de(4, 2), Tema::Zapateado);
        assert_eq!(Tema::de(4, 99), Tema::Remate);
        // Cada tema tiene su hueco, y todas las salas comparten el suyo.
        for (i, t) in TEMAS.into_iter().enumerate() {
            assert_eq!(t.indice(), i);
        }
        assert_eq!(Tema::Sala(31).indice(), Tema::Sala(0).indice());
    }

    #[test]
    fn la_cumparsita_empieza_por_donde_tiene_que_empezar() {
        // Los cuatro golpes secos del primer compas: mi, re, si, sol#. Es la
        // figura por la que se reconoce el tango mas conocido que hay, y va
        // picada: cada nota dura la mitad y la otra mitad es silencio.
        let golpes = &CUMPARSITA_A[..8];
        let tonos: Vec<i32> = golpes.iter().step_by(2).map(|n| n.tono).collect();
        assert_eq!(tonos, vec![-5, 5, 2, -1]);
        assert!(golpes.iter().all(|n| n.unidades == 2));
        assert!(golpes.iter().skip(1).step_by(2).all(|n| n.tono == SILENCIO));
        // Y el compas de antes, que es el ultimo del bucle, acaba en los dos
        // mi repetidos que llevan a ellos.
        let fin: Vec<(i32, u32)> = CUMPARSITA_A[CUMPARSITA_A.len() - 3..]
            .iter()
            .map(|n| (n.tono, n.unidades))
            .collect();
        assert_eq!(fin, vec![(-5, 2), (-5, 1), (-5, 1)]);
    }

    #[test]
    fn el_cancan_empieza_por_donde_tiene_que_empezar() {
        // "Ce bal est original": la blanca, las cuatro corcheas la do si la,
        // y las dos negras repetidas en el re.
        let tonos: Vec<i32> = GALOP_A[..7].iter().map(|n| n.tono).collect();
        assert_eq!(tonos, vec![0, 2, 5, 4, 2, 7, 7]);
        let largos: Vec<u32> = GALOP_A[..7].iter().map(|n| n.unidades).collect();
        assert_eq!(largos, vec![8, 2, 2, 2, 2, 4, 4]);
    }

    #[test]
    fn cada_figura_suena_distinto() {
        let firma = |b: Tema| {
            tema(b)
                .iter()
                .map(|v| (v.frecuencia() as u32).wrapping_mul(31) ^ (v.fin() * 1000.0) as u32)
                .fold(0u32, |a, x| a.wrapping_mul(17).wrapping_add(x))
        };
        let todos = todos();
        let f: Vec<u32> = todos.iter().map(|b| firma(*b)).collect();
        for i in 0..f.len() {
            for j in i + 1..f.len() {
                assert_ne!(f[i], f[j], "{:?} y {:?} suenan igual", todos[i], todos[j]);
            }
        }
    }

    #[test]
    fn la_coda_es_el_tema_del_principio_acelerado() {
        // Mismas notas que la segunda mitad del paso base, mas rapido y mas
        // arriba. Si alguien la convierte en otra pieza, esto avisa.
        let base = partitura(Tema::PasoBase);
        let coda = partitura(Tema::Coda);
        let mitad: Vec<i32> = base.melodia[DANUBIO_MITAD..]
            .iter()
            .map(|n| n.tono)
            .collect();
        let suya: Vec<i32> = coda.melodia.iter().map(|n| n.tono).collect();
        assert_eq!(mitad, suya);
        assert!(coda.bpm > base.bpm);
        assert!(coda.tonica > base.tonica);
        assert!(!coda.espejo);
    }

    #[test]
    fn el_modo_de_cada_tema_sale_de_su_armonia() {
        // Los efectos en el tono del tema se apoyan en esto: si un tango
        // saliera en mayor, la ficha sonaria con la tercera equivocada.
        for t in TEMAS {
            let menor = matches!(
                t,
                Tema::Espejo
                    | Tema::Caminata
                    | Tema::Corte
                    | Tema::Quebrada
                    | Tema::Llamada
                    | Tema::Rasgueo
                    | Tema::Zapateado
                    | Tema::Remate
            );
            let [r, iii, v] = triada_tonica(t);
            assert_eq!(r, partitura(t).tonica, "{t:?}");
            assert_eq!(iii - r, if menor { 3 } else { 4 }, "{t:?}");
            assert_eq!(v - r, 7, "{t:?}");
        }
    }

    #[test]
    fn asturias_empieza_por_donde_tiene_que_empezar() {
        // Mi, y el canto: si do la si sol, tres compases; luego la fa# sol dos
        // veces. Todo en corcheas, con el pedal entre medias.
        let tonos: Vec<i32> = ASTURIAS_A[..24].iter().map(|n| n.tono).collect();
        let uno = [0, 7, 8, 5, 7, 3];
        assert_eq!(tonos, [uno, uno, uno, [5, 2, 3, 5, 2, 3]].concat());
        assert!(ASTURIAS_A.iter().all(|n| n.unidades == 2));
        assert!(ASTURIAS_B.iter().all(|n| n.unidades == 2));
        // Y en el registro de la guitarra: el si del pedal es el si3 de la
        // segunda cuerda, 247 Hz, no el de una octava mas arriba.
        let p = partitura(Tema::Llamada);
        assert!((frecuencia(p.tonica + 7) - 246.9).abs() < 0.5);
        // A negra 150, veinticuatro ticks por tiempo, que es el compas del
        // paseo de Triana.
        assert_eq!(compas(Tema::Llamada).0, 24.0);
    }

    #[test]
    fn en_asturias_el_pedal_no_para() {
        // Una semicorchea si y otra no, de principio a fin, y es el si menos
        // en los compases de la menor, donde sube al do. Sin el pedal es otra
        // pieza.
        assert_eq!(pedal(men(0)), 7);
        assert_eq!(pedal(may(7)), 7);
        assert_eq!(pedal(men(5)), 8);
        for t in [Tema::Llamada, Tema::Rasgueo, Tema::Zapateado, Tema::Remate] {
            let p = partitura(t);
            let u = 60.0 / p.bpm / UNIDADES_POR_TIEMPO as f32;
            let pedales: Vec<f32> = [7, 8].iter().map(|n| frecuencia(p.tonica + n)).collect();
            let golpes = tema(t)
                .iter()
                .filter(|v| matches!(v.forma(), Wave::Sine) && pedales.contains(&v.frecuencia()))
                .filter(|v| !((v.inicio() / u).round() as u32).is_multiple_of(2))
                .count();
            assert_eq!(golpes, p.acordes.len() * 6, "{t:?}");
        }
    }

    #[test]
    fn la_afinacion_es_correcta() {
        assert!((frecuencia(0) - 110.0).abs() < 0.01, "La2");
        assert!((frecuencia(12) - 220.0).abs() < 0.2, "una octava arriba");
        assert!((frecuencia(-12) - 55.0).abs() < 0.1, "una octava abajo");
        assert!((frecuencia(24) - 440.0).abs() < 0.6, "dos octavas");
        assert!((frecuencia(15) - 261.6).abs() < 0.5, "do central");
    }
}
