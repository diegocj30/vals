//! **El Flamenco**: una guitarra del salon que se fue a Sevilla.
//!
//! La caja es el cuerpo y la boca es la del propio instrumento: canta por la
//! boca de la guitarra, con la roseta de oro alrededor, y los ojos van encima,
//! en la tapa. El mastil es el cuello y el clavijero la cabeza, con un clavel
//! prendido. Lleva volante de lunares por debajo de la caja, como un traje de
//! gitana, brazos de manguera y tacones.
//!
//! Cambia con cada figura, como un jefe de Cuphead entre fases:
//!
//! - **La llamada**: erguida, marcando. En cada acento sube un brazo, a un
//!   lado y al otro, y las cuerdas vibran.
//! - **El rasgueo**: se toca sola. Una mano pisa en el mastil y la otra barre
//!   las seis cuerdas en cada acento, abajo y arriba, como sus balas.
//! - **El zapateado**: brazos en jarra y los tacones a contratiempo, rebotando
//!   en cada golpe.
//! - **El remate**: todo a la vez, con un brazo en alto y la otra mano
//!   rasgueando.
//!
//! **Y se tensa**: cuanto mas avanzado va el baile y menos vida le queda,
//! mas se arquea el mastil, mas se frunce el ceno y mas deprisa vibran las
//! cuerdas. Todo sale de `Escena`: funcion pura del reloj, sin estado ni azar.

use std::f32::consts::PI;

use macroquad::prelude::*;
use vals_core::bullets::{ACENTOS_FLAMENCO, TIEMPO_FLAMENCO};

use super::{Escena, bote, suave, tramo};
use crate::bailarines;
use crate::paleta::{ORO, TINTA};

/// El compas de doce, el mismo con el que se quiebran sus balas.
const TIEMPO: f32 = TIEMPO_FLAMENCO as f32;
const COMPAS: f32 = TIEMPO * 12.0;
/// Lo que vibran las cuerdas despues de un acento.
const VIBRA: f32 = 16.0;
/// Lo que tarda el rasgueo en cruzar las cuerdas: seis abanicos cada tres
/// ticks, como en `boss5.ron`.
const RASGUEO: f32 = 15.0;

/// Los colores del material, que no son la tinta del baile.
const BLANCO: Color = color_u8!(246, 238, 222, 255);
const MASTIL: Color = color_u8!(66, 38, 28, 255);
const BOCA: Color = color_u8!(34, 18, 20, 255);
const CUERDA: Color = color_u8!(236, 226, 200, 255);
const CLAVEL: Color = color_u8!(206, 30, 48, 255);
const HOJA: Color = color_u8!(70, 112, 60, 255);

/// Cuanto se encoge el dibujo entero, para que el clavijero quepa en el
/// cartel del panel lateral.
const TAM: f32 = 0.8;

/// Donde gira el cuerpo al inclinarse: la cintura, encima del volante.
const CINTURA: Vec2 = vec2(0.0, 36.0);
/// Las dos curvas de la caja: el lazo de abajo y el de arriba.
const LAZO_BAJO: (Vec2, f32, f32) = (vec2(0.0, 14.0), 28.0, 22.0);
const LAZO_ALTO: (Vec2, f32, f32) = (vec2(0.0, -10.0), 21.0, 17.0);
/// La boca de la guitarra, el puente y el arranque del mastil.
const BOCA_C: Vec2 = vec2(0.0, 4.0);
const PUENTE: f32 = 26.0;
const MASTIL_Y: f32 = -24.0;
/// Lo que mide el mastil hasta la cejuela.
const MASTIL_LARGO: f32 = 42.0;
/// Media separacion entre la primera y la sexta cuerda.
const CUERDAS: f32 = 3.6;
/// Donde estan los pies en reposo, en el marco del suelo.
const PIE: Vec2 = vec2(12.0, 60.0);

