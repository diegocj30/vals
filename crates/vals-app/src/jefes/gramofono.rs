//! **El Charleston**: un gramofono de los anos veinte que cobra vida.
//!
//! Un mueble de madera con patitas que hacen el paso del charleston, el disco
//! girando encima y la bocina de flor como cara: la garganta es la boca, que
//! canta en cada tiempo, y los ojos van en la flor. Diadema de flapper con su
//! pluma y guantes blancos al final de unos brazos de manguera.
//!
//! Cambia con cada figura, como un jefe de Cuphead entre fases:
//!
//! - **El basico**: contento. Rodillas adentro y afuera, patada en la
//!   clave 3-3-2 y los ojos que se cierran de gusto al patear.
//! - **Bee's knees**: agachado, con las manos en las rodillas, que se cambian
//!   de rodilla cada vez que se juntan; los ojos van de un lado a otro y el
//!   disco va al doble.
//! - **El cambio de lado**: dramatico. Las patas son muelles y salta en cada
//!   golpe de la clave; el disco se despega del plato y cabecea en el aire,
//!   y el brazo del fonografo va suelto.
//!
//! Con poca vida todo tiembla y va mas deprisa. Todo sale de `Escena`: es una
//! funcion pura del reloj, sin estado ni azar.

use std::f32::consts::{PI, TAU};

use macroquad::prelude::*;

use super::{Escena, bote, suave, tramo};
use crate::bailarines;
use crate::draw::fade;
use crate::paleta::{ORO, TINTA};

/// El compas del charleston: 100 negras en 4/4, 36 ticks por tiempo.
const TIEMPO: f32 = 36.0;
const COMPAS: f32 = 144.0;
/// La clave 3-3-2: tres golpes por compas, igual que en `bailarines::charleston`.
const CLAVE: [f32; 3] = [0.0, 54.0, 108.0];
/// Lo que dura una patada despues de su golpe.
const PATADA: f32 = 26.0;

/// Colores de detalle: fijos, porque no son la tinta del baile sino el
/// material. Un ojo o un disco no se vuelven blancos con el golpe, y en el
/// cartel en silueta los ojos blancos son justo lo que hace que tenga cara.
const BLANCO: Color = color_u8!(246, 238, 222, 255);
const DISCO: Color = color_u8!(36, 30, 36, 255);
const SURCO: Color = color_u8!(96, 88, 98, 255);
const ETIQUETA: Color = color_u8!(196, 58, 66, 255);
const GARGANTA: Color = color_u8!(62, 22, 30, 255);
const LENGUA: Color = color_u8!(214, 92, 96, 255);
/// El laton del tubo, mas oscuro que el `ORO` de los filos: si fuera igual de
/// claro, el tubo se leia como una vela suelta al lado de la cara.
const LATON: Color = color_u8!(196, 146, 72, 255);

/// Cuanto se encoge el dibujo entero. El cartel del panel lateral solo deja
/// unas 48 unidades a cada lado del centro, y con los brazos abiertos el
/// gramofono a tamano 1 se salia.
const TAM: f32 = 0.85;

/// Pivote del cuerpo: las caderas, bajo el mueble. Al inclinarse gira todo lo
/// de arriba alrededor de aqui, y las patas se quedan en el suelo.
const CADERA: Vec2 = vec2(0.0, 36.0);
/// Donde estan los pies en reposo.
const PIE_X: f32 = 15.0;
const PIE_Y: f32 = 60.0;
/// Medio alto del zapato.
const ZAPATO: f32 = 3.6;

/// En cual de los tres golpes de la clave va el compas, cuanta patada queda
/// (1 justo al caer el golpe y 0 a los `PATADA` ticks) y cuantos ticks van
/// desde el golpe.
fn clave(t: f32) -> (usize, f32, f32) {
    let dentro = t.rem_euclid(COMPAS);
    let golpe = CLAVE.iter().rposition(|&c| dentro >= c).unwrap_or(0);
    let desde = dentro - CLAVE[golpe];
    (golpe, (1.0 - desde / PATADA).max(0.0), desde)
}

/// Cuanto hay que subir el dibujo (en pixeles) para que lo mas bajo, que cae
/// en `bajo` de pantalla, no pase de las tablas.
fn alzar(bajo: f32, tablas: Option<f32>) -> f32 {
    tablas.map_or(0.0, |y| (bajo - y).max(0.0))
}

/// La postura de un instante, en unidades de arena alrededor del centro.
struct Pose {
    /// Desplazamiento y giro del cuerpo entero (mueble, bocina y brazos).
    desp: Vec2,
    giro: f32,
    /// Los pies, ya en su sitio final: no giran con el cuerpo.
    pies: [Vec2; 2],
    /// Rodillas afuera (+) o adentro (-). Es el paso del charleston.
    rodilla: f32,
    /// Las manos, en el marco del cuerpo.
    manos: [Vec2; 2],
    /// De 0 cerrada a 1 abierta del todo.
    boca: f32,
    bocina: f32,
    /// Angulo del disco, cuanto se despega del plato y cuanto se ladea.
    disco: f32,
    vuelo: f32,
    ladeo: f32,
    pluma: f32,
    /// Ojos cerrados de gusto (solo en la primera figura).
    gusto: bool,
    muelles: bool,
    /// Lo que se ha ido el disco del plato volando, en el marco del cuerpo.
    /// Solo al caer.
    fuera: Vec2,
    /// Del susto de la caida: ojos como platos y pupilas de alfiler.
    susto: bool,
    /// Fuera de combate: ojos en X y la lengua fuera.
    ko: bool,
}

