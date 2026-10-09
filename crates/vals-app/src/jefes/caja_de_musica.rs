//! **El Vals**: una caja de musica con cara, la tapa abierta y su bailarina
//! girando encima.
//!
//! Es lo que es un vals en 1900 metido en un objeto: da vueltas porque le han
//! dado cuerda, y baila en tres tiempos porque es lo unico que sabe tocar. Se
//! mece de una pata a otra —un compas a la derecha, el siguiente a la
//! izquierda—, se aplasta en cada tiempo y canta con el peine de la maquina
//! como dientes.
//!
//! Cambia con cada figura, como un jefe de Cuphead entre fases: la tapa se abre
//! mas, la llave gira mas rapido y la cara pasa de contenta a fuera de si. En
//! el fleckerl se rompe: grietas, muelles saltando y la caja botando en cada
//! tiempo.
//!
//! Todo en unidades de la arena, no de la bailarina: el aro de golpeo mide 46
//! de radio en esas unidades, y asi las medidas de aqui se leen contra el aro
//! sin hacer cuentas.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use macroquad::prelude::*;

use super::{Escena, boing, bote, suave, tramo};
use crate::bailarines;
use crate::draw::{Layout, draw_figura, fade};
use crate::music::{self, Tema};
use crate::paleta::{ORO, PAPEL, TINTA};
use crate::skeleton;

/// La y de las patas: el suelo de la caja. Todo se aplasta y se mece desde
/// aqui, que es donde se apoya, y es lo mas bajo del dibujo.
const SUELO: f32 = 48.0;
/// Donde van las patas, a cada lado. Se mece sobre la de fuera.
const PATA_X: f32 = 29.0;
/// El canto de arriba de la caja: de ahi sale la tapa.
const CANTO: f32 = -32.0;
/// Media caja de ancho. Estrecha a proposito: el cartel del panel lateral
/// mide 30 x 44 unidades de bailarina, o sea unas 48 de estas a cada lado, y
/// la llave tiene que caber tambien.
const MEDIO: f32 = 35.0;
/// Unidades de la arena por unidad de bailarina: `Escena::escala` viene en las
/// segundas, porque el andamio dibujaba bailarinas.
const ARENA: f32 = 1.0 / bailarines::ESCALA;
/// La bailarina, algo menos de la mitad que la de antes: `bailarines::ESCALA`
/// la ponia a llenar el aro, y ahora el aro lo llena la caja.
const BAILARINA: f32 = 1.4 * ARENA;

/// El cristal del espejo de la tapa. Las cajas de musica de verdad lo llevan,
/// y aqui ademas separa a la bailarina de la tapa, que es de su misma tinta.
const ESPEJO: Color = color_u8!(188, 204, 212, 255);
/// El fondo de la boca.
const GARGANTA: Color = color_u8!(74, 22, 34, 255);
/// El rubor. Sube con la furia.
const RUBOR: Color = color_u8!(232, 104, 118, 255);

/// Lo que cambia de una figura a la siguiente. Funcion pura de la figura y la
/// vida, para poder probarla sin dibujar nada.
#[derive(Debug, Clone, Copy)]
struct Figura {
    /// Lo que se ve de la tapa abierta, en unidades. Mas alto es mas abierta.
    tapa: f32,
    /// Vueltas de la llave por tick.
    llave: f32,
    /// Cuanto se mece, en radianes.
    vaiven: f32,
    /// De 0 (contenta) a 1 (fuera de si): cejas, parpados, boca y rubor.
    furia: f32,
    /// Lo que bota en cada tiempo. Solo en el fleckerl.
    bote: f32,
    /// Cuantas grietas tiene. Solo en el fleckerl, y mas cuanto menos vida.
    grietas: usize,
}

fn figura(fase: usize, vida: f32) -> Figura {
    let f = fase.min(3);
    // Con poca vida todo va con prisa: la cuerda se acaba y lo nota.
    let prisa = 1.0 - vida.clamp(0.0, 1.0);
    let fleckerl = f == 3;
    Figura {
        tapa: [24.0, 32.0, 40.0, 46.0][f],
        llave: (0.006 + f as f32 * 0.006) * (1.0 + prisa),
        vaiven: (0.06 + f as f32 * 0.02) * (1.0 + 0.5 * prisa),
        furia: (f as f32 / 3.0 + 0.25 * prisa).min(1.0),
        bote: if fleckerl { 9.0 } else { 0.0 },
        grietas: if fleckerl {
            (2 + (prisa * 3.0) as usize).min(GRIETAS.len())
        } else {
            0
        },
    }
}

/// Cuanto hay que subir el dibujo para que nada baje de las tablas.
fn alzar(fondo: f32, tablas: Option<f32>) -> f32 {
    tablas.map_or(0.0, |y| (fondo - y).max(0.0))
}