/// Cuanto acento queda en `t` (1 justo en el golpe y 0 a los `VIBRA` ticks),
/// cuantos ticks van desde el, y cuantos acentos han caido antes: lo que dice
/// a que lado toca.
fn acento(t: f32) -> (f32, f32, i32) {
    let compas = (t / COMPAS).floor();
    let dentro = t - compas * COMPAS;
    let base = compas as i32 * ACENTOS_FLAMENCO.len() as i32;
    let ultimo = ACENTOS_FLAMENCO
        .iter()
        .map(|&a| (a - 1) as f32 * TIEMPO)
        .enumerate()
        .rfind(|(_, inicio)| dentro >= *inicio);
    // Antes del primer acento del compas, el ultimo fue el doce del anterior.
    let (n, desde) = match ultimo {
        Some((k, inicio)) => (base + k as i32, dentro - inicio),
        None => (base - 1, dentro + COMPAS - 11.0 * TIEMPO),
    };
    ((1.0 - desde / VIBRA).max(0.0), desde, n)
}

/// La postura de un instante, en unidades de arena alrededor del centro.
struct Pose {
    /// Desplazamiento y giro del cuerpo entero.
    desp: Vec2,
    giro: f32,
    /// Lo que se arquea el mastil hacia un lado, en radianes: la tension.
    arco: f32,
    /// Lo que vibran las cuerdas, y lo deprisa.
    vibra: f32,
    prisa: f32,
    /// Las manos, en el marco del cuerpo.
    manos: [Vec2; 2],
    /// Lo que se comban los brazos. En jarra, mucho.
    comba: f32,
    /// Los pies, ya en su sitio: no giran con el cuerpo.
    pies: [Vec2; 2],
    /// De 0 cerrada a 1 abierta del todo.
    boca: f32,
    /// De 0 tranquila a 1 con el ceno fruncido del todo.
    ceno: f32,
    /// Cada cuerda, de la prima al bordon: 0 entera, y de ahi a 1 lo que se
    /// ha enroscado al saltar. Solo al caer.
    rotas: [f32; 6],
    susto: bool,
    ko: bool,
}

impl Pose {
    fn de(e: &Escena) -> Self {
        let (fuerza, desde, n) = acento(e.t);
        let lado = if n.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
        let fase = e.fase.min(3) as f32;
        let tension = (fase / 3.0 * 0.7 + (1.0 - e.vida) * 0.3).clamp(0.0, 1.0);
        let mece = (e.t / COMPAS * 2.0 * PI).sin();
        let floreo = vec2((e.t * 0.12).sin(), (e.t * 0.12).cos()) * 3.0;
        let mut p = Pose {
            desp: vec2(mece * 2.0, -fuerza * 2.0),
            giro: mece * 0.05,
            arco: 0.08 + tension * 0.32,
            vibra: 0.4 * e.pulso + fuerza * (1.0 + tension),
            prisa: 1.2 + tension * 1.4,
            manos: [vec2(-34.0, -30.0) + floreo, vec2(34.0, -30.0) - floreo],
            comba: 7.0,
            pies: [vec2(-PIE.x, PIE.y), vec2(PIE.x, PIE.y)],
            boca: 0.2 + 0.5 * e.pulso + 0.3 * fuerza,
            ceno: tension,
            rotas: [0.0; 6],
            susto: false,
            ko: false,
        };
        // El brazo del lado que toca sube en el acento, y el pie de ese lado
        // pisa.
        let brazo = if lado > 0.0 { 1 } else { 0 };
        let pisa = |p: &mut Pose| {
            p.pies[brazo].y -= 5.0 * (1.0 - fuerza) * fuerza * 4.0;
        };
        // Una mano barriendo las cuerdas sobre la boca, abajo o arriba segun
        // el acento.
        let rasguea = (desde / RASGUEO).clamp(0.0, 1.0);
        let barre = vec2(lado * (14.0 - 28.0 * rasguea), BOCA_C.y + 2.0);
        match e.fase {
            0 => {
                let arriba = vec2(lado * 22.0, -60.0);
                p.manos[brazo] = p.manos[brazo].lerp(arriba, fuerza.max(0.35));
                pisa(&mut p);
            }
            1 => {
                p.manos = [vec2(-7.0, -44.0), barre];
                p.desp.x += mece * 3.0;
                p.giro += lado * fuerza * 0.06;
                p.vibra += 0.6 * (1.0 - rasguea);
            }
            2 => {
                // Tacon y punta, uno cada medio tiempo y alternando pie: se
                // levanta y cae de golpe, y el cuerpo rebota con el.
                let medio = TIEMPO * 0.5;
                let golpe = (e.t / medio).floor() as i32;
                let u = (e.t / medio).fract();
                let pie = golpe.rem_euclid(2) as usize;
                let sube = 4.0 * u * (1.0 - u) * 7.0;
                p.pies[pie].y -= sube;
                p.desp.y += (1.0 - u * 3.0).max(0.0) * 2.5 - fuerza * 3.0;
                p.manos = [vec2(-27.0, 24.0), vec2(27.0, 24.0)];
                p.comba = 13.0;
                p.boca = 0.5 + 0.5 * e.pulso;
            }
            _ => {
                let arriba = vec2(lado * 24.0, -62.0) + floreo;
                p.manos = if brazo == 1 {
                    [barre, arriba]
                } else {
                    [arriba, barre]
                };
                p.giro += lado * 0.08;
                p.vibra += 0.8;
                pisa(&mut p);
            }
        }
        p
    }

