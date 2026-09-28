//! La pista vista como un mapa: el mundo por el que se anda entre baile y baile.
//!
//! Hasta ahora cada baile era un emblema girando —un hexagono y un triangulo en
//! sus tintas— con el nombre debajo. Se distinguian por el color, pero no
//! decian **a donde** se iba. En el mapa de Cuphead cada nivel es un sitio: una
//! casa, un barco, un circo. Aqui cada baile es **el sitio donde se baila**, el
//! mismo de su decorado (`escenarios.rs`) en pequeno: el palacio de Viena, la
//! esquina del arrabal, el club de jazz y el molino del Moulin Rouge. Y los une
//! un camino a trazos, que es lo que convierte cuatro dibujos sueltos en un
//! recorrido.
//!
//! Todo es dibujo. La pista sigue siendo plana en el core (`vals_core::pista`):
//! la perspectiva, los caminos y los monumentos solo existen aqui.

use macroquad::prelude::*;
use vals_core::pista::{ENTRADA, MODISTA, Nodo, Pista};
use vals_core::{ARENA_H, ARENA_W};

use crate::draw::{Layout, METER_FULL, TEXT_DIM, TITLE, fade, paspartu};
use crate::fuentes::{self, Cara};
use crate::modista;
use crate::paleta::{self, LUZ, ORO, PAPEL, PARED, TARIMA, TINTA, TINTA_TENUE, VETA};
use crate::protagonista;
use crate::skeleton;

/// La tarima de la pista, un punto mas oscura que la del combate: aqui lo que
/// tiene que resaltar son los monumentos y el camino.
const SUELO: Color = color_u8!(42, 29, 28, 255);
/// El foco que planta cada cosa en el suelo en vez de dejarla flotando.
const FOCO: Color = color_u8!(255, 212, 148, 60);
/// La madera de las tablas que cierran un baile.
const TABLON: Color = color_u8!(150, 112, 78, 255);

/// Altura en pantalla del fondo de la pista. Por encima queda la pared.
const HORIZONTE: f32 = ARENA_H * 0.20;
/// Cuanto se estrecha la pista al fondo. Es lo unico que hace falta para que
/// se lea como un suelo y no como una pared.
const FONDO_ANCHO: f32 = 0.42;
/// Y cuanto encogen las figuras alli.
const FONDO_ESCALA: f32 = 0.52;
/// Lineas de la tarima, por eje.
const TABLAS: usize = 9;

/// Lo que miden los monumentos respecto a sus propias unidades. Estan
/// dibujados en numeros redondos —unos noventa de ancho— y esto los ajusta sin
/// tocar ni uno. Mas grandes y los dos del nivel medio se pisan, aun con el
/// aire que les da `sitio` en el core.
const TAMANO: f32 = 1.5;
/// Cada cuanto va una raya del camino, en unidades de pista.
const PASO_RAYA: f32 = 9.0;

/// Lleva un punto de la pista a la pantalla. Devuelve tambien cuanto encoge
/// alli lo que se dibuje.
fn suelo(l: &Layout, px: f32, py: f32) -> (Vec2, f32) {
    // 0 al fondo, 1 delante. La curva junta las lineas cerca del horizonte,
    // que es lo que da la sensacion de profundidad.
    let t = (py / ARENA_H).clamp(0.0, 1.0).powf(1.35);
    let ancho = FONDO_ANCHO + (1.0 - FONDO_ANCHO) * t;
    let x = ARENA_W * 0.5 + (px - ARENA_W * 0.5) * ancho;
    let y = HORIZONTE + (ARENA_H - 24.0 - HORIZONTE) * t;
    (l.to_screen(x, y), FONDO_ESCALA + (1.0 - FONDO_ESCALA) * t)
}

/// La tarima: un trapecio con vetas.
fn dibujar_tarima(l: &Layout) {
    let esquina = |x: f32, y: f32| suelo(l, x, y).0;
    let (fi, fd) = (esquina(0.0, 0.0), esquina(ARENA_W, 0.0));
    let (ci, cd) = (esquina(0.0, ARENA_H), esquina(ARENA_W, ARENA_H));
    draw_triangle(fi, fd, cd, SUELO);
    draw_triangle(fi, cd, ci, SUELO);

    // Vetas a lo ancho: se van juntando hacia el fondo solas, porque la
    // perspectiva ya esta en `suelo`.
    for i in 0..=TABLAS {
        let y = ARENA_H * i as f32 / TABLAS as f32;
        let (a, b) = (esquina(0.0, y), esquina(ARENA_W, y));
        let cerca = i as f32 / TABLAS as f32;
        draw_line(a.x, a.y, b.x, b.y, 1.0, fade(VETA, 0.35 + cerca * 0.5));
    }
    // Y a lo largo, que son las que apuntan al fondo.
    for i in 0..=TABLAS {
        let x = ARENA_W * i as f32 / TABLAS as f32;
        let (a, b) = (esquina(x, 0.0), esquina(x, ARENA_H));
        draw_line(a.x, a.y, b.x, b.y, 1.0, fade(VETA, 0.55));
    }
}

// ---------------------------------------------------------------------------
// Los caminos
// ---------------------------------------------------------------------------

/// Los tramos del camino, como pares de indices en `nodos`.
///
/// Se une cada baile con **todos los del siguiente nivel que tenga bailes**, y
/// no en el orden de la lista. El camino tiene que contar lo mismo que
/// `Pista::abierto`: que al acabar un nivel se abren todos los del siguiente a
/// la vez. Una cadena vals - charleston - cancan - tango diria que el cancan va
/// despues del charleston, y no es verdad: se pueden bailar en cualquier orden.
/// Con los bailes de hoy (uno, dos y uno) sale un rombo.
///
/// Saltarse los niveles vacios es la misma regla que `nivel_abierto`: un nivel
/// sin bailes no bloquea, asi que tampoco corta el camino.
///
/// ponytail: todos con todos. Con dos por nivel son cuatro tramos; con cuatro
/// por nivel serian dieciseis y habria que unir cada uno con el mas cercano.
pub fn tramos(nodos: &[Nodo]) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for (i, a) in nodos.iter().enumerate() {
        let siguiente = nodos.iter().map(|n| n.nivel).filter(|n| *n > a.nivel).min();
        for (k, b) in nodos.iter().enumerate() {
            if Some(b.nivel) == siguiente {
                v.push((i, k));
            }
        }
    }
    v
}