/// Las grietas de el fleckerl, en el frente de la caja. Fijas: una grieta que
/// cambia de sitio cada frame no es una grieta, es ruido.
const GRIETAS: [&[(f32, f32)]; 5] = [
    &[(-35.0, -18.0), (-28.0, -13.0), (-31.0, -5.0), (-24.0, 0.0)],
    &[(35.0, -24.0), (29.0, -19.0), (32.0, -12.0), (26.0, -8.0)],
    &[(-3.0, -24.0), (1.0, -19.0), (-3.0, -15.0)],
    &[(35.0, 28.0), (28.0, 24.0), (31.0, 17.0)],
    &[(-35.0, 18.0), (-29.0, 22.0), (-32.0, 28.0)],
];

pub fn dibujar(e: &Escena) {
    let fig = figura(e.fase, e.vida);
    let fleckerl = e.fase >= 3;
    let pulso = e.pulso.clamp(0.0, 1.0);
    // El uno del compas pesa mas: es el "oom" del oom-pah-pah.
    let golpe = pulso * if e.fuerte { 1.0 } else { 0.55 };

    // Donde va del compas. La caja se mece un compas hacia cada lado, asi que
    // el balanceo tiene que salir del mismo reloj que la musica, no de un seno
    // cualquiera: si no, se desacompasa en cuanto cambia el tempo de figura.
    let (ticks, tiempos) = music::compas(Tema::de(0, e.fase));
    let compas = e.t / (ticks * tiempos as f32);
    let (lado, _) = (compas * PI).sin_cos();
    let mut giro = fig.vaiven * lado;
    // Con poca vida tiembla. Determinista: sale del reloj, no de un dado.
    giro += (e.t * 1.9).sin() * 0.03 * (1.0 - e.vida) * fig.furia;

    // En el fleckerl bota: despega entre tiempos y cae justo en el golpe.
    let dentro = 1.0 - pulso.powf(1.0 / 2.2);
    let bote = fig.bote * (dentro * PI).sin();

    let k = e.escala * ARENA;
    let aplasta = 0.07 * golpe;
    let (sx, sy) = (1.0 + aplasta * 0.6, 1.0 - aplasta);
    let g = (1.8 * k).max(1.5);
    // Nada por debajo de las tablas: lo mas bajo es la pata de apoyo.
    let fondo = e.centro.y + SUELO * k + g;
    let o = e.centro - vec2(0.0, alzar(fondo, e.tablas) + bote * k);
    let lp = Lapiz {
        o,
        k,
        g,
        sx,
        sy,
        rot: giro.sin_cos(),
        pivote: vec2(PATA_X * sx * giro.signum(), SUELO),
    };

    // --- La tapa, detras de todo. En el fleckerl aletea en cada golpe.
    tapa(&lp, e, fig.tapa + if fleckerl { 10.0 * pulso } else { 0.0 });

    // --- La bailarina, en su peana. No se mece con la caja: gira en su eje,
    // que es lo que hace la de una caja de verdad.
    lp.caja(-6.0, CANTO - 9.0, 12.0, 9.0, ORO);
    let pie = lp.p(0.0, CANTO - 9.0);
    bailarina(e, e.t, pie, 0.0);

    // --- Los muelles de el fleckerl, saltando por las esquinas.
    if fleckerl {
        for (i, s) in [-1.0f32, 1.0].into_iter().enumerate() {
            // Cada uno a contratiempo del otro, para que no parezca uno solo.
            let estira = if i == 0 { pulso } else { 1.0 - pulso };
            muelle(
                &lp,
                vec2((MEDIO - 2.0) * s, CANTO + 2.0),
                vec2(s * 0.7, -1.0),
                12.0 + 12.0 * estira,
            );
        }
    }

    cuerpo(&lp, e);
    for grieta in GRIETAS.iter().take(fig.grietas) {
        lp.trazo(grieta, 1.6, TINTA);
    }

    cara(&lp, e, &fig, lado, fleckerl, golpe);

    llave(&lp, e.t * fig.llave * TAU);
}

/// La tapa abierta, con su espejo. `alto` es lo que se ve de ella: por debajo
/// de unas pocas unidades esta cerrada, y del espejo no se ve nada.
fn tapa(lp: &Lapiz, e: &Escena, alto: f32) {
    let arriba = CANTO - alto;
    lp.poli(
        &[
            vec2(-MEDIO + 2.0, CANTO),
            vec2(-MEDIO + 5.0, arriba),
            vec2(MEDIO - 5.0, arriba),
            vec2(MEDIO - 2.0, CANTO),
        ],
        e.tinta,
    );
    lp.caja(-MEDIO + 4.0, arriba - 1.0, 2.0 * MEDIO - 8.0, 4.0, e.ropa);
    if alto < 10.0 {
        return;
    }
    let medio = CANTO - alto * 0.5;
    lp.elipse(0.0, medio, MEDIO - 11.0, alto * 0.34 + 2.0, ORO);
    lp.elipse(0.0, medio, MEDIO - 14.0, alto * 0.34, ESPEJO);
    // Dos brillos en diagonal: es lo que hace que un gris sea un espejo.
    let b = alto * 0.22;
    lp.linea((-13.0, medio + b), (-5.0, medio - b), 1.6, WHITE);
    lp.linea((-6.0, medio + b), (-1.0, medio - b * 0.4), 1.0, WHITE);
}

