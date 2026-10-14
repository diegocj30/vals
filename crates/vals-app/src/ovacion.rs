//! La ovacion: lo que se ve cuando sale el super.
//!
//! Antes el super era un fogonazo verde de un tercio de segundo y una sacudida:
//! se notaba que habia pasado algo gordo, pero no que lo habias hecho tu. Ahora
//! es un numero de revista en pequeno. La sala se apaga, un foco cae sobre
//! ella, una onda escarlata —el rojo de su lazo— sale de ella y se lleva las
//! balas por delante dejando petalos, y un rotulo de cartel grita la palabra
//! del baile.
//!
//! Todo es presentacion. La simulacion ya ha limpiado las balas en el tick del
//! super (`World::resolve_super`); lo que la onda borra son sus fantasmas, una
//! foto de donde estaban justo antes. El replay dorado no se entera, y el reloj
//! es de ticks, asi que el entreacto y el hitstop la paran igual que al mundo:
//! durante el frenazo del golpe se queda en el fogonazo, que es el fotograma
//! que tiene que quedarse en la retina.
//!
//! **Dura poco, y lo que tapa dura menos.** El apagon y la onda acaban dentro
//! de la invulnerabilidad del super (`SUPER_TICKS`): cuando una bala vuelve a
//! poder hacer dano la sala ya esta encendida y no queda ni un fantasma. Solo el
//! rotulo se queda algo mas, y va en la mitad de la pantalla donde no esta ella.
//!
//! En el paseo sale la version ligera: sin apagon ni foco —la calle es de
//! dia y se anda, no se baila— y sin fantasmas, con la onda, los petalos y un
//! rotulo mas pequeno.

use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::draw::{Layout, fade};
use crate::fuentes::{self, Cara};
use crate::idioma::{Texto, tx};
use crate::jefes::{boing, suave, tramo};
use crate::paleta::{self, LUZ, PAPEL, TINTA};
use crate::protagonista::LAZO;

/// Lo que dura el fogonazo de pelicula: un fotograma quemado, el del golpe.
const FOGONAZO: f32 = 5.0;
/// Lo que tarda la onda en llegar a la esquina mas lejana.
const ONDA: f32 = 18.0;
/// Cuando acaba de encenderse la sala. Sube en 3 ticks, se queda hasta el 10.
const APAGON: f32 = 20.0;
/// Lo que se ve el rotulo, y con el la ovacion entera.
const DURA: f32 = 42.0;

/// Lo oscura que se queda la sala, como mucho. Las balas que salgan durante el
/// apagon se tienen que seguir viendo, asi que nunca es negro.
const OSCURO: f32 = 0.65;

/// Lo que grita el cartel, por baile. "¡BRAVO!" ya es el sello de la victoria,
/// asi que el super grita otra cosa, y una distinta por baile: dice que el
/// numero era de ese baile.
const PALABRAS: [Texto; 4] = [
    Texto::SuperVals,
    Texto::SuperTango,
    Texto::SuperCharleston,
    Texto::SuperCancan,
];

pub struct Ovacion {
    /// Ticks desde que salio el super, o `None` si no hay ninguno en marcha.
    tick: Option<u32>,
    baile: usize,
    /// Donde estaban las balas justo antes del super, en unidades de arena.
    fantasmas: Vec<Vec2>,
    ligera: bool,
}

impl Ovacion {
    pub fn new() -> Self {
        Self {
            tick: None,
            baile: 0,
            fantasmas: Vec::new(),
            ligera: false,
        }
    }

    /// Arranca la ovacion del baile `baile`. `fantasmas` es la foto de las
    /// balas del tick anterior; `ligera`, la del paseo.
    pub fn disparar(&mut self, baile: usize, fantasmas: Vec<Vec2>, ligera: bool) {
        *self = Self {
            tick: Some(0),
            baile,
            fantasmas,
            ligera,
        };
    }

    /// Un tick, al paso de la simulacion.
    pub fn step(&mut self) {
        if let Some(t) = self.tick {
            self.tick = (((t + 1) as f32) < DURA).then_some(t + 1);
        }
    }