    /// Lo mas bajo del dibujo: la suela del tacon mas bajo, con su tinta.
    fn bajo(&self, g: f32) -> f32 {
        self.pies[0].y.max(self.pies[1].y) + 3.0 + g
    }
}

/// Cuanto hay que subir el dibujo (en pixeles) para que lo mas bajo, que cae
/// en `bajo` de pantalla, no pase de las tablas.
fn alzar(bajo: f32, tablas: Option<f32>) -> f32 {
    tablas.map_or(0.0, |y| (bajo - y).max(0.0))
}

/// El lapiz: cada forma dos veces, primero engordada en tinta y luego del
/// tamano real en color, con el giro del cuerpo alrededor de la cintura.
struct Lapiz {
    o: Vec2,
    k: f32,
    g: f32,
    giro: f32,
    desp: Vec2,
}

impl Lapiz {
    fn local(&self, v: Vec2) -> Vec2 {
        CINTURA + Vec2::from_angle(self.giro).rotate(v - CINTURA) + self.desp
    }

    fn pv(&self, v: Vec2) -> Vec2 {
        self.o + self.local(v) * self.k
    }

    fn disco(&self, c: Vec2, r: f32, color: Color) {
        let a = self.pv(c);
        draw_circle(a.x, a.y, (r + self.g) * self.k, TINTA);
        draw_circle(a.x, a.y, r * self.k, color);
    }

    fn disco_liso(&self, c: Vec2, r: f32, color: Color) {
        let a = self.pv(c);
        draw_circle(a.x, a.y, r * self.k, color);
    }

    fn elipse_lisa(&self, c: Vec2, ra: f32, rb: f32, color: Color) {
        let a = self.pv(c);
        let rot = self.giro.to_degrees();
        draw_ellipse(a.x, a.y, ra * self.k, rb * self.k, rot, color);
    }

    fn raya(&self, a: Vec2, b: Vec2, w: f32, c: Color) {
        let (a, b) = (self.pv(a), self.pv(b));
        draw_line(a.x, a.y, b.x, b.y, w * self.k, c);
    }

    /// Una linea quebrada con contorno y juntas redondas. Toda la tinta
    /// primero, para que las juntas no corten el color.
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

    /// Lo mismo sin contorno: las cuerdas, que van encima de todo.
    fn hilo(&self, pts: &[Vec2], w: f32, c: Color) {
        let w = (w * self.k).max(1.0);
        for par in pts.windows(2) {
            let (a, b) = (self.pv(par[0]), self.pv(par[1]));
            draw_line(a.x, a.y, b.x, b.y, w, c);
        }
    }
}

/// Una Bezier cuadratica de `a` a `b` con el control desplazado en
/// perpendicular: el brazo de manguera.
fn manguera(a: Vec2, b: Vec2, comba: f32) -> [Vec2; 6] {
    let d = b - a;
    let control = (a + b) * 0.5 + vec2(-d.y, d.x).normalize_or_zero() * comba;
    std::array::from_fn(|i| {
        let s = i as f32 / 5.0;
        a.lerp(control, s).lerp(control.lerp(b, s), s)
    })
}