impl Pose {
    fn de(e: &Escena) -> Self {
        let (golpe, patada, desde) = clave(e.t);
        // La pierna que patea cambia en el segundo golpe: 3-3-2 es
        // derecha-izquierda-derecha.
        let lado = if golpe == 1 { -1.0 } else { 1.0 };
        // Rodillas adentro y afuera, cambiando en cada tiempo.
        let vaiven = (e.t / TIEMPO * PI).sin();
        let frenesi = 1.0 - e.vida;
        let temblor = (e.t * 2.3).sin() * frenesi * 1.8;
        let mece = (e.t / COMPAS * TAU).sin();
        // La flor se abre en cada tiempo, y mas en el uno del compas: es la
        // bocina la que marca donde empieza la frase.
        let abre = e.pulso * if e.fuerte { 1.6 } else { 1.0 };
        let mut p = Pose {
            desp: vec2(temblor, 0.0),
            giro: 0.0,
            pies: [vec2(-PIE_X, PIE_Y), vec2(PIE_X, PIE_Y)],
            rodilla: vaiven * 6.0,
            manos: [vec2(-40.0, 10.0), vec2(40.0, 10.0)],
            boca: 0.25 + 0.75 * e.pulso,
            bocina: 30.0 * (1.0 + 0.05 * abre),
            disco: e.t * 0.13 * (1.0 + frenesi),
            vuelo: 0.0,
            ladeo: 0.0,
            pluma: (e.t * 0.09).sin() * 0.15 - e.pulso * 0.2,
            gusto: false,
            muelles: false,
            fuera: Vec2::ZERO,
            susto: false,
            ko: false,
        };
        let pie = if lado > 0.0 { 1 } else { 0 };
        match e.fase {
            0 => {
                p.desp += vec2(lado * patada * 3.0, -patada * 5.0 - vaiven.abs() * 1.5);
                p.giro = -lado * patada * 0.10;
                p.pies[pie] += vec2(lado * 9.0, -11.0) * patada;
                // Los brazos van al contrario que las piernas, y el del lado de
                // la patada se dispara hacia arriba.
                p.manos[0].y += 12.0 * vaiven;
                p.manos[1].y -= 12.0 * vaiven;
                p.manos[pie].y -= 14.0 * patada;
                p.gusto = patada > 0.55;
            }
            1 => {
                // Bee's knees: agachado, sin patada, con las manos en las
                // rodillas. Cada vez que las rodillas se juntan (vaiven al
                // minimo) las manos se cambian a la rodilla contraria, una
                // encima de otra, y al abrirse parece que se han atravesado.
                p.desp += vec2(mece * 3.0, 4.0 - e.pulso * 2.0);
                p.giro = mece * 0.06;
                p.rodilla = vaiven * 9.0;
                let cruzadas = ((e.t / TIEMPO - 1.5) / 2.0).floor().rem_euclid(2.0) == 1.0;
                // La rodilla cae a media manguera, combada la mitad de
                // `rodilla`; se pasa al marco del cuerpo restando el desp.
                let (desp, comba) = (p.desp, p.rodilla);
                let rodilla = |s: f32| {
                    let x = s * (PIE_X + 14.0 + comba) * 0.5;
                    vec2(x, (CADERA.y + PIE_Y) * 0.5) - desp * 0.5
                };
                p.manos = if cruzadas {
                    [
                        rodilla(1.0) + vec2(0.0, -2.0),
                        rodilla(-1.0) + vec2(0.0, 2.0),
                    ]
                } else {
                    [rodilla(-1.0), rodilla(1.0)]
                };
                p.boca = 0.45 + 0.55 * e.pulso;
                p.bocina = 31.0 * (1.0 + 0.09 * abre);
                p.disco *= 2.0;
                p.pluma = (e.t * 0.3).sin() * 0.35;
            }
            _ => {
                // Salta de golpe en golpe de la clave: parabola entre dos
                // golpes, y aplastado al caer.
                let largo = if golpe == 2 { COMPAS - CLAVE[2] } else { 54.0 };
                let u = desde / largo;
                let salto = 4.0 * u * (1.0 - u) * 18.0;
                let aplasta = (1.0 - u * 8.0).max(0.0) * 5.0;
                p.desp += vec2(mece * 6.0, aplasta - salto);
                p.giro = (u * TAU).sin() * 0.12 * lado;
                // Los muelles se estiran en el aire y se encogen al caer.
                let muelle = 22.0 + salto * 0.35 - aplasta * 1.2;
                for (i, s) in [-1.0, 1.0].into_iter().enumerate() {
                    p.pies[i] = vec2(s * (PIE_X + 2.0) + p.desp.x, CADERA.y + p.desp.y + muelle);
                }
                p.rodilla = 0.0;
                let brazo = (e.t * 0.2).sin() * 6.0;
                p.manos = [vec2(-38.0, -36.0 + brazo), vec2(38.0, -36.0 - brazo)];
                p.boca = 0.6 + 0.4 * e.pulso;
                p.disco *= 2.4;
                // Poco hacia arriba y mucho de ladeo: la bocina tapa lo que
                // sube, y lo que se lee de lejos es el disco cabeceando.
                p.vuelo = 3.0 + (e.t * 0.2).sin() * 1.5 + salto * 0.15;
                p.ladeo = (e.t * 0.15).sin() * 22.0;
                p.pluma = -0.3 + salto * 0.02;
                p.muelles = true;
            }
        }
        p
    }

