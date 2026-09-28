//! El Tango: un bandoneon vivo.
//!
//! Dos cajas de madera con sus filas de botones y el fuelle en medio, que es
//! donde tiene la cara. **Su baile es el fuelle**: abre y cierra al compas, y
//! lo hace como se baila un tango, de golpe. Llega a la postura en un suspiro,
//! se pasa un pelo y se clava hasta el tiempo siguiente: el corte. Un fuelle
//! que respirase suave seria un vals.
//!
//! Y cambia con cada figura, como los jefes de Cuphead entre fases:
//!
//! - **La caminata**: el galan. Sombrero ladeado, ojos a media asta, la rosa
//!   entre los dientes y el paso lento-lento-rapido-rapido.
//! - **El corte**: se enfada. Ceja en V, dientes apretados, el sombrero calado
//!   y el fuelle entero abierto y cerrado en cada tiempo, con estocadas.
//! - **La quebrada**: se rompe. Las cajas se abren en V, el sombrero sale
//!   volando, el fuelle se raja y echa aire por la raja, saltan botones y la
//!   rosa se deshace. Con poca vida, en cualquier figura, baila a doble tiempo.
//!
//! Todo se dibuja en unidades de arena, las del aro de golpeo (46 de radio):
//! `e.escala` viene multiplicada por `bailarines::ESCALA` y aqui se deshace,
//! que el bandoneon no es una bailarina y no tiene por que medir como ella.

use macroquad::prelude::*;

use super::{Escena, bote, suave, tramo};
use crate::bailarines;
use crate::draw::fade;
use crate::music::{self, Tema};
use crate::paleta::{ORO, TINTA};

/// Los blancos del dibujo: ojos, guantes, dientes. Crema y no blanco, como
/// el papel, para que el blanco puro quede para el destello del golpe.
const CREMA: Color = color_u8!(246, 236, 214, 255);
/// La rosa. Mas oscura que el carmin del cuerpo, para que no se funda con el.
const ROSA: Color = color_u8!(150, 16, 36, 255);
const HOJA: Color = color_u8!(74, 112, 58, 255);
const SUDOR: Color = color_u8!(196, 222, 236, 255);
/// Los petalos que caen al final: mas claros que la rosa, que caen sobre el
/// fuelle y del mismo rojo no se verian.
const PETALO: Color = color_u8!(226, 70, 96, 255);

/// Media caja: 17 x 58. Mas alta que ancha, como las de verdad.
///
/// Abierto del todo mide 80 de ancho y cerrado 58: llena el aro de golpeo (92)
/// sin salirse del cartel del panel lateral, que es lo mas estrecho donde se
/// dibuja. Por eso los brazos van casi siempre hacia arriba o hacia abajo y
/// solo salen hacia fuera con el fuelle cerrado.
const CAJA: Vec2 = Vec2::new(8.5, 29.0);
/// Altura del centro de las cajas: el sombrero sube el conjunto y esto lo baja
/// para que el aro quede lleno por igual.
const CAJA_Y: f32 = 6.0;
/// Media altura del fuelle, un poco menos que la caja.
const FUELLE: f32 = 25.0;
/// Pliegues del fuelle.
const PLIEGUES: usize = 7;
/// Lo mas bajo que llega el dibujo desde el centro: las cajas mas el pisoton
/// y lo que baja una esquina al inclinarse.
const BAJO: f32 = 44.0;
/// Por debajo de esta vida baila a doble tiempo: se le nota el apuro.
const PRISA: f32 = 0.3;
/// Lo que tarda en llegar a una postura, en fraccion de tiempo. El resto del
/// tiempo esta quieto.
const ATAQUE: f32 = 0.14;
/// La cara va mas grande que el resto: a tamano de juego el bandoneon mide
/// ochenta pixeles, y una cara que no se lee no es una cara.
const CARA: f32 = 1.3;

/// Una postura del bandoneon: la que clava en un tiempo del compas.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Paso {
    /// El fuelle: 0 cerrado, 1 abierto del todo.
    abre: f32,
    /// Grados que se inclina el cuerpo entero.
    inclina: f32,
    /// Grados que se abren las cajas en V. Solo en la quebrada.
    abanico: f32,
    /// Las manos, desde el hombro, con la x hacia fuera.
    mano_i: Vec2,
    mano_d: Vec2,
}

const fn paso(abre: f32, inclina: f32, abanico: f32, i: (f32, f32), d: (f32, f32)) -> Paso {
    Paso {
        abre,
        inclina,
        abanico,
        mano_i: Vec2::new(i.0, i.1),
        mano_d: Vec2::new(d.0, d.1),
    }
}