/// Un tramo de camino a trazos, pintado sobre la tarima.
///
/// Va en curva y no recto: un camino de mapa serpentea, y ademas asi dos
/// tramos que salen del mismo sitio se abren en vez de montarse. La curva se
/// tuerce siempre **hacia fuera** del centro de la pista, que es lo que dibuja
/// el rombo en vez de una pajarita.
///
/// Las rayas se cortan en coordenadas de pista y se llevan despues al suelo,
/// asi que se acortan y adelgazan al fondo solas, con la misma perspectiva que
/// todo lo demas.
fn sendero(l: &Layout, a: Vec2, b: Vec2, abierto: bool) {
    let d = b - a;
    let largo = d.length();
    if largo < 1.0 {
        return;
    }
    let normal = d.perp() / largo;
    let fuera = (a.x + b.x) * 0.5 - ARENA_W * 0.5;
    let lado = if normal.x * fuera >= 0.0 { 1.0 } else { -1.0 };
    let control = (a + b) * 0.5 + normal * largo * 0.16 * lado;
    let punto = |f: f32| {
        let q = a.lerp(control, f).lerp(control.lerp(b, f), f);
        suelo(l, q.x, q.y)
    };

    // Un camino que lleva a un baile cerrado se ve, pero apagado: se sabe que
    // esta ahi y que todavia no toca.
    let relleno = if abierto { PAPEL } else { TINTA_TENUE };
    let n = ((largo / PASO_RAYA) as usize).max(2);
    for i in (0..n).step_by(2) {
        let (p0, e) = punto(i as f32 / n as f32);
        let (p1, _) = punto((i + 1) as f32 / n as f32);
        let g = l.len(3.0) * e;
        draw_line(p0.x, p0.y, p1.x, p1.y, g + 3.0, TINTA);
        draw_line(p0.x, p0.y, p1.x, p1.y, g, relleno);
    }
}

// ---------------------------------------------------------------------------
// Los monumentos
// ---------------------------------------------------------------------------

/// Un lapiz para dibujar un monumento en sus propias unidades: el origen en el
/// suelo, la `y` hacia arriba en negativo, y la escala de la perspectiva ya
/// puesta. Asi cada monumento se escribe una vez, en numeros pequenos, y el del
/// fondo y el de delante salen del mismo dibujo.
///
/// Y **todo pasa con contorno**: cada pieza se pinta primero en tinta, un poco
/// mas grande, y encima su color. Es el mismo truco que `draw_figura`, y lo que
/// hace que un monumento parezca impreso y no un monton de rectangulos.
struct Lapiz {
    o: Vec2,
    k: f32,
    /// Grueso de la tinta, en pixeles.
    g: f32,
    /// Un baile cerrado se pinta entero apagado: ver `apagar`.
    cerrado: bool,
}

impl Lapiz {
    fn new(o: Vec2, k: f32, cerrado: bool) -> Self {
        Self {
            o,
            k,
            g: (2.0 * k).max(1.5),
            cerrado,
        }
    }

    fn p(&self, x: f32, y: f32) -> Vec2 {
        self.o + vec2(x, y) * self.k
    }

    fn c(&self, c: Color) -> Color {
        if self.cerrado { apagar(c) } else { c }
    }

    fn caja(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        let a = self.p(x, y);
        let (w, h, g) = (w * self.k, h * self.k, self.g);
        draw_rectangle(a.x - g, a.y - g, w + g * 2.0, h + g * 2.0, TINTA);
        draw_rectangle(a.x, a.y, w, h, self.c(c));
    }

    fn disco(&self, x: f32, y: f32, r: f32, c: Color) {
        let a = self.p(x, y);
        draw_circle(a.x, a.y, r * self.k + self.g, TINTA);
        draw_circle(a.x, a.y, r * self.k, self.c(c));
    }

    /// Un triangulo relleno, sin contorno: las aspas y los rayos lo ponen
    /// aparte, porque si no se ve la diagonal de dentro.
    fn tri(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: Color) {
        let (a, b, c) = (self.p(a.0, a.1), self.p(b.0, b.1), self.p(c.0, c.1));
        draw_triangle(a, b, c, self.c(color));
    }

    fn tri_tinta(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), color: Color) {
        self.tri(a, b, c, color);
        let (a, b, c) = (self.p(a.0, a.1), self.p(b.0, b.1), self.p(c.0, c.1));
        draw_triangle_lines(a, b, c, self.g, TINTA);
    }

    fn linea(&self, a: (f32, f32), b: (f32, f32), w: f32, c: Color) {
        let (a, b) = (self.p(a.0, a.1), self.p(b.0, b.1));
        draw_line(a.x, a.y, b.x, b.y, (w * self.k).max(1.0), self.c(c));
    }

    /// Una ventana o una puerta con arco de medio punto: `y` es donde empieza
    /// el arco. Las dos tintas van antes que los dos rellenos para que el
    /// contorno de una pieza no corte a la otra.
    fn arco(&self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        let (a, r) = (self.p(x, y), w * 0.5 * self.k);
        let (w, h, g) = (w * self.k, h * self.k, self.g);
        draw_circle(a.x + r, a.y, r + g, TINTA);
        draw_rectangle(a.x - g, a.y, w + g * 2.0, h + g, TINTA);
        draw_circle(a.x + r, a.y, r, self.c(c));
        draw_rectangle(a.x, a.y, w, h, self.c(c));
    }

    /// La luz de las ventanas. Late despacio, como una sala con gente dentro;
    /// en un baile cerrado esta apagada, que es la forma mas clara de decir
    /// que ahi no hay nadie todavia.
    fn luz(&self, t: f32) -> Color {
        if self.cerrado {
            PARED
        } else {
            let f = 0.5 + 0.5 * (t * 0.05).sin();
            Color::new(
                ORO.r + (LUZ.r - ORO.r) * f,
                ORO.g + (LUZ.g - ORO.g) * f,
                ORO.b + (LUZ.b - ORO.b) * f,
                1.0,
            )
        }
    }
}