    /// Lo mas bajo del dibujo: la suela del zapato mas bajo, con su tinta de
    /// grueso `g`. Tiene que cuadrar con como `patas` pone el zapato.
    fn bajo(&self, g: f32) -> f32 {
        self.pies[0].y.max(self.pies[1].y) + 1.0 + ZAPATO + g
    }
}

/// El lapiz: cada forma dos veces, primero engordada en tinta y luego del
/// tamano real en color. Es el contorno del cartel, igual que
/// en `mapa.rs`, pero con giro: el gramofono se inclina al bailar.
struct Lapiz {
    o: Vec2,
    k: f32,
    /// Grueso de la tinta en unidades de arena.
    g: f32,
    giro: f32,
    desp: Vec2,
}

impl Lapiz {
    /// De unidades del cuerpo a unidades de arena: gira alrededor de las
    /// caderas y se desplaza con el baile.
    fn local(&self, x: f32, y: f32) -> Vec2 {
        let (s, c) = self.giro.sin_cos();
        let v = vec2(x, y) - CADERA;
        CADERA + vec2(v.x * c - v.y * s, v.x * s + v.y * c) + self.desp
    }

    fn p(&self, x: f32, y: f32) -> Vec2 {
        self.o + self.local(x, y) * self.k
    }

    fn pv(&self, v: Vec2) -> Vec2 {
        self.p(v.x, v.y)
    }

    fn disco(&self, x: f32, y: f32, r: f32, c: Color) {
        let a = self.p(x, y);
        draw_circle(a.x, a.y, (r + self.g) * self.k, TINTA);
        draw_circle(a.x, a.y, r * self.k, c);
    }

    fn elipse(&self, x: f32, y: f32, ra: f32, rb: f32, ang: f32, c: Color) {
        let a = self.p(x, y);
        let (k, g, rot) = (self.k, self.g, ang + self.giro.to_degrees());
        draw_ellipse(a.x, a.y, (ra + g) * k, (rb + g) * k, rot, TINTA);
        draw_ellipse(a.x, a.y, ra * k, rb * k, rot, c);
    }

    fn elipse_lisa(&self, x: f32, y: f32, ra: f32, rb: f32, ang: f32, c: Color) {
        let a = self.p(x, y);
        let rot = ang + self.giro.to_degrees();
        draw_ellipse(a.x, a.y, ra * self.k, rb * self.k, rot, c);
    }

    /// Un rectangulo por su centro, girado con el cuerpo.
    fn caja(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        let a = self.p(x, y);
        let (k, g) = (self.k, self.g);
        for (w, h, c) in [(w + g * 2.0, h + g * 2.0, TINTA), (w, h, c)] {
            let params = DrawRectangleParams {
                offset: vec2(0.5, 0.5),
                rotation: self.giro,
                color: c,
            };
            draw_rectangle_ex(a.x, a.y, w * k, h * k, params);
        }
    }

    /// Un trazo sin contorno: detalles de tinta encima de otra forma.
    fn raya(&self, a: Vec2, b: Vec2, w: f32, c: Color) {
        let (a, b) = (self.pv(a), self.pv(b));
        draw_line(a.x, a.y, b.x, b.y, w * self.k, c);
    }

    /// Una linea quebrada con contorno y juntas redondas: miembros, tubos,
    /// muelles. Toda la tinta primero, para que las juntas no corten el color.
    fn trazo(&self, pts: &[Vec2], w: f32, c: Color) {
        let pts: Vec<Vec2> = pts.iter().map(|&v| self.pv(v)).collect();
        for (ancho, color) in [((w + self.g * 2.0) * self.k, TINTA), (w * self.k, c)] {
            for par in pts.windows(2) {
                draw_line(par[0].x, par[0].y, par[1].x, par[1].y, ancho, color);
            }
            for q in &pts {
                draw_circle(q.x, q.y, ancho * 0.5, color);
            }
        }
    }
}