/// El mastil, arqueado `arco` hacia un lado: del arranque a la cejuela.
fn mastil(arco: f32) -> [Vec2; 6] {
    let base = vec2(0.0, MASTIL_Y);
    let punta = base + Vec2::from_angle(-PI / 2.0 + arco) * MASTIL_LARGO;
    let control = base + vec2(0.0, -MASTIL_LARGO * 0.55);
    std::array::from_fn(|i| {
        let s = i as f32 / 5.0;
        base.lerp(control, s).lerp(control.lerp(punta, s), s)
    })
}

/// Hacia donde apunta el mastil en su punta.
fn direccion(m: &[Vec2; 6]) -> Vec2 {
    (m[5] - m[4]).normalize_or(vec2(0.0, -1.0))
}

pub fn dibujar(e: &Escena) {
    dibujar_pose(e, &Pose::de(e));
}

fn dibujar_pose(e: &Escena, pose: &Pose) {
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
    let suelo = Lapiz {
        giro: 0.0,
        desp: Vec2::ZERO,
        ..cuerpo
    };

    piernas(&cuerpo, &suelo, pose);
    let m = mastil(pose.arco);
    clavijero(&cuerpo, &m, e.t);
    cuerpo.trazo(&m, 8.0, MASTIL);
    caja(&cuerpo, e);
    volante(&cuerpo, e);
    cara(&cuerpo, pose, e);
    cuerdas(&cuerpo, pose, &m, e.t);
    brazos(&cuerpo, pose, e.ropa);
}

/// Las piernas: cortas, de manguera, con medias de la tinta y tacones.
fn piernas(cuerpo: &Lapiz, suelo: &Lapiz, pose: &Pose) {
    for (i, s) in [-1.0f32, 1.0].into_iter().enumerate() {
        let cadera = cuerpo.local(vec2(s * 9.0, 40.0));
        let pie = pose.pies[i];
        suelo.trazo(&manguera(cadera, pie, -s * 2.0), 4.0, MASTIL);
        // El zapato de flamenca: puntera redonda y el tacon por detras.
        suelo.trazo(
            &[pie + vec2(-s * 1.0, 0.0), pie + vec2(s * 5.0, 0.5)],
            4.4,
            TINTA,
        );
        suelo.trazo(
            &[pie + vec2(-s * 2.0, 0.0), pie + vec2(-s * 2.5, 3.0)],
            1.8,
            TINTA,
        );
    }
}

/// La caja: el ocho de la guitarra, con el puente y la roseta.
fn caja(cuerpo: &Lapiz, e: &Escena) {
    let lazos = [LAZO_BAJO, LAZO_ALTO];
    for (extra, tinta) in [(cuerpo.g, true), (0.0, false)] {
        for (c, ra, rb) in lazos {
            let color = if tinta { TINTA } else { e.tinta };
            cuerpo.elipse_lisa(c, ra + extra, rb + extra, color);
        }
    }
    // El puente.
    cuerpo.trazo(&[vec2(-8.0, PUENTE), vec2(8.0, PUENTE)], 2.6, MASTIL);
}

/// El volante del traje de gitana: festones de la segunda tinta colgando por
/// debajo de la caja, con sus lunares.
fn volante(cuerpo: &Lapiz, e: &Escena) {
    const FESTONES: usize = 7;
    let ancho = 50.0;
    for (extra, tinta) in [(cuerpo.g, true), (0.0, false)] {
        for i in 0..FESTONES {
            let x = -ancho * 0.5 + ancho * (i as f32 + 0.5) / FESTONES as f32;
            let ola = (e.t * 0.1 + i as f32 * 1.3).sin() * 1.2;
            let c = vec2(x, 39.0 + ola);
            let color = if tinta { TINTA } else { e.ropa };
            cuerpo.disco_liso(c, 5.0 + extra, color);
        }
    }
    for i in 0..FESTONES {
        let x = -ancho * 0.5 + ancho * (i as f32 + 0.5) / FESTONES as f32;
        let dy = if i % 2 == 0 { 38.0 } else { 41.0 };
        cuerpo.disco_liso(vec2(x, dy), 1.2, BLANCO);
    }
}