    /// Dibuja la ovacion sobre una lamina de `tam` unidades, con ella en el
    /// punto de pantalla `jugadora`.
    pub fn dibujar(&self, l: &Layout, tam: Vec2, jugadora: Vec2, alpha: f32) {
        let Some(tick) = self.tick else {
            return;
        };
        let t = tick as f32 + alpha;
        let o = l.to_screen(0.0, 0.0);
        let (w, h) = (l.len(tam.x), l.len(tam.y));
        // Hasta la esquina mas lejana: la onda tiene que salir de la pantalla
        // entera, no de un circulo dentro de ella.
        let alcance = [o, o + vec2(w, 0.0), o + vec2(0.0, h), o + vec2(w, h)]
            .iter()
            .map(|e| e.distance(jugadora))
            .fold(0.0, f32::max)
            + l.len(30.0);

        // Todo dentro de la lamina, como el resto del juego: se apaga el
        // escenario, no el papel, y la onda se corta en el marco.
        crate::prologo::recortado(l, (0.0, 0.0, tam.x, tam.y), || {
            if !self.ligera {
                foco(l, o, w, jugadora, apagon(t));
            }
            let r = onda(t) * alcance;
            self.borrar_fantasmas(l, jugadora, r, t);
            petalos(l, jugadora, r, t);
            if t < ONDA {
                lazos(l, jugadora, r, t);
            }
            self.rotulo(o, w, h, jugadora, t);
        });

        // El fogonazo va encima de todo lo suyo: es la pelicula quemada.
        let f = fogonazo(t) * if self.ligera { 0.35 } else { 0.55 };
        if f > 0.0 {
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), fade(PAPEL, f));
        }
    }

    /// Las balas que habia, quietas, hasta que la onda pasa por encima y
    /// revientan en un aro rojo. Es lo que hace que la limpieza **se vea**:
    /// sin esto desaparecian todas en el mismo fotograma y la onda pasaba por
    /// una pantalla ya vacia.
    fn borrar_fantasmas(&self, l: &Layout, c: Vec2, r: f32, t: f32) {
        if t >= ONDA {
            return;
        }
        let estalla = l.len(40.0);
        for p in &self.fantasmas {
            let s = l.to_screen(p.x, p.y);
            let d = s.distance(c);
            if d > r {
                // Papel recortado, no bala: crema con su canto de tinta. Si se
                // pintasen con su color de siempre se leerian como balas vivas.
                draw_circle(s.x, s.y, l.len(4.5), fade(TINTA, 0.75));
                draw_circle(s.x, s.y, l.len(3.0), fade(PAPEL, 0.6));
            } else if d > r - estalla {
                let u = (r - d) / estalla;
                draw_circle_lines(s.x, s.y, l.len(4.0 + 9.0 * u), 2.0, fade(LAZO, 1.0 - u));
            }
        }
    }

    /// El rotulo: la palabra del baile en una banda de tinta torcida, como un
    /// cartel pegado de prisa. Entra grande y se clava con un rebote.
    fn rotulo(&self, o: Vec2, w: f32, h: f32, jugadora: Vec2, t: f32) {
        let (alfa, escala) = rotulo(t);
        if alfa <= 0.0 {
            return;
        }
        let texto = tx(PALABRAS[self.baile.min(PALABRAS.len() - 1)]);
        let tam = if self.ligera { 44.0 } else { 64.0 } * escala;
        let cx = o.x + w * 0.5;
        // En la mitad donde no esta ella, que no la tape nunca, y sin llegar
        // al jefe, que es de donde salen las balas nuevas.
        let cy = o.y
            + h * if jugadora.y < o.y + h * 0.5 {
                0.70
            } else {
                0.36
            };
        let (tinta, _) = paleta::del_baile(self.baile);

        let medio = fuentes::medir(texto, tam, Cara::Titulo).width * 0.5 + tam * 0.6;
        let alto = tam * 0.55;
        let sesgo = tam * 0.3;
        // La banda: un paralelogramo, tinta dentro y el rojo del lazo de canto.
        let esq = [
            vec2(cx - medio + sesgo, cy - alto),
            vec2(cx + medio + sesgo, cy - alto),
            vec2(cx + medio - sesgo, cy + alto),
            vec2(cx - medio - sesgo, cy + alto),
        ];
        draw_triangle(esq[0], esq[1], esq[2], fade(TINTA, 0.88 * alfa));
        draw_triangle(esq[0], esq[2], esq[3], fade(TINTA, 0.88 * alfa));
        for i in 0..4 {
            let (a, b) = (esq[i], esq[(i + 1) % 4]);
            draw_line(a.x, a.y, b.x, b.y, 3.0, fade(LAZO, alfa));
        }
        // Y la palabra a dos tintas: la del baile corrida debajo y el papel
        // encima, que es como sale una plancha mal casada en un cartel.
        let base = cy + tam * 0.32;
        let corrido = (tam * 0.05).max(2.0);
        fuentes::centrado(
            texto,
            cx + corrido,
            base + corrido,
            tam,
            Cara::Titulo,
            fade(tinta, alfa),
        );
        fuentes::centrado(texto, cx, base, tam, Cara::Titulo, fade(PAPEL, alfa));
    }
}