/// La bailarina de la caja, con los pies en `pie` (pixeles) y tumbada
/// `giro` radianes sobre ellos: cero de pie, menos un cuarto de vuelta
/// tendida con la cabeza a la izquierda. `t` es el reloj de su giro.
fn bailarina(e: &Escena, t: f32, pie: Vec2, giro: f32) {
    let sd = e.escala * BAILARINA;
    let rot = Vec2::from_angle(giro);
    for mut pose in bailarines::poses(0, e.fase, t, e.vida) {
        let pies = pose.joints[skeleton::PIE_I]
            .y
            .max(pose.joints[skeleton::PIE_D].y);
        // Todo lo de la pose gira alrededor de los pies. Con giro cero queda
        // igual que estaba, solo que con los pies en el origen.
        let g = |v: &mut Vec2| *v = rot.rotate(*v - vec2(0.0, pies));
        pose.joints.iter_mut().for_each(g);
        pose.falda.iter_mut().for_each(g);
        pose.corpino.iter_mut().for_each(g);
        pose.cintas.iter_mut().flatten().for_each(g);
        g(&mut pose.mono);
        draw_figura(
            &pose,
            pie,
            &Layout::con_escala(sd),
            1.0,
            // Las tintas al reves que la caja: de azul sobre la tapa azul se
            // perdia, y en el cartel lateral, negra sobre negro, desaparecia.
            e.ropa,
            e.tinta,
        );
    }
}

/// El cuerpo: canto, caja y zocalo, con sus tachuelas y las patas.
fn cuerpo(lp: &Lapiz, e: &Escena) {
    lp.caja(-MEDIO - 3.0, CANTO, 2.0 * MEDIO + 6.0, 8.0, e.ropa);
    lp.caja(-MEDIO, CANTO + 8.0, 2.0 * MEDIO, 54.0, e.tinta);
    lp.caja(-MEDIO - 3.0, 30.0, 2.0 * MEDIO + 6.0, 10.0, e.ropa);
    for x in [-30.0, -15.0, 0.0, 15.0, 30.0] {
        lp.disco(x, CANTO + 4.0, 1.6, ORO);
        lp.disco(x, 35.0, 1.6, ORO);
    }
    lp.disco(-PATA_X, SUELO - 5.0, 5.0, ORO);
    lp.disco(PATA_X, SUELO - 5.0, 5.0, ORO);
}

/// La llave, al costado, girada `angulo` radianes. Se ve de canto y gira
/// sobre su eje: basta con aplastar las dos orejas con el coseno para que se
/// lea que da vueltas.
fn llave(lp: &Lapiz, angulo: f32) {
    let c = angulo.cos();
    lp.caja(MEDIO, 1.5, 7.0, 4.0, ORO);
    for s in [-1.0, 1.0] {
        lp.elipse(
            MEDIO + 11.0,
            3.5 + s * 6.5 * c,
            3.5,
            6.5 * c.abs() + 1.2,
            ORO,
        );
    }
    lp.disco(MEDIO + 8.5, 3.5, 2.6, ORO);
}

/// Cuando se cierra la tapa de golpe: es el KO. Hasta ahi la caja se
/// desarma asustada; desde ahi tiene los ojos en X.
const PORTAZO: f32 = 0.6;
/// Las puas del peine que le saltan de la boca.
const PUAS: usize = 7;

/// La caida del vals: **la caja se queda sin cuerda**. La llave gira hacia
/// atras, saltan las puas y los muelles, la bailarina se cae de la peana y la
/// tapa se cierra de un portazo. Cada campo es una curva de `k`, pura, para
/// poder probarla sin dibujar.
#[derive(Debug, Clone, Copy)]
struct Desarme {
    /// Vueltas de la llave hacia atras: rapido al principio, frenando.
    llave: f32,
    /// Lo que se ve de la tapa, en unidades.
    tapa: f32,
    /// De 1 a 0: lo que tiembla la caja y aletea la tapa antes del portazo.
    tiembla: f32,
    /// Lo que se tumba la bailarina sobre sus pies, en radianes: hacia la
    /// izquierda, hasta quedar tendida.
    bailarina: f32,
    /// Por donde va su caida de la peana al suelo, de 0 a 1.
    baja: f32,
    /// El aplastado del portazo y lo que se desploma despues.
    sx: f32,
    sy: f32,
    giro: f32,
    /// Por donde va el vuelo de cada pua, de 0 (en la boca) a 1 (en el suelo).
    puas: [f32; PUAS],
    /// Lo que asoma cada muelle, de 0 a 1 con su rebote.
    muelles: [f32; 3],
    /// Ya fuera de combate: ojos en X.
    ko: bool,
}

