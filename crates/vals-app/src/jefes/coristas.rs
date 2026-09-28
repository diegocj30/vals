//! **El Cancan**: una fila de coristas del Moulin Rouge, cogidas de la mano y
//! pateando al galop.
//!
//! Los otros tres jefes son objetos; este sigue siendo gente, y
//! precisamente porque es **mucha**: una fila de piernas levantandose es lo que
//! dispara el cancan (sus abanicos salen en fila, uno detras de otro) y es lo
//! que se veia en el music-hall. Una bailarina sola no era el cancan: era una
//! bailarina.
//!
//! Cada corista es un cuerpo de cartel —tinta gruesa, rosa plano, faldas
//! granate y las enaguas blancas asomando— con **cara**: ojos grandes, rimel,
//! colorete y los labios pintados. Un cartel de 1900 no dibuja caras
//!, pero un jefe de Cuphead si, y aqui la cara es lo que cambia
//! entre figuras: coqueta en la primera, pendiente de la ola en la segunda y
//! desencajada en el infierno.
//!
//! Las tres figuras se leen sin cartela:
//!
//! - **La patada**: todas a la vez, patadas educadas a la altura de la cadera.
//! - **La fila**: la ola. Cada una patea un poco despues que la de su
//!   izquierda y la patada recorre la fila en un tiempo, como el abanico de
//!   balas que barre el suelo.
//! - **El infierno**: patadas a la corchea desde el centro hacia fuera, las de
//!   las puntas dan la rueda, todas caen al espagat y vuelan plumas.
//!
//! Como todo jefe, es **funcion pura** de la `Escena`: ni estado ni azar. Las
//! plumas sueltas salen de una `Pcg32` con semilla fija y su sitio es funcion
//! del reloj, igual que el confeti de la victoria.

use macroquad::prelude::*;
use vals_core::math::{PI, TAU, sin_cos};
use vals_core::rng::Pcg32;

use super::{Escena, bote, suave, tramo};
use crate::draw::fade;
use crate::paleta::TINTA;
use crate::skeleton::trazo;

// --- El compas del galop: 150 negras en 2/4 ---
const TIEMPO: f32 = 24.0;
const COMPAS: f32 = 48.0;
/// La vuelta del infierno: dos compases de patadas, uno de ruedas y uno de
/// espagat. Cuatro compases es la frase del galop.
const VUELTA: f32 = COMPAS * 4.0;

// --- Medidas, en unidades del jefe (se multiplican por `Escena::escala`) ---
/// Cada corista mide esto de la bailarina de antes. Una sola llenaba el aro
/// de golpeo; cinco a ese tamano eran una pared de faldas de 60 unidades y no
/// una fila. A dos tercios caben cinco en ~190 de arena y cada una se lee
/// entera. Todas las medidas de cuerpo de abajo son a tamano 1 y se
/// multiplican por su talla.
const TALLA: f32 = 0.62;
/// La y de las tablas bajo sus pies, contando desde el centro del aro. Todas
/// pisan el mismo suelo: la estrella es mas alta de cadera, no de zapato.
/// A esta altura el aro queda a la altura de los corpinos.
const PISO: f32 = 9.0;
/// Lo que asoma por debajo del pie: el zapato y su tinta.
const BAJO_EL_PISO: f32 = 1.2;
const MUSLO: f32 = 7.2;
const PANTORRILLA: f32 = 7.0;
const TORSO: f32 = 5.6;
const CUELLO: f32 = 3.2;
/// Cabezas grandes: a tamano de juego una cara necesita sitio para dos ojos y
/// una boca, y un cabezon se lee como dibujo animado, no como maniqui.
const CABEZA: f32 = 5.0;
const BRAZO: f32 = 5.0;
const ANTEBRAZO: f32 = 4.8;
const HOMBROS: f32 = 3.1;
const CINTURA: f32 = 2.0;
/// Media separacion de las caderas: de ahi cuelga cada pierna.
const CADERAS: f32 = 1.2;
const FALDA: f32 = 8.4;
const N_FALDA: usize = 9;
/// Grosor de la tinta del cuerpo y de la cara, en unidades.
const TINTA_G: f32 = 0.6;
const TINTA_F: f32 = 0.28;
/// De cadera a cadera entre coristas vecinas. Cinco a 11 son 44 unidades,
/// 140 de arena; con las faldas, ~190: la fila puede ser mas ancha que el aro,
/// pero no mucho mas.
const PASO: f32 = 11.0;

// --- Lo que no es tinta del baile: la cara, las medias y las enaguas ---
const ENAGUA: Color = color_u8!(244, 234, 214, 255);
const MEDIA: Color = color_u8!(44, 30, 40, 255);
const LABIOS: Color = color_u8!(200, 24, 52, 255);
const COLORETE: Color = color_u8!(236, 84, 112, 150);
const OJO: Color = color_u8!(252, 248, 238, 255);
/// El penacho de la estrella: plumas de avestruz. Es lo unico de la fila que
/// no lleva las tintas del baile, y por eso se la ve la primera.
const AVESTRUZ: Color = color_u8!(250, 240, 214, 255);
const ORO: Color = color_u8!(246, 196, 82, 255);

/// Una corista en este instante, en unidades del jefe y con el centro del aro
/// en el origen.
#[derive(Clone, Copy)]
struct Corista {
    talla: f32,
    estrella: bool,
    /// Dando la rueda: sin manos que dar a nadie.
    rueda: bool,
    cadera: Vec2,
    pecho: Vec2,
    cabeza: Vec2,
    /// Hacia donde apunta la cabeza. Es (0, -1) salvo en la rueda, y es lo
    /// que orienta la cara y el penacho.
    arriba: Vec2,
    /// Pierna izquierda y derecha de la pantalla.
    rodilla: [Vec2; 2],
    pie: [Vec2; 2],
    /// La pierna que patea va por delante de todo; la de apoyo, debajo de la
    /// falda.
    patea: Option<usize>,
    falda: [Vec2; N_FALDA],
    enagua: [Vec2; N_FALDA],
    codo: [Vec2; 2],
    mano: [Vec2; 2],
    /// Apertura de los ojos: 0.5 es coqueta, 1.2 desencajada.
    ojos: f32,
    /// Hacia donde mira, de -1 a 1.
    mirada: f32,
    /// 0 sonrisa, 1 boca abierta.
    boca: f32,
    guino: bool,
    /// Fuera de combate: ojos en X. Solo al caer.
    ko: bool,
    /// Ya se le han caido las plumas del tocado.
    sin_plumas: bool,
}