/// El escenario a oscuras con un foco desde el peine sobre ella, que es lo que
/// hace un teatro cuando alguien va a hacer su numero.
fn foco(l: &Layout, o: Vec2, w: f32, c: Vec2, k: f32) {
    if k <= 0.0 {
        return;
    }
    let (dentro, fuera) = (l.len(46.0), l.len(72.0));
    let lejos = vec2(screen_width(), screen_height()).length() * 2.0;
    // El borde del circulo de luz, en escalones: un corte seco se lee como un
    // agujero en la pantalla y no como luz.
    let escalones = 4;
    for i in 0..escalones {
        let (a, b) = (
            i as f32 / escalones as f32,
            (i + 1) as f32 / escalones as f32,
        );
        let r0 = dentro + (fuera - dentro) * a;
        let r1 = dentro + (fuera - dentro) * b;
        anillo(c, r0, r1, fade(TINTA, OSCURO * k * (a + b) * 0.5));
    }
    anillo(c, fuera, lejos, fade(TINTA, OSCURO * k));

    // El haz: del peine, un poco hacia el centro del escenario, hasta ella.
    let arriba = vec2(c.x + (o.x + w * 0.5 - c.x) * 0.4, o.y);
    let boca = l.len(14.0);
    let luz = fade(LUZ, 0.16 * k);
    let (a, b) = (arriba - vec2(boca, 0.0), arriba + vec2(boca, 0.0));
    let (d, e) = (c - vec2(dentro, 0.0), c + vec2(dentro, 0.0));
    draw_triangle(a, b, e, luz);
    draw_triangle(a, e, d, luz);
    anillo(c, 0.0, dentro, fade(LUZ, 0.10 * k));
}

/// Los lazos que lleva la onda en el canto: la onda es su lazo, abierto.
fn lazos(l: &Layout, c: Vec2, r: f32, t: f32) {
    let k = onda(t);
    let grosor = l.len(12.0) * (1.0 - 0.6 * k);
    anillo(c, r - grosor * 0.5 - 2.5, r + grosor * 0.5 + 2.5, TINTA);
    anillo(c, r - grosor * 0.5, r + grosor * 0.5, LAZO);

    let n = 8;
    let b = l.len(9.0) * (1.4 - 0.6 * k);
    for i in 0..n {
        let ang = i as f32 * TAU / n as f32 + t * 0.03;
        let dir = vec2(ang.cos(), ang.sin());
        let p = c + dir * r;
        // Las alas van a lo largo de la onda, el nudo en ella.
        let tg = vec2(-dir.y, dir.x);
        for (escala, color) in [(1.35, TINTA), (1.0, LAZO)] {
            let bb = b * escala;
            for lado in [-1.0, 1.0] {
                let punta = p + tg * lado * bb * 2.0;
                draw_triangle(p, punta + dir * bb, punta - dir * bb, color);
            }
        }
        draw_circle(p.x, p.y, b * 0.55, TINTA);
        draw_circle(p.x, p.y, b * 0.38, LAZO);
    }
}

/// Petalos de rosa detras de la onda, como si los fuera soltando. Sus numeros
/// salen del indice, no de un azar: son los mismos en cada frame.
fn petalos(l: &Layout, c: Vec2, r: f32, t: f32) {
    let alfa = 1.0 - tramo(t, ONDA * 0.5, ONDA + 8.0);
    if alfa <= 0.0 {
        return;
    }
    for i in 0..28 {
        let h1 = ((i as f32 * 12.9898).sin() * 43_758.547).fract().abs();
        let h2 = ((i as f32 * 78.233).sin() * 12_543.21).fract().abs();
        let ang = h1 * TAU;
        let p = c + vec2(ang.cos(), ang.sin()) * r * (0.55 + 0.4 * h2);
        let giro = (ang + t * (0.08 + 0.1 * h2)).to_degrees();
        let (a, b) = (l.len(5.5), l.len(2.8));
        draw_ellipse(p.x, p.y, a + 1.5, b + 1.5, giro, fade(TINTA, alfa));
        draw_ellipse(p.x, p.y, a, b, giro, fade(LAZO, alfa));
    }
}