/// La boca de la guitarra es la boca, y los ojos van encima.
fn cara(cuerpo: &Lapiz, pose: &Pose, e: &Escena) {
    // La roseta de oro y la boca, que canta.
    let (ra, rb) = (8.0, 5.0 + pose.boca * 5.0);
    cuerpo.elipse_lisa(BOCA_C, ra + 2.6, rb + 2.6, TINTA);
    cuerpo.elipse_lisa(BOCA_C, ra + 1.6, rb + 1.6, ORO);
    cuerpo.elipse_lisa(BOCA_C, ra, rb, BOCA);
    if pose.ko {
        // La lengua fuera, como el gramofono: tumbado se le va la boca.
        cuerpo.disco(BOCA_C + vec2(3.0, rb + 1.0), 2.6, CLAVEL);
    }

    // Miran abajo, a la jugadora; en el rasgueo, a la mano que rasguea.
    let mira = match e.fase {
        1 => vec2(pose.manos[1].x.clamp(-14.0, 14.0) * 0.12, 1.6),
        _ => vec2(0.0, 1.8),
    };
    let pupila = if pose.susto { 1.1 } else { 2.4 };
    for s in [-1.0f32, 1.0] {
        let c = vec2(s * 9.0, -13.0);
        if pose.ko {
            for t in [-1.0, 1.0] {
                cuerpo.trazo(
                    &[c + vec2(-4.0, -4.0 * t), c + vec2(4.0, 4.0 * t)],
                    1.6,
                    TINTA,
                );
            }
            continue;
        }
        cuerpo.disco(c, 4.8, BLANCO);
        cuerpo.disco_liso(c + mira, pupila, TINTA);
        cuerpo.disco_liso(c + mira + vec2(-0.8, -1.0), 0.7, BLANCO);
        // La ceja: arqueada y tranquila al principio, en V cuando se tensa.
        let dentro = -19.5 + pose.ceno * 3.0 - if pose.susto { 3.0 } else { 0.0 };
        let fuera = -20.5 - pose.ceno * 1.0;
        cuerpo.trazo(&[vec2(s * 4.0, dentro), vec2(s * 13.0, fuera)], 1.6, TINTA);
    }
}

/// El clavijero, con sus seis clavijas y el clavel prendido.
fn clavijero(cuerpo: &Lapiz, m: &[Vec2; 6], t: f32) {
    let d = direccion(m);
    let n = vec2(-d.y, d.x);
    let base = m[5];
    let punta = base + d * 14.0;
    cuerpo.trazo(&[base, punta], 11.0, MASTIL);
    for j in 0..3 {
        let p = base.lerp(punta, 0.25 + 0.3 * j as f32);
        for s in [-1.0, 1.0] {
            cuerpo.disco(p + n * s * 8.0, 1.8, ORO);
        }
    }
    // El clavel, a un lado de la cabeza, que se mece.
    let flor = punta + n * 6.0 + vec2((t * 0.07).sin() * 0.8, 0.0);
    cuerpo.trazo(&[flor, flor + n * 4.0 + d * -4.0], 1.6, HOJA);
    for i in 0..5 {
        let a = i as f32 / 5.0 * 2.0 * PI + t * 0.01;
        cuerpo.disco(flor + Vec2::from_angle(a) * 2.6, 2.4, CLAVEL);
    }
    cuerpo.disco_liso(flor, 1.4, color_u8!(150, 16, 30, 255));
}