impl Corista {
    fn derecha(&self) -> Vec2 {
        vec2(-self.arriba.y, self.arriba.x)
    }

    fn hombros(&self) -> [Vec2; 2] {
        let d = self.derecha() * HOMBROS * self.talla;
        [self.pecho - d, self.pecho + d]
    }

    /// Las plumas del tocado: de donde salen, hasta donde llegan y cuanto se
    /// curvan. La estrella lleva cinco y mas altas: se la busca por el penacho.
    fn penacho(&self, pulso: f32, fuerte: bool) -> Vec<(Vec2, Vec2, f32)> {
        const COMUN: [(f32, f32); 3] = [(-0.55, 4.2), (0.0, 5.6), (0.55, 4.2)];
        const ESTRELLA: [(f32, f32); 5] =
            [(-1.0, 5.4), (-0.5, 7.4), (0.0, 9.0), (0.5, 7.4), (1.0, 5.4)];
        let plumas: &[(f32, f32)] = if self.estrella { &ESTRELLA } else { &COMUN };
        let base = self.cabeza + self.arriba * CABEZA * self.talla * 0.8;
        // En el uno del compas el penacho de la estrella se abre de golpe.
        let abre = if self.estrella && fuerte {
            pulso * 0.18
        } else {
            0.0
        };
        plumas
            .iter()
            .map(|&(sep, alto)| {
                let sep = sep * (1.0 + abre * 2.0);
                let dir = (self.arriba + self.derecha() * sep).normalize_or_zero();
                // Tiemblan con el golpe, cada una a un lado.
                let tiembla = self.derecha() * pulso * 0.5 * self.talla * sep.signum();
                let punta = base + dir * alto * self.talla * (1.0 + abre) + tiembla;
                (base, punta, sep)
            })
            .collect()
    }
}

/// Cuantos ticks va retrasada la corista `i` de `n` respecto al compas.
///
/// Es la figura entera en una linea: en la patada van todas a una, en la fila
/// la patada corre de izquierda a derecha y cruza la fila en un tiempo, y en
/// el infierno revienta desde la estrella hacia las puntas.
fn desfase(fase: usize, i: usize, n: usize) -> f32 {
    match fase {
        0 => 0.0,
        1 => i as f32 * TIEMPO / n as f32,
        _ => (i as f32 - (n - 1) as f32 * 0.5).abs() * 3.0,
    }
}

/// Lo alta que va la patada, de 0 a 1, `d` ticks despues del golpe.
///
/// Sube de golpe, aguanta un instante arriba y baja despacio, que es como es
/// una patada y como son las balas de este baile: salen y se van solas.
fn subida(d: f32, periodo: f32) -> f32 {
    if d < 3.0 {
        d / 3.0
    } else {
        let k = ((d - 5.0) / (periodo * 0.6)).clamp(0.0, 1.0);
        1.0 - k * k * (3.0 - 2.0 * k)
    }
}

/// Cuanto hay que subir la fila para que ningun zapato atraviese las tablas.
///
/// El cancan baja al borde del escenario (y 718) a patear a ras de suelo, y
/// su aro queda a un palmo de las tablas (741): colgada del aro, la fila se
/// hundiria bajo las candilejas. Se sube lo justo y solo cuando hace falta;
/// arriba, en el aire, no se toca.
///
/// Se mide contra el `PISO` fijo y no contra la pose de este instante: si se
/// midiera el pie mas bajo, cada salto se comeria su propia subida y la fila
/// no despegaria nunca del suelo.
fn elevar(centro_y: f32, tablas: Option<f32>, escala: f32) -> f32 {
    tablas.map_or(0.0, |y| {
        (centro_y + (PISO + BAJO_EL_PISO) * escala - y).max(0.0)
    })
}

/// Un codo o una rodilla: el punto que deja los dos huesos con su largo y
/// doblado hacia `afuera`. Si no llegan, el miembro se estira recto.
fn articula(a: Vec2, b: Vec2, l1: f32, l2: f32, afuera: Vec2) -> Vec2 {
    let v = b - a;
    let d = v.length().clamp(1e-3, l1 + l2 - 1e-3);
    let eje = v.normalize_or(vec2(0.0, 1.0));
    let x = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let h = (l1 * l1 - x * x).max(0.0).sqrt();
    let mut perp = vec2(-eje.y, eje.x);
    if perp.dot(afuera) < 0.0 {
        perp = -perp;
    }
    a + eje * x.min(l1) + perp * h
}