/// `abierta` es lo que se veia de la tapa al caer.
fn desarme(k: f32, abierta: f32) -> Desarme {
    let vuelta = 1.0 - (1.0 - tramo(k, 0.0, 0.75)).powi(3);
    let cierra = tramo(k, 0.46, PORTAZO);
    // Se abre de par en par del susto y luego cae acelerando, como cae una
    // tapa soltada. Al cerrar rebota una vez.
    let tapa = if k < PORTAZO {
        (abierta + 8.0 * tramo(k, 0.0, 0.2)) * (1.0 - cierra * cierra)
    } else {
        5.0 * (PI * tramo(k, PORTAZO, 0.7)).sin()
    };
    let golpe = (PI * tramo(k, PORTAZO, PORTAZO + 0.1)).sin();
    let hunde = suave(tramo(k, 0.66, 0.9));
    let vuelca = tramo(k, 0.12, 0.32);
    Desarme {
        llave: -6.0 * vuelta,
        tapa,
        tiembla: 1.0 - suave(tramo(k, 0.3, PORTAZO)),
        // Primero se inclina despacio sobre la peana y luego, ya cayendo, se
        // acaba de tumbar en el aire.
        bailarina: -1.3 * vuelca * vuelca - (FRAC_PI_2 - 1.3) * tramo(k, 0.32, 0.5),
        baja: tramo(k, 0.3, 0.56),
        sx: 1.0 + 0.08 * golpe + 0.04 * hunde,
        sy: 1.0 - 0.12 * golpe - 0.06 * hunde,
        giro: 0.07 * hunde,
        puas: std::array::from_fn(|i| {
            let a = 0.04 + 0.05 * i as f32;
            tramo(k, a, a + 0.3)
        }),
        muelles: std::array::from_fn(|i| {
            let a = 0.08 + 0.1 * i as f32;
            boing(tramo(k, a, a + 0.4))
        }),
        ko: k >= PORTAZO,
    }
}

