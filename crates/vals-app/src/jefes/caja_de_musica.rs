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
//! la coda se rompe: grietas, muelles saltando y la caja botando en cada
//! tiempo.
//!
//! Todo en unidades de la arena, no de la bailarina: el aro de golpeo mide 46
//! de radio en esas unidades, y asi las medidas de aqui se leen contra el aro
//! sin hacer cuentas.

use macroquad::prelude::*;

use super::Escena;
use crate::bailarines;
use crate::draw::{Layout, draw_figura, fade};
use crate::music::{self, Tema};
use crate::paleta::{ORO, TINTA};
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
    /// Lo que bota en cada tiempo. Solo en la coda.
    bote: f32,
    /// Cuantas grietas tiene. Solo en la coda, y mas cuanto menos vida.
    grietas: usize,
}

fn figura(fase: usize, vida: f32) -> Figura {
    let f = fase.min(3);
    // Con poca vida todo va con prisa: la cuerda se acaba y lo nota.
    let prisa = 1.0 - vida.clamp(0.0, 1.0);
    let coda = f == 3;
    Figura {
        tapa: [24.0, 32.0, 40.0, 46.0][f],
        llave: (0.006 + f as f32 * 0.006) * (1.0 + prisa),
        vaiven: (0.06 + f as f32 * 0.02) * (1.0 + 0.5 * prisa),
        furia: (f as f32 / 3.0 + 0.25 * prisa).min(1.0),
        bote: if coda { 9.0 } else { 0.0 },
        grietas: if coda {
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

/// Las grietas de la coda, en el frente de la caja. Fijas: una grieta que
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
    let coda = e.fase >= 3;
    let pulso = e.pulso.clamp(0.0, 1.0);
    // El uno del compas pesa mas: es el "oom" del oom-pah-pah.
    let golpe = pulso * if e.fuerte { 1.0 } else { 0.55 };

    // Donde va del compas. La caja se mece un compas hacia cada lado, asi que
    // el balanceo tiene que salir del mismo reloj que la musica, no de un seno
    // cualquiera: si no, se desacompasa en cuanto cambia el tempo de figura.
    let (ticks, tiempos) = music::compas(Tema::de(0, e.fase));
    let compas = e.t / (ticks * tiempos as f32);
    let (lado, _) = (compas * std::f32::consts::PI).sin_cos();
    let mut giro = fig.vaiven * lado;
    // Con poca vida tiembla. Determinista: sale del reloj, no de un dado.
    giro += (e.t * 1.9).sin() * 0.03 * (1.0 - e.vida) * fig.furia;

    // En la coda bota: despega entre tiempos y cae justo en el golpe.
    let dentro = 1.0 - pulso.powf(1.0 / 2.2);
    let bote = fig.bote * (dentro * std::f32::consts::PI).sin();

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

    // --- La tapa, detras de todo. En la coda aletea en cada golpe.
    let alto = fig.tapa + if coda { 10.0 * pulso } else { 0.0 };
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
    let medio = CANTO - alto * 0.5;
    lp.elipse(0.0, medio, MEDIO - 11.0, alto * 0.34 + 2.0, ORO);
    lp.elipse(0.0, medio, MEDIO - 14.0, alto * 0.34, ESPEJO);
    // Dos brillos en diagonal: es lo que hace que un gris sea un espejo.
    let b = alto * 0.22;
    lp.linea((-13.0, medio + b), (-5.0, medio - b), 1.6, WHITE);
    lp.linea((-6.0, medio + b), (-1.0, medio - b * 0.4), 1.0, WHITE);

    // --- La bailarina, en su peana. No se mece con la caja: gira en su eje,
    // que es lo que hace la de una caja de verdad.
    lp.caja(-6.0, CANTO - 9.0, 12.0, 9.0, ORO);
    let sd = e.escala * BAILARINA;
    let pie = lp.p(0.0, CANTO - 9.0);
    for pose in bailarines::poses(0, e.fase, e.t, e.vida) {
        let pies = pose.joints[skeleton::PIE_I]
            .y
            .max(pose.joints[skeleton::PIE_D].y);
        draw_figura(
            &pose,
            pie - vec2(0.0, pies * sd),
            &Layout::con_escala(sd),
            1.0,
            // Las tintas al reves que la caja: de azul sobre la tapa azul se
            // perdia, y en el cartel lateral, negra sobre negro, desaparecia.
            e.ropa,
            e.tinta,
        );
    }

    // --- Los muelles de la coda, saltando por las esquinas.
    if coda {
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

    // --- El cuerpo: canto, caja y zocalo.
    lp.caja(-MEDIO - 3.0, CANTO, 2.0 * MEDIO + 6.0, 8.0, e.ropa);
    lp.caja(-MEDIO, CANTO + 8.0, 2.0 * MEDIO, 54.0, e.tinta);
    lp.caja(-MEDIO - 3.0, 30.0, 2.0 * MEDIO + 6.0, 10.0, e.ropa);
    for x in [-30.0, -15.0, 0.0, 15.0, 30.0] {
        lp.disco(x, CANTO + 4.0, 1.6, ORO);
        lp.disco(x, 35.0, 1.6, ORO);
    }
    lp.disco(-PATA_X, SUELO - 5.0, 5.0, ORO);
    lp.disco(PATA_X, SUELO - 5.0, 5.0, ORO);

    for grieta in GRIETAS.iter().take(fig.grietas) {
        lp.trazo(grieta, 1.6, TINTA);
    }

    cara(&lp, e, &fig, lado, coda, golpe);

    // --- La llave, al costado. Se ve de canto y gira sobre su eje: basta con
    // aplastar las dos orejas con el coseno para que se lea que da vueltas.
    let (_, c) = (e.t * fig.llave * std::f32::consts::TAU).sin_cos();
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

/// Los ojos, las cejas, el rubor y la boca.
fn cara(lp: &Lapiz, e: &Escena, fig: &Figura, lado: f32, coda: bool, golpe: f32) {
    let furia = fig.furia;
    let (ey, ew) = (-8.0, 9.5);
    // En la coda los ojos se abren de par en par y las pupilas se encogen:
    // fuera de si. Antes, cuanta mas furia mas entornados.
    let eh = if coda { 13.5 } else { 11.5 };
    let pupila = if coda { 2.8 } else { 4.8 };
    // Parpadea de vez en cuando mientras aun esta tranquila.
    let parpadeo = e.fase < 2 && e.t.rem_euclid(220.0) < 7.0;
    // Las pupilas miran hacia donde se mece; en la coda, ademas, tiemblan.
    let mira = vec2(
        3.0 * lado + if coda { (e.t * 2.3).sin() * 1.2 } else { 0.0 },
        if coda { (e.t * 3.1).cos() * 1.0 } else { 1.0 },
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
        } else if coda {
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
        let a = std::f32::consts::PI * i as f32 / LADOS as f32;
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
        let largo = 4.5 - 2.0 * f + if coda { 1.5 } else { 0.0 };
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
}