// ---------------------------------------------------------------------------
// El atrezo
// ---------------------------------------------------------------------------

/// Lo que hay por la pista que no es un baile.
///
/// Sin esto la tarima es un tablero con cuatro fichas. Farolas, bancos,
/// palmeras en maceta y un templete la convierten en un paseo de 1900, que es
/// lo que hacen los arboles y las casitas del mapa de Cuphead: que el camino
/// entre dos combates sea un sitio.
#[derive(Clone, Copy)]
enum Atrezo {
    Farola,
    Banco,
    Palmera,
    Templete,
}

/// Donde va cada cosa, en coordenadas de la pista. Por los lados, que el
/// centro es de los bailes y de sus caminos.
const ATREZO: [(f32, f32, Atrezo); 13] = [
    (70.0, 640.0, Atrezo::Farola),
    (570.0, 640.0, Atrezo::Farola),
    (82.0, 470.0, Atrezo::Farola),
    (558.0, 470.0, Atrezo::Farola),
    (98.0, 310.0, Atrezo::Farola),
    (542.0, 310.0, Atrezo::Farola),
    (100.0, 560.0, Atrezo::Banco),
    (540.0, 560.0, Atrezo::Banco),
    (58.0, 745.0, Atrezo::Palmera),
    (582.0, 745.0, Atrezo::Palmera),
    (122.0, 222.0, Atrezo::Palmera),
    (520.0, 226.0, Atrezo::Palmera),
    (470.0, 170.0, Atrezo::Templete),
];

const HIERRO: Color = color_u8!(44, 36, 40, 255);
const MADERA: Color = color_u8!(128, 84, 56, 255);
const TERRACOTA: Color = color_u8!(172, 90, 64, 255);
const PALMA: Color = color_u8!(74, 116, 74, 255);
const TRONCO: Color = color_u8!(100, 72, 50, 255);
const TEJADO: Color = color_u8!(78, 116, 96, 255);

fn dibujar_atrezo(l: &Layout, x: f32, y: f32, que: Atrezo, t: f32) {
    let (s, escala) = suelo(l, x, y);
    let k = l.len(TAMANO) * escala;
    // La sombra, que es lo que lo planta en el suelo.
    draw_ellipse(s.x, s.y, 16.0 * k, 4.5 * k, 0.0, fade(TINTA, 0.35));
    let lp = Lapiz::new(s, k, false);
    match que {
        Atrezo::Farola => {
            // La luz primero, para que el farol quede encima de su halo.
            let farol = lp.p(0.0, -72.0);
            let pulso = 0.8 + 0.2 * (t * 0.04 + x).sin();
            draw_circle(farol.x, farol.y, 20.0 * k, fade(LUZ, 0.10 * pulso));
            lp.caja(-4.0, -4.0, 8.0, 4.0, HIERRO);
            lp.caja(-1.5, -64.0, 3.0, 60.0, HIERRO);
            lp.caja(-7.0, -67.0, 14.0, 3.0, HIERRO);
            lp.caja(-5.0, -79.0, 10.0, 12.0, lp.luz(t));
            lp.tri_tinta((-7.0, -79.0), (7.0, -79.0), (0.0, -86.0), HIERRO);
        }
        Atrezo::Banco => {
            lp.linea((-13.0, -11.0), (-13.0, 0.0), 2.5, HIERRO);
            lp.linea((13.0, -11.0), (13.0, 0.0), 2.5, HIERRO);
            lp.caja(-17.0, -24.0, 34.0, 4.0, MADERA);
            lp.caja(-17.0, -13.0, 34.0, 4.0, MADERA);
            lp.linea((-15.0, -20.0), (-15.0, -13.0), 2.0, HIERRO);
            lp.linea((15.0, -20.0), (15.0, -13.0), 2.0, HIERRO);
        }
        Atrezo::Palmera => {
            // Las hojas se mecen un poco, cada palmera a su aire.
            let (cx, cy) = (2.0, -48.0);
            let brisa = (t * 0.03 + x * 0.1).sin() * 2.0;
            for (dx, dy) in [
                (-22.0, -4.0),
                (-14.0, -18.0),
                (0.0, -24.0),
                (14.0, -18.0),
                (22.0, -4.0),
            ] {
                let punta = (cx + dx + brisa, cy + dy);
                let lado = if dx < 0.0 { 4.0 } else { -4.0 };
                lp.tri_tinta((cx, cy - 3.0), (cx + lado * 0.5, cy + 3.0), punta, PALMA);
            }
            lp.linea((0.0, -14.0), (cx, cy), 4.0, TRONCO);
            lp.tri_tinta((-9.0, -16.0), (9.0, -16.0), (6.0, 0.0), TERRACOTA);
            lp.tri_tinta((-9.0, -16.0), (6.0, 0.0), (-6.0, 0.0), TERRACOTA);
            lp.caja(-10.0, -18.0, 20.0, 3.0, TERRACOTA);
        }
        Atrezo::Templete => {
            // El templete de musica del paseo: tarima, columnas, tejado y
            // la bola dorada arriba.
            lp.caja(-36.0, -10.0, 72.0, 10.0, PAPEL);
            for cx in [-30.0, -11.0, 8.0, 27.0] {
                lp.caja(cx, -46.0, 3.0, 36.0, PAPEL);
            }
            lp.linea((-34.0, -22.0), (34.0, -22.0), 1.5, HIERRO);
            lp.tri_tinta((-42.0, -46.0), (42.0, -46.0), (0.0, -72.0), TEJADO);
            lp.disco(0.0, -75.0, 3.5, ORO);
        }
    }
}