/// Las seis cuerdas, del puente a la cejuela: rectas sobre la caja y
/// siguiendo el mastil. Vibran en cada acento, mas fuerte cuanto mas tensa.
/// Al caer, las rotas se enroscan hacia fuera desde sus dos puntas.
fn cuerdas(cuerpo: &Lapiz, pose: &Pose, m: &[Vec2; 6], t: f32) {
    for i in 0..6 {
        let x = -CUERDAS + 2.0 * CUERDAS * i as f32 / 5.0;
        let ancho = 0.6 + 0.12 * i as f32;
        let n = {
            let d = direccion(m);
            vec2(-d.y, d.x)
        };
        let cejuela = m[5] + n * x * 0.6;
        let rota = pose.rotas[i];
        if rota > 0.0 {
            // Un latigazo desde cada punta, que se enrosca hacia su lado.
            let s = if x < 0.0 { -1.0 } else { 1.0 };
            let latigo = |desde: Vec2, hacia: f32, largo: f32| -> [Vec2; 5] {
                std::array::from_fn(|j| {
                    let f = j as f32 / 4.0;
                    desde
                        + vec2(
                            s * rota * 12.0 * f * f,
                            hacia * largo * f * (1.0 - 0.5 * rota),
                        )
                })
            };
            cuerpo.hilo(&latigo(vec2(x, PUENTE), -1.0, 22.0), ancho, CUERDA);
            cuerpo.hilo(&latigo(cejuela, 1.0, 16.0), ancho, CUERDA);
            continue;
        }
        // Onda estacionaria: quieta en las puntas y ancha en medio.
        let fase = t * pose.prisa + i as f32 * 1.3;
        let caja: Vec<Vec2> = (0..=8)
            .map(|j| {
                let f = j as f32 / 8.0;
                let y = PUENTE + (MASTIL_Y - PUENTE) * f;
                let onda = (f * PI).sin() * (fase + f * 2.0).sin() * pose.vibra * 1.6;
                vec2(x + onda, y)
            })
            .collect();
        cuerpo.hilo(&caja, ancho, CUERDA);
        let arriba: Vec<Vec2> = m
            .iter()
            .enumerate()
            .map(|(j, p)| *p + n * x * (1.0 - 0.4 * j as f32 / 5.0))
            .collect();
        // En el mastil van mas juntas y a media luz: a tamano de combate seis
        // hilos claros sobre ocho de madera se leian como un mastil blanco.
        cuerpo.hilo(&arriba, ancho * 0.6, Color { a: 0.45, ..CUERDA });
    }
}

/// Los brazos, de manguera en la segunda tinta, con guante blanco.
fn brazos(cuerpo: &Lapiz, pose: &Pose, ropa: Color) {
    for (i, s) in [-1.0f32, 1.0].into_iter().enumerate() {
        let hombro = vec2(s * 18.0, -14.0);
        let mano = pose.manos[i];
        cuerpo.trazo(&manguera(hombro, mano, s * pose.comba), 3.4, ropa);
        cuerpo.disco(mano, 4.4, BLANCO);
        cuerpo.raya(mano + vec2(-1.6, 0.0), mano + vec2(-1.6, 2.8), 0.8, TINTA);
        cuerpo.raya(mano + vec2(1.2, 0.0), mano + vec2(1.2, 3.0), 0.8, TINTA);
    }
}

/// Cuando cae redonda: desde ahi, ojos en X.
const REDONDA: f32 = 0.6;

/// La caida del flamenco: **se le saltan las cuerdas**. Una detras de otra,
/// de la prima al bordon, cada una con su latigazo; el mastil se dobla como
/// un cuello que se rinde, los brazos caen y se sienta de golpe sobre el
/// volante. Curvas de `k`, puras.
#[derive(Debug, Clone, Copy)]
struct Rota {
    /// Por donde va cada cuerda: 0 entera, 1 enroscada del todo.
    cuerdas: [f32; 6],
    /// De 0 a 1: lo que se ha sentado, con su bote.
    sienta: f32,
    /// Lo que se ha doblado el mastil, en radianes.
    dobla: f32,
    /// Los brazos: de arriba del susto a colgando.
    cuelgan: f32,
}

fn rota(k: f32) -> Rota {
    Rota {
        cuerdas: std::array::from_fn(|i| {
            let a = 0.1 + 0.05 * i as f32;
            suave(tramo(k, a, a + 0.12))
        }),
        sienta: bote(tramo(k, 0.4, 0.7)),
        dobla: 1.3 * bote(tramo(k, 0.35, 0.75)),
        cuelgan: bote(tramo(k, 0.45, 0.7)),
    }
}