/// Un anillo relleno entre `r0` y `r1`. Los circulos de macroquad son de 20
/// lados, y a la escala de la pantalla entera se ven los picos.
fn anillo(c: Vec2, r0: f32, r1: f32, color: Color) {
    let n = 72;
    for i in 0..n {
        let (a0, a1) = (i as f32 * TAU / n as f32, (i + 1) as f32 * TAU / n as f32);
        let (d0, d1) = (vec2(a0.cos(), a0.sin()), vec2(a1.cos(), a1.sin()));
        draw_triangle(c + d0 * r0, c + d0 * r1, c + d1 * r1, color);
        if r0 > 0.0 {
            draw_triangle(c + d0 * r0, c + d1 * r1, c + d1 * r0, color);
        }
    }
}

// ---------------------------------------------------------------------------
// Las curvas. Funciones puras de los ticks, sin estado: cada fotograma de la
// ovacion sale de aqui, y aqui es donde se vigila que nada tape de mas.
// ---------------------------------------------------------------------------

/// Lo quemado del fotograma del golpe, de 1 a 0.
fn fogonazo(t: f32) -> f32 {
    (1.0 - tramo(t, 0.0, FOGONAZO)).powi(2)
}

/// Por donde va la onda, de 0 (en ella) a 1 (fuera de la pantalla). Sale
/// disparada y frena, como una onda de verdad.
fn onda(t: f32) -> f32 {
    1.0 - (1.0 - tramo(t, 0.0, ONDA)).powi(3)
}

/// Lo apagada que esta la sala, de 0 a 1.
fn apagon(t: f32) -> f32 {
    suave(tramo(t, 0.0, 3.0).min(1.0 - tramo(t, 10.0, APAGON)))
}

/// La opacidad del rotulo y su tamano relativo. Entra casi al doble y se clava
/// con un rebote por debajo de 1, que es el golpe de un sello.
fn rotulo(t: f32) -> (f32, f32) {
    let alfa = tramo(t, 0.0, 2.0).min(1.0 - tramo(t, DURA - 12.0, DURA));
    (alfa, 1.0 + 0.9 * (1.0 - boing(tramo(t, 0.0, 16.0))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::player::SUPER_TICKS;

    #[test]
    fn la_sala_se_enciende_antes_de_que_las_balas_vuelvan() {
        // La regla de legibilidad: mientras el super te protege se puede
        // apagar la sala y llenar la pantalla de fantasmas; en cuanto una bala
        // puede volver a darte, ni sombra ni fantasma.
        let fin = SUPER_TICKS as f32;
        assert!(apagon(5.0) > 0.99, "la sala no llega a apagarse");
        assert_eq!(apagon(fin), 0.0);
        assert_eq!(onda(fin), 1.0, "quedan fantasmas sin borrar");
        assert_eq!(fogonazo(FOGONAZO), 0.0);
    }

    #[test]
    fn la_ovacion_se_acaba_sola_y_no_deja_nada() {
        let mut o = Ovacion::new();
        o.disparar(1, vec![Vec2::ZERO], false);
        for _ in 0..DURA as u32 {
            assert!(o.tick.is_some());
            o.step();
        }
        assert!(o.tick.is_none(), "la ovacion no se acaba");
        assert_eq!(rotulo(DURA).0, 0.0, "el rotulo se queda puesto");
    }

    #[test]
    fn el_rotulo_entra_grande_y_se_clava() {
        let escalas: Vec<f32> = (0..30).map(|i| rotulo(i as f32).1).collect();
        assert!(escalas[0] > 1.8, "no entra grande: {}", escalas[0]);
        assert!(escalas.iter().any(|&e| e < 1.0), "no rebota al clavarse");
        assert!((escalas[29] - 1.0).abs() < 0.01, "no se queda a su tamano");
    }
}