/// Las tres figuras, un compas de 4/4 cada una.
const PASOS: [[Paso; 4]; 3] = [
    // La caminata: lento, lento, rapido, rapido. Abre en el uno y AGUANTA el
    // dos, que es lo elegante; cierra en el tres y respira en el cuatro. Un
    // brazo en alto como si llevase pareja, y cambia de lado.
    [
        paso(0.85, -7.0, 0.0, (3.0, -24.0), (2.0, 16.0)),
        paso(0.9, -7.0, 0.0, (4.0, -27.0), (3.0, 17.0)),
        paso(0.1, 6.0, 0.0, (8.0, 16.0), (10.0, -22.0)),
        paso(0.5, 6.0, 0.0, (6.0, 10.0), (6.0, -26.0)),
    ],
    // El corte: abierto, cerrado, abierto, cerrado. Se tuerce a un lado, se
    // recoge soltando los punos, al otro lado, y los brazos arriba en el
    // cuatro: parado.
    [
        paso(1.0, -10.0, 0.0, (3.0, -24.0), (2.0, 18.0)),
        paso(0.0, 0.0, 0.0, (14.0, -4.0), (14.0, -4.0)),
        paso(1.0, 10.0, 0.0, (2.0, 18.0), (3.0, -24.0)),
        paso(0.0, 0.0, 0.0, (9.0, -26.0), (9.0, -26.0)),
    ],
    // La quebrada: el abrazo roto. Las cajas se tuercen cada una a un lado.
    [
        paso(1.0, -10.0, 10.0, (2.0, -28.0), (2.0, 18.0)),
        paso(0.25, 10.0, -8.0, (12.0, 14.0), (12.0, -24.0)),
        paso(1.0, 6.0, 12.0, (1.0, -30.0), (1.0, -30.0)),
        paso(0.6, -10.0, -10.0, (8.0, 6.0), (8.0, 6.0)),
    ],
];

/// De 0 a 1 en `ATAQUE` y quieto despues. Se pasa un poco antes de clavarse
/// (un "back" de los de animacion): sin ese rebote la parada se lee como que
/// se acaba la cuerda, con el se lee como un golpe de tacon.
fn golpe(dentro: f32) -> f32 {
    if dentro >= ATAQUE {
        return 1.0;
    }
    let x = dentro / ATAQUE - 1.0;
    let (c1, c3) = (1.70158, 2.70158);
    1.0 + c3 * x * x * x + c1 * x * x
}

/// La postura en un instante. `tiempos` cuenta tiempos del compas desde el
/// principio: en cada tiempo entero salta de la postura anterior a la nueva.
fn postura(fase: usize, tiempos: f32) -> Paso {
    let figura = &PASOS[fase.min(PASOS.len() - 1)];
    let b = tiempos.floor();
    let de = figura[(b as i64 - 1).rem_euclid(4) as usize];
    let a = figura[(b as i64).rem_euclid(4) as usize];
    let f = golpe(tiempos - b);
    // Escrito asi y no como `x + (y - x) * f` para que con `f = 1` salga
    // la postura exacta: clavada es clavada.
    let l = |x: f32, y: f32| x * (1.0 - f) + y * f;
    Paso {
        abre: l(de.abre, a.abre),
        inclina: l(de.inclina, a.inclina),
        abanico: l(de.abanico, a.abanico),
        mano_i: de.mano_i.lerp(a.mano_i, f),
        mano_d: de.mano_d.lerp(a.mano_d, f),
    }
}

fn mezcla(a: Color, b: Color, f: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * f,
        a.g + (b.g - a.g) * f,
        a.b + (b.b - a.b) * f,
        1.0,
    )
}

/// Lo que en `mapa.rs` es el `Lapiz`, con giro: todo en unidades de arena
/// alrededor del centro, girado con la inclinacion del cuerpo, y cada forma
/// primero en tinta y un pelo mas grande y luego su relleno.
#[derive(Clone, Copy)]
struct Pluma {
    o: Vec2,
    k: f32,
    ang: f32,
    giro: Vec2,
    /// Grueso de la tinta, en unidades. Al menos pixel y medio: por debajo de
    /// uno el contorno desaparece.
    g: f32,
}

impl Pluma {
    fn new(o: Vec2, k: f32, ang: f32) -> Self {
        Self {
            o,
            k,
            ang,
            giro: Vec2::from_angle(ang),
            g: (2.0 * k).max(1.5) / k,
        }
    }

    /// Otra pluma con el origen en `c` y el dibujo `f` veces mas grande, con
    /// la misma tinta: la raya no engorda porque engorde lo que contornea.
    fn escalada(&self, c: Vec2, f: f32) -> Self {
        Self {
            o: self.p(c),
            k: self.k * f,
            g: self.g / f,
            ..*self
        }
    }

    fn p(&self, v: Vec2) -> Vec2 {
        self.o + self.giro.rotate(v) * self.k
    }

    fn tri(&self, a: Vec2, b: Vec2, c: Vec2, col: Color) {
        draw_triangle(self.p(a), self.p(b), self.p(c), col);
    }

    fn cuad(&self, q: [Vec2; 4], col: Color) {
        self.tri(q[0], q[1], q[2], col);
        self.tri(q[0], q[2], q[3], col);
    }

    fn rect(&self, c: Vec2, m: Vec2, ang: f32, col: Color) {
        let r = Vec2::from_angle(ang);
        let q = [vec2(-m.x, -m.y), vec2(m.x, -m.y), m, vec2(-m.x, m.y)];
        self.cuad(q.map(|v| c + r.rotate(v)), col);
    }

    fn caja(&self, c: Vec2, m: Vec2, ang: f32, col: Color) {
        self.rect(c, m + Vec2::splat(self.g), ang, TINTA);
        self.rect(c, m, ang, col);
    }

    fn disco(&self, c: Vec2, r: f32, col: Color) {
        self.disco_con(c, r, self.g, col);
    }

    /// Un disco con su propia tinta: a un boton de dos unidades la tinta
    /// normal se lo come.
    fn disco_con(&self, c: Vec2, r: f32, g: f32, col: Color) {
        let s = self.p(c);
        draw_circle(s.x, s.y, (r + g) * self.k, TINTA);
        draw_circle(s.x, s.y, r * self.k, col);
    }