/// La caja cayendo, con `k` de 0 a 1.
pub fn dibujar_muerte(e: &Escena, k: f32) {
    let fig = figura(e.fase, 0.0);
    let d = desarme(k, fig.tapa);
    let px = e.escala * ARENA;
    let g = (1.8 * px).max(1.5);
    let fondo = e.centro.y + SUELO * px + g;
    let o = e.centro - vec2(0.0, alzar(fondo, e.tablas));
    let giro = d.giro + (e.t * 2.1).sin() * 0.05 * d.tiembla;
    let lp = Lapiz {
        o,
        k: px,
        g,
        sx: d.sx,
        sy: d.sy,
        rot: giro.sin_cos(),
        pivote: vec2(PATA_X * d.sx, SUELO),
    };
    // Lo que ya no va con la caja —la bailarina, las puas, el polvo— se
    // dibuja quieto, sin mecerse ni aplastarse con ella.
    let suelo = Lapiz {
        sx: 1.0,
        sy: 1.0,
        rot: (0.0, 1.0),
        ..lp
    };

    tapa(&lp, e, d.tapa + (e.t * 0.9).sin() * 5.0 * d.tiembla);
    if !d.ko {
        lp.caja(-6.0, CANTO - 9.0, 12.0, 9.0, ORO);
    }

    // Los muelles salen de las esquinas y del costado, y se quedan fuera
    // cimbreandose.
    let salen = [
        (vec2(-MEDIO + 2.0, CANTO + 2.0), vec2(-0.7, -1.0)),
        (vec2(MEDIO - 2.0, CANTO + 2.0), vec2(0.8, -1.0)),
        (vec2(-MEDIO, 18.0), vec2(-1.0, -0.25)),
    ];
    for (i, ((base, dir), m)) in salen.into_iter().zip(d.muelles).enumerate() {
        if m > 0.02 {
            let cimbrea = Vec2::from_angle((e.t * 0.5 + i as f32 * 2.0).sin() * 0.12);
            muelle(&lp, base, cimbrea.rotate(dir), 26.0 * m);
        }
    }

    cuerpo(&lp, e);
    let rotas = 2 + (k * 4.0) as usize;
    for grieta in GRIETAS.iter().take(rotas) {
        lp.trazo(grieta, 1.6, TINTA);
    }
    if d.tapa < 4.0 {
        // Cerrada: la tapa es una tabla encima del canto, y tapa la peana.
        lp.caja(
            -MEDIO - 1.0,
            CANTO - 7.0 - d.tapa,
            2.0 * MEDIO + 2.0,
            7.0,
            e.tinta,
        );
        lp.caja(
            -MEDIO + 3.0,
            CANTO - 8.0 - d.tapa,
            2.0 * MEDIO - 6.0,
            2.5,
            e.ropa,
        );
    }
    cara_ko(&lp, e, &d);
    llave(&lp, d.llave * TAU);

    // La bailarina: se tumba sobre la peana y se cae por la izquierda,
    // botando al llegar al suelo.
    let (desde, hasta) = (vec2(0.0, CANTO - 9.0), vec2(-MEDIO - 12.0, SUELO - 3.0));
    let pie = vec2(
        desde.x + (hasta.x - desde.x) * d.baja,
        desde.y + (hasta.y - desde.y) * bote(d.baja),
    );
    bailarina(e, 0.0, suelo.p(pie.x, pie.y), d.bailarina);

    // Las puas del peine, en parabola hasta el suelo, y ahi se quedan
    // tumbadas. Cada una a un lado y a su distancia.
    for (i, &v) in d.puas.iter().enumerate() {
        if v <= 0.0 {
            continue;
        }
        let x0 = -14.0 + 28.0 * (i as f32 + 0.5) / PUAS as f32;
        let lado = if i % 2 == 0 { -1.0 } else { 1.0 };
        let x = x0 + lado * (26.0 + 9.0 * (i % 3) as f32) * v;
        let alto = 18.0 + 4.0 * (i % 2) as f32;
        let y = 10.0 + (SUELO - 12.0) * v - alto * 4.0 * v * (1.0 - v);
        let dir = Vec2::from_angle(FRAC_PI_2 + v * TAU * 2.0) * 3.4;
        let (a, b) = ((x - dir.x, y - dir.y), (x + dir.x, y + dir.y));
        suelo.linea(a, b, 5.0, TINTA);
        suelo.linea(a, b, 2.6, ORO);
    }

    // El portazo levanta polvo en las esquinas y suelta unas rayas de golpe.
    let polvo = tramo(k, PORTAZO, PORTAZO + 0.25);
    if polvo > 0.0 && polvo < 1.0 {
        let a = 1.0 - polvo;
        for s in [-1.0f32, 1.0] {
            for j in 0..3 {
                let j = j as f32;
                let c = suelo.p(
                    s * (MEDIO + 4.0 + 14.0 * polvo + 3.0 * j),
                    CANTO - 4.0 - 6.0 * j * polvo,
                );
                let r = (2.0 + 5.0 * polvo) * px;
                draw_circle(c.x, c.y, r + g, fade(TINTA, a));
                draw_circle(c.x, c.y, r, fade(PAPEL, a));
            }
            for j in 0..3 {
                let dir = Vec2::from_angle(-FRAC_PI_2 + s * (0.5 + 0.35 * j as f32));
                let base = vec2(s * (MEDIO - 6.0), CANTO - 12.0);
                let (p, q) = (
                    base + dir * (6.0 + 8.0 * polvo),
                    base + dir * (12.0 + 12.0 * polvo),
                );
                suelo.linea((p.x, p.y), (q.x, q.y), 2.2, fade(TINTA, a));
            }
        }
    }

    // Y fuera de combate, estrellitas dando vueltas por encima.
    if d.ko {
        for i in 0..3 {
            let a = e.t * 0.08 + i as f32 * TAU / 3.0;
            let c = vec2(a.cos() * 24.0, CANTO - 20.0 + a.sin() * 5.0);
            let puntas: Vec<Vec2> = (0..8)
                .map(|j| {
                    let r = if j % 2 == 0 { 4.5 } else { 1.8 };
                    c + Vec2::from_angle(j as f32 * TAU / 8.0 + a) * r
                })
                .collect();
            suelo.poli(&puntas, ORO);
        }
    }
}

