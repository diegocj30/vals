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
    Sala,

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
}

/// Todos, en el orden en que se sintetizan y se guardan.
pub const TEMAS: [Tema; 11] = [
    Tema::Sala,
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
            (_, 0) => Tema::CharlestonA,
            (_, 1) => Tema::CharlestonB,
            (_, _) => Tema::CharlestonFin,
        }
    }

    pub fn indice(self) -> usize {
        TEMAS.iter().position(|t| *t == self).unwrap_or(0)
    }

    pub fn por_indice(i: usize) -> Self {
        TEMAS.get(i).copied().unwrap_or(Tema::Sala)
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
    /// Ni una cosa ni la otra: notas largas y nada de golpes. Una sala no
    /// tiene compas porque no se esta bailando todavia.
    Sala,
}

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
/// En la menor: los tonos son semitonos sobre el la. Aqui un compas son
/// dieciseis unidades, no doce, porque un tango va en 4/4.
///
/// Es el arranque de tango mas reconocible que existe, y la figura que lo hace
/// reconocible son los cuatro golpes secos del primer compas: mi, re, si, sol
/// sostenido. Ni un adorno.
#[rustfmt::skip]
const CUMPARSITA_A: [Nota; 46] = [
    n(-5, 4), n(5, 4), n(2, 4), n(-1, 4),                                   // 1
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 2), n(-5, 1), n(-5, 1), // 2
    n(-5, 4), n(7, 4), n(3, 4), n(0, 4),                                    // 3
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 2), n(-5, 1), n(-5, 1), // 4
    n(-5, 4), n(5, 4), n(2, 4), n(-1, 4),                                   // 5
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 2), n(-5, 1), n(-5, 1), // 6
    n(-5, 4), n(7, 4), n(3, 4), n(0, 4),                                    // 7
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-6, 4), n(-5, 4),                 // 8
];

#[rustfmt::skip]
const CUMPARSITA_A_ACORDES: [Acorde; 8] = [
    sep(7), sep(7), men(0), men(0),
    sep(7), sep(7), men(0), men(0),
];

/// Su segunda parte: la melodia baja y baja hasta el suelo.
#[rustfmt::skip]
const CUMPARSITA_B: [Nota; 40] = [
    n(5, 4), n(0, 4), n(-1, 4), n(0, 4),                                    // 9
    z(2), n(-5, 2), n(-4, 2), n(-5, 2), n(-4, 4), n(-1, 4),                 // 10
    n(-5, 6), n(-7, 1), n(-8, 1), n(-9, 8),                                 // 11
    z(2), n(-6, 2), n(-5, 2), n(-7, 2), n(-5, 4), n(-4, 4),                 // 12
    n(-7, 6), n(-8, 1), n(-9, 1), n(-11, 8),                                // 13
    z(2), n(-7, 2), n(-9, 2), n(-11, 2), n(-12, 4), n(-13, 4),              // 14
    n(-12, 6), n(-4, 2), n(-5, 2), n(-7, 2), n(-9, 2), n(-11, 2),           // 15
    n(-12, 6), n(-13, 2), n(-12, 2), z(6),                                  // 16
];

#[rustfmt::skip]
const CUMPARSITA_B_ACORDES: [Acorde; 8] = [
    men(5), men(5), men(0), men(0),
    sep(7), sep(7), men(0), men(0),
];

/// La sala. No es una pieza de nadie: son cuatro acordes largos.
///
/// Y es a proposito que no se reconozca ni suene a ningun baile. En la pista no
/// se esta bailando: se esta eligiendo. Poner ahi el vals en bucle decia que el
/// vals era el juego, y el vals es **un** jefe del juego.
#[rustfmt::skip]
const SALA: [Nota; 9] = [
    n(12, 10), z(6),
    n(15, 10), z(6),
    n(19, 8), n(17, 4), z(4),
    n(12, 12), z(4),
];