    fn ovalo(&self, c: Vec2, r: Vec2, ang: f32, col: Color) {
        let s = self.p(c);
        let grados = (ang + self.ang).to_degrees();
        let (k, g) = (self.k, self.g);
        draw_ellipse(s.x, s.y, (r.x + g) * k, (r.y + g) * k, grados, TINTA);
        draw_ellipse(s.x, s.y, r.x * k, r.y * k, grados, col);
    }

    /// Una linea quebrada, con las juntas redondas para que no se vean los
    /// cortes entre tramos.
    fn trazo(&self, pts: &[Vec2], w: f32, col: Color) {
        let w = (w * self.k).max(1.0);
        for par in pts.windows(2) {
            let (a, b) = (self.p(par[0]), self.p(par[1]));
            draw_line(a.x, a.y, b.x, b.y, w, col);
        }
        for q in &pts[1..pts.len() - 1] {
            let s = self.p(*q);
            draw_circle(s.x, s.y, w * 0.5, col);
        }
    }

    fn trazo_tinta(&self, pts: &[Vec2], w: f32, col: Color) {
        self.trazo(pts, w + self.g * 2.0, TINTA);
        self.trazo(pts, w, col);
    }
}

fn fraccion(x: f32) -> f32 {
    x - x.floor()
}

pub fn dibujar(e: &Escena) {
    let k = e.escala / bailarines::ESCALA;
    let quebrada = e.fase >= 2;

    // El compas sale de la musica de la figura, asi que las posturas caen
    // en el mismo instante que el pulso que ilumina los focos.
    let (ticks, _) = music::compas(Tema::de(1, e.fase));
    let prisa = if e.vida < PRISA { 2.0 } else { 1.0 };
    let tiempos = e.t / ticks * prisa;
    let ps = postura(e.fase, tiempos);
    // El uno pisa mas fuerte que los otros tres: el marcato del tango.
    let acento = e.pulso * if e.fuerte { 1.0 } else { 0.5 };

    // Tiembla con los nervios: nada entero, y en la quebrada siempre algo.
    let nervio = (1.0 - e.vida).powi(3) * if quebrada { 2.5 } else { 1.2 }
        + if quebrada { 0.6 } else { 0.0 };
    let temblor = vec2((e.t * 1.9).sin(), (e.t * 2.7).cos()) * nervio;
    let mut o = e.centro + (temblor + vec2(0.0, 2.0 * acento)) * k;
    if let Some(tablas) = e.tablas {
        o.y -= (o.y + BAJO * k - tablas).max(0.0);
    }
    let pl = Pluma::new(o, k, ps.inclina.to_radians());

    let cu = Cuerpo {
        medio: 12.0 + 11.0 * ps.abre,
        abanico: ps.abanico,
        aplasta: 1.0 - 0.06 * acento,
        comba: 0.0,
    };
    let [it, ib, dt, db] = cuerpo(&pl, e, &cu, e.pulso);
    let caja = |lado| cu.caja(lado);

    // --- Los brazos, por delante de las cajas: pegados a su canto, detras
    // no se verian, y un brazo de manguera que no se ve no dice nada.
    for (lado, mano) in [(-1.0, ps.mano_i), (1.0, ps.mano_d)] {
        let loco = if quebrada {
            vec2((e.t * 0.4 + lado).sin(), (e.t * 0.33 + lado).cos()) * 3.0
        } else {
            Vec2::ZERO
        };
        let hombro = cu.hombro(lado);
        brazo(
            &pl,
            hombro,
            hombro + vec2(lado * mano.x, mano.y) + loco,
            lado,
            e.tinta,
        );
    }

    // --- La cara, en medio del fuelle: se estira cuando el fuelle abre.
    let cara = (it + dt + ib + db) * 0.25 - vec2(0.0, 3.0);
    let separa = 2.5 + 0.3 * cu.medio;
    let mira = -ps.inclina.signum();
    cara_del_tango(&pl, e, cara, separa, mira, acento);

    // --- El sombrero, en el borde de arriba del fuelle.
    let cabeza = it.lerp(dt, 0.5);
    let (donde, giro) = match e.fase {
        0 => (cabeza + vec2(3.0, -1.0), 12f32.to_radians()),
        1 => (cabeza + vec2(0.0, 3.0), (-6f32).to_radians()),
        // Por los aires, dando vueltas.
        _ => (
            cabeza + vec2((e.t * 0.07).sin() * 7.0, -20.0 - 8.0 * acento),
            e.t * 0.12,
        ),
    };
    sombrero(&pl, donde, giro, e.tinta, e.ropa);

    if quebrada {
        // Los botones que saltan, uno cada medio compas, en parabola.
        for (i, &(lado, col, fila)) in SALTAN.iter().enumerate() {
            let f = fraccion(tiempos * 0.5 + i as f32 * 0.25);
            if f > 0.85 {
                continue;
            }
            let lado = lado as f32;
            let (c, ang) = caja(lado);
            let desde = c + Vec2::from_angle(ang).rotate(boton(col, fila));
            let vuelo = vec2(lado * (5.0 + 2.0 * i as f32) * f, -34.0 * f + 50.0 * f * f);
            pl.disco_con(desde + vuelo, 1.8, 0.8, ORO);
        }
    }
    if e.vida < 0.35 {
        // Sudor, una gota a cada lado en cada tiempo.
        let f = fraccion(tiempos);
        for lado in [-1.0, 1.0] {
            let gota = cara
                + vec2(
                    lado * (separa + 6.0 + 10.0 * f),
                    -12.0 - 6.0 * f + 18.0 * f * f,
                );
            pl.ovalo(gota, vec2(1.4, 2.0), 0.0, SUDOR);
        }
    }
}