/// Una Bezier cuadratica de `a` a `b` con el control desplazado en
/// perpendicular: el miembro de manguera de los dibujos de los anos veinte.
fn manguera(a: Vec2, b: Vec2, comba: f32) -> [Vec2; 6] {
    let d = b - a;
    let control = (a + b) * 0.5 + vec2(-d.y, d.x).normalize_or_zero() * comba;
    std::array::from_fn(|i| {
        let s = i as f32 / 5.0;
        a.lerp(control, s).lerp(control.lerp(b, s), s)
    })
}

pub fn dibujar(e: &Escena) {
    let pose = Pose::de(e);
    // Pixeles por unidad de arena: la escala del jefe viene multiplicada por
    // la de los bailarines, y este gramofono se mide en unidades de arena para
    // llenar el aro de golpeo, que tiene 46 de radio.
    let k = e.escala / bailarines::ESCALA * TAM;
    let g = (2.2 * k).max(1.5) / k;
    let mut o = e.centro;
    o.y -= alzar(o.y + pose.bajo(g) * k, e.tablas);
    let cuerpo = Lapiz {
        o,
        k,
        g,
        giro: pose.giro,
        desp: pose.desp,
    };
    // Las patas no giran ni se desplazan: sus pies ya estan donde toca.
    let suelo = Lapiz {
        giro: 0.0,
        desp: Vec2::ZERO,
        ..cuerpo
    };
    // La sombra de la tinta: un tono mas oscuro de la misma plancha, para los
    // pliegues de la bocina. Sale de `tinta`, asi que con el golpe tambien
    // se pone blanca y en el cartel sigue siendo silueta.
    let sombra = Color::new(e.tinta.r * 0.78, e.tinta.g * 0.72, e.tinta.b * 0.70, 1.0);

    // El cuello de la bocina, por detras de todo: sale del brazo y se mete
    // detras de la flor.
    let tubo = [vec2(21.0, -4.0), vec2(24.0, -11.0), vec2(15.0, -22.0)];
    cuerpo.trazo(&tubo, 6.5, LATON);
    cuerpo.raya(
        tubo[0] + vec2(1.5, -1.0),
        tubo[1] + vec2(1.5, 0.0),
        1.2,
        ORO,
    );

    patas(&cuerpo, &suelo, &pose, e.ropa);
    brazos(&cuerpo, &pose, e.ropa);
    mueble(&cuerpo, &pose, e.tinta, e.ropa);
    plato(&cuerpo, &pose);
    bocina(&cuerpo, &pose, e, sombra);
}

/// Donde se dobla la bocina al mustiarse: el pie de la flor, sobre el plato.
const TALLO: Vec2 = vec2(0.0, -10.0);
/// Cuando cae redondo: desde ahi, ojos en X.
const REDONDO: f32 = 0.55;

/// La caida del charleston: **el disco se raya**. La aguja rasca, todo
/// tiembla, las patas se doblan y se sienta de golpe, la bocina se mustia
/// como una flor sin agua y el disco sale volando dando vueltas. Curvas de
/// `k`, puras.
#[derive(Debug, Clone, Copy)]
struct Rayada {
    /// Lo fuerte que rasca la aguja, de 1 a 0.
    rasca: f32,
    /// De 0 a 1: lo que se ha sentado, con su bote.
    sienta: f32,
    /// Lo que se ha doblado la bocina hacia la izquierda, en radianes.
    mustia: f32,
    /// Por donde va el vuelo del disco, de 0 (en el plato) a 1 (fuera).
    vuela: f32,
    /// Los brazos: de arriba del susto a colgando.
    cuelgan: f32,
}

fn rayada(k: f32) -> Rayada {
    Rayada {
        rasca: 1.0 - suave(tramo(k, 0.18, 0.4)),
        sienta: bote(tramo(k, 0.15, 0.45)),
        mustia: -1.75 * bote(tramo(k, 0.3, 0.72)),
        vuela: tramo(k, 0.36, 1.0),
        cuelgan: bote(tramo(k, 0.25, 0.55)),
    }
}