/// Como se baila: alas si es en el aire, un zapato si es en el suelo.
///
/// Cambia el juego entero —en el suelo se salta y no se vuela—, asi que tiene
/// que verse antes de entrar, igual que el mapa de Cuphead marca con un
/// avioncito los niveles de avion. Un icono y no una palabra: se lee de un
/// vistazo y no ocupa la placa.
fn modo(c: Vec2, suelo: bool, cerrado: bool) {
    let color = if cerrado { TINTA_TENUE } else { PAPEL };
    if suelo {
        // Un zapato de baile: la suela, el empeine y el tacon.
        draw_ellipse(c.x, c.y + 2.0, 9.0, 4.5, 0.0, TINTA);
        draw_ellipse(c.x, c.y + 2.0, 7.0, 3.0, 0.0, color);
        draw_rectangle(c.x + 3.0, c.y + 2.0, 4.0, 6.0, TINTA);
        draw_circle(c.x - 3.0, c.y - 1.0, 4.0, TINTA);
        draw_circle(c.x - 3.0, c.y - 1.0, 2.5, color);
    } else {
        // Dos alas, una a cada lado.
        for lado in [-1.0, 1.0] {
            let (x, rot) = (c.x + lado * 5.0, lado * 25.0);
            draw_ellipse(x, c.y, 6.5, 3.5, rot, TINTA);
            draw_ellipse(x, c.y, 5.0, 2.2, rot, color);
        }
        draw_circle(c.x, c.y, 2.0, TINTA);
    }
}

/// Un color de baile cerrado: casi gris y mas oscuro. **Se mezcla, no se
/// transparenta**: con alfa, la tinta de debajo de cada pieza se veria a
/// traves y el dibujo se ensuciaria.
fn apagar(c: Color) -> Color {
    let gris = c.r * 0.3 + c.g * 0.59 + c.b * 0.11;
    let m = |v: f32| (v * 0.25 + gris * 0.75) * 0.6;
    Color::new(m(c.r), m(c.g), m(c.b), c.a)
}

/// Lo que devuelve cada monumento: donde tiene la puerta, para clavarle las
/// tablas si esta cerrado, y cuanto mide, para poner la flecha encima.
struct Silueta {
    puerta: Rect,
    alto: f32,
}

/// El Vals: un palacio de Viena, con su cupula y sus ventanales encendidos.
fn palacio(lp: &Lapiz, tinta: Color, ropa: Color, luz: Color) -> Silueta {
    // La cupula va antes que el cuerpo, que le tapa la mitad de abajo; y la
    // linterna antes que la cupula, por lo mismo.
    lp.caja(-2.5, -86.0, 5.0, 10.0, tinta);
    lp.disco(0.0, -88.0, 2.6, ORO);
    lp.disco(0.0, -62.0, 17.0, tinta);

    // Las alas, su cornisa y el cuerpo central, que asoma por encima.
    lp.caja(-42.0, -44.0, 84.0, 38.0, ropa);
    lp.caja(-44.0, -47.0, 88.0, 4.0, tinta);
    lp.caja(-18.0, -60.0, 36.0, 54.0, ropa);
    lp.caja(-20.0, -63.0, 40.0, 4.0, tinta);

    // Los ventanales del salon: es un baile, asi que hay luz dentro.
    for x in [-38.0, -27.0, 20.0, 31.0] {
        lp.arco(x, -32.0, 7.0, 16.0, luz);
    }
    lp.disco(0.0, -44.0, 4.5, luz);
    lp.arco(-6.0, -24.0, 12.0, 18.0, tinta);

    // La escalinata.
    lp.caja(-46.0, -6.0, 92.0, 6.0, ropa);
    Silueta {
        puerta: Rect::new(-6.0, -30.0, 12.0, 24.0),
        alto: 90.0,
    }
}

/// El Tango: una esquina del arrabal, con su balcon de hierro y su farol.
fn esquina(lp: &Lapiz, tinta: Color, ropa: Color, luz: Color, t: f32) -> Silueta {
    lp.caja(-38.0, -58.0, 56.0, 58.0, tinta);
    lp.caja(-41.0, -63.0, 62.0, 5.0, ropa);

    // Abajo: la puerta y una ventana con reja.
    lp.arco(-4.0, -20.0, 12.0, 20.0, ropa);
    lp.caja(-31.0, -24.0, 14.0, 14.0, luz);
    for x in [-27.5, -24.0, -20.5] {
        lp.linea((x, -24.0), (x, -10.0), 1.0, TINTA);
    }

    // Arriba: dos puertas al balcon, la losa y la baranda de hierro.
    lp.caja(-30.0, -51.0, 11.0, 18.0, luz);
    lp.caja(1.0, -51.0, 11.0, 18.0, luz);
    lp.caja(-35.0, -33.0, 50.0, 3.0, ropa);
    lp.linea((-35.0, -41.0), (15.0, -41.0), 1.4, TINTA);
    let mut x = -35.0;
    while x <= 15.0 {
        lp.linea((x, -41.0), (x, -33.0), 0.8, TINTA);
        x += 3.5;
    }

    // El farol, con su halo latiendo. El halo va sin contorno y con alfa: es
    // luz, no una pieza.
    if !lp.cerrado {
        let h = lp.p(28.5, -60.0);
        let r = (11.0 + 2.5 * (t * 0.07).sin()) * lp.k;
        draw_circle(h.x, h.y, r, fade(LUZ, 0.18));
    }
    lp.caja(27.0, -56.0, 3.0, 56.0, TINTA);
    lp.caja(25.0, -4.0, 7.0, 4.0, TINTA);
    lp.caja(24.5, -64.0, 8.0, 8.0, luz);
    lp.tri_tinta((23.5, -64.0), (33.5, -64.0), (28.5, -70.0), TINTA);
    Silueta {
        puerta: Rect::new(-4.0, -26.0, 12.0, 26.0),
        alto: 70.0,
    }
}