/// Como estan las cajas y el fuelle en un instante: lo que se abre, lo que
/// se tuercen en V y cuanto se aplasta y se comba el fuelle por el medio.
struct Cuerpo {
    /// Del centro al canto de dentro de cada caja.
    medio: f32,
    /// Grados que se abren las cajas en V.
    abanico: f32,
    /// El pisoton: el fuelle se encoge un pelo de alto en cada golpe.
    aplasta: f32,
    /// Lo que cuelga el fuelle por el medio, en unidades. Solo al
    /// desinflarse: un fuelle sin aire no se sostiene recto.
    comba: f32,
}

impl Cuerpo {
    /// Centro y giro de una caja. Cada caja gira sobre su centro; el fuelle
    /// se engancha a los cantos de dentro, asi que se abre en V solo con que
    /// las cajas se tuerzan.
    fn caja(&self, lado: f32) -> (Vec2, f32) {
        let c = vec2(lado * (self.medio + CAJA.x), CAJA_Y);
        (c, (lado * self.abanico).to_radians())
    }

    fn canto(&self, lado: f32, y: f32) -> Vec2 {
        let (c, ang) = self.caja(lado);
        c + Vec2::from_angle(ang).rotate(vec2(-lado * CAJA.x, y * self.aplasta))
    }

    fn hombro(&self, lado: f32) -> Vec2 {
        let (c, ang) = self.caja(lado);
        c + Vec2::from_angle(ang).rotate(vec2(lado * CAJA.x, -2.0))
    }
}

/// El fuelle y las dos cajas. Devuelve las cuatro esquinas del fuelle
/// —arriba y abajo de la izquierda, arriba y abajo de la derecha—, que es de
/// donde cuelgan la cara y el sombrero.
fn cuerpo(pl: &Pluma, e: &Escena, cu: &Cuerpo, pulso: f32) -> [Vec2; 4] {
    let quebrada = e.fase >= 2;
    let (it, ib) = (cu.canto(-1.0, -FUELLE), cu.canto(-1.0, FUELLE));
    let (dt, db) = (cu.canto(1.0, -FUELLE), cu.canto(1.0, FUELLE));

    // --- El fuelle: pliegues alternos claro y oscuro, que es lo que lo hace
    // leerse como fuelle y no como una caja roja. Los picos de arriba y abajo
    // son los dobleces asomando.
    let oscuro = mezcla(e.tinta, TINTA, 0.2);
    let borde = |i: usize| {
        let s = i as f32 / PLIEGUES as f32;
        let pico = if !i.is_multiple_of(2) { 2.5 } else { 0.0 };
        let cuelga = vec2(0.0, cu.comba * (s * std::f32::consts::PI).sin());
        (
            it.lerp(dt, s) - vec2(0.0, pico) + cuelga,
            ib.lerp(db, s) + vec2(0.0, pico) + cuelga,
        )
    };
    let g = vec2(0.0, pl.g);
    for i in 0..PLIEGUES {
        let ((a, b), (c, d)) = (borde(i), borde(i + 1));
        pl.cuad([a - g, c - g, d + g, b + g], TINTA);
    }
    for i in 0..PLIEGUES {
        let ((a, b), (c, d)) = (borde(i), borde(i + 1));
        let col = if i.is_multiple_of(2) { e.tinta } else { oscuro };
        pl.cuad([a, c, d, b], col);
    }
    for i in 1..PLIEGUES {
        let (a, b) = borde(i);
        pl.trazo(&[a, b], 0.8, TINTA);
    }

    // La raja de la quebrada, con el aire saliendo a cada golpe.
    if quebrada {
        let (a, b) = borde(2);
        let (c, d) = borde(3);
        let centro = a.lerp(b, 0.72).lerp(c.lerp(d, 0.72), 0.5);
        raja(pl, centro, pulso);
    }

    // --- Las cajas: marco de la tinta del baile, tapa de la segunda y los
    // botones en dorado. En la quebrada faltan algunos: son los que saltan.
    for lado in [-1.0, 1.0] {
        let (c, ang) = cu.caja(lado);
        pl.caja(c, CAJA, ang, e.tinta);
        pl.rect(c, CAJA - vec2(3.0, 3.5), ang, e.ropa);
        let giro = Vec2::from_angle(ang);
        for col in 0..2 {
            for fila in 0..6 {
                let donde = c + giro.rotate(boton(col, fila));
                let falta = quebrada && SALTAN.contains(&(lado as i32, col, fila));
                let (r, color) = if falta { (1.1, TINTA) } else { (1.8, ORO) };
                pl.disco_con(donde, r, 0.8, color);
            }
        }
    }
    let cuelga = vec2(0.0, cu.comba);
    [it + cuelga, ib + cuelga, dt + cuelga, db + cuelga]
}

/// Cuando se queda sin aire del todo: desde ahi, ojos en X.
const SIN_AIRE: f32 = 0.8;
/// El suelo de la caida, en unidades desde el centro: lo mas bajo del dibujo.
const PISO: f32 = BAJO;
/// Los petalos que le quedan a la rosa.
const PETALOS: usize = 4;

