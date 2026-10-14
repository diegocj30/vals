//! Los botones dibujados: la ayuda en pantalla con la cruz azul del mando de
//! PlayStation, la A verde del de Xbox o la tapa de la tecla, en vez de su
//! nombre escrito.
//!
//! Un nombre se lee y un dibujo se reconoce. "cuadrado" obliga a traducir la
//! palabra al boton; el cuadrado rosa lo encuentra el pulgar sin mirar el
//! mando. Por eso cada boton se dibuja como viene en su mando —las formas de
//! colores de PlayStation, las letras en su disco de Xbox, las teclas con su
//! tapa—, y con primitivas, como todo lo demas: ni una imagen.
//!
//! Una linea de ayuda es una lista de `Trozo`s, texto y botones, que se mide y
//! se dibuja de una vez (`ancho`, `centrado`...). Se arma con `rellenar` desde
//! las plantillas de `idioma` de siempre: cada `{}` se llena con un boton o con
//! un texto. La frase la sigue decidiendo cada idioma, y el boton cae donde la
//! frase lo pide.
//!
//! El boton va con el tamano de la letra y sobre su misma linea base: una
//! ayuda encogida para que quepa (`encajado`) encoge sus botones con ella, y
//! una linea centrada sigue centrada porque se mide con ellos dentro.
//!
//! Lo de dentro (el disco, la tapa) va en tinta y lo de alrededor en el color
//! de la linea: el mismo boton se lee sobre la sala oscura y sobre el papel.

use std::fmt::Display;

use macroquad::prelude::*;

use crate::fuentes::{self, Cara};
use crate::idioma::{Texto, tx};
use crate::paleta::{PAPEL, TINTA};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Arriba,
    Abajo,
    Izquierda,
    Derecha,
}

impl Dir {
    /// Hacia donde apunta, con la y de la pantalla (hacia abajo).
    fn v(self) -> Vec2 {
        match self {
            Dir::Arriba => vec2(0.0, -1.0),
            Dir::Abajo => vec2(0.0, 1.0),
            Dir::Izquierda => vec2(-1.0, 0.0),
            Dir::Derecha => vec2(1.0, 0.0),
        }
    }
}

/// Los cuatro simbolos de la cara de PlayStation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forma {
    Cruz,
    Circulo,
    Cuadrado,
    Triangulo,
}