/// El Charleston: la fachada de un club de jazz, con su corona escalonada, su
/// sol Art Deco sobre la puerta y las bombillas de la marquesina corriendo.
fn club(lp: &Lapiz, tinta: Color, ropa: Color, luz: Color, t: f32) -> Silueta {
    // La corona escalonada, de arriba abajo: cada escalon tapa el pie del de
    // encima.
    lp.caja(-12.0, -68.0, 24.0, 10.0, ropa);
    lp.caja(-24.0, -60.0, 48.0, 10.0, ropa);
    lp.caja(-40.0, -52.0, 80.0, 52.0, ropa);
    for x in [-36.0, 32.0] {
        lp.caja(x, -52.0, 4.0, 52.0, tinta);
    }
    lp.caja(-30.0, -26.0, 9.0, 12.0, luz);
    lp.caja(21.0, -26.0, 9.0, 12.0, luz);

    // La marquesina, con las bombillas corriendo: una de cada tres encendida
    // y el patron avanza solo, igual que en el decorado del club.
    lp.caja(-30.0, -46.0, 60.0, 12.0, tinta);
    let paso = (t / 9.0) as usize;
    let apagada = color_u8!(110, 86, 44, 255);
    for i in 0..10 {
        let x = -27.0 + i as f32 * 6.0;
        // Arriba corren hacia la derecha y abajo hacia la izquierda: la
        // vuelta entera a la marquesina, como las de verdad.
        for (y, n) in [(-46.0, i), (-34.0, 9 - i)] {
            let encendida = !lp.cerrado && (n + paso).is_multiple_of(3);
            lp.disco(x, y, 1.6, if encendida { LUZ } else { apagada });
        }
    }

    // El sol sobre la puerta: rayos alternos y el borde en tinta.
    let (cx, cy, r) = (0.0, -22.0, 10.0);
    let rayos = 8;
    for i in 0..rayos {
        let a0 = std::f32::consts::PI * (1.0 + i as f32 / rayos as f32);
        let a1 = std::f32::consts::PI * (1.0 + (i + 1) as f32 / rayos as f32);
        let c = if i % 2 == 0 { tinta } else { luz };
        lp.tri(
            (cx, cy),
            (cx + a0.cos() * r, cy + a0.sin() * r),
            (cx + a1.cos() * r, cy + a1.sin() * r),
            c,
        );
    }
    let o = lp.p(cx, cy);
    draw_arc(o.x, o.y, 24, r * lp.k, 180.0, lp.g, 180.0, TINTA);
    lp.caja(-8.0, -22.0, 16.0, 22.0, TINTA);
    lp.linea((0.0, -22.0), (0.0, 0.0), 0.8, ORO);
    Silueta {
        puerta: Rect::new(-8.0, -22.0, 16.0, 22.0),
        alto: 70.0,
    }
}

/// El Cancan: el Moulin Rouge, con la nave, el molino y las aspas girando. Que
/// giren es lo que hace que se reconozca sin letrero.
fn molino(lp: &Lapiz, tinta: Color, ropa: Color, luz: Color, t: f32) -> Silueta {
    // El molino va detras de la nave.
    lp.caja(-12.0, -60.0, 24.0, 34.0, tinta);
    lp.tri_tinta((-16.0, -60.0), (16.0, -60.0), (0.0, -73.0), ropa);

    lp.caja(-40.0, -28.0, 80.0, 28.0, ropa);
    lp.caja(-42.0, -32.0, 84.0, 5.0, tinta);
    lp.caja(-34.0, -22.0, 9.0, 10.0, luz);
    lp.caja(25.0, -22.0, 9.0, 10.0, luz);
    lp.arco(-7.0, -16.0, 14.0, 16.0, luz);

    // Las aspas, por delante de todo. Mismo dibujo que las del decorado: dos
    // triangulos de lona, los cantos en tinta y el enrejado.
    let eje = (0.0, -62.0);
    let giro = t * 0.02;
    let lona = color_u8!(232, 206, 164, 255);
    for k in 0..4 {
        let a = giro + std::f32::consts::FRAC_PI_2 * k as f32;
        let (s, c) = a.sin_cos();
        let (largo, ancho) = (32.0, 7.0);
        let base = (eje.0 + c * 4.0, eje.1 + s * 4.0);
        let punta = (eje.0 + c * largo, eje.1 + s * largo);
        let lado = |p: (f32, f32), d: f32| (p.0 - s * d, p.1 + c * d);
        let (a0, a1) = (lado(base, 0.0), lado(base, ancho));
        let (b0, b1) = (lado(punta, 0.0), lado(punta, ancho));
        lp.tri(a0, a1, b1, lona);
        lp.tri(a0, b1, b0, lona);
        for (p, q) in [(a0, b0), (a1, b1), (b0, b1)] {
            lp.linea(p, q, 1.3, TINTA);
        }
        for f in [0.4, 0.7] {
            let p = (
                base.0 + (punta.0 - base.0) * f,
                base.1 + (punta.1 - base.1) * f,
            );
            lp.linea(lado(p, 0.0), lado(p, ancho), 0.8, TINTA);
        }
    }
    lp.disco(eje.0, eje.1, 3.0, ORO);
    Silueta {
        puerta: Rect::new(-7.0, -23.0, 14.0, 23.0),
        alto: 96.0,
    }
}

/// Dos tablones en aspa sobre la puerta: el cartel de "cerrado" que se
/// entiende sin leer.
///
/// Van sin apagar, a proposito: todo lo demas del monumento esta gris y las
/// tablas son lo unico con color, que es lo que tiene que verse primero.
fn tablones(lp: &Lapiz, p: Rect) {
    let ancho = 3.4 * lp.k;
    for (a, b) in [
        (
            (p.x - 3.0, p.y + p.h * 0.3),
            (p.x + p.w + 3.0, p.y + p.h * 0.7),
        ),
        (
            (p.x - 3.0, p.y + p.h * 0.7),
            (p.x + p.w + 3.0, p.y + p.h * 0.3),
        ),
    ] {
        let (a, b) = (lp.p(a.0, a.1), lp.p(b.0, b.1));
        draw_line(a.x, a.y, b.x, b.y, ancho + lp.g * 2.0, TINTA);
        draw_line(a.x, a.y, b.x, b.y, ancho, TABLON);
    }
}