/// La caida del tango: **se desinfla**. Coge aire por ultima vez, lo suelta
/// en un resoplido largo, el fuelle se descuelga hasta el suelo, se le caen
/// los ultimos petalos a la rosa y el sombrero se va rodando. Curvas de `k`,
/// puras.
#[derive(Debug, Clone, Copy)]
struct Desinfla {
    /// El fuelle, como en `Paso`: de 0 cerrado a 1 abierto (y algo mas en la
    /// ultima bocanada).
    abre: f32,
    /// De 0 a 1, lo que se ha descolgado: baja entero, comba el fuelle y
    /// abre las cajas hacia fuera.
    hunde: f32,
    /// Lo fuerte que resopla, de 0 a 1. Es lo que echa aire por la boca.
    resopla: f32,
    /// Las manos: de arriba del susto a colgando.
    manos: f32,
    /// El sombrero: cayendo (0 a 1) y rodando despues (0 a 1).
    cae: f32,
    rueda: f32,
    /// Por donde va cada petalo, de la flor (0) al suelo (1).
    petalos: [f32; PETALOS],
    ko: bool,
}

fn desinfla(k: f32) -> Desinfla {
    let toma = suave(tramo(k, 0.0, 0.12));
    let suelta = tramo(k, 0.12, SIN_AIRE);
    // Suelta el aire deprisa al principio y cada vez mas despacio: el
    // resoplido se alarga hasta el final, que es lo que lo hace gracioso.
    let vacio = 1.0 - (1.0 - suelta).powi(2);
    Desinfla {
        abre: (0.6 + 0.55 * toma) * (1.0 - vacio),
        hunde: suave(tramo(k, 0.3, 0.9)),
        resopla: if (0.12..SIN_AIRE).contains(&k) {
            1.0 - suelta * 0.7
        } else {
            0.0
        },
        manos: bote(tramo(k, 0.18, 0.55)),
        cae: tramo(k, 0.05, 0.35),
        rueda: 1.0 - (1.0 - tramo(k, 0.35, 0.95)).powi(2),
        petalos: std::array::from_fn(|i| {
            let a = 0.2 + 0.13 * i as f32;
            tramo(k, a, a + 0.35)
        }),
        ko: k >= SIN_AIRE,
    }
}

/// El bandoneon cayendo, con `k` de 0 a 1.
pub fn dibujar_muerte(e: &Escena, k: f32) {
    use std::f32::consts::{PI, TAU};

    let px = e.escala / bailarines::ESCALA;
    let d = desinfla(k);
    let mut o = e.centro;
    if let Some(tablas) = e.tablas {
        o.y -= (o.y + BAJO * px - tablas).max(0.0);
    }
    // Se mece un poco con cada resoplido; lo que baja lo hace el cuerpo.
    let mece = (e.t * 0.5).sin() * 4.0 * d.resopla;
    let pl = Pluma::new(o, px, mece.to_radians());
    let quieta = Pluma::new(o, px, 0.0);
    let aleteo = (e.t * 1.7).sin() * 0.05 * d.resopla;
    let cu = Cuerpo {
        medio: 12.0 + 11.0 * (d.abre + aleteo).max(0.0),
        abanico: 16.0 * d.hunde,
        aplasta: 1.0 - 0.25 * d.hunde,
        comba: 8.0 * d.hunde,
    };
    // Todo el cuerpo baja hasta apoyar las cajas en el suelo.
    let baja = vec2(0.0, (PISO - CAJA_Y - CAJA.y) * d.hunde);
    let pl = Pluma {
        o: pl.p(baja),
        ..pl
    };
    let [it, ib, dt, db] = cuerpo(&pl, e, &cu, d.resopla);

    // Los brazos: arriba del susto y luego se desploman a los costados.
    let arriba = vec2(4.0, -30.0);
    let colgando = vec2(7.0, 24.0 - 6.0 * d.hunde);
    let mano = arriba.lerp(colgando, d.manos);
    for lado in [-1.0, 1.0] {
        let hombro = cu.hombro(lado);
        brazo(
            &pl,
            hombro,
            hombro + vec2(lado * mano.x, mano.y),
            lado,
            e.tinta,
        );
    }

    let cara = (it + dt + ib + db) * 0.25 - vec2(0.0, 3.0);
    let separa = 2.5 + 0.3 * cu.medio;
    let boca = cara_desinflada(&pl, e, &d, cara, separa);

    // El aire: bocanadas que salen de la boca mientras resopla, cada una
    // creciendo y deshaciendose hacia arriba.
    if d.resopla > 0.0 {
        for j in 0..3 {
            let f = fraccion(e.t * 0.03 + j as f32 / 3.0);
            let p = boca + vec2(6.0 + 14.0 * f, -4.0 - 16.0 * f);
            let r = (2.5 + 6.0 * f) * d.resopla.sqrt();
            let s = pl.p(p);
            let a = (1.0 - f).sqrt();
            draw_circle(s.x, s.y, (r + pl.g) * px, fade(TINTA, a));
            draw_circle(s.x, s.y, r * px, fade(CREMA, a));
        }
    }

    // La rosa: un tallo mustio colgando de la boca, y los ultimos petalos
    // cayendo al suelo meciendose. Los que llegan se quedan alli.
    let flor = boca + vec2(9.0, 7.0 + 5.0 * d.hunde);
    pl.trazo_tinta(
        &[boca + vec2(2.0, 0.0), boca + vec2(7.0, 1.0), flor],
        0.9,
        HOJA,
    );
    let quedan = d.petalos.iter().filter(|&&v| v <= 0.0).count();
    pl.disco(flor, 1.2 + 0.7 * quedan as f32, PETALO);
    let desde = pl.p(flor);
    for (j, &v) in d.petalos.iter().enumerate() {
        if v <= 0.0 {
            continue;
        }
        // Se sueltan de la flor y caen al suelo en la pluma quieta: el
        // suelo no se mece.
        let de = (desde - quieta.o) / px;
        // Cada uno se va a un lado al caer: dentro del fuelle no se verian.
        let lado = if j % 2 == 0 { 1.0 } else { -1.0 };
        let x = de.x
            + (v * 9.0 + j as f32).sin() * 4.0 * (1.0 - v)
            + lado * (22.0 + 9.0 * j as f32) * suave(v);
        let y = de.y + (PISO + 1.5 - de.y) * v;
        quieta.ovalo(vec2(x, y), vec2(3.0, 1.7), v * 5.0 + j as f32, PETALO);
    }

    // El sombrero: se cae de la cabeza, bota en el suelo y se va rodando
    // hacia la derecha como una rueda, hasta quedarse de pie en el suelo.
    let cabeza = pl.p(it.lerp(dt, 0.5));
    let cabeza = (cabeza - quieta.o) / px + vec2(0.0, -20.0);
    const RADIO: f32 = 9.0;
    let rueda = RADIO * TAU * d.rueda;
    let suelo_del_ala = PISO - 2.5;
    let cae = vec2(
        cabeza.x + 46.0 * suave(d.cae * 1.6),
        cabeza.y + (suelo_del_ala - cabeza.y) * bote(d.cae),
    );
    let (donde, giro) = if d.rueda <= 0.0 {
        (cae, (1.0 - d.cae) * PI * 1.5 + e.t * 0.12 * (1.0 - d.cae))
    } else {
        // Rueda sobre el borde: gira sobre un punto a media copa, que va a
        // ras de suelo, y el ala va dando la vuelta a su alrededor.
        let eje = vec2(cae.x + rueda, suelo_del_ala - RADIO);
        let giro = rueda / RADIO;
        (eje + Vec2::from_angle(giro).rotate(vec2(0.0, RADIO)), giro)
    };
    sombrero(&quieta, donde, giro, e.tinta, e.ropa);
}