/// El gramofono cayendo, con `k` de 0 a 1.
pub fn dibujar_muerte(e: &Escena, k: f32) {
    let r = rayada(k);
    // Se parte de la pose del charleston en reposo, y se va rompiendo.
    let quieto = Escena {
        t: 40.0,
        pulso: 0.0,
        fuerte: false,
        vida: 1.0,
        fase: 0,
        ..*e
    };
    let mut pose = Pose::de(&quieto);
    let tirita = (e.t * 3.1).sin() * 2.5 * r.rasca;
    pose.desp = vec2(tirita, 13.0 * r.sienta);
    pose.giro = (e.t * 2.3).cos() * 0.06 * r.rasca + 0.1 * r.sienta;
    // Al sentarse se le abren las patas, con las rodillas hacia fuera.
    let abre = 9.0 * r.sienta;
    pose.pies = [vec2(-PIE_X - abre, PIE_Y), vec2(PIE_X + abre, PIE_Y)];
    pose.rodilla = 7.0 * r.sienta;
    let arriba = [vec2(-36.0, -30.0), vec2(36.0, -30.0)];
    let colgando = [vec2(-37.0, 30.0), vec2(37.0, 30.0)];
    pose.manos = [0, 1].map(|i| arriba[i].lerp(colgando[i], r.cuelgan));
    pose.susto = k < REDONDO;
    pose.ko = !pose.susto;
    // Grita mientras rasca; tumbado, la boca floja.
    pose.boca = if pose.ko { 0.1 } else { 0.6 + 0.4 * r.rasca };
    pose.bocina = 30.0 - 4.0 * suave(tramo(k, 0.3, 0.8));
    pose.pluma = 1.1 * r.mustia.abs() / 1.75;
    // El disco: primero salta en el plato con cada rascada; luego sale
    // volando hacia arriba y a la derecha, dando vueltas de campana.
    let v = r.vuela;
    pose.disco = e.t * 0.1 * r.rasca + v * 40.0;
    pose.vuelo = if v > 0.0 {
        1.0
    } else {
        ((e.t * 1.3).sin() * 3.0).max(0.0) * r.rasca
    };
    pose.fuera = vec2(260.0 * v, -260.0 * v + 140.0 * v * v);
    pose.ladeo = v * 900.0;

    let px = e.escala / bailarines::ESCALA * TAM;
    let g = (2.2 * px).max(1.5) / px;
    let mut o = e.centro;
    o.y -= alzar(o.y + pose.bajo(g) * px, e.tablas);
    let cuerpo = Lapiz {
        o,
        k: px,
        g,
        giro: pose.giro,
        desp: pose.desp,
    };
    let suelo = Lapiz {
        giro: 0.0,
        desp: Vec2::ZERO,
        ..cuerpo
    };
    // La bocina gira sobre su tallo: otro lapiz, con el giro de mas y el
    // desplazamiento que deja el tallo donde estaba.
    let giro = pose.giro + r.mustia;
    let gira = |a: f32, v: Vec2| Vec2::from_angle(a).rotate(v);
    let flor = Lapiz {
        giro,
        desp: pose.desp + gira(pose.giro, TALLO - CADERA) - gira(giro, TALLO - CADERA),
        ..cuerpo
    };
    let sombra = Color::new(e.tinta.r * 0.78, e.tinta.g * 0.72, e.tinta.b * 0.70, 1.0);

    // El cuello de la bocina, que ahora baja hasta el tallo doblado.
    let tubo = [
        vec2(21.0, -4.0),
        vec2(24.0, -11.0),
        vec2(15.0, -18.0),
        TALLO,
    ];
    cuerpo.trazo(&tubo, 6.5, LATON);

    patas(&cuerpo, &suelo, &pose, e.ropa);
    brazos(&cuerpo, &pose, e.ropa);
    mueble(&cuerpo, &pose, e.tinta, e.ropa);
    if v > 0.0 {
        // El disco ya vuela: por delante de la flor, que se le cruza.
        bocina(&flor, &pose, e, sombra);
        plato(&cuerpo, &pose);
    } else {
        plato(&cuerpo, &pose);
        rayon(&cuerpo, e, r.rasca);
        bocina(&flor, &pose, e, sombra);
    }
}

/// La rayada: un aranazo blanco en zigzag sobre el disco y unos rayos de
/// tinta saltando de la aguja, que parpadean como un disco que salta.
fn rayon(cuerpo: &Lapiz, e: &Escena, rasca: f32) {
    if rasca <= 0.05 {
        return;
    }
    let aranazo = [
        vec2(-18.0, -3.0),
        vec2(-10.0, 0.5),
        vec2(-4.0, -4.0),
        vec2(3.0, 1.0),
        vec2(9.0, -1.0),
    ];
    for par in aranazo.windows(2) {
        cuerpo.raya(par[0], par[1], 1.3, fade(BLANCO, rasca));
    }
    // Cada cuatro ticks cambia de lado: el disco salta.
    let salto = if (e.t / 4.0) as i32 % 2 == 0 {
        1.0
    } else {
        -1.0
    };
    let aguja = vec2(9.0, -1.0);
    for i in 0..3 {
        let dir = Vec2::from_angle(-2.3 + i as f32 * 0.8 + salto * 0.15);
        let n = vec2(-dir.y, dir.x) * 2.2;
        let a = aguja + dir * 6.0;
        let rayo = [
            a,
            a + dir * 4.0 + n,
            a + dir * 7.0 - n,
            a + dir * 11.0 * rasca,
        ];
        for par in rayo.windows(2) {
            cuerpo.raya(par[0], par[1], 1.4, TINTA);
        }
    }
}