/// La escarapela de "bailado": un roseton de oro con el centro en la tinta del
/// baile y dos cintas colgando. Es el premio de feria, y lo que en el mapa de
/// Cuphead es la bandera clavada en un nivel hecho.
fn escarapela(c: Vec2, r: f32, tinta: Color) {
    let lp = Lapiz::new(c, r / 10.0, false);
    lp.tri_tinta((-6.0, 4.0), (-1.0, 4.0), (-7.0, 20.0), tinta);
    lp.tri_tinta((1.0, 4.0), (6.0, 4.0), (7.0, 20.0), tinta);
    draw_poly(c.x, c.y, 14, r + lp.g, 0.0, TINTA);
    draw_poly(c.x, c.y, 14, r, 0.0, ORO);
    lp.disco(0.0, 0.0, 5.0, tinta);
}

/// La tienda de la modista: una fachada verde con su toldo a
/// rayas, el escaparate encendido con un vestido dentro y una ficha de oro
/// colgando de la muestra, que dice de un vistazo que ahi se paga con fichas.
fn tienda(l: &Layout, t: f32, cerca: bool) {
    let (s, escala) = suelo(l, MODISTA.x, MODISTA.y);
    let k = l.len(TAMANO) * escala;
    let foco = if cerca {
        0.6 + 0.4 * (0.5 + 0.5 * (t * 0.08).sin())
    } else {
        0.45
    };
    draw_ellipse(s.x, s.y, 46.0 * k, 12.0 * k, 0.0, fade(FOCO, foco));
    let lp = Lapiz::new(s, k, false);
    let verde = color_u8!(30, 54, 46, 255);
    let carmin = color_u8!(176, 44, 58, 255);

    lp.caja(-34.0, -52.0, 68.0, 52.0, verde);
    lp.caja(-37.0, -57.0, 74.0, 6.0, TINTA);
    // El escaparate, con el vestido de ella dentro.
    lp.caja(-28.0, -34.0, 36.0, 28.0, lp.luz(t));
    lp.tri_tinta(
        (-10.0, -28.0),
        (-17.0, -8.0),
        (-3.0, -8.0),
        protagonista::VESTIDO,
    );
    lp.disco(-10.0, -28.0, 2.2, protagonista::LAZO);
    // La puerta.
    lp.caja(14.0, -30.0, 13.0, 30.0, color_u8!(98, 50, 40, 255));
    lp.disco(24.0, -15.0, 1.0, ORO);
    // El toldo: rayas en triangulo, colgando por delante.
    for i in 0..8 {
        let x = -36.0 + i as f32 * 9.0;
        let color = if i % 2 == 0 { PAPEL } else { carmin };
        lp.tri((x, -46.0), (x + 9.0, -46.0), (x + 4.5, -38.0), color);
        lp.caja(x, -50.0, 9.0, 4.0, color);
    }
    // La muestra: un brazo de hierro y una ficha colgando que da vueltas.
    lp.linea((34.0, -46.0), (46.0, -46.0), 1.6, TINTA);
    lp.linea((44.0, -46.0), (44.0, -42.0), 1.0, TINTA);
    modista::ficha(lp.p(44.0, -36.0), 6.0 * k, t * 0.04);

    // La placa, como la de un baile.
    placa(s, "La modista", false);
}

/// La placa de papel con un nombre debajo de un sitio del mapa. Devuelve su
/// rectangulo, que es donde se cuelga lo demas.
fn placa(s: Vec2, nombre: &str, cerrado: bool) -> Rect {
    // Va en Barlow y no en Poiret: la Poiret es de trazo tan fino que a este
    // tamano, en tinta sobre papel, se quedaba en gris.
    let tam = 16.0;
    let m = fuentes::medir(nombre, tam, Cara::Cuerpo);
    let (w, h) = (m.width + 14.0, tam + 6.0);
    let (x, y) = (s.x - w * 0.5, s.y + 6.0);
    let (fondo, letra) = if cerrado {
        (TINTA_TENUE, PAPEL)
    } else {
        (PAPEL, TINTA)
    };
    draw_rectangle(x - 2.0, y - 2.0, w + 4.0, h + 4.0, TINTA);
    draw_rectangle(x, y, w, h, fondo);
    // Dos pasadas separadas medio pixel: una negrita de imprenta. A este
    // tamano la letra sola salia gris.
    for dx in [0.0, 0.6] {
        fuentes::centrado(nombre, s.x + dx, y + tam * 0.9, tam, Cara::Cuerpo, letra);
    }
    Rect::new(x, y, w, h)
}

/// Un baile plantado en la pista: el foco, su monumento, su placa y lo que
/// diga su estado.
fn dibujar_nodo(l: &Layout, nodo: &Nodo, abierto: bool, t: f32, fichas: Option<(usize, usize)>) {
    let (s, escala) = suelo(l, nodo.pos.x, nodo.pos.y);
    let k = l.len(TAMANO) * escala;
    let cerrado = !abierto;
    // El siguiente objetivo: abierto y sin bailar. Es lo que tiene que verse
    // desde la entrada sin pensar.
    let siguiente = abierto && !nodo.vencido;
    let pulso = 0.5 + 0.5 * (t * 0.08).sin();

    let foco = if cerrado {
        0.25
    } else if siguiente {
        0.6 + 0.4 * pulso
    } else {
        0.5
    };
    draw_ellipse(s.x, s.y, 50.0 * k, 13.0 * k, 0.0, fade(FOCO, foco));

    // Un baile cerrado no se mueve: ni aspas ni bombillas. Parado es otra
    // forma de decir que ahi no pasa nada todavia.
    let t_mon = if cerrado { 0.0 } else { t };
    let lp = Lapiz::new(s, k, cerrado);
    let (tinta, ropa) = paleta::del_baile(nodo.jefe);
    let luz = lp.luz(t_mon);
    let silueta = match nodo.jefe {
        0 => palacio(&lp, tinta, ropa, luz),
        1 => esquina(&lp, tinta, ropa, luz, t_mon),
        2 => club(&lp, tinta, ropa, luz, t_mon),
        _ => molino(&lp, tinta, ropa, luz, t_mon),
    };
    if cerrado {
        tablones(&lp, silueta.puerta);
    }
    if siguiente {
        // Una flecha de oro botando encima: el "aqui" de los mapas.
        let y = -silueta.alto - 8.0 - 5.0 * pulso;
        lp.tri_tinta((-8.0, y - 13.0), (8.0, y - 13.0), (0.0, y), ORO);
    }

    // La placa con el nombre, en papel y tinta: es un rotulo impreso, y asi se
    // lee igual sobre la tarima oscura que sobre cualquier monumento. Un baile
    // cerrado no dice lo dificil que es —eso se descubre bailandolo—, solo que
    // esta cerrado.
    let Rect { x, y, w, h } = placa(s, &nodo.nombre, cerrado);
    modo(vec2(x - 13.0, y + h * 0.5), nodo.suelo, cerrado);

    if nodo.vencido {
        escarapela(vec2(x + w, y + h * 0.5), 9.0, tinta);
        fuentes::centrado(
            "bailado",
            s.x,
            y + h + 16.0,
            14.0,
            Cara::Cuerpo,
            fade(LUZ, 0.85),
        );
    }

    // Las fichas de su paseo, si ya se anduvo: llenas las tuyas y huecas las
    // que quedan en la calle. Es lo que dice, sin acercarse, a donde volver.
    if let Some((tuyas, total)) = fichas {
        let paso = 15.0;
        let x0 = s.x - paso * (total as f32 - 1.0) * 0.5;
        let fy = y + h + if nodo.vencido { 30.0 } else { 16.0 };
        for i in 0..total {
            let c = vec2(x0 + paso * i as f32, fy);
            if i < tuyas {
                modista::ficha(c, 5.5, 0.0);
            } else {
                draw_circle_lines(c.x, c.y, 5.5, 1.5, fade(ORO, 0.6));
            }
        }
    }
}