#[rustfmt::skip]
const SALA_ACORDES: [Acorde; 4] = [men(0), may(8), men(5), men(0)];

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

    match t {
        Tema::Sala => Partitura {
            titulo: "la sala",
            bpm: 60.0,
            tiempos: 4,
            acompanamiento: Acompanamiento::Sala,
            tonica: 12, // la
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
        Tema::CharlestonFin => rag(
            "Maple Leaf Rag, al galope",
            128.0,
            22,
            &RAG_A,
            &RAG_A_ACORDES,
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
fn frecuencia(semitonos: i32) -> f32 {
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
fn unidades_por_compas(p: &Partitura) -> u32 {
    UNIDADES_POR_TIEMPO * p.tiempos as u32
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
            v.push(
                Voz::nota(Wave::Sine, f, dur * 0.92, 0.22, decay)
                    .tras((unidad as f32 + atraso) * u),
            );
        }
        unidad += nota.unidades;
    }

    v
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
                    v.push(Voz::nota(Wave::Sine, grave, t * 0.45, 0.34, 3.0).tras(cuando));
                }
                for paso in (1..p.tiempos).step_by(2) {
                    let cuando = t0 + paso as f32 * t;
                    for f in triada() {
                        v.push(Voz::nota(Wave::Square, f, t * 0.22, 0.06, 4.5).tras(cuando));
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
            // Sin golpes: el grave y el acorde duran el compas entero. No hay
            // compas que marcar porque todavia no se esta bailando.
            Acompanamiento::Sala => {
                let largo = p.tiempos as f32 * t;
                v.push(Voz::nota(Wave::Sine, grave, largo * 0.95, 0.22, 1.0).tras(t0));
                for f in triada() {
                    v.push(Voz::nota(Wave::Sine, f, largo * 0.9, 0.05, 0.9).tras(t0));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODOS: [Tema; 11] = TEMAS;

    #[test]
    fn la_melodia_y_la_armonia_cuadran() {
        // Un acorde por compas y doce unidades por compas. Si una transcripcion
        // se descuadra por una corchea, la melodia se va desplazando del
        // acompanamiento y el tema se deshace a la mitad. Aqui salta antes.
        for b in TODOS {
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
        for b in TODOS {
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
        for b in TODOS {
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
        for b in TODOS {
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
    fn la_sala_no_suena_a_ningun_baile() {
        // Sin golpes: un grave por compas y nada mas percutido. Si algun dia
        // alguien le pone un compas, la pista vuelve a decir que el juego es
        // ese baile.
        let p = partitura(Tema::Sala);
        assert!(p.acompanamiento == Acompanamiento::Sala);
        let voces = tema(Tema::Sala);
        assert!(
            voces.iter().all(|v| !matches!(v.forma(), Wave::Square)),
            "la sala no lleva golpes secos"
        );
        assert_eq!(graves_por_compas(Tema::Sala).len(), p.acordes.len());
    }

    #[test]
    fn el_bucle_dura_los_compases_que_dice() {
        for b in TODOS {
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
        assert_eq!(Tema::de(0, 99), Tema::Coda);
        assert_eq!(Tema::de(1, 99), Tema::Quebrada);
        assert_eq!(Tema::de(2, 99), Tema::CharlestonFin);
        // Y el indice y su vuelta cuadran para todos.
        for t in TEMAS {
            assert_eq!(Tema::por_indice(t.indice()), t);
        }
    }

    #[test]
    fn la_cumparsita_empieza_por_donde_tiene_que_empezar() {
        // Los cuatro golpes secos del primer compas: mi, re, si, sol#. Es la
        // figura por la que se reconoce el tango mas conocido que hay.
        let tonos: Vec<i32> = CUMPARSITA_A[..4].iter().map(|n| n.tono).collect();
        assert_eq!(tonos, vec![-5, 5, 2, -1]);
        assert!(CUMPARSITA_A[..4].iter().all(|n| n.unidades == 4));
    }

    #[test]
    fn cada_figura_suena_distinto() {
        let firma = |b: Tema| {
            tema(b)
                .iter()
                .map(|v| (v.frecuencia() as u32).wrapping_mul(31) ^ (v.fin() * 1000.0) as u32)
                .fold(0u32, |a, x| a.wrapping_mul(17).wrapping_add(x))
        };
        let f: Vec<u32> = TODOS.iter().map(|b| firma(*b)).collect();
        for i in 0..f.len() {
            for j in i + 1..f.len() {
                assert_ne!(f[i], f[j], "{:?} y {:?} suenan igual", TODOS[i], TODOS[j]);
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
    fn la_afinacion_es_correcta() {
        assert!((frecuencia(0) - 110.0).abs() < 0.01, "La2");
        assert!((frecuencia(12) - 220.0).abs() < 0.2, "una octava arriba");
        assert!((frecuencia(-12) - 55.0).abs() < 0.1, "una octava abajo");
        assert!((frecuencia(24) - 440.0).abs() < 0.6, "dos octavas");
        assert!((frecuencia(15) - 261.6).abs() < 0.5, "do central");
    }
}
