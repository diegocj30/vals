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

/// El baile de cada jefe. Es lo que define compas, tempo y armonia.
///
/// De momento los tres son de la familia del vals. Cuando llegue el tango,
/// sera una variante mas aqui: cambia el compas a 4/4 y el patron de
/// acompanamiento, y el resto del modulo sirve.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Baile {
    /// El Danubio azul. 3/4, oom-pah-pah, el vals de toda la vida.
    Vals,
    /// El mismo Danubio, reflejado y en menor. Mas lento y mas oscuro.
    ValsEspejo,
    /// Sobre las olas. Rapido y girando: el ultimo.
    ValsCoda,
}

impl Baile {
    /// El baile de cada jefe, por orden de aparicion.
    pub fn del_jefe(indice: usize) -> Self {
        match indice {
            0 => Baile::Vals,
            1 => Baile::ValsEspejo,
            _ => Baile::ValsCoda,
        }
    }

    pub fn indice(self) -> usize {
        match self {
            Baile::Vals => 0,
            Baile::ValsEspejo => 1,
            Baile::ValsCoda => 2,
        }
    }

    pub fn por_indice(i: usize) -> Self {
        Self::del_jefe(i)
    }

    /// Que suena, para poder decirlo en pantalla.
    pub fn titulo(self) -> &'static str {
        partitura(self).titulo
    }
}

/// Tiempos por compas. Tres: es un vals.
const TIEMPOS: usize = 3;
/// En que se mide la duracion de una nota. Cuatro por tiempo, doce por compas:
/// da para negras, corcheas y el puntillo del ultimo compas del Danubio.
const UNIDADES_POR_TIEMPO: u32 = 4;
const UNIDADES_POR_COMPAS: u32 = UNIDADES_POR_TIEMPO * TIEMPOS as u32;

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

/// Todo lo que hace falta para sonar.
struct Partitura {
    titulo: &'static str,
    bpm: f32,
    /// Semitonos de la tonica sobre La2 (110 Hz).
    tonica: i32,
    melodia: &'static [Nota],
    acordes: &'static [Acorde],
    /// Si se refleja la melodia y la armonia antes de sonar.
    espejo: bool,
}

fn partitura(b: Baile) -> Partitura {
    match b {
        Baile::Vals => Partitura {
            titulo: "El Danubio azul - Johann Strauss II, 1866",
            bpm: 174.0,
            tonica: 15, // do central
            melodia: &DANUBIO,
            acordes: &DANUBIO_ACORDES,
            espejo: false,
        },
        Baile::ValsEspejo => Partitura {
            titulo: "El Danubio azul, reflejado",
            bpm: 132.0,
            tonica: 12, // la
            melodia: &DANUBIO[DANUBIO_MITAD..],
            acordes: &DANUBIO_ACORDES[16..],
            espejo: true,
        },
        Baile::ValsCoda => Partitura {
            titulo: "Sobre las olas - Juventino Rosas, 1888",
            bpm: 192.0,
            tonica: 22, // sol
            melodia: &OLAS,
            acordes: &OLAS_ACORDES,
            espejo: false,
        },
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

/// Duracion del bucle completo.
pub fn duracion(b: Baile) -> f32 {
    let p = partitura(b);
    let unidades = p.acordes.len() as u32 * UNIDADES_POR_COMPAS;
    unidades as f32 * (60.0 / p.bpm) / UNIDADES_POR_TIEMPO as f32
}

/// Construye el tema de un baile como lista de voces.
pub fn tema(b: Baile) -> Vec<Voz> {
    let p = partitura(b);
    let t = 60.0 / p.bpm;
    let u = t / UNIDADES_POR_TIEMPO as f32;
    let mut v = Vec::with_capacity(512);

    // --- Acompanamiento: el oom-pah-pah, compas a compas.
    // Grave al uno, acorde al dos y al tres. Este patron *es* el vals: es lo
    // que hace que se reconozca el genero antes de oir la melodia.
    for (compas, acorde) in p.acordes.iter().enumerate() {
        let a = if p.espejo {
            reflejar_acorde(*acorde)
        } else {
            *acorde
        };
        let t0 = compas as f32 * TIEMPOS as f32 * t;
        let raiz = p.tonica + a.raiz;

        v.push(Voz::nota(Wave::Sine, frecuencia(bajo(raiz)), t * 0.9, 0.30, 1.6).tras(t0));

        for paso in 1..TIEMPOS {
            let cuando = t0 + paso as f32 * t;
            for iv in a.intervalos() {
                let f = frecuencia(registro(raiz) + iv);
                v.push(Voz::nota(Wave::Square, f, t * 0.35, 0.055, 3.2).tras(cuando));
            }
        }
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
            let dur = nota.unidades as f32 * u;
            let f = frecuencia(p.tonica + tono);
            v.push(Voz::nota(Wave::Sine, f, dur * 0.92, 0.22, decay).tras(unidad as f32 * u));
        }
        unidad += nota.unidades;
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODOS: [Baile; 3] = [Baile::Vals, Baile::ValsEspejo, Baile::ValsCoda];

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
                p.acordes.len() as u32 * UNIDADES_POR_COMPAS,
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
        assert_eq!(unidades, 16 * UNIDADES_POR_COMPAS);
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

    #[test]
    fn el_compas_es_de_tres() {
        // El bajo cae solo en el primer tiempo de cada compas: es lo que hace
        // que se oiga como un vals y no como cualquier otra cosa.
        for b in TODOS {
            let p = partitura(b);
            let t = 60.0 / p.bpm;
            let graves: Vec<f32> = tema(b)
                .iter()
                .filter(|v| v.frecuencia() < 150.0)
                .map(|v| v.inicio())
                .collect();
            assert_eq!(graves.len(), p.acordes.len(), "{b:?}");
            for (i, inicio) in graves.iter().enumerate() {
                let esperado = i as f32 * TIEMPOS as f32 * t;
                assert!(
                    (inicio - esperado).abs() < 0.001,
                    "{b:?}: el bajo {i} esta fuera de sitio"
                );
            }
        }
    }

    #[test]
    fn el_bucle_dura_los_compases_que_dice() {
        for b in TODOS {
            let p = partitura(b);
            let esperado = 60.0 / p.bpm * (p.acordes.len() * 3) as f32;
            assert!((duracion(b) - esperado).abs() < 0.001);
        }
    }

    #[test]
    fn los_tres_bailes_suenan_distinto() {
        let firma = |b: Baile| {
            tema(b)
                .iter()
                .map(|v| (v.frecuencia() as u32).wrapping_mul(31) ^ (v.fin() * 1000.0) as u32)
                .fold(0u32, |a, x| a.wrapping_mul(17).wrapping_add(x))
        };
        let f: Vec<u32> = TODOS.iter().map(|b| firma(*b)).collect();
        assert_ne!(f[0], f[1]);
        assert_ne!(f[1], f[2]);
        assert_ne!(f[0], f[2]);
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