/// La cara de la caida: del susto —ojos como platos, pupilas dando vueltas y
/// la boca abierta soltando las puas— a los ojos en X del KO, con la lengua
/// fuera y sin un diente.
fn cara_ko(lp: &Lapiz, e: &Escena, d: &Desarme) {
    let ey = -8.0;
    for s in [-1.0f32, 1.0] {
        let ex = 14.5 * s;
        lp.disco_liso(ex + 5.0 * s, ey + 13.0, 5.0, fade(RUBOR, 0.6));
        if d.ko {
            for t in [-1.0, 1.0] {
                lp.linea(
                    (ex - 5.0, ey - 5.0 * t),
                    (ex + 5.0, ey + 5.0 * t),
                    3.2,
                    TINTA,
                );
            }
            continue;
        }
        lp.elipse(ex, ey, 10.0, 13.5, WHITE);
        // Mareada: las pupilas dan vueltas, cada una a su aire.
        let a = e.t * 0.35 + s;
        lp.disco_liso(ex + 3.5 * a.cos(), ey + 4.0 * a.sin(), 2.4, TINTA);
        lp.linea(
            (ex - 8.0 * s, ey - 20.0),
            (ex + 8.0 * s, ey - 23.0),
            3.0,
            TINTA,
        );
    }
    if d.ko {
        // Una raya que hace ondas, y la lengua colgando por un lado.
        lp.disco(7.0, 13.5, 3.2, RUBOR);
        let pts: Vec<(f32, f32)> = (0..=8)
            .map(|i| {
                let x = -13.0 + 26.0 * i as f32 / 8.0;
                (x, 11.0 + (x * 0.7).sin() * 1.6)
            })
            .collect();
        lp.trazo(&pts, 2.4, TINTA);
        return;
    }
    // La boca abierta del susto, con las puas que aun no han saltado.
    lp.elipse(0.0, 14.0, 11.0, 8.5, GARGANTA);
    for (i, &v) in d.puas.iter().enumerate() {
        if v <= 0.0 {
            let x = -14.0 + 28.0 * (i as f32 + 0.5) / PUAS as f32;
            let y = 14.0 - 8.5 * (1.0 - (x / 11.0).powi(2)).max(0.0).sqrt();
            lp.caja(x - 1.0, y, 2.0, 3.5, ORO);
        }
    }
}

/// Los ojos, las cejas, el rubor y la boca.
fn cara(lp: &Lapiz, e: &Escena, fig: &Figura, lado: f32, fleckerl: bool, golpe: f32) {
    let furia = fig.furia;
    let (ey, ew) = (-8.0, 9.5);
    // En el fleckerl los ojos se abren de par en par y las pupilas se encogen:
    // fuera de si. Antes, cuanta mas furia mas entornados.
    let eh = if fleckerl { 13.5 } else { 11.5 };
    let pupila = if fleckerl { 2.8 } else { 4.8 };
    // Parpadea de vez en cuando mientras aun esta tranquila.
    let parpadeo = e.fase < 2 && e.t.rem_euclid(220.0) < 7.0;
    // Las pupilas miran hacia donde se mece; en el fleckerl, ademas, tiemblan.
    let mira = vec2(
        3.0 * lado
            + if fleckerl {
                (e.t * 2.3).sin() * 1.2
            } else {
                0.0
            },
        if fleckerl {
            (e.t * 3.1).cos() * 1.0
        } else {
            1.0
        },
    );

    for s in [-1.0f32, 1.0] {
        let ex = 14.5 * s;
        // El rubor, antes que el ojo para quedar debajo.
        lp.disco_liso(
            ex + 5.0 * s,
            ey + 13.0,
            5.0,
            fade(RUBOR, 0.25 + 0.5 * furia),
        );
        lp.elipse(ex, ey, ew, eh, e.ropa);
        if !parpadeo {
            lp.disco_liso(ex + mira.x, ey + mira.y, pupila, TINTA);
            lp.disco_liso(ex + mira.x - 1.6, ey + mira.y - 1.8, pupila * 0.3, WHITE);
        }

        // El parpado: tapa el ojo desde arriba hasta una linea que se inclina
        // hacia dentro con la furia. Es lo que convierte dos circulos en un
        // gesto.
        let (dentro, fuera) = if parpadeo {
            (1.0, 1.0)
        } else if fleckerl {
            (0.0, 0.0)
        } else {
            (0.18 + 0.4 * furia, 0.18 - 0.08 * furia)
        };
        if dentro > 0.0 {
            let arriba = ey - eh - 2.0;
            let yi = ey - eh + 2.0 * eh * dentro;
            let yo = ey - eh + 2.0 * eh * fuera;
            let (xi, xo) = (ex - (ew + 1.0) * s, ex + (ew + 1.0) * s);
            lp.quad([(xi, arriba), (xo, arriba), (xo, yo), (xi, yi)], e.tinta);
            lp.linea((xi + 1.5 * s, yi), (xo - 1.5 * s, yo), 2.2, TINTA);
        }

        // La ceja: arqueada y alta de contenta, baja y hacia dentro de furia.
        let alta = ey - eh - 5.0 + 3.0 * furia;
        lp.linea(
            (ex - 8.0 * s, alta + 5.0 * furia - 1.5 * (1.0 - furia)),
            (ex + 9.0 * s, alta - 3.0 * furia),
            3.0,
            TINTA,
        );
    }

    // La boca, con el peine de la maquina como dientes. Canta en cada tiempo:
    // se abre con el golpe, mas en el uno.
    let ancho = 15.0 + 5.0 * furia;
    let abre = 6.0 + 5.0 * furia + 7.0 * golpe;
    let top = 8.0;
    // De contenta las comisuras suben; de furia bajan.
    let comisura = top - 5.0 + 9.0 * furia;
    let mut borde = vec![
        vec2(-ancho, comisura),
        vec2(0.0, top),
        vec2(ancho, comisura),
    ];
    const LADOS: usize = 8;
    for i in 1..LADOS {
        let a = PI * i as f32 / LADOS as f32;
        let (sa, ca) = a.sin_cos();
        borde.push(vec2(
            ancho * ca,
            top + (comisura - top) * ca.abs() + abre * sa,
        ));
    }
    lp.poli(&borde, GARGANTA);
    // Las puas del peine van de largas a cortas, como en una caja de verdad.
    const PUAS: usize = 7;
    for i in 0..PUAS {
        let f = (i as f32 + 0.5) / PUAS as f32;
        let x = -ancho * 0.7 + f * ancho * 1.4;
        let y = top + (comisura - top) * (x / ancho).abs();
        let largo = 4.5 - 2.0 * f + if fleckerl { 1.5 } else { 0.0 };
        lp.caja(x - 1.0, y, 2.0, largo, ORO);
    }
}