/// Las patas: medias verdes de manguera y zapatos de dos tonos. En la ultima
/// figura, muelles.
fn patas(cuerpo: &Lapiz, suelo: &Lapiz, pose: &Pose, ropa: Color) {
    for (i, s) in [-1.0f32, 1.0].into_iter().enumerate() {
        let cadera = cuerpo.local(s * 14.0, CADERA.y - 2.0);
        let pie = pose.pies[i];
        if pose.muelles {
            // Un zigzag de laton entre la cadera y el pie.
            let pts: Vec<Vec2> = (0..=9)
                .map(|j| {
                    let lado = match j {
                        0 | 9 => 0.0,
                        _ if j % 2 == 0 => 1.0,
                        _ => -1.0,
                    };
                    cadera.lerp(pie, j as f32 / 9.0) + vec2(lado * 5.0, 0.0)
                })
                .collect();
            suelo.trazo(&pts, 2.2, ORO);
        } else {
            // La rodilla se va afuera o adentro en perpendicular al muslo.
            suelo.trazo(&manguera(cadera, pie, -s * pose.rodilla), 4.5, ropa);
        }
        // El zapato gira sobre el talon al contrario que la rodilla: rodillas
        // adentro, puntas afuera, que es el giro del charleston. De frente
        // eso se ve como un zapato que se alarga y se acorta.
        //
        // Siempre alargado y con la puntera en un extremo: un ovalo blanco
        // con un punto negro en medio se leia como un ojo.
        let giro = s * (1.0 - pose.rodilla / 9.0).clamp(0.4, 1.6);
        let largo = 5.0 + 2.0 * giro.abs();
        let y = pie.y + 1.0;
        suelo.elipse(pie.x + giro * 2.5, y, largo, ZAPATO, 0.0, BLANCO);
        // La puntera y el tacon de charol: zapatos de dos tonos.
        let punta = pie.x + giro * 2.5 + s * (largo - 2.4);
        suelo.elipse_lisa(punta, y, 2.6, ZAPATO - 0.4, 0.0, TINTA);
        let tacon = pie.x + giro * 2.5 - s * (largo - 1.2);
        suelo.elipse_lisa(tacon, y + 0.8, 1.4, ZAPATO - 1.2, 0.0, TINTA);
    }
}

/// Los brazos: flacos, de manguera, con guante blanco.
fn brazos(cuerpo: &Lapiz, pose: &Pose, ropa: Color) {
    for (i, s) in [-1.0f32, 1.0].into_iter().enumerate() {
        let hombro = vec2(s * 27.0, 10.0);
        let mano = pose.manos[i];
        cuerpo.trazo(&manguera(hombro, mano, s * 7.0), 3.5, ropa);
        // El guante: el punio vuelto y la palma, con dos rayas de dedos.
        let d = (mano - hombro).normalize_or_zero();
        let punio = mano - d * 5.0;
        let ang = d.y.atan2(d.x).to_degrees();
        cuerpo.elipse(punio.x, punio.y, 3.4, 4.2, ang, BLANCO);
        cuerpo.disco(mano.x, mano.y, 5.2, BLANCO);
        cuerpo.raya(mano + vec2(-2.2, 0.0), mano + vec2(-2.2, 3.2), 0.8, TINTA);
        cuerpo.raya(mano + vec2(1.0, 0.0), mano + vec2(1.0, 3.4), 0.8, TINTA);
    }
}

/// El mueble: caja de madera con su rejilla de sol Art Deco y la manivela.
fn mueble(cuerpo: &Lapiz, pose: &Pose, tinta: Color, ropa: Color) {
    // La manivela, detras del lateral: el eje sale y el brazo da vueltas.
    let vuelta = pose.disco * 0.5;
    let fin = vec2(35.0, 24.0 + vuelta.sin() * 5.0);
    cuerpo.trazo(&[vec2(29.0, 24.0), vec2(34.0, 24.0), fin], 1.8, LATON);
    cuerpo.elipse(fin.x + 1.8, fin.y, 1.6, 2.6, 0.0, ORO);

    cuerpo.caja(0.0, 34.0, 62.0, 5.0, ropa);
    cuerpo.caja(0.0, 17.0, 56.0, 30.0, tinta);
    cuerpo.caja(0.0, 1.5, 62.0, 5.0, ropa);
    // La rejilla del altavoz, en abanico: el sol de un club de jazz.
    cuerpo.caja(0.0, 19.0, 38.0, 18.0, ropa);
    let base = vec2(0.0, 27.0);
    for x in [-15.0, -8.0, 0.0, 8.0, 15.0] {
        cuerpo.raya(base, vec2(x, 12.0), 1.3, TINTA);
    }
    cuerpo.elipse(0.0, 27.0, 5.0, 3.0, 0.0, ORO);
}