/// La guitarra cayendo, con `k` de 0 a 1.
pub fn dibujar_muerte(e: &Escena, k: f32) {
    let r = rota(k);
    let quieta = Escena {
        t: 0.0,
        pulso: 0.0,
        fuerte: false,
        vida: 1.0,
        fase: 0,
        ..*e
    };
    let mut pose = Pose::de(&quieta);
    // Tiembla mientras saltan las cuerdas, y se queda quieta al sentarse.
    let tiembla = (e.t * 2.9).sin() * 2.0 * (1.0 - r.sienta);
    pose.desp = vec2(tiembla, 12.0 * r.sienta);
    pose.giro = 0.18 * r.sienta;
    pose.arco = 0.08 + r.dobla;
    pose.vibra = 1.5 * (1.0 - r.cuerdas[5]);
    pose.prisa = 3.0;
    let abre = 8.0 * r.sienta;
    pose.pies = [vec2(-PIE.x - abre, PIE.y), vec2(PIE.x + abre, PIE.y)];
    let arriba = [vec2(-30.0, -48.0), vec2(30.0, -48.0)];
    let colgando = [vec2(-30.0, 30.0), vec2(30.0, 30.0)];
    pose.manos = [0, 1].map(|i| arriba[i].lerp(colgando[i], r.cuelgan));
    pose.rotas = r.cuerdas;
    pose.susto = k < REDONDA;
    pose.ko = !pose.susto;
    pose.boca = if pose.ko { 0.1 } else { 1.0 };
    pose.ceno = 0.0;
    dibujar_pose(e, &pose);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escena(fase: usize, t: f32, vida: f32) -> Escena {
        Escena {
            centro: Vec2::ZERO,
            escala: 1.0,
            t,
            fase,
            vida,
            pulso: 0.0,
            fuerte: false,
            tinta: WHITE,
            ropa: WHITE,
            tablas: None,
        }
    }

    #[test]
    fn las_cuerdas_vibran_en_los_acentos_del_compas_de_doce() {
        // Los mismos ticks en que se quiebran sus balas: el 3, el 6, el 8,
        // el 10 y el 12 de cada compas de 240.
        for compas in [0.0, 2.0] {
            for a in ACENTOS_FLAMENCO {
                let t = compas * COMPAS + (a - 1) as f32 * TIEMPO;
                assert_eq!(acento(t).0, 1.0, "el {a} no es acento");
            }
        }
        // Entre acento y acento, la cuerda se calla, y en el uno no suena.
        assert_eq!(acento(30.0).0, 0.0);
        assert_eq!(acento(COMPAS).0, 0.0);
        // Y cada acento va al lado contrario que el anterior.
        let lados: Vec<i32> = [40.0, 100.0, 140.0, 180.0, 220.0, 280.0]
            .map(|t| acento(t).2)
            .to_vec();
        assert_eq!(lados, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn se_tensa_con_cada_figura_y_con_la_vida() {
        let arco = |fase, vida| Pose::de(&escena(fase, 30.0, vida)).arco;
        assert!(arco(1, 1.0) > arco(0, 1.0));
        assert!(arco(3, 1.0) > arco(2, 1.0));
        assert!(arco(0, 0.2) > arco(0, 1.0), "con poca vida no se tensa");
    }

    #[test]
    fn en_el_zapateado_levanta_los_pies_a_contratiempo() {
        let pie = |t: f32, i: usize| Pose::de(&escena(2, t, 1.0)).pies[i].y;
        // A mitad del primer medio tiempo sube el izquierdo, y en el
        // siguiente el derecho.
        assert!(pie(5.0, 0) < pie(5.0, 1));
        assert!(pie(15.0, 1) < pie(15.0, 0));
    }

    #[test]
    fn al_caer_se_le_saltan_todas_las_cuerdas() {
        let antes = rota(0.0);
        assert_eq!(antes.cuerdas, [0.0; 6]);
        assert_eq!((antes.sienta, antes.dobla), (0.0, 0.0));
        let despues = rota(1.0);
        assert!(despues.cuerdas.iter().all(|c| (*c - 1.0).abs() < 1e-4));
        assert!((despues.sienta - 1.0).abs() < 1e-3);
        // Y una detras de otra, la prima la primera.
        let medio = rota(0.2);
        for par in medio.cuerdas.windows(2) {
            assert!(par[0] >= par[1], "{:?}", medio.cuerdas);
        }
        assert!(medio.cuerdas[0] > medio.cuerdas[5]);
    }

    #[test]
    fn nunca_baja_de_las_tablas() {
        assert_eq!(alzar(500.0, None), 0.0);
        assert_eq!(alzar(500.0, Some(700.0)), 0.0);
        assert_eq!(alzar(750.0, Some(700.0)), 50.0);
    }
}