/// La bailarina, en su sitio de la pista.
fn dibujar_bailarina(p: &Pista, alpha: f32, t: f32, l: &Layout) {
    // Es la misma figura del combate: se le pasa la velocidad para que la
    // falda se quede atras y la cara mire hacia donde anda, y bailas mejor
    // cuanto mas llevas hecho.
    let pos = p.render_pos(alpha);
    let (s, escala) = suelo(l, pos.x, pos.y);
    let figura = p.figura();
    let pose = skeleton::pose(&figura, t, false, p.respeto());
    // Mas grande que su esqueleto, como en el combate: al lado de un palacio,
    // a tamano 1, la cara no era mas que un punto.
    let ls = l.escalado(escala * 1.5);

    // Los pies van justo donde pisa, no el centro del cuerpo: si no, la
    // bailarina flota y se despega de su sombra.
    let pie = pose.joints.iter().map(|j| j.y).fold(f32::MIN, f32::max);
    let centro = s - vec2(0.0, pie * ls.scale());

    draw_ellipse(
        s.x,
        s.y,
        l.len(26.0) * escala,
        l.len(7.0) * escala,
        0.0,
        fade(FOCO, 0.55),
    );
    let gesto = protagonista::Gesto::de(&figura, t);
    protagonista::dibujar(&pose, &gesto, centro, &ls, 1.0, false);
}

/// Lo que el mapa cuenta de las fichas.
pub struct Fichas {
    /// Las que quedan en el bolsillo.
    pub bolsillo: u32,
    /// Por nodo: si su paseo ya se anduvo, cuantas de sus fichas son tuyas y
    /// cuantas tiene. Un paseo andado es uno al que se puede volver a por
    /// ellas.
    pub paseos: Vec<Option<(usize, usize)>>,
}