/// La cara desinflandose: los ojos como platos al coger aire, cerrados de
/// esfuerzo al soltarlo y en X al quedarse vacio. Devuelve donde tiene la
/// boca, que es de donde salen el aire y la rosa.
fn cara_desinflada(pl: &Pluma, e: &Escena, d: &Desinfla, c: Vec2, separa: f32) -> Vec2 {
    let boca = c + vec2(0.0, 11.0) * CARA;
    let pl = &pl.escalada(c, CARA);
    let (c, separa) = (Vec2::ZERO, separa / CARA);
    for lado in [-1.0, 1.0] {
        let o = c + vec2(lado * separa, -5.0);
        // Los blancos siempre: sin ellos, una X de tinta sobre el fuelle
        // rojo oscuro no se ve.
        if d.ko || d.resopla > 0.0 {
            pl.ovalo(o, vec2(4.6, 4.6), 0.0, CREMA);
        }
        if d.ko {
            for s in [-1.0, 1.0] {
                pl.trazo(
                    &[o + vec2(-3.6, -3.6 * s), o + vec2(3.6, 3.6 * s)],
                    1.6,
                    TINTA,
                );
            }
        } else if d.resopla > 0.0 {
            // Apretados del esfuerzo: dos angulos, > <.
            pl.trazo(
                &[
                    o + vec2(-3.5 * lado, -3.0),
                    o + vec2(3.0 * lado, 0.0),
                    o + vec2(-3.5 * lado, 3.0),
                ],
                1.5,
                TINTA,
            );
        } else {
            pl.ovalo(o, vec2(5.2, 6.8), 0.0, CREMA);
            pl.disco_con(o + vec2(0.0, 0.5), 1.2, 0.0, TINTA);
        }
    }
    nariz_y_bigote(pl, c, e.tinta);
    let b = vec2(0.0, 11.0);
    if d.ko {
        // La lengua fuera, de lado.
        pl.trazo(&[b + vec2(-5.0, 0.0), b + vec2(5.0, 0.6)], 1.4, TINTA);
        pl.ovalo(b + vec2(2.5, 2.2), vec2(2.0, 2.6), 0.0, ROSA);
    } else {
        // La boca en O, soplando: mas chica cuanto menos aire le queda.
        let r = 2.0 + 2.0 * d.resopla + if d.resopla > 0.0 { 0.0 } else { 1.5 };
        pl.ovalo(b, vec2(r * 0.8, r), 0.0, TINTA);
    }
    boca
}

/// Los botones que saltan en la quebrada: (lado, columna, fila).
const SALTAN: [(i32, usize, usize); 4] = [(-1, 0, 1), (1, 1, 3), (-1, 1, 4), (1, 0, 0)];

/// Donde va un boton en su caja, relativo al centro.
fn boton(col: usize, fila: usize) -> Vec2 {
    vec2(-3.6 + col as f32 * 7.2, -22.5 + fila as f32 * 9.0)
}

/// Un brazo de manguera con su guante. Es una Bezier con el control echado
/// hacia fuera y abajo: el codo no se ve, que es lo que lo hace de goma.
fn brazo(pl: &Pluma, hombro: Vec2, mano: Vec2, lado: f32, col: Color) {
    let dir = mano - hombro;
    let codo =
        (hombro + mano) * 0.5 + vec2(lado * dir.y.abs(), dir.x.abs()).normalize_or_zero() * 3.5;
    let pts: Vec<Vec2> = (0..=8)
        .map(|i| {
            let s = i as f32 / 8.0;
            hombro.lerp(codo, s).lerp(codo.lerp(mano, s), s)
        })
        .collect();
    pl.trazo_tinta(&pts, 2.6, col);
    // El pulgar asoma hacia dentro, por detras de la palma.
    pl.disco(mano + vec2(-lado * 3.0, -3.0), 2.0, CREMA);
    pl.disco(mano, 4.4, CREMA);
    // Los dedos, dos rayas de tinta: sin ellas el guante es una pelota.
    for dy in [-1.2, 1.2] {
        pl.trazo(
            &[mano + vec2(lado * 0.8, dy), mano + vec2(lado * 3.6, dy)],
            0.7,
            TINTA,
        );
    }
}