/// Un muelle que asoma de la caja: zigzag de tinta y oro, con una bola en la
/// punta.
fn muelle(lp: &Lapiz, base: Vec2, dir: Vec2, largo: f32) {
    let d = dir.normalize();
    let n = vec2(-d.y, d.x);
    const VUELTAS: usize = 7;
    let puntos: Vec<(f32, f32)> = (0..=VUELTAS)
        .map(|i| {
            let f = i as f32 / VUELTAS as f32;
            let lado = if i == 0 || i == VUELTAS {
                0.0
            } else if i % 2 == 0 {
                3.0
            } else {
                -3.0
            };
            let p = base + d * largo * f + n * lado;
            (p.x, p.y)
        })
        .collect();
    lp.trazo(&puntos, 3.2, TINTA);
    lp.trazo(&puntos, 1.4, ORO);
    let punta = base + d * largo;
    lp.disco(punta.x, punta.y, 3.0, ORO);
}

/// La plumilla: pasa de unidades de la caja a pixeles, aplastando y meciendo
/// por el camino, y pinta cada pieza primero en tinta un poco mas gorda y
/// luego en su color. Es el idioma de `mapa::Lapiz`, con un giro: aqui la
/// caja se mece, asi que todo pasa por `p` y nada usa rectangulos de pantalla.
struct Lapiz {
    /// Donde cae el origen de la caja en pantalla.
    o: Vec2,
    /// Pixeles por unidad.
    k: f32,
    /// Grueso de la tinta, en pixeles.
    g: f32,
    /// El aplastado del golpe, desde el suelo.
    sx: f32,
    sy: f32,
    /// Seno y coseno del balanceo.
    rot: (f32, f32),
    /// La pata sobre la que se mece. Cambia de lado cuando el giro pasa por
    /// cero, que es cuando las dos patas estan en el suelo: no salta.
    pivote: Vec2,
}

impl Lapiz {
    fn p(&self, x: f32, y: f32) -> Vec2 {
        let q = vec2(x * self.sx, SUELO + (y - SUELO) * self.sy) - self.pivote;
        let (s, c) = self.rot;
        self.o + (self.pivote + vec2(q.x * c - q.y * s, q.x * s + q.y * c)) * self.k
    }

    fn grados(&self) -> f32 {
        self.rot.0.atan2(self.rot.1).to_degrees()
    }

    /// Un cuadrilatero relleno, sin tinta.
    fn quad(&self, q: [(f32, f32); 4], c: Color) {
        let [a, b, d, e] = q.map(|(x, y)| self.p(x, y));
        draw_triangle(a, b, d, c);
        draw_triangle(a, d, e, c);
    }

    /// Una caja con contorno: la tinta es la misma caja engordada, en
    /// unidades, para que gire con todo lo demas.
    fn caja(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        let m = self.g / self.k;
        let (x0, y0, x1, y1) = (x - m, y - m, x + w + m, y + h + m);
        self.quad([(x0, y0), (x1, y0), (x1, y1), (x0, y1)], TINTA);
        self.quad([(x, y), (x + w, y), (x + w, y + h), (x, y + h)], c);
    }

    /// Un poligono con contorno, convexo o casi. El trazo va centrado en el borde y
    /// el relleno encima le tapa la mitad de dentro: sale la misma tinta de
    /// fuera que en las cajas, con cualquier forma.
    fn poli(&self, puntos: &[Vec2], c: Color) {
        let v: Vec<Vec2> = puntos.iter().map(|p| self.p(p.x, p.y)).collect();
        for (i, a) in v.iter().enumerate() {
            let b = v[(i + 1) % v.len()];
            draw_line(a.x, a.y, b.x, b.y, self.g * 2.0, TINTA);
            draw_circle(a.x, a.y, self.g, TINTA);
        }
        // En abanico desde el centro y no desde un vertice: la boca sonriente
        // tiene el borde de arriba hundido, y desde una esquina se saldria.
        let m = v.iter().copied().sum::<Vec2>() / v.len() as f32;
        for (i, a) in v.iter().enumerate() {
            draw_triangle(m, *a, v[(i + 1) % v.len()], c);
        }
    }