/// El mapa entero. `teclas` son los nombres de aceptar y de dash en lo que se
/// este usando, para que la ayuda diga el boton que hay que pulsar.
pub fn pista(p: &Pista, alpha: f32, l: &Layout, fichas: &Fichas, teclas: [&str; 2]) {
    clear_background(PAPEL);
    let o = l.to_screen(0.0, 0.0);
    draw_rectangle(o.x, o.y, l.len(ARENA_W), l.len(ARENA_H), TARIMA);
    dibujar_tarima(l);

    let t = p.tick as f32 + alpha;
    let pos = |n: &Nodo| vec2(n.pos.x, n.pos.y);

    // Los caminos van antes que nada: pasan por debajo de los monumentos y de
    // las placas. El primero sale de la entrada, que es donde empiezas.
    let primero = p.nodos.iter().map(|n| n.nivel).min();
    for (i, n) in p.nodos.iter().enumerate() {
        if Some(n.nivel) == primero {
            sendero(l, vec2(ENTRADA.x, ENTRADA.y), pos(n), p.abierto(i));
        }
    }
    for (a, b) in tramos(&p.nodos) {
        sendero(l, pos(&p.nodos[a]), pos(&p.nodos[b]), p.abierto(b));
    }
    // Y la salida: un medallon de oro en el suelo, para que el camino empiece
    // en algun sitio y no en mitad de la tarima.
    let (e, escala) = suelo(l, ENTRADA.x, ENTRADA.y);
    let (rx, ry) = (l.len(20.0) * escala, l.len(7.0) * escala);
    draw_ellipse(e.x, e.y, rx + 2.5, ry + 2.5, 0.0, TINTA);
    draw_ellipse(e.x, e.y, rx, ry, 0.0, ORO);
    draw_ellipse_lines(e.x, e.y, rx * 0.6, ry * 0.6, 0.0, 1.5, TINTA);

    // De fondo a frente, para que lo cercano tape a lo lejano, **y la
    // bailarina entre medias**: con monumentos altos, pintarla siempre encima
    // la pondria delante de un palacio que tiene delante.
    // El atrezo entra en el mismo orden: una farola puede quedar delante o
    // detras de ella segun por donde pase.
    enum Pieza {
        Baile(usize),
        Atrezo(usize),
        Modista,
    }
    let mut orden: Vec<(f32, Pieza)> = p
        .nodos
        .iter()
        .enumerate()
        .map(|(i, n)| (n.pos.y, Pieza::Baile(i)))
        .chain(
            ATREZO
                .iter()
                .enumerate()
                .map(|(k, a)| (a.1, Pieza::Atrezo(k))),
        )
        .chain([(MODISTA.y, Pieza::Modista)])
        .collect();
    orden.sort_by(|a, b| a.0.total_cmp(&b.0));
    let ella = p.render_pos(alpha).y;
    let detras = orden.iter().take_while(|(y, _)| *y <= ella).count();
    let pinta = |pieza: &Pieza| match *pieza {
        Pieza::Baile(i) => {
            let suyas = fichas.paseos.get(i).copied().flatten();
            dibujar_nodo(l, &p.nodos[i], p.abierto(i), t, suyas);
        }
        Pieza::Modista => tienda(l, t, p.en_la_modista()),
        Pieza::Atrezo(k) => {
            let (x, y, que) = ATREZO[k];
            dibujar_atrezo(l, x, y, que, t);
        }
    };
    orden[..detras].iter().for_each(|(_, pieza)| pinta(pieza));
    dibujar_bailarina(p, alpha, t, l);
    orden[detras..].iter().for_each(|(_, pieza)| pinta(pieza));

    // --- Cartel de arriba y ayuda de abajo ---
    let cx = l.to_screen(ARENA_W * 0.5, 0.0).x;
    fuentes::centrado(
        "LA PISTA",
        cx,
        o.y + 44.0,
        40.0,
        Cara::Titulo,
        fade(TITLE, 0.9),
    );

    let hechos = p.nodos.iter().filter(|n| n.vencido).count();
    let cuenta = format!("{hechos} / {} bailes", p.nodos.len());
    fuentes::centrado(&cuenta, cx, o.y + 66.0, 15.0, Cara::Cuerpo, TEXT_DIM);

    // El bolsillo, arriba a la derecha: la ficha dando vueltas y cuantas hay.
    let b = l.to_screen(ARENA_W - 30.0, 0.0);
    let n = fichas.bolsillo.to_string();
    let ancho = fuentes::medir(&n, 22.0, Cara::Cuerpo).width;
    modista::ficha(vec2(b.x - ancho - 16.0, o.y + 36.0), 10.0, t * 0.05);
    fuentes::derecha(&n, b.x, o.y + 44.0, 22.0, Cara::Cuerpo, METER_FULL);

    let pie_y = l.to_screen(0.0, ARENA_H).y - 16.0;
    let [ok, dash] = teclas;
    let aviso = match p.nodo_cerca() {
        Some(i) if p.abierto(i) => {
            let donde = if p.nodos[i].suelo {
                "en el suelo"
            } else {
                "volando"
            };
            let bailar = format!("{ok}    bailar {}, {donde}", p.nodos[i].nombre);
            // Si su paseo ya se anduvo, se puede volver a el a por fichas.
            match fichas.paseos.get(i).copied().flatten() {
                Some((tuyas, total)) => {
                    format!("{bailar}     {dash}    pasear ({tuyas}/{total} fichas)")
                }
                None => bailar,
            }
        }
        // Un baile cerrado dice **por que** lo esta. "Bloqueado" a secas manda
        // a probar cosas al azar; decir que falta el nivel de antes no.
        Some(i) => format!(
            "{} todavia no: antes hay que sacar lo anterior",
            p.nodos[i].nombre
        ),
        None if p.en_la_modista() => format!("{ok}    entrar a la modista"),
        None => "flechas andar    ESC menu".to_string(),
    };
    let color = if p.nodo_cerca().is_some_and(|i| p.abierto(i)) || p.en_la_modista() {
        // Late, para que se vea que ahi hay algo que hacer.
        let pulso = 0.7 + 0.3 * (t * 0.12).sin();
        fade(METER_FULL, pulso)
    } else {
        TEXT_DIM
    };
    fuentes::centrado(&aviso, cx, pie_y, 19.0, Cara::Cuerpo, color);
    paspartu(l);
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::boss::Nivel;

    #[test]
    fn el_atrezo_no_pisa_ningun_baile() {
        // Las farolas y las palmeras van por los lados, pero los bailes se
        // recolocan solos segun cuantos haya de cada nivel: en cuanto entre un
        // quinto, una farola podria acabar encima de un palacio.
        let bailes = vals_core::boss::BossDef::default_bosses()
            .into_iter()
            .map(|d| (d.name, d.nivel))
            .collect();
        let p = Pista::new(bailes);
        for (x, y, _) in ATREZO {
            // Ni la tienda de la modista, que es mas ancha que un baile.
            let d = vec2(x - MODISTA.x, y - MODISTA.y).length();
            assert!(
                d > 80.0,
                "el atrezo en ({x}, {y}) pisa la modista (a {d:.0})"
            );
            for n in &p.nodos {
                let d = vec2(x - n.pos.x, y - n.pos.y).length();
                assert!(
                    d > vals_core::pista::RADIO_NODO,
                    "el atrezo en ({x}, {y}) pisa {} (a {d:.0})",
                    n.nombre
                );
            }
        }
    }

    fn indice(p: &Pista, nombre: &str) -> usize {
        p.nodos.iter().position(|n| n.nombre == nombre).unwrap()
    }

    #[test]
    fn el_camino_une_cada_nivel_con_el_siguiente_y_no_la_lista() {
        // Los bailes de hoy: uno facil, dos medios y uno dificil. El camino es
        // un rombo, no una cadena: el charleston y el cancan se abren a la vez
        // y ninguno va detras del otro.
        let p = Pista::new(vec![
            ("Vals".into(), Nivel::Facil),
            ("Tango".into(), Nivel::Dificil),
            ("Charleston".into(), Nivel::Media),
            ("Cancan".into(), Nivel::Media),
        ]);
        let [v, t, ch, cc] = ["Vals", "Tango", "Charleston", "Cancan"].map(|n| indice(&p, n));
        let mut tramos = tramos(&p.nodos);
        tramos.sort();
        let mut esperados = vec![(v, ch), (v, cc), (ch, t), (cc, t)];
        esperados.sort();
        assert_eq!(tramos, esperados);
    }

    #[test]
    fn un_nivel_vacio_no_corta_el_camino() {
        // La misma regla que `nivel_abierto`: si falta el nivel medio, del
        // facil se va directo al dificil.
        let p = Pista::new(vec![
            ("Vals".into(), Nivel::Facil),
            ("Tango".into(), Nivel::Dificil),
        ]);
        assert_eq!(
            tramos(&p.nodos),
            vec![(indice(&p, "Vals"), indice(&p, "Tango"))]
        );
    }
}