/// El plato: el disco girando, con su brazo y la aguja.
fn plato(cuerpo: &Lapiz, pose: &Pose) {
    let c = vec2(0.0, -2.0 - pose.vuelo) + pose.fuera;
    let (a, b, rot) = (26.0, 7.0, pose.ladeo);
    cuerpo.elipse(c.x, c.y, a, b, rot, DISCO);
    let q = cuerpo.p(c.x, c.y);
    let giro = rot + cuerpo.giro.to_degrees();
    let (k, w) = (cuerpo.k, (0.7 * cuerpo.k).max(1.0));
    for f in [0.78, 0.56] {
        draw_ellipse_lines(q.x, q.y, a * f * k, b * f * k, giro, w, SURCO);
    }
    // Un brillo que da la vuelta y una marca en la etiqueta: es lo que dice
    // que gira.
    let (s, co) = pose.disco.sin_cos();
    let (sr, cr) = rot.to_radians().sin_cos();
    let en = |f: f32| {
        let v = vec2(co * a * f, s * b * f);
        c + vec2(v.x * cr - v.y * sr, v.x * sr + v.y * cr)
    };
    let brillo = en(0.68);
    cuerpo.elipse_lisa(brillo.x, brillo.y, 3.2, 1.0, rot, SURCO);
    cuerpo.elipse_lisa(c.x, c.y, 7.5, 2.2, rot, ETIQUETA);
    let marca = en(0.18);
    cuerpo.elipse_lisa(marca.x, marca.y, 1.6, 0.9, rot, BLANCO);

    // El brazo del fonografo: sobre el disco en el suelo; en los saltos va
    // suelto y se agita, porque el disco se le ha ido.
    let pivote = vec2(21.0, -4.0);
    let aguja = if pose.vuelo > 0.0 {
        // Al caer, el disco sale dando vueltas de campana y el brazo no lo
        // sigue: se queda colgando.
        pivote + vec2(-9.0, -6.0 + (pose.ladeo * 0.3).clamp(-9.0, 9.0))
    } else {
        vec2(9.0, -1.0)
    };
    cuerpo.trazo(&[pivote, aguja], 2.2, ORO);
    cuerpo.disco(aguja.x, aguja.y, 3.0, ORO);
    cuerpo.disco(pivote.x, pivote.y, 2.6, ORO);
}

/// La bocina de flor: es la cara. Los petalos alternan la tinta y su sombra,
/// la garganta es la boca y los ojos van encima.
fn bocina(cuerpo: &Lapiz, pose: &Pose, e: &Escena, sombra: Color) {
    const PETALOS: usize = 8;
    let cen = vec2(0.0, -38.0);
    let r = pose.bocina;
    let angulo = |i: f32| -PI / 2.0 + (i / PETALOS as f32) * TAU;
    // Cada petalo es un abanico de dos triangulos con la punta abombada y el
    // valle entre petalos un poco hundido: la flor festoneada de una bocina
    // de verdad.
    let borde = |i: usize, rr: f32| -> [Vec2; 5] {
        const PERFIL: [(f32, f32); 5] = [
            (0.0, 0.82),
            (0.22, 0.96),
            (0.5, 1.0),
            (0.78, 0.96),
            (1.0, 0.82),
        ];
        PERFIL.map(|(f, radio)| {
            let a = angulo(i as f32 + f);
            cen + vec2(a.cos(), a.sin()) * (rr * radio)
        })
    };
    let pc = cuerpo.pv(cen);
    for (extra, tinta) in [(cuerpo.g, true), (0.0, false)] {
        for i in 0..PETALOS {
            let color = match (tinta, i % 2) {
                (true, _) => TINTA,
                (false, 0) => e.tinta,
                _ => sombra,
            };
            let pts = borde(i, r + extra * 1.2).map(|v| cuerpo.pv(v));
            for par in pts.windows(2) {
                draw_triangle(pc, par[0], par[1], color);
            }
        }
    }
    // El filo de laton y las costuras entre petalos.
    for i in 0..PETALOS {
        for par in borde(i, r - 1.2).windows(2) {
            cuerpo.raya(par[0], par[1], 1.4, ORO);
        }
        let valle = borde(i, r * 0.97)[0];
        let dentro = cen + (valle - cen) * 0.42;
        cuerpo.raya(dentro, valle, 1.0, TINTA);
    }

    // La diadema de flapper y su pluma, por detras de la cara.
    let joya = vec2(11.0, -64.0);
    let ang = -1.05 + pose.pluma;
    let dir = vec2(ang.cos(), ang.sin());
    let medio = joya + dir * 12.0;
    cuerpo.elipse(medio.x, medio.y, 13.0, 4.2, ang.to_degrees(), BLANCO);
    cuerpo.raya(joya, joya + dir * 23.0, 0.9, TINTA);
    // Arqueada hacia arriba, como sobre una frente: combada hacia abajo se
    // montaba en las cejas y parecia un antifaz.
    let cinta = manguera(vec2(-22.0, -60.0), vec2(22.0, -60.0), -5.0);
    cuerpo.trazo(&cinta, 4.0, e.ropa);
    cuerpo.disco(joya.x, joya.y, 3.4, ORO);

    ojos(cuerpo, pose, e);

    // La boca es la garganta: canta en cada tiempo.
    let (ra, rb) = (8.5 + pose.boca * 2.5, 3.5 + pose.boca * 8.0);
    let bc = vec2(0.0, -26.0);
    cuerpo.elipse(bc.x, bc.y, ra + 1.6, rb + 1.6, 0.0, ORO);
    cuerpo.elipse_lisa(bc.x, bc.y, ra, rb, 0.0, GARGANTA);
    let lengua = rb * 0.42;
    let y = bc.y + rb - lengua - 0.6;
    cuerpo.elipse_lisa(bc.x, y, ra * 0.55, lengua, 0.0, LENGUA);
    if pose.ko {
        // La lengua fuera, colgando por el borde de la boca.
        cuerpo.elipse(bc.x + 3.0, bc.y + rb + 3.0, 3.2, 4.5, 0.0, LENGUA);
    }
}