/// La cara cambia entera con la figura: es donde mas se nota el cambio de
/// fase, igual que en Cuphead.
fn cara_del_tango(pl: &Pluma, e: &Escena, c: Vec2, separa: f32, mira: f32, acento: f32) {
    let pl = &pl.escalada(c, CARA);
    let (c, separa) = (Vec2::ZERO, separa / CARA);
    let ojo = |lado: f32| c + vec2(lado * separa, -5.0);
    match e.fase {
        // El galan: ojos entornados mirando a donde se inclina, una ceja
        // levantada y media sonrisa con la rosa.
        0 => {
            for lado in [-1.0, 1.0] {
                let o = ojo(lado);
                pl.ovalo(o, vec2(4.6, 3.2), 0.0, CREMA);
                pl.disco_con(o + vec2(mira * 1.8, 0.9), 2.0, 0.0, TINTA);
                // El parpado: una raya gruesa encima, caida.
                pl.trazo(
                    &[
                        o + vec2(-5.2, -1.4),
                        o + vec2(0.0, -2.6),
                        o + vec2(5.2, -1.4),
                    ],
                    1.6,
                    TINTA,
                );
                let (alto, ang) = if lado < 0.0 {
                    (-10.5, -10.0)
                } else {
                    (-8.0, 4.0)
                };
                pl.rect(
                    o + vec2(0.0, alto),
                    vec2(5.0, 1.4),
                    f32::to_radians(ang),
                    TINTA,
                );
            }
            nariz_y_bigote(pl, c, e.tinta);
            pl.trazo(
                &[c + vec2(-5.0, 9.0), c + vec2(2.0, 9.4), c + vec2(6.0, 7.6)],
                1.3,
                TINTA,
            );
            rosa(pl, c + vec2(-3.0, 9.2), c + vec2(12.0, 7.0));
        }
        // Enfadado: ojos redondos clavados en la jugadora, cejas en V y los
        // dientes apretados sobre el tallo.
        1 => {
            for lado in [-1.0, 1.0] {
                let o = ojo(lado);
                pl.ovalo(o, vec2(4.8, 6.0), 0.0, CREMA);
                pl.disco_con(o + vec2(-lado * 1.0, 2.0), 1.7, 0.0, TINTA);
                pl.rect(
                    o + vec2(0.0, -7.0),
                    vec2(5.6, 1.8),
                    f32::to_radians(lado * -24.0),
                    TINTA,
                );
            }
            nariz_y_bigote(pl, c, e.tinta);
            let boca = c + vec2(0.0, 10.0);
            pl.caja(boca, vec2(5.8, 2.6), 0.0, CREMA);
            pl.trazo(&[boca - vec2(5.8, 0.0), boca + vec2(5.8, 0.0)], 0.6, TINTA);
            for x in [-2.9, 0.0, 2.9] {
                pl.trazo(&[boca + vec2(x, -2.6), boca + vec2(x, 2.6)], 0.6, TINTA);
            }
            rosa(pl, boca + vec2(4.0, 0.0), c + vec2(-13.0, 8.0));
        }
        // Fuera de si: un ojo enorme y otro chico, las pupilas bailando, las
        // cejas cada una a su aire y la boca abierta gritando. La rosa ya no
        // esta: se le deshace en petalos.
        _ => {
            let bizco = vec2((e.t * 0.9).sin(), (e.t * 1.3).cos());
            for (lado, r, pupila) in [(-1.0, vec2(5.4, 7.2), 2.5), (1.0, vec2(3.8, 4.6), 1.3)] {
                let o = ojo(lado);
                pl.ovalo(o, r, 0.0, CREMA);
                pl.disco_con(o + bizco * lado * 1.2, pupila, 0.0, TINTA);
                let (alto, ang) = if lado < 0.0 {
                    (-10.5, -28.0)
                } else {
                    (-7.0, 20.0)
                };
                pl.rect(
                    o + vec2(0.0, alto),
                    vec2(5.4, 1.7),
                    f32::to_radians(ang),
                    TINTA,
                );
            }
            nariz_y_bigote(pl, c, e.tinta);
            let boca = c + vec2(0.0, 11.0);
            pl.ovalo(boca, vec2(5.0, 3.6 + 2.0 * acento), 0.0, TINTA);
            pl.ovalo(boca + vec2(0.0, 2.0 + acento), vec2(2.8, 1.4), 0.0, ROSA);
            for i in 0..4 {
                let f = fraccion(e.t * 0.008 + i as f32 * 0.25);
                if f > 0.9 {
                    continue;
                }
                let x = -10.0 + i as f32 * 7.0 + (f * 9.0 + i as f32).sin() * 5.0;
                pl.ovalo(c + vec2(x, 6.0 + f * 46.0), vec2(2.2, 1.3), f * 6.0, ROSA);
            }
        }
    }
}