/// Un boton o una tecla, tal como se dibuja.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boton {
    /// Una tecla con su nombre ya traducido: Z, ESC, ESPACIO...
    Tecla(&'static str),
    Flecha(Dir),
    /// Las cuatro flechas en su T invertida: moverse con el teclado.
    Flechas,
    /// La cara de PlayStation: la forma, en su color.
    Ps(Forma),
    /// La cara de Xbox: la letra, en su disco de color.
    Xbox(&'static str),
    /// Hombros y gatillos: L1, RT...
    Hombro(&'static str),
    /// Los pequenos del centro: Options, Share, Menu, View.
    Pastilla(&'static str),
    /// Un stick con su nombre (L, LS, L3...) y hacia donde, si es empujarlo.
    Stick(&'static str, Option<Dir>),
    Cruceta(Option<Dir>),
}

/// Un trozo de una linea de ayuda.
#[derive(Clone, Debug, PartialEq)]
pub enum Trozo {
    Texto(String),
    Boton(Boton),
}

/// Un boton solo, para llenar un hueco.
pub fn boton(b: Boton) -> Vec<Trozo> {
    vec![Trozo::Boton(b)]
}

/// Un texto solo, para llenar un hueco: un numero, un nombre.
pub fn txt(s: impl Display) -> Vec<Trozo> {
    vec![Trozo::Texto(s.to_string())]
}

/// Una plantilla con sus `{}` llenos en orden, como `idioma::txf` pero con
/// botones. Un hueco sin nada se queda vacio y lo que sobra no se usa, igual
/// que alli. Los textos seguidos se juntan en uno: se miden igual y se
/// dibujan en una llamada.
pub fn rellenar(plantilla: &str, args: &[&[Trozo]]) -> Vec<Trozo> {
    let mut out = Vec::new();
    let mut args = args.iter();
    for (i, t) in plantilla.split("{}").enumerate() {
        if i > 0
            && let Some(a) = args.next()
        {
            for x in a.iter() {
                empujar(&mut out, x.clone());
            }
        }
        empujar(&mut out, Trozo::Texto(t.to_owned()));
    }
    out
}

/// `rellenar` con un texto de `idioma`, en el idioma de ahora.
pub fn txg(t: Texto, args: &[&[Trozo]]) -> Vec<Trozo> {
    rellenar(tx(t), args)
}

fn empujar(out: &mut Vec<Trozo>, t: Trozo) {
    if let Trozo::Texto(s) = &t {
        if s.is_empty() {
            return;
        }
        if let Some(Trozo::Texto(a)) = out.last_mut() {
            a.push_str(s);
            return;
        }
    }
    out.push(t);
}

// ---------------------------------------------------------------------------
// Medir
// ---------------------------------------------------------------------------

/// El alto de un boton, en tamanos de letra. Algo mas que las mayusculas: un
/// boton tiene que verse como una cosa, no como una letra mas.
const ALTO: f32 = 1.15;
/// Cuanto sube el centro del boton sobre la linea base: la mitad de las
/// mayusculas de Barlow, asi el boton queda a la altura del texto de al lado.
const CENTRO: f32 = 0.36;
/// El aire a cada lado de un boton, en tamanos de letra.
const MARGEN: f32 = 0.1;
/// Las teclas de las flechas juntas, y lo que las separa, en altos de boton.
const TECLITA: f32 = 0.56;
const HUECO: f32 = 0.05;

/// La letra mas pequena dentro de un boton. Por debajo, Barlow ya no es
/// letra sino una mancha: mejor un boton algo mas ancho que uno ilegible.
const LETRA_MIN: f32 = 9.0;

/// Lo escrito dentro de un boton, y a que tamano. Una letra sola va grande;
/// "ESPACIO" tiene que caber sin hacer la tecla enorme.
fn rotulo_de(b: Boton, tam: f32) -> Option<(&'static str, f32)> {
    let (s, k) = match b {
        Boton::Tecla(s) if s.chars().count() == 1 => (s, 0.68),
        Boton::Tecla(s) => (s, 0.56),
        Boton::Xbox(s) => (s, 0.8),
        Boton::Hombro(s) => (s, 0.62),
        Boton::Pastilla(s) => (s, 0.54),
        Boton::Stick(s, _) if !s.is_empty() => (s, 0.45),
        _ => return None,
    };
    Some((s, (tam * k).max(LETRA_MIN)))
}

/// El ancho de un boton, sin su aire. `medir` es lo que mide un texto a un
/// tamano: `fuentes::medir` en el juego y lo que haga falta en los tests,
/// que no tienen ventana ni fuentes.
fn medida(b: Boton, tam: f32, medir: &dyn Fn(&str, f32) -> f32) -> f32 {
    let alto = tam * ALTO;
    let escrito = rotulo_de(b, tam).map_or(0.0, |(s, t)| medir(s, t));
    match b {
        Boton::Tecla(_) => (escrito + tam * 0.55).max(alto * 0.95),
        Boton::Flecha(_) => alto * 0.95,
        Boton::Flechas => alto * (3.0 * TECLITA + 2.0 * HUECO),
        Boton::Ps(_) | Boton::Xbox(_) | Boton::Stick(..) | Boton::Cruceta(_) => alto,
        Boton::Hombro(_) => (escrito + tam * 0.6).max(alto * 1.25),
        Boton::Pastilla(_) => escrito + tam * 0.75,
    }
}

fn ancho_con(l: &[Trozo], tam: f32, medir: &dyn Fn(&str, f32) -> f32) -> f32 {
    l.iter()
        .map(|t| match t {
            Trozo::Texto(s) => medir(s, tam),
            Trozo::Boton(b) => medida(*b, tam, medir) + 2.0 * tam * MARGEN,
        })
        .sum()
}

fn medir_cuerpo(s: &str, tam: f32) -> f32 {
    fuentes::medir(s, tam, Cara::Cuerpo).width
}

/// Lo que mide una linea, con sus botones.
pub fn ancho(l: &[Trozo], tam: f32) -> f32 {
    ancho_con(l, tam, &medir_cuerpo)
}

/// El tamano al que `l` cabe en `ancho`: el mismo si ya cabe.
pub fn encoger(l: &[Trozo], tam: f32, ancho_max: f32) -> f32 {
    let mide = ancho(l, tam);
    if mide > ancho_max {
        tam * ancho_max / mide
    } else {
        tam
    }
}

// ---------------------------------------------------------------------------
// Dibujar
// ---------------------------------------------------------------------------

/// Dibuja `l` desde `x`, con la linea base en `y`. Toda la ayuda va en
/// Barlow: es letra para leer rapido, y el titulo no lleva botones.
pub fn texto(l: &[Trozo], x: f32, y: f32, tam: f32, color: Color) {
    pintar(l, x, y, tam, color, true);
}

pub fn centrado(l: &[Trozo], cx: f32, y: f32, tam: f32, color: Color) {
    texto(l, cx - ancho(l, tam) * 0.5, y, tam, color);
}

pub fn derecha(l: &[Trozo], dx: f32, y: f32, tam: f32, color: Color) {
    texto(l, dx - ancho(l, tam), y, tam, color);
}

/// Centrado y encogido si no cabe en `ancho`, como `fuentes::encajado`.
pub fn encajado(l: &[Trozo], cx: f32, y: f32, tam: f32, ancho_max: f32, color: Color) {
    centrado(l, cx, y, encoger(l, tam, ancho_max), color);
}

/// Centrado y con la sombra de tinta de los rotulos (`draw::con_sombra`). La
/// sombra es solo del texto: el boton ya trae su tinta, y doblado se emborrona.
pub fn centrado_con_sombra(l: &[Trozo], cx: f32, y: f32, tam: f32, color: Color) {
    let x = cx - ancho(l, tam) * 0.5;
    pintar(l, x + 1.6, y + 1.6, tam, alfa(TINTA, 0.85 * color.a), false);
    pintar(l, x, y, tam, color, true);
}

fn pintar(l: &[Trozo], x: f32, y: f32, tam: f32, color: Color, botones: bool) {
    let mut x = x;
    for t in l {
        match t {
            Trozo::Texto(s) => {
                fuentes::texto(s, x, y, tam, Cara::Cuerpo, color);
                x += medir_cuerpo(s, tam);
            }
            Trozo::Boton(b) => {
                let (m, w) = (tam * MARGEN, medida(*b, tam, &medir_cuerpo));
                if botones {
                    dibujar(*b, vec2(x + m + w * 0.5, y - tam * CENTRO), w, tam, color);
                }
                x += w + 2.0 * m;
            }
        }
    }
}

/// La letra de una tecla sobre el papel va en tinta llena aunque la linea
/// vaya en tinta tenue: a nueve pixeles, el marron sobre el papel no se lee.
/// Sobre la sala oscura la linea es clara, hasta la tenue (`draw::TEXT_DIM`
/// anda por 0.49), y se queda como esta.
fn entintado(c: Color) -> Color {
    let luz = 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
    if luz < 0.44 { alfa(TINTA, c.a) } else { c }
}

fn alfa(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

/// Los colores de la cara de PlayStation, los de siempre, aclarados un punto
/// para que se lean sobre el disco de tinta.
fn color_ps(f: Forma) -> Color {
    match f {
        Forma::Cruz => color_u8!(125, 167, 217, 255),
        Forma::Circulo => color_u8!(236, 98, 98, 255),
        Forma::Cuadrado => color_u8!(226, 142, 200, 255),
        Forma::Triangulo => color_u8!(72, 200, 162, 255),
    }
}

/// Los discos de Xbox, y si su letra va en tinta. A catorce pixeles no vale
/// la misma letra para los cuatro: la blanca se pierde en el verde y el
/// amarillo, y la de tinta en el rojo.
fn color_xbox(letra: &str) -> (Color, bool) {
    match letra {
        "A" => (color_u8!(112, 190, 82, 255), true),
        "B" => (color_u8!(206, 58, 54, 255), false),
        "X" => (color_u8!(52, 116, 210, 255), false),
        _ => (color_u8!(246, 200, 62, 255), true),
    }
}

/// Un boton centrado en `c`, de ancho `w`. `color` es el de la linea: el
/// filo y las teclas van en el, y su alfa la lleva todo el boton (la ayuda
/// que late, el aviso que se apaga).
fn dibujar(b: Boton, c: Vec2, w: f32, tam: f32, color: Color) {
    let a = color.a;
    let h = tam * ALTO;
    let r = h * 0.5;
    let fino = (tam * 0.075).max(1.0);
    let tinta = alfa(TINTA, a);
    let papel = alfa(PAPEL, a);
    let escrito = rotulo_de(b, tam);
    let escribir = |c: Vec2, color: Color| {
        if let Some((s, t)) = escrito {
            rotulo(s, c, t, color);
        }
    };
    match b {
        Boton::Ps(f) => {
            disco(c, r, tinta, color, fino);
            let (s, g, col) = (r * 0.48, (r * 0.22).max(1.5), alfa(color_ps(f), a));
            match f {
                Forma::Cruz => {
                    let d = s * 0.8;
                    draw_line(c.x - d, c.y - d, c.x + d, c.y + d, g, col);
                    draw_line(c.x - d, c.y + d, c.x + d, c.y - d, g, col);
                }
                Forma::Circulo => draw_circle_lines(c.x, c.y, s * 0.9, g, col),
                Forma::Cuadrado => {
                    let d = s * 0.78;
                    draw_rectangle_lines(c.x - d, c.y - d, d * 2.0, d * 2.0, g, col);
                }
                Forma::Triangulo => {
                    // Bajado un poco: el centro de un triangulo no es el de
                    // su caja, y centrado por la caja parece que flota.
                    let p = |ang: f32| c + vec2(0.0, s * 0.15) + Vec2::from_angle(ang) * s * 1.05;
                    let (k, t) = (std::f32::consts::FRAC_PI_2, std::f32::consts::TAU / 3.0);
                    draw_triangle_lines(p(-k), p(-k + t), p(-k + 2.0 * t), g, col);
                }
            }
        }
        Boton::Xbox(letra) => {
            let (fondo, en_tinta) = color_xbox(letra);
            disco(c, r, alfa(fondo, a), tinta, fino * 1.4);
            escribir(c, if en_tinta { tinta } else { papel });
        }
        Boton::Hombro(_) => {
            redondeado(c, w, h * 0.8, h * 0.26, Some(tinta), color, fino);
            escribir(c, papel);
        }
        Boton::Pastilla(_) => {
            let hh = h * 0.72;
            redondeado(c, w, hh, hh * 0.5, Some(tinta), color, fino);
            escribir(c, papel);
        }
        Boton::Tecla(_) => {
            tecla(c, w, h * 0.92, color, fino);
            escribir(c - vec2(0.0, h * 0.03), entintado(color));
        }
        Boton::Flecha(d) => {
            tecla(c, w, h * 0.92, color, fino);
            flecha(c - vec2(0.0, h * 0.03), d, h * 0.24, color);
        }
        Boton::Flechas => {
            let (k, hueco) = (h * TECLITA, h * HUECO);
            let paso = k + hueco;
            let fino = (fino * 0.8).max(1.0);
            for (dx, dy, d) in [
                (0.0, -0.5, Dir::Arriba),
                (-1.0, 0.5, Dir::Izquierda),
                (0.0, 0.5, Dir::Abajo),
                (1.0, 0.5, Dir::Derecha),
            ] {
                let q = c + vec2(dx, dy) * paso;
                tecla(q, k, k, color, fino);
                flecha(q, d, k * 0.24, color);
            }
        }
        Boton::Stick(_, dir) => {
            disco(c, r, tinta, color, fino);
            // La seta, vista desde arriba y corrida hacia donde se empuja.
            let q = c + dir.map_or(Vec2::ZERO, |d| d.v() * r * 0.16);
            draw_circle(q.x, q.y, r * 0.6, alfa(color, 0.3));
            draw_circle_lines(q.x, q.y, r * 0.6, fino, alfa(color, 0.8));
            escribir(q, papel);
            if let Some(d) = dir {
                flecha(c + d.v() * r * 0.8, d, r * 0.24, papel);
            }
        }
        Boton::Cruceta(dir) => {
            let (largo, brazo) = (h * 0.98, h * 0.36);
            cruz(c, largo, brazo, color);
            cruz(c, largo - 2.0 * fino, brazo - 2.0 * fino, tinta);
            if let Some(d) = dir {
                // El brazo que se pulsa, encendido.
                let q = c + d.v() * largo * 0.3;
                let (bw, bh) = if d.v().x == 0.0 {
                    (brazo - 2.0 * fino, largo * 0.3)
                } else {
                    (largo * 0.3, brazo - 2.0 * fino)
                };
                draw_rectangle(q.x - bw * 0.5, q.y - bh * 0.5, bw, bh, alfa(color, 0.85));
            }
        }
    }
}

fn disco(c: Vec2, r: f32, relleno: Color, filo: Color, fino: f32) {
    draw_circle(c.x, c.y, r, relleno);
    draw_circle_lines(c.x, c.y, r, fino, filo);
}

/// Una tecla: la tapa en linea, con el borde de abajo mas grueso, que es el
/// canto que se ve de una tecla de verdad.
fn tecla(c: Vec2, w: f32, h: f32, color: Color, fino: f32) {
    redondeado(c, w, h, h * 0.22, Some(alfa(color, 0.12)), color, fino);
    let (x0, x1, y) = (
        c.x - w * 0.5 + h * 0.2,
        c.x + w * 0.5 - h * 0.2,
        c.y + h * 0.5 - fino,
    );
    draw_line(x0, y, x1, y, fino, color);
}

/// Una punta de flecha llena, de "radio" `s`.
fn flecha(c: Vec2, d: Dir, s: f32, color: Color) {
    let (v, n) = (d.v(), d.v().perp());
    draw_triangle(
        c + v * s,
        c - v * s * 0.7 + n * s,
        c - v * s * 0.7 - n * s,
        color,
    );
}

fn cruz(c: Vec2, largo: f32, brazo: f32, color: Color) {
    draw_rectangle(c.x - largo * 0.5, c.y - brazo * 0.5, largo, brazo, color);
    // El palo vertical sin el cruce, que con alfa se veria doble.
    let resto = (largo - brazo) * 0.5;
    draw_rectangle(c.x - brazo * 0.5, c.y - largo * 0.5, brazo, resto, color);
    draw_rectangle(c.x - brazo * 0.5, c.y + brazo * 0.5, brazo, resto, color);
}

/// Un rectangulo de esquinas redondas centrado en `c`. Sin rectangulos
/// redondos en macroquad, se hace con un poligono: abanico para el relleno y
/// segmentos para el filo.
fn redondeado(c: Vec2, w: f32, h: f32, radio: f32, relleno: Option<Color>, filo: Color, fino: f32) {
    let r = radio.min(w * 0.5).min(h * 0.5);
    let (dx, dy) = (w * 0.5 - r, h * 0.5 - r);
    let mut p = Vec::with_capacity(24);
    // Las cuatro esquinas, en el sentido de las agujas con la y hacia abajo.
    for (k, (sx, sy)) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
        .into_iter()
        .enumerate()
    {
        let centro = c + vec2(sx * dx, sy * dy);
        for i in 0..=5 {
            let ang = (k as f32 + i as f32 / 5.0) * std::f32::consts::FRAC_PI_2;
            p.push(centro + Vec2::from_angle(ang) * r);
        }
    }
    if let Some(color) = relleno {
        for i in 0..p.len() {
            draw_triangle(c, p[i], p[(i + 1) % p.len()], color);
        }
    }
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        draw_line(a.x, a.y, b.x, b.y, fino, filo);
    }
}

/// Un nombre centrado en `c`. Va dos veces, corrido casi un pixel: Barlow es
/// fina, y a seis pixeles dentro de un boton se rompe sin ese poco de tinta.
fn rotulo(s: &str, c: Vec2, tam: f32, color: Color) {
    let w = medir_cuerpo(s, tam);
    let (x, y) = (c.x - w * 0.5, c.y + tam * 0.35);
    fuentes::texto(s, x, y, tam, Cara::Cuerpo, color);
    fuentes::texto(s, x + 0.8, y, tam, Cara::Cuerpo, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una letra que mide medio tamano por caracter: sin ventana no hay
    /// fuentes, y para medir la cuenta basta con algo lineal.
    fn medir(s: &str, tam: f32) -> f32 {
        s.chars().count() as f32 * tam * 0.5
    }

    #[test]
    fn la_plantilla_pone_cada_boton_en_su_hueco() {
        let l = rellenar(
            "{} siguiente   {} saltar",
            &[&boton(Boton::Ps(Forma::Cruz)), &boton(Boton::Tecla("ESC"))],
        );
        assert_eq!(
            l,
            vec![
                Trozo::Boton(Boton::Ps(Forma::Cruz)),
                Trozo::Texto(" siguiente   ".into()),
                Trozo::Boton(Boton::Tecla("ESC")),
                Trozo::Texto(" saltar".into()),
            ]
        );
        // Un texto en un hueco se junta con los de al lado, y un hueco sin
        // nada se queda vacio, como en `idioma::txf`.
        let l = rellenar("({} de {}) {}", &[&txt(3), &txt(5)]);
        assert_eq!(l, vec![Trozo::Texto("(3 de 5) ".into())]);
        // Una linea dentro de otra (la del mapa lo hace) se aplana.
        let dentro = rellenar("{} bailar", &[&boton(Boton::Xbox("A"))]);
        let l = rellenar("{}   {} pasear", &[&dentro, &boton(Boton::Xbox("X"))]);
        assert_eq!(l.len(), 4);
        assert_eq!(l[1], Trozo::Texto(" bailar   ".into()));
    }

    #[test]
    fn una_linea_mide_su_texto_y_sus_botones() {
        let tam = 20.0;
        let solo = ancho_con(&txt("abcd"), tam, &medir);
        assert_eq!(solo, 40.0);
        let l = rellenar("ab{}cd", &[&boton(Boton::Ps(Forma::Circulo))]);
        let disco = tam * ALTO + 2.0 * tam * MARGEN;
        assert!((ancho_con(&l, tam, &medir) - (solo + disco)).abs() < 1e-3);
        // El boton va con la letra: al doble de tamano, la linea mide el
        // doble, y por eso encoger una linea encoge tambien sus botones.
        let doble = ancho_con(&l, tam * 2.0, &medir);
        assert!((doble - 2.0 * ancho_con(&l, tam, &medir)).abs() < 1e-3);
        // Una tecla de nombre largo se ensancha para que quepa; una de una
        // letra no baja de cuadrada.
        let z = medida(Boton::Tecla("Z"), tam, &medir);
        let espacio = medida(Boton::Tecla("ESPACIO"), tam, &medir);
        assert!(z >= tam * ALTO * 0.95 - 1e-3);
        let (_, letra) = rotulo_de(Boton::Tecla("ESPACIO"), tam).unwrap();
        assert!(espacio > medir("ESPACIO", letra));
        assert!(espacio > z);
        // Y por pequena que sea la linea, lo de dentro se sigue leyendo.
        let (_, letra) = rotulo_de(Boton::Pastilla("Options"), 6.0).unwrap();
        assert_eq!(letra, LETRA_MIN);
    }
}