/// Los ojos y las cejas: lo que mas dice de en que figura va.
fn ojos(cuerpo: &Lapiz, pose: &Pose, e: &Escena) {
    // Miran abajo, a la jugadora; en bee's knees bailan de un lado a otro.
    let mira = match e.fase {
        0 => vec2(0.0, 1.8),
        1 => vec2((e.t * 0.35).sin() * 2.2, 0.6),
        _ => vec2(0.0, 2.4),
    };
    let pupila = if pose.susto {
        1.2
    } else if e.fase == 1 {
        1.8
    } else {
        2.8
    };
    // Las cejas: alegres, disparadas hacia arriba o fruncidas.
    let (ceja_dentro, ceja_fuera) = match e.fase {
        _ if pose.susto => (-60.0, -62.0),
        0 => (-56.5 - e.pulso * 1.5, -55.0),
        1 => (-61.0, -57.0),
        _ => (-52.5, -58.5),
    };
    for s in [-1.0f32, 1.0] {
        let c = vec2(s * 10.0, -46.0);
        if pose.ko {
            for t in [-1.0, 1.0] {
                let raya = [c + vec2(-4.5, -4.5 * t), c + vec2(4.5, 4.5 * t)];
                cuerpo.trazo(&raya, 1.8, TINTA);
            }
            continue;
        }
        if pose.gusto {
            // Ojos cerrados de gusto: un arco hacia arriba.
            let arco = manguera(c + vec2(-5.0, 1.5), c + vec2(5.0, 1.5), -6.0);
            cuerpo.trazo(&arco, 2.2, TINTA);
            // Sin cejas: con ellas encima, dos rayas por ojo se leian enfadadas.
            continue;
        } else {
            cuerpo.elipse(c.x, c.y, 5.6, 7.0, 0.0, BLANCO);
            let p = c + mira;
            cuerpo.elipse_lisa(p.x, p.y, pupila, pupila * 1.2, 0.0, TINTA);
            cuerpo.elipse_lisa(p.x - 0.9, p.y - 1.1, 0.8, 0.8, 0.0, BLANCO);
        }
        let ceja = [vec2(s * 4.0, ceja_dentro), vec2(s * 15.0, ceja_fuera)];
        cuerpo.trazo(&ceja, 1.8, TINTA);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_patada_cae_en_la_clave_3_3_2() {
        // Tres golpes por compas de 144, en 0, 54 y 108: el 3-3-2 en corcheas.
        for (i, &c) in CLAVE.iter().enumerate() {
            for compas in [0.0, 3.0] {
                let (golpe, patada, _) = clave(c + compas * COMPAS);
                assert_eq!(golpe, i);
                assert!((patada - 1.0).abs() < 1e-4, "golpe {i}: patada {patada}");
            }
        }
        // Y entre golpe y golpe la pierna vuelve a su sitio.
        assert_eq!(clave(40.0).1, 0.0);
        assert_eq!(clave(140.0).0, 2);
    }

    #[test]
    fn nunca_baja_de_las_tablas() {
        // Solo sube, y solo lo que haga falta.
        assert_eq!(alzar(500.0, None), 0.0);
        assert_eq!(alzar(500.0, Some(700.0)), 0.0);
        assert_eq!(alzar(750.0, Some(700.0)), 50.0);
    }

    #[test]
    fn en_el_cambio_de_lado_despega_de_verdad() {
        // A mitad de un salto los pies tienen que quedar por encima de donde
        // se apoyan en las otras figuras: si no, los muelles no dicen nada.
        let escena = |fase, t| Escena {
            centro: Vec2::ZERO,
            escala: 1.0,
            t,
            fase,
            vida: 1.0,
            pulso: 0.0,
            fuerte: false,
            tinta: WHITE,
            ropa: WHITE,
            tablas: None,
        };
        let suelo = Pose::de(&escena(0, 40.0)).bajo(0.0);
        let aire = Pose::de(&escena(2, 27.0)).bajo(0.0);
        assert!(
            aire < suelo - 10.0,
            "el cambio de lado no despega: {aire} contra {suelo}"
        );
    }

    #[test]
    fn la_caida_raya_el_disco_y_lo_echa_a_volar() {
        let antes = rayada(0.0);
        assert_eq!(antes.rasca, 1.0);
        assert_eq!((antes.sienta, antes.mustia, antes.vuela), (0.0, 0.0, 0.0));
        let despues = rayada(1.0);
        assert_eq!(despues.rasca, 0.0);
        assert_eq!(despues.vuela, 1.0);
        assert!((despues.sienta - 1.0).abs() < 1e-3);
        assert!(
            (despues.mustia + 1.75).abs() < 1e-3,
            "la bocina no se mustia"
        );
    }
}