    fn disco(&self, x: f32, y: f32, r: f32, c: Color) {
        let a = self.p(x, y);
        draw_circle(a.x, a.y, r * self.k + self.g, TINTA);
        draw_circle(a.x, a.y, r * self.k, c);
    }

    fn disco_liso(&self, x: f32, y: f32, r: f32, c: Color) {
        let a = self.p(x, y);
        draw_circle(a.x, a.y, r * self.k, c);
    }

    fn elipse(&self, x: f32, y: f32, a: f32, b: f32, c: Color) {
        let o = self.p(x, y);
        let (a, b) = (a * self.k * self.sx, b * self.k * self.sy);
        let r = self.grados();
        draw_ellipse(o.x, o.y, a + self.g, b + self.g, r, TINTA);
        draw_ellipse(o.x, o.y, a, b, r, c);
    }

    fn linea(&self, a: (f32, f32), b: (f32, f32), w: f32, c: Color) {
        let (a, b) = (self.p(a.0, a.1), self.p(b.0, b.1));
        draw_line(a.x, a.y, b.x, b.y, (w * self.k).max(1.0), c);
    }

    /// Una linea quebrada, con las juntas redondas para que no se abra en los
    /// picos.
    fn trazo(&self, puntos: &[(f32, f32)], w: f32, c: Color) {
        let r = (w * self.k).max(1.0) * 0.5;
        for par in puntos.windows(2) {
            self.linea(par[0], par[1], w, c);
            let b = self.p(par[1].0, par[1].1);
            draw_circle(b.x, b.y, r, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_figura_va_a_mas() {
        // Como en Cuphead entre fases: la tapa se abre, la llave corre y la
        // cara se enfada. Una figura que se viese mas tranquila que la
        // anterior diria que el combate afloja, y no afloja.
        for fase in 1..4 {
            let (a, b) = (figura(fase - 1, 1.0), figura(fase, 1.0));
            assert!(b.tapa > a.tapa, "la tapa se cierra en la figura {fase}");
            assert!(b.llave > a.llave, "la llave frena en la figura {fase}");
            assert!(b.furia > a.furia, "se calma en la figura {fase}");
        }
    }

    #[test]
    fn solo_la_coda_se_rompe() {
        for fase in 0..3 {
            let f = figura(fase, 0.0);
            assert_eq!(f.grietas, 0, "grietas en la figura {fase}");
            assert_eq!(f.bote, 0.0, "bota en la figura {fase}");
        }
        let entera = figura(3, 1.0);
        let rota = figura(3, 0.0);
        assert!(entera.grietas > 0 && entera.bote > 0.0);
        assert!(
            rota.grietas > entera.grietas,
            "con poca vida no se rompe mas"
        );
        assert!(rota.grietas <= GRIETAS.len());
    }

    #[test]
    fn con_poca_vida_va_con_prisa() {
        for fase in 0..4 {
            assert!(figura(fase, 0.1).llave > figura(fase, 1.0).llave);
        }
    }

    #[test]
    fn nunca_baja_de_las_tablas() {
        assert_eq!(alzar(500.0, None), 0.0);
        assert_eq!(alzar(500.0, Some(600.0)), 0.0);
        assert_eq!(alzar(640.0, Some(600.0)), 40.0);
    }

    #[test]
    fn la_caida_acaba_con_la_tapa_cerrada_y_la_bailarina_en_el_suelo() {
        let (antes, despues) = (desarme(0.0, 30.0), desarme(1.0, 30.0));
        assert_eq!(antes.tapa, 30.0);
        assert_eq!(antes.bailarina, 0.0);
        assert!(!antes.ko && antes.puas.iter().all(|&v| v == 0.0));
        assert!(despues.tapa.abs() < 1e-3, "la tapa no se cierra");
        assert!((despues.bailarina + FRAC_PI_2).abs() < 1e-3);
        assert_eq!(despues.baja, 1.0);
        assert!(despues.ko && despues.puas.iter().all(|&v| v == 1.0));
        // La bailarina se cae y no se levanta, y la llave no vuelve atras.
        for i in 1..=100 {
            let (a, b) = (
                desarme((i - 1) as f32 / 100.0, 30.0),
                desarme(i as f32 / 100.0, 30.0),
            );
            assert!(b.bailarina <= a.bailarina && b.llave <= a.llave);
        }
    }
}