/// Nariz de patata y bigote de lapiz, el del galan de tango de los anos
/// veinte. Es lo que dice "tango" antes que el sombrero.
fn nariz_y_bigote(pl: &Pluma, c: Vec2, tinta: Color) {
    for lado in [-1.0, 1.0] {
        let pts =
            [(0.6, 4.6), (4.0, 5.0), (7.0, 4.1), (8.4, 2.4)].map(|(x, y)| c + vec2(lado * x, y));
        pl.trazo(&pts, 1.5, TINTA);
    }
    pl.ovalo(
        c + vec2(0.0, 1.2),
        vec2(2.6, 2.2),
        0.0,
        mezcla(tinta, TINTA, 0.35),
    );
}

/// Una rosa roja de tallo largo, sujeta en `boca` y con la flor en `flor`.
fn rosa(pl: &Pluma, boca: Vec2, flor: Vec2) {
    pl.trazo_tinta(&[boca, flor], 0.9, HOJA);
    let hoja = boca.lerp(flor, 0.55);
    pl.ovalo(hoja + vec2(0.0, 1.8), vec2(2.4, 1.1), 0.5, HOJA);
    pl.disco(flor, 4.0, ROSA);
    // El remolino de petalos, dos trazos en tinta.
    pl.trazo(
        &[
            flor + vec2(-2.0, 0.5),
            flor + vec2(0.0, -1.8),
            flor + vec2(2.0, 0.2),
        ],
        0.6,
        TINTA,
    );
    pl.trazo(&[flor + vec2(-0.8, 1.6), flor + vec2(1.0, 1.2)], 0.6, TINTA);
}

/// Un fedora: ala plana, copa con la hendidura y la cinta en la tinta del
/// baile. `giro` en radianes, sobre el centro del ala.
fn sombrero(pl: &Pluma, c: Vec2, giro: f32, cinta: Color, fieltro: Color) {
    let r = Vec2::from_angle(giro);
    let a = |x: f32, y: f32| c + r.rotate(vec2(x, y));
    let g = pl.g;
    let copa = |g: f32| {
        [
            a(-12.0 - g, 0.0),
            a(-9.5 - g, -15.0 - g),
            a(9.5 + g, -15.0 - g),
            a(12.0 + g, 0.0),
        ]
    };
    pl.cuad(copa(g), TINTA);
    pl.cuad(copa(0.0), fieltro);
    // La hendidura de arriba.
    pl.tri(a(-4.0, -15.0 - g), a(4.0, -15.0 - g), a(0.0, -11.5), TINTA);
    pl.rect(a(0.0, -3.5), vec2(11.5, 2.0), giro, cinta);
    pl.caja(a(0.0, 0.5), vec2(20.0, 2.0), giro, fieltro);
}

/// La raja del fuelle: un agujero de tinta en estrella y el aire saliendo en
/// cada golpe, mas grande cuanto mas reciente.
fn raja(pl: &Pluma, c: Vec2, pulso: f32) {
    let punta = |i: usize| {
        let r = if i.is_multiple_of(2) { 5.5 } else { 2.4 };
        c + Vec2::from_angle(i as f32 * std::f32::consts::TAU / 10.0 + 0.3) * r
    };
    for i in 0..10 {
        pl.tri(c, punta(i), punta(i + 1), TINTA);
    }
    if pulso > 0.15 {
        for (i, dir) in [vec2(-0.8, 0.9), vec2(-1.0, 0.2), vec2(-0.3, 1.0)]
            .iter()
            .enumerate()
        {
            let lejos = 5.0 + (1.0 - pulso) * 10.0 + i as f32 * 2.0;
            pl.disco(c + *dir * lejos, 1.2 + 3.0 * pulso, CREMA);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_fuelle_se_para_en_seco() {
        // El corte del tango: llega a la postura enseguida y se queda clavado
        // hasta el tiempo siguiente. Si se moviese en la segunda mitad del
        // tiempo seria un vals.
        for (fase, figura) in PASOS.iter().enumerate() {
            for b in 0..8 {
                let b = b as f32;
                assert_eq!(postura(fase, b + ATAQUE), postura(fase, b + 0.99));
                assert_eq!(postura(fase, b + 0.5).abre, figura[b as usize % 4].abre);
            }
        }
        // Y en el golpe sale justo de donde estaba: no hay salto de un frame.
        assert_eq!(golpe(0.0), 0.0);
        assert_eq!(golpe(ATAQUE), 1.0);
        // Se pasa un pelo antes de clavarse.
        let pico = (0..100)
            .map(|i| golpe(i as f32 * ATAQUE / 100.0))
            .fold(0.0, f32::max);
        assert!(pico > 1.02, "sin rebote: {pico}");
    }

    #[test]
    fn el_corte_abre_y_cierra_entero_en_cada_tiempo() {
        let abre: Vec<f32> = (0..4).map(|b| postura(1, b as f32 + 0.5).abre).collect();
        assert_eq!(abre, [1.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn la_caida_coge_aire_y_acaba_vacio_en_el_suelo() {
        let antes = desinfla(0.0);
        assert!(!antes.ko && antes.hunde == 0.0 && antes.rueda == 0.0);
        // La ultima bocanada abre el fuelle mas de lo que estaba.
        assert!(desinfla(0.12).abre > antes.abre + 0.3);
        let despues = desinfla(1.0);
        assert_eq!(despues.abre, 0.0);
        assert_eq!(despues.hunde, 1.0);
        assert_eq!(despues.rueda, 1.0);
        assert!(despues.ko && despues.resopla == 0.0);
        assert!(despues.petalos.iter().all(|&v| v == 1.0));
    }
}