fn girar(v: Vec2, ang: f32) -> Vec2 {
    let (s, c) = sin_cos(ang);
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Cuando empieza a caer la corista `i` de `n`: de izquierda a derecha, en
/// fila, como fichas de domino. Cada una cae empujada por la de su izquierda.
fn empieza_a_caer(i: usize, n: usize) -> f32 {
    0.06 + 0.4 * i as f32 / (n.max(2) - 1) as f32
}

/// Lo que tarda cada una en caer.
const CAE: f32 = 0.3;

/// La caida de una corista en `k`: cuanto ha bajado al espagat (de 0 a 1,
/// con su bote), cuanto se inclina hacia la derecha mientras cae, en
/// radianes, y si ya ha tocado el suelo. Se inclina empujada y se endereza al
/// llegar abajo: el numero acaba como acaba un cancan, todas en el suelo con
/// las piernas abiertas.
fn domino(k: f32, i: usize, n: usize) -> (f32, f32, bool) {
    let a = empieza_a_caer(i, n);
    let c = tramo(k, a, a + CAE);
    // `bote` toca el suelo por primera vez a 1 / 2.75 de su recorrido.
    (bote(c), 0.6 * (PI * c).sin(), c >= 1.0 / 2.75)
}

/// La fila entera en este instante.
///
/// `cartel` es el panel lateral: alli cabe una figura de 30 unidades de ancho y
/// no una fila de 60, asi que salen tres, mas juntas, con la estrella delante.
///
/// Con `caida` la fila no baila: cae en domino (ver `domino`).
fn fila(e: &Escena, cartel: bool, caida: Option<f32>) -> Vec<Corista> {
    let (n, paso) = if cartel { (3, 8.4) } else { (5, PASO) };
    let fase = e.fase.min(2);
    let apuro = 1.0 - e.vida.clamp(0.0, 1.0);
    let ciclo = e.t.rem_euclid(VUELTA);
    let compas = (ciclo / COMPAS).floor() as usize;
    let en_compas = ciclo - compas as f32 * COMPAS;

    let mut todas: Vec<Corista> = (0..n)
        .map(|i| {
            let estrella = i == n / 2;
            let talla = TALLA * if estrella { 1.18 } else { 1.0 };
            let x0 = (i as f32 - (n - 1) as f32 * 0.5) * paso;

            // La patada que toca: a la negra, y en el infierno a la corchea.
            let periodo = if fase == 2 { TIEMPO * 0.5 } else { TIEMPO };
            let tl = e.t - desfase(fase, i, n);
            let golpe = (tl / periodo).floor();
            let lado = golpe.rem_euclid(2.0) as usize;
            let mut brio = if caida.is_some() {
                0.0
            } else {
                subida(tl - golpe * periodo, periodo)
            };
            let mut alto = ([1.75, 2.45, 2.85][fase] + apuro * 0.35).min(3.0);

            // El infierno por compases: patadas, ruedas en las puntas y todas
            // al espagat menos la estrella, que se queda con la pierna arriba.
            let punta = i == 0 || i == n - 1;
            let rueda = fase == 2 && !cartel && compas == 2 && punta && caida.is_none();
            let (cae, vuelco, llego) = caida.map_or((0.0, 0.0, false), |k| domino(k, i, n));
            let espagat = if caida.is_some() {
                cae
            } else if fase == 2 && !cartel && compas == 3 && !estrella {
                suave(en_compas / 5.0) * suave((COMPAS - en_compas) / 8.0)
            } else {
                0.0
            };
            if fase == 2 && compas == 3 && estrella {
                brio = 1.0;
                alto = 3.0;
            }

            let pierna = (MUSLO + PANTORRILLA) * talla;
            // Se sube a la punta del pie en cada patada, y la fila entera bota
            // con el golpe del compas.
            let rebote = (e.pulso * 0.5 + brio * 0.6) * talla;
            let de_pie = PISO - pierna * 0.9 - rebote;
            let cadera = vec2(x0, de_pie + (PISO - 2.4 * talla - de_pie) * espagat);

            let arriba = vec2(0.0, -1.0);
            let pecho = cadera + arriba * TORSO * talla;
            let cabeza = pecho + arriba * (CUELLO + CABEZA * 0.55) * talla;

            // --- Piernas ---
            let caderas = [
                cadera - vec2(CADERAS * talla, 0.0),
                cadera + vec2(CADERAS * talla, 0.0),
            ];
            let mut pie = [vec2(x0 - 2.6 * talla, PISO), vec2(x0 + 2.6 * talla, PISO)];
            let patea = (brio > 0.12 && espagat == 0.0 && !rueda).then_some(lado);
            if let Some(k) = patea {
                // La pierna estirada entera, barriendo por su lado hasta
                // arriba. Estirada y no recogida: una rodilla doblada a media
                // patada se leia como una rana, no como un cancan. Nunca por
                // debajo de las tablas: en reposo es la pierna de apoyo.
                let fuera = if k == 0 { -1.0 } else { 1.0 };
                let (s, c) = sin_cos(brio * alto);
                let dir = vec2(fuera * (0.12 + 0.65 * s), c).normalize_or_zero();
                let destino = caderas[k] + dir * pierna;
                pie[k] = vec2(destino.x, destino.y.min(PISO));
            }
            if espagat > 0.0 {
                for (k, fuera) in [(0, -1.0), (1, 1.0)] {
                    let lejos = (2.6 * talla) + (pierna - 2.6 * talla) * espagat;
                    pie[k] = vec2(x0 + fuera * lejos, PISO - 0.8 * espagat);
                }
            }
            let rodilla = [0, 1].map(|k| {
                articula(
                    caderas[k],
                    pie[k],
                    MUSLO * talla,
                    PANTORRILLA * talla,
                    // Las rodillas se abren hacia fuera, como en un plie. Al
                    // bajar al espagat, hacia arriba: hacia abajo atravesaban
                    // las tablas.
                    caderas[k] - cadera + vec2(0.0, if espagat > 0.0 { -1.0 } else { 0.1 }),
                )
            });

            // --- Falda y enaguas ---
            // Del lado de la patada la falda se levanta y deja ver las
            // enaguas, que es literalmente para lo que se baila el cancan.
            let levanta = if patea.is_some() { brio } else { 0.0 };
            let hacia = if lado == 0 { 1.0 } else { -1.0 };
            let bajo = |largo: f32, alza: f32| {
                let mut f = [cadera; N_FALDA];
                for (j, p) in f.iter_mut().enumerate() {
                    let u = j as f32 / (N_FALDA - 1) as f32;
                    // u = 0 es el costado derecho y 1 el izquierdo.
                    let de_su_lado = if lado == 0 { u } else { 1.0 - u };
                    let w = suave((de_su_lado - 0.3) / 0.7) * levanta;
                    // Al espagat el abanico se abre por arriba: vista de frente,
                    // la falda de una corista en el suelo es una flor de
                    // volantes alrededor de la cadera.
                    let abre = 0.12 + (-0.22 - 0.12) * espagat;
                    let ang = PI * (abre + (1.0 - 2.0 * abre) * u) + hacia * alza * w;
                    let (s, c) = sin_cos(ang);
                    let (onda, _) = sin_cos(e.t * 0.35 + u * 7.0 + i as f32);
                    let r = largo * talla * (0.8 + 0.2 * sin_cos(u * PI).0) * (1.0 + 0.25 * w)
                        + onda * 0.35 * talla;
                    // Por debajo se aplasta contra las tablas en vez de
                    // atravesarlas. Y por si acaso, nada del bajo pasa del piso.
                    let aplasta = if s > 0.0 { 1.0 - 0.7 * espagat } else { 1.0 };
                    let v = vec2(c * (1.0 + 0.3 * espagat), s * aplasta);
                    *p = cadera + v * r;
                    p.y = p.y.min(PISO - 0.4);
                }
                f
            };
            let falda = bajo(FALDA, 1.25);
            let enagua = bajo(FALDA * 1.14, 0.95);

            // --- La cara ---
            let ojos = [0.55, 0.9, 1.12][fase] + apuro * 0.2;
            let boca = if fase == 2 || apuro > 0.7 {
                brio.max(espagat)
            } else {
                0.0
            };
            let mirada = if patea.is_some() {
                -hacia * brio * 0.9
            } else {
                0.0
            };
            // La estrella guina en el uno de cada dos compases: es la unica
            // que sabe que la estan mirando.
            let guino =
                estrella && fase < 2 && e.fuerte && e.pulso > 0.45 && golpe.rem_euclid(4.0) < 1.0;

            let mut c = Corista {
                talla,
                estrella,
                rueda,
                cadera,
                pecho,
                cabeza,
                arriba,
                rodilla,
                pie,
                patea,
                falda,
                enagua,
                codo: [pecho; 2],
                mano: [pecho; 2],
                ojos,
                mirada,
                boca,
                guino,
                ko: false,
                sin_plumas: false,
            };

            if rueda {
                // La rueda: abierta en aspa y girando entera, hacia fuera.
                // Sube el eje a medio giro para que cabeza abajo el penacho no
                // barra las tablas.
                let fuera = if i == 0 { -1.0 } else { 1.0 };
                let k = suave(en_compas / COMPAS);
                let ang = fuera * TAU * k;
                let eje = cadera + arriba * 2.5 * talla;
                let sube = vec2(0.0, -7.5 * talla * sin_cos(k * PI).0);
                let g = |p: Vec2| eje + girar(p - eje, ang) + sube;
                for (k2, lado) in [(0, -1.0), (1, 1.0)] {
                    c.pie[k2] = cadera + vec2(lado * 0.55, 0.83) * pierna;
                    c.rodilla[k2] = cadera + vec2(lado * 0.55, 0.83) * MUSLO * talla;
                }
                for p in c
                    .rodilla
                    .iter_mut()
                    .chain(c.pie.iter_mut())
                    .chain(c.falda.iter_mut())
                    .chain(c.enagua.iter_mut())
                {
                    *p = g(*p);
                }
                c.cadera = g(c.cadera);
                c.pecho = g(c.pecho);
                c.cabeza = g(c.cabeza);
                c.arriba = girar(arriba, ang);
                c.boca = 1.0;
            }
            if caida.is_some() {
                // Cae con la boca en O del susto y llega abajo mareada, con
                // los ojos en X y sonriendo: el numero se acaba igual.
                c.ko = llego;
                c.sin_plumas = llego;
                c.ojos = 1.2;
                c.boca = if cae > 0.0 && !llego { 1.0 } else { 0.0 };
                c.mirada = 0.0;
                // Empujada por la vecina: se inclina desde la cadera.
                let g = |p: Vec2| cadera + girar(p - cadera, vuelco);
                c.pecho = g(c.pecho);
                c.cabeza = g(c.cabeza);
                c.arriba = girar(c.arriba, vuelco);
            }
            c
        })
        .collect();

    // --- Brazos ---
    // Cogidas de la mano con la vecina, en alto; la que no tiene vecina (las
    // puntas, y las de al lado de una que da la rueda) saluda con el brazo
    // libre.
    let manos_juntas: Vec<Option<Vec2>> = (0..n.saturating_sub(1))
        .map(|i| {
            let (a, b) = (&todas[i], &todas[i + 1]);
            (!a.rueda && !b.rueda).then(|| {
                (a.hombros()[1] + b.hombros()[0]) * 0.5 + vec2(0.0, -(3.4 + e.pulso * 0.8) * TALLA)
            })
        })
        .collect();
    for (i, c) in todas.iter_mut().enumerate() {
        let hombros = c.hombros();
        for (k, fuera) in [(0usize, -1.0f32), (1, 1.0)] {
            let junta = if k == 0 {
                i.checked_sub(1).and_then(|j| manos_juntas[j])
            } else {
                manos_juntas.get(i).copied().flatten()
            };
            let libre = || {
                // El brazo libre en alto, abierto y agitandose a la negra.
                let (s, _) = sin_cos(e.t * TAU / TIEMPO + i as f32);
                let ang = fuera * (0.55 + 0.25 * s);
                hombros[k] + girar(c.arriba, ang) * (BRAZO + ANTEBRAZO) * c.talla * 0.92
            };
            let mano = junta.unwrap_or_else(libre);
            let afuera = (hombros[k] - c.pecho) + vec2(0.0, 1.5);
            c.mano[k] = mano;
            c.codo[k] = articula(
                hombros[k],
                mano,
                BRAZO * c.talla,
                ANTEBRAZO * c.talla,
                afuera,
            );
        }
    }
    todas
}

/// Las plumas que se le caen a la fila en el infierno: salen de los penachos,
/// suben y caen dando vueltas. Posicion, giro y opacidad.
fn plumas_sueltas(e: &Escena, todas: &[Corista]) -> Vec<(Vec2, f32, f32)> {
    if e.fase < 2 && e.vida > 0.25 {
        return Vec::new();
    }
    let mut rng = Pcg32::new(0xCA_2CA2);
    (0..12)
        .map(|k| {
            let periodo = rng.range_f32(50.0, 90.0);
            let desde = rng.range_f32(0.0, periodo);
            let v = vec2(rng.range_f32(-0.22, 0.22), rng.range_f32(-0.34, -0.16));
            let gira = rng.range_f32(-0.2, 0.2);
            let edad = (e.t + desde).rem_euclid(periodo);
            let origen = vec2(todas[k % todas.len()].cadera.x, PISO - 30.0 * TALLA);
            let (vaiven, _) = sin_cos(edad * 0.15 + k as f32);
            let p = origen + v * edad + vec2(vaiven * 1.2, 0.004 * edad * edad);
            // Opacas casi toda su vida: una pluma transparente parecia humo.
            (p, gira * edad, ((1.0 - edad / periodo) * 3.0).min(1.0))
        })
        .collect()
}

pub fn dibujar(e: &Escena) {
    // El cartel lateral imprime al jefe en silueta, con una tinta casi negra:
    // es la unica forma de saberlo desde aqui, y alli la fila no cabe entera.
    // En combate la tinta es el rosa del baile o el blanco del golpe.
    let cartel = e.tinta.r + e.tinta.g + e.tinta.b < 0.5;
    let todas = fila(e, cartel, None);
    // En el cartel no: es una lamina quieta, y las plumas se salian del marco.
    let plumas = if cartel {
        Vec::new()
    } else {
        plumas_sueltas(e, &todas)
    };
    pintar(e, &todas, &plumas, 2.6);
}

/// La fila, y encima las plumas sueltas: posicion, giro y opacidad, y de
/// largo `largo` (media pluma).
fn pintar(e: &Escena, todas: &[Corista], plumas: &[(Vec2, f32, f32)], largo: f32) {
    let esc = e.escala;
    let origen = e.centro - vec2(0.0, elevar(e.centro.y, e.tablas, esc));
    let a = |p: Vec2| origen + p * esc;
    let u = |v: f32| v * esc;

    // Tinta y color: todo se pinta dos veces, engordado en tinta y encima del
    // tamano real, igual que `draw_figura`.
    let miembro = |p0: Vec2, p1: Vec2, r0: f32, r1: f32, arqueo: f32, c: Color| {
        for (extra, color) in [(TINTA_G, TINTA), (0.0, c)] {
            hueso(a(p0), a(p1), u(r0 + extra), u(r1 + extra), u(arqueo), color);
        }
    };
    let pierna = |c: &Corista, k: usize| {
        let lado = if k == 0 { -1.0 } else { 1.0 };
        let cadera = c.cadera + c.derecha() * lado * CADERAS * c.talla;
        let t = c.talla;
        miembro(cadera, c.rodilla[k], 1.5 * t, 1.1 * t, 0.0, e.tinta);
        miembro(c.rodilla[k], c.pie[k], 1.1 * t, 0.75 * t, 0.0, MEDIA);
        // La liga, en la tinta del baile.
        let liga = cadera.lerp(c.rodilla[k], 0.72);
        let (lp, lr) = (a(liga), u(0.75 * t));
        circulo(lp, lr + u(TINTA_F), TINTA);
        circulo(lp, lr, e.ropa);
        // El zapato, en punta, siguiendo la espinilla.
        let d = (c.pie[k] - c.rodilla[k]).normalize_or_zero();
        let centro = a(c.pie[k] + d * 0.7 * t);
        let ang = d.y.atan2(d.x).to_degrees();
        draw_ellipse(
            centro.x,
            centro.y,
            u(1.6 * t + TINTA_G),
            u(0.85 * t + TINTA_G),
            ang,
            TINTA,
        );
        draw_ellipse(centro.x, centro.y, u(1.6 * t), u(0.85 * t), ang, MEDIA);
    };

    for c in todas {
        // Las piernas de apoyo, detras de la falda.
        for k in 0..2 {
            if c.patea != Some(k) {
                pierna(c, k);
            }
        }
        // Enaguas y falda: abanicos desde la cadera, con volantes.
        abanico(&c.enagua, c.cadera, &a, u(TINTA_G), ENAGUA);
        for q in &c.enagua {
            let p = a(*q);
            circulo(p, u(1.25 * c.talla + TINTA_F), TINTA);
            circulo(p, u(1.25 * c.talla), ENAGUA);
        }
        abanico(&c.falda, c.cadera, &a, u(TINTA_G), e.ropa);
        // El ribete del bajo: una tira en la tinta del baile.
        for w in c.falda.windows(2) {
            let (p, q) = (a(w[0]), a(w[1]));
            draw_line(p.x, p.y, q.x, q.y, u(0.9), e.tinta);
        }

        // El corpino, estrecho de cintura.
        let d = c.derecha();
        let cuerpo = [
            c.pecho - d * HOMBROS * c.talla,
            c.pecho + d * HOMBROS * c.talla,
            c.cadera + d * CINTURA * c.talla,
            c.cadera - d * CINTURA * c.talla,
        ];
        poligono(&cuerpo.map(a), u(TINTA_G), e.ropa);
        // El escote y el cuello.
        miembro(
            c.pecho,
            c.cabeza,
            1.3 * c.talla,
            1.1 * c.talla,
            0.0,
            e.tinta,
        );

        // Penacho, cabeza, pelo y cara.
        let penacho = if c.sin_plumas {
            Vec::new()
        } else {
            c.penacho(e.pulso, e.fuerte)
        };
        for (base, punta, sep) in penacho {
            let color = if c.estrella { AVESTRUZ } else { e.tinta };
            miembro(base, punta, 1.1 * c.talla, 0.3, sep * 1.6, color);
        }
        cara(c, &a, &u, e.tinta, e.ropa);
        if c.ko {
            // Mareada: dos estrellitas dando vueltas alrededor de la cabeza.
            for j in 0..2 {
                let (s, co) = sin_cos(e.t * 0.12 + j as f32 * PI + c.cadera.x);
                let p = a(c.cabeza + vec2(co * 5.0, -CABEZA * c.talla - 1.5 + s) * c.talla);
                circulo(p, u(0.75 + TINTA_F), TINTA);
                circulo(p, u(0.75), ORO);
            }
        }
    }

    // Los brazos por encima de los cuerpos: son los que atan la fila.
    for c in todas {
        for k in 0..2 {
            let t = c.talla;
            let hombro = c.hombros()[k];
            miembro(hombro, c.codo[k], 1.05 * t, 0.85 * t, 0.0, e.tinta);
            miembro(c.codo[k], c.mano[k], 0.85 * t, 0.65 * t, 0.0, e.tinta);
        }
    }
    for c in todas {
        for m in c.mano {
            let p = a(m);
            circulo(p, u(1.35 * c.talla + TINTA_G), TINTA);
            circulo(p, u(1.35 * c.talla), e.tinta);
        }
    }

    // Y las patadas por delante de todo, que es el numero.
    for c in todas {
        if let Some(k) = c.patea {
            pierna(c, k);
        }
    }

    for &(p, giro, alfa) in plumas {
        let dir = girar(vec2(0.0, -1.0), giro) * largo;
        let (p0, p1) = (a(p - dir), a(p + dir));
        hueso(
            p0,
            p1,
            u(0.7 + TINTA_F),
            u(0.2 + TINTA_F),
            u(1.2),
            fade(TINTA, alfa),
        );
        hueso(p0, p1, u(0.7), u(0.2), u(1.2), fade(e.tinta, alfa));
    }
}

/// La fila cayendo, con `k` de 0 a 1: en domino hasta el espagat, y las
/// plumas de los tocados bajando meciendose hasta las tablas.
pub fn dibujar_muerte(e: &Escena, k: f32) {
    // Ya no bailan: sin compas ni figura.
    let quieta = Escena {
        fase: 0,
        vida: 1.0,
        pulso: 0.0,
        fuerte: false,
        ..*e
    };
    let todas = fila(&quieta, false, Some(k));
    let n = todas.len();
    let mut plumas = Vec::new();
    for i in 0..n {
        // Se le sueltan al tocar el suelo, del tocado de ese instante: asi
        // salen justo de donde estaban y no dan un salto.
        let toca = empieza_a_caer(i, n) + CAE / 2.75;
        let v = tramo(k, toca, 1.0);
        if v <= 0.0 {
            continue;
        }
        let c = fila(&quieta, false, Some(toca))[i];
        for (j, (base, punta, _)) in c.penacho(0.0, false).into_iter().enumerate() {
            let medio = (base + punta) * 0.5;
            let fase = i as f32 * 1.7 + j as f32 * 2.3;
            let (vaiven, _) = sin_cos(v * 8.0 + fase);
            let (mece, _) = sin_cos(v * 10.0 + fase);
            let x = medio.x + vaiven * 3.0 * (1.0 - v) + (j as f32 - 1.0) * 5.0 * v;
            let y = medio.y + (PISO - 1.0 - medio.y) * v;
            // Se mecen al bajar y se quedan tumbadas en las tablas.
            plumas.push((vec2(x, y), mece * 0.9 * (1.0 - v) + PI * 0.5 * v, 1.0));
        }
    }
    // Las del tocado, mas grandes que las sueltas del infierno: son plumas
    // enteras.
    pintar(&quieta, &todas, &plumas, 4.2);
}

/// La cabeza con su cara: pelo, ojos con rimel, colorete y los labios
/// pintados. Todo orientado con `arriba`, que en la rueda da vueltas.
fn cara(c: &Corista, a: &impl Fn(Vec2) -> Vec2, u: &impl Fn(f32) -> f32, piel: Color, ropa: Color) {
    let r = CABEZA * c.talla;
    let (arriba, derecha) = (c.arriba, c.derecha());
    let abajo = -arriba;
    let grados = derecha.y.atan2(derecha.x).to_degrees();
    let en = |dx: f32, dy: f32| a(c.cabeza + derecha * dx * r + abajo * dy * r);
    let elipse = |p: Vec2, rx: f32, ry: f32, color: Color| {
        draw_ellipse(p.x, p.y, u(rx * r), u(ry * r), grados, color);
    };

    let cen = a(c.cabeza);
    circulo(cen, u(r + TINTA_G), TINTA);
    circulo(cen, u(r), piel);

    // El pelo: casquete negro con dos caracoles, y la diadema del penacho.
    elipse(en(0.0, -0.5), 1.04, 0.6, TINTA);
    for lado in [-1.0, 1.0] {
        let p = en(lado * 0.88, -0.05);
        circulo(p, u(0.34 * r), TINTA);
    }
    let joya = en(0.0, -0.86);
    circulo(joya, u(0.28 * r + TINTA_F), TINTA);
    circulo(joya, u(0.28 * r), if c.estrella { ORO } else { ropa });

    // Los ojos: grandes, blancos, con la pupila mirando a la patada y el
    // parpado del rimel encima.
    for (k, lado) in [(0, -1.0f32), (1, 1.0)] {
        let guina = c.guino && k == 1;
        let ojo = en(lado * 0.36, 0.12);
        if c.ko {
            // Mareada: una X de rimel en cada ojo, sobre su blanco.
            circulo(en(lado * 0.36, 0.2), u(0.3 * r), OJO);
            for s in [-1.0, 1.0] {
                let (p, q) = (
                    en(lado * 0.36 - 0.22, 0.2 - 0.22 * s),
                    en(lado * 0.36 + 0.22, 0.2 + 0.22 * s),
                );
                draw_line(p.x, p.y, q.x, q.y, u(TINTA_F * 2.2), TINTA);
            }
            continue;
        }
        if guina {
            // Un guino es una raya de rimel curvada hacia abajo.
            let (p, q) = (en(lado * 0.36 - 0.24, 0.1), en(lado * 0.36 + 0.24, 0.1));
            let m = en(lado * 0.36, 0.2);
            for (s, t) in [(p, m), (m, q)] {
                draw_line(s.x, s.y, t.x, t.y, u(TINTA_F * 1.6), TINTA);
            }
            continue;
        }
        let alto = 0.33 * c.ojos;
        elipse(ojo, 0.25 + TINTA_F / r, alto + TINTA_F / r, TINTA);
        elipse(ojo, 0.25, alto, OJO);
        let pupila = en(lado * 0.36 + c.mirada * 0.1, 0.14 + alto * 0.2);
        let rp = (0.15 - 0.03 * (c.ojos - 0.9).max(0.0)) * r;
        circulo(pupila, u(rp.min(alto * r)), TINTA);
        // El rimel: una raya gorda encima y una pestana hacia fuera.
        let (p, q) = (
            en(lado * 0.36 - 0.28, 0.12 - alto),
            en(lado * 0.36 + 0.28, 0.12 - alto),
        );
        draw_line(p.x, p.y, q.x, q.y, u(TINTA_F * 1.7), TINTA);
        let pestana = en(lado * 0.72, 0.0 - alto * 1.2);
        let rabillo = en(lado * 0.56, 0.12 - alto * 0.8);
        draw_line(
            rabillo.x,
            rabillo.y,
            pestana.x,
            pestana.y,
            u(TINTA_F * 1.4),
            TINTA,
        );
    }

    // El colorete.
    for lado in [-1.0, 1.0] {
        let p = en(lado * 0.6, 0.5);
        circulo(p, u(0.18 * r), COLORETE);
    }

    // La boca: una sonrisa de pintalabios que se abre en una O al patear.
    let boca = en(0.0, 0.56);
    if c.boca > 0.3 {
        let alto = 0.14 + 0.14 * c.boca;
        elipse(boca, 0.2 + TINTA_F / r, alto + TINTA_F / r, TINTA);
        elipse(boca, 0.2, alto, LABIOS);
        elipse(en(0.0, 0.56 + alto * 0.15), 0.11, alto * 0.62, TINTA);
    } else {
        let sonrisa = |ancho: f32, alto: f32| {
            let pts: Vec<Vec2> = (0..=6)
                .map(|k| {
                    let (s, co) = sin_cos(k as f32 / 6.0 * PI);
                    en(co * ancho, 0.5 + s * alto)
                })
                .collect();
            pts
        };
        for (extra, color) in [(TINTA_F / r, TINTA), (0.0, LABIOS)] {
            let pts = sonrisa(0.3 + extra, 0.2 + extra);
            let centro = en(0.0, 0.5 - extra);
            for w in pts.windows(2) {
                draw_triangle(centro, w[0], w[1], color);
            }
        }
    }
}

/// Un trazo que se estrecha y se arquea, como el hueso de `draw.rs`.
///
/// Alli es una hilera de circulos; aqui una tira de cuadrilateros a lo largo
/// de la Bezier de `skeleton::trazo`, con un circulo en cada punta. Sale igual
/// y cuesta la cuarta parte, que en una fila de cinco coristas con plumas es
/// la diferencia entre un jefe y cinco.
fn hueso(a: Vec2, b: Vec2, r0: f32, r1: f32, arqueo: f32, color: Color) {
    const PASOS: usize = 6;
    let punto = |i: usize| trazo(a, b, arqueo, i as f32 / PASOS as f32);
    let radio = |i: usize| (r0 + (r1 - r0) * i as f32 / PASOS as f32).max(0.6);
    let mut antes = (punto(0), Vec2::ZERO);
    for i in 1..=PASOS {
        let p = punto(i);
        let d = (p - antes.0).normalize_or_zero();
        let n = vec2(-d.y, d.x);
        let (na, nb) = (n * radio(i - 1), n * radio(i));
        let q = antes.0;
        draw_triangle(q + na, q - na, p + nb, color);
        draw_triangle(q - na, p - nb, p + nb, color);
        antes = (p, n);
    }
    circulo(a, radio(0), color);
    circulo(b, radio(PASOS), color);
    // Las juntas de la tira, redondeadas donde el trazo se curva mucho.
    if arqueo.abs() > r0 {
        circulo(punto(PASOS / 2), radio(PASOS / 2), color);
    }
}

/// Un circulo con los lados que pide su tamano: a tres pixeles de radio,
/// veinte lados son trabajo tirado.
fn circulo(p: Vec2, r: f32, color: Color) {
    let lados = (r * 1.5).clamp(8.0, 24.0) as u8;
    draw_poly(p.x, p.y, lados, r, 0.0, color);
}

/// Un abanico desde la cadera, con contorno: la falda y las enaguas.
fn abanico(bajo: &[Vec2], cadera: Vec2, a: &impl Fn(Vec2) -> Vec2, tinta: f32, color: Color) {
    let c = a(cadera);
    let pts: Vec<Vec2> = bajo.iter().map(|q| a(*q)).collect();
    for (engorde, col) in [(tinta, TINTA), (0.0, color)] {
        let inflado: Vec<Vec2> = pts
            .iter()
            .map(|p| *p + (*p - c).normalize_or_zero() * engorde)
            .collect();
        for w in inflado.windows(2) {
            draw_triangle(c, w[0], w[1], col);
        }
        if engorde > 0.0 {
            for extremo in [inflado[0], inflado[inflado.len() - 1]] {
                draw_line(c.x, c.y, extremo.x, extremo.y, engorde * 2.0, col);
            }
        }
    }
}

/// Un cuadrilatero convexo con contorno: se engorda empujando cada esquina
/// desde el centro, como el corpino de `draw_figura`.
fn poligono(q: &[Vec2; 4], tinta: f32, color: Color) {
    let centro = (q[0] + q[1] + q[2] + q[3]) * 0.25;
    for (engorde, col) in [(tinta, TINTA), (0.0, color)] {
        let p = q.map(|v| v + (v - centro).normalize_or_zero() * engorde);
        draw_triangle(p[0], p[1], p[2], col);
        draw_triangle(p[0], p[2], p[3], col);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escena(fase: usize, t: f32, vida: f32) -> Escena {
        let dentro = (t / TIEMPO).rem_euclid(1.0);
        Escena {
            centro: Vec2::ZERO,
            escala: 1.0,
            t,
            fase,
            vida,
            pulso: (1.0 - dentro).powf(2.2),
            fuerte: (t / TIEMPO).rem_euclid(2.0) < 1.0,
            tinta: WHITE,
            ropa: WHITE,
            tablas: None,
        }
    }

    /// Lo alto que lleva cada corista el pie mas alto, sobre su cadera y a
    /// su talla: la estrella es mas grande y patea igual que las demas.
    fn pies(fase: usize, t: f32) -> Vec<f32> {
        fila(&escena(fase, t, 1.0), false, None)
            .iter()
            .map(|c| (c.cadera.y - c.pie[0].y.min(c.pie[1].y)) / c.talla)
            .collect()
    }

    #[test]
    fn la_patada_va_a_una_y_la_fila_hace_la_ola() {
        // Es lo que distingue las dos primeras figuras sin leer la cartela, y
        // lo mismo que hacen sus balas: la patada sale a la vez, la fila barre.
        let a_una = pies(0, 3.0);
        assert!(
            a_una.iter().all(|y| (y - a_una[0]).abs() < 0.01),
            "en la patada no van a una: {a_una:?}"
        );
        // En la fila cada una va un quinto de tiempo detras de la de su
        // izquierda, asi que la patada cruza la fila entera en un tiempo.
        for i in 0..4 {
            let paso = desfase(1, i + 1, 5) - desfase(1, i, 5);
            assert!((paso - TIEMPO / 5.0).abs() < 1e-4, "la ola va a saltos");
        }
        // Y se ve: cuando la del centro esta arriba, las puntas no.
        let ola = pies(1, desfase(1, 2, 5) + 3.0);
        assert!(
            ola[2] > ola[0] + 1.0 && ola[2] > ola[4] + 1.0,
            "la ola no pasa por el centro: {ola:?}"
        );
    }

    #[test]
    fn nadie_pisa_por_debajo_de_las_tablas() {
        // Ni en el espagat ni cabeza abajo en la rueda: todo queda por encima
        // del PISO, y `elevar` pone el PISO sobre las tablas.
        for fase in 0..3 {
            for paso in 0..400 {
                let t = paso as f32 * 0.97;
                for vida in [1.0, 0.1] {
                    let e = escena(fase, t, vida);
                    for c in fila(&e, false, None) {
                        let mut puntos = vec![c.cadera, c.pecho, c.cabeza + c.arriba * -CABEZA];
                        puntos.extend(c.rodilla);
                        puntos.extend(c.pie);
                        puntos.extend(c.mano);
                        puntos.extend(c.falda);
                        puntos.extend(c.enagua);
                        puntos.extend(c.penacho(e.pulso, e.fuerte).iter().map(|p| p.1));
                        for p in puntos {
                            assert!(
                                p.y <= PISO + 0.01,
                                "figura {fase}, t {t}: un punto baja a {p} bajo el piso"
                            );
                        }
                    }
                    for (p, ..) in plumas_sueltas(&e, &fila(&e, false, None)) {
                        assert!(p.y <= PISO, "una pluma atraviesa las tablas: {p}");
                    }
                }
            }
        }
        // En el borde del escenario se sube lo justo; arriba no se toca.
        let escala = 3.0;
        let sube = elevar(718.0, Some(741.0), escala);
        assert!((718.0 - sube + (PISO + BAJO_EL_PISO) * escala - 741.0).abs() < 1e-3);
        assert_eq!(elevar(270.0, Some(741.0), escala), 0.0);
        assert_eq!(elevar(718.0, None, escala), 0.0);
    }

    #[test]
    fn la_fila_esta_centrada_en_su_aro_y_cabe() {
        // El aro de golpeo esta en el centro: la fila tiene que estar
        // repartida a los dos lados por igual, y no pasar de ~200 de arena.
        for fase in 0..3 {
            let todas = fila(&escena(fase, 7.0, 1.0), false, None);
            let medio = todas.iter().map(|c| c.cadera.x).sum::<f32>() / todas.len() as f32;
            assert!(medio.abs() < 0.01, "la fila esta descentrada: {medio}");
            let ancho = todas.last().unwrap().cadera.x - todas[0].cadera.x + 2.0 * FALDA * TALLA;
            assert!(
                ancho * crate::bailarines::ESCALA <= 210.0,
                "fila de {ancho}"
            );
        }
    }

    #[test]
    fn caen_en_domino_hasta_el_espagat_sin_atravesar_las_tablas() {
        // Cada una empieza a caer despues que la de su izquierda.
        for i in 1..5 {
            assert!(empieza_a_caer(i, 5) > empieza_a_caer(i - 1, 5));
        }
        // Al final todas abajo, derechas y fuera de combate.
        for i in 0..5 {
            let (espagat, vuelco, llego) = domino(1.0, i, 5);
            assert!((espagat - 1.0).abs() < 1e-3 && vuelco.abs() < 1e-3 && llego);
            assert_eq!(domino(0.0, i, 5), (0.0, 0.0, false));
        }
        let e = escena(0, 50.0, 1.0);
        for paso in 0..=100 {
            for c in fila(&e, false, Some(paso as f32 / 100.0)) {
                let mut puntos = vec![c.cadera, c.pecho, c.cabeza];
                puntos.extend(c.pie);
                puntos.extend(c.falda);
                puntos.extend(c.enagua);
                for p in puntos {
                    assert!(p.y <= PISO + 0.01, "en la caida un punto baja a {p}");
                }
            }
        }
    }
}
