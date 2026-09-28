//! La modista: la tienda del mapa donde las fichas de los paseos se cambian
//! por equipo.
//!
//! En Cuphead es la tienda de Porkrind, con sus monedas. Aqui es **una casa de
//! modas de 1900**: un escaparate con su maniqui, su toldo a rayas y el
//! catalogo impreso como un programa de mano. Lo que vende es lo que vende una
//! modista —una aguja, un abanico, un guante, un corse— y cada cosa hace algo
//! en el baile.
//!
//! Aqui vive todo lo que es tienda y no simulacion: el catalogo, los precios,
//! comprar, ponerse algo y el dibujo. Lo que cada cosa **hace** esta en
//! `vals_core::equipo`, porque eso si cambia la partida y tiene que poder
//! reproducirse.

use macroquad::prelude::*;
use vals_core::InputFrame;
use vals_core::equipo::{Amuleto, Tiro};

use crate::audio::Sfx;
use crate::draw::{Layout, fade, paspartu};
use crate::fuentes::{self, Cara};
use crate::guardado::Guardado;
use crate::mando::Mando;
use crate::paleta::{LUZ, ORO, PAPEL, TINTA, TINTA_TENUE};
use crate::protagonista;
use vals_core::{ARENA_H, ARENA_W};

/// Algo que se vende: un tiro o un amuleto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Articulo {
    Tiro(Tiro),
    Amuleto(Amuleto),
}

/// El catalogo en dos columnas: los tiros y los amuletos. Lo primero de cada
/// una es lo de serie, que no cuesta nada: es lo que hay que poder volver a
/// ponerse.
const CATALOGO: [[Articulo; 4]; 2] = [
    [
        Articulo::Tiro(Tiro::Aguja),
        Articulo::Tiro(Tiro::Abanico),
        Articulo::Tiro(Tiro::Castanuela),
        Articulo::Tiro(Tiro::Serpentina),
    ],
    [
        Articulo::Amuleto(Amuleto::Ninguno),
        Articulo::Amuleto(Amuleto::Relicario),
        Articulo::Amuleto(Amuleto::Guante),
        Articulo::Amuleto(Amuleto::Corse),
    ],
];

impl Articulo {
    /// Lo que cuesta. Suma 16, que son **todas** las fichas de los cuatro
    /// paseos: se puede comprar todo, pero solo cogiendolas todas.
    pub fn precio(self) -> u32 {
        match self {
            Articulo::Tiro(Tiro::Aguja) | Articulo::Amuleto(Amuleto::Ninguno) => 0,
            Articulo::Amuleto(Amuleto::Guante | Amuleto::Corse) => 2,
            _ => 3,
        }
    }

    /// Como se guarda: el nombre del core, que es unico entre los dos.
    fn clave(self) -> &'static str {
        match self {
            Articulo::Tiro(t) => t.nombre(),
            Articulo::Amuleto(a) => a.nombre(),
        }
    }

    fn por_clave(clave: &str) -> Option<Self> {
        CATALOGO
            .iter()
            .flatten()
            .copied()
            .find(|a| a.clave() == clave)
    }

    fn nombre(self) -> &'static str {
        match self {
            Articulo::Tiro(Tiro::Aguja) => "La aguja",
            Articulo::Tiro(Tiro::Abanico) => "El abanico",
            Articulo::Tiro(Tiro::Castanuela) => "La castanuela",
            Articulo::Tiro(Tiro::Serpentina) => "La serpentina",
            Articulo::Amuleto(Amuleto::Ninguno) => "Sin amuleto",
            Articulo::Amuleto(Amuleto::Relicario) => "El relicario",
            Articulo::Amuleto(Amuleto::Guante) => "El guante",
            Articulo::Amuleto(Amuleto::Corse) => "El corse",
        }
    }

    /// Lo que hace, en dos lineas y sin adornos: si pega la mitad, dice la
    /// mitad. Una tienda que exagera obliga a comprar para enterarse.
    fn lineas(self) -> [&'static str; 2] {
        match self {
            Articulo::Tiro(Tiro::Aguja) => ["Dos chorros rectos.", "La de siempre."],
            Articulo::Tiro(Tiro::Abanico) => ["Tres a la vez, en abanico.", "De lejos entra una."],
            Articulo::Tiro(Tiro::Castanuela) => ["Pega mas fuerte, pero", "no llega lejos."],
            Articulo::Tiro(Tiro::Serpentina) => ["Busca sola al blanco,", "y pega la mitad."],
            Articulo::Amuleto(Amuleto::Ninguno) => ["Nada colgado.", "El cuello libre."],
            Articulo::Amuleto(Amuleto::Relicario) => {
                ["Una vida mas, en el", "baile y en la calle."]
            }
            Articulo::Amuleto(Amuleto::Guante) => ["El parry se queda", "abierto casi el doble."],
            Articulo::Amuleto(Amuleto::Corse) => ["Un talle mas estrecho:", "te da menos."],
        }
    }
}

/// Las fichas que quedan en el bolsillo: las cogidas menos lo gastado.
///
/// No se guarda: se calcula. Guardar el saldo aparte seria guardar dos veces
/// lo mismo, y dos copias acaban diciendo cosas distintas.
pub fn saldo(g: &Guardado) -> u32 {
    let gastado: u32 = g
        .compras
        .iter()
        .filter_map(|c| Articulo::por_clave(c))
        .map(Articulo::precio)
        .sum();
    (g.fichas.len() as u32).saturating_sub(gastado)
}

fn lo_tiene(g: &Guardado, a: Articulo) -> bool {
    a.precio() == 0 || g.compro(a.clave())
}

fn lo_lleva(g: &Guardado, a: Articulo) -> bool {
    match a {
        Articulo::Tiro(t) => g.equipo.tiro == t,
        Articulo::Amuleto(m) => g.equipo.amuleto == m,
    }
}

/// Lo que pasa al pedir algo en el mostrador.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pedido {
    /// Ya lo llevabas.
    YaPuesto,
    /// Era tuyo y te lo pones.
    Puesto,
    /// Lo compras y te lo pones, que para eso lo has comprado.
    Comprado,
    /// Te faltan tantas fichas.
    Falta(u32),
}

fn pedir(g: &mut Guardado, a: Articulo) -> Pedido {
    if lo_lleva(g, a) {
        return Pedido::YaPuesto;
    }
    let pedido = if lo_tiene(g, a) {
        Pedido::Puesto
    } else if saldo(g) >= a.precio() {
        g.compras.push(a.clave().to_owned());
        Pedido::Comprado
    } else {
        return Pedido::Falta(a.precio() - saldo(g));
    };
    match a {
        Articulo::Tiro(t) => g.equipo.tiro = t,
        Articulo::Amuleto(m) => g.equipo.amuleto = m,
    }
    pedido
}

/// La tienda abierta: por donde va el dedo y lo ultimo que ha dicho la
/// modista.
pub struct Modista {
    col: usize,
    fila: usize,
    aviso: Option<(String, f64)>,
}

impl Modista {
    pub fn new() -> Self {
        Self {
            col: 0,
            fila: 0,
            aviso: None,
        }
    }

    /// Una pulsacion. Si se ha comprado o puesto algo lo guarda, y devuelve
    /// el sonido que toca.
    ///
    /// Teclado y mando por separado, como el menu: aqui se reacciona a la
    /// pulsacion, no a tenerla apretada, y el teclado ya da el flanco hecho.
    pub fn entrada(&mut self, g: &mut Guardado, mando: &Mando) -> Option<Sfx> {
        let tecla = |ks: &[KeyCode]| ks.iter().any(|k| is_key_pressed(*k));
        let (fila, col) = (self.fila, self.col);
        if tecla(&[KeyCode::Up, KeyCode::W]) || mando.pulsado(InputFrame::UP) {
            self.fila = self.fila.saturating_sub(1);
        }
        if tecla(&[KeyCode::Down, KeyCode::S]) || mando.pulsado(InputFrame::DOWN) {
            self.fila = (self.fila + 1).min(3);
        }
        if tecla(&[KeyCode::Left, KeyCode::A]) || mando.pulsado(InputFrame::LEFT) {
            self.col = 0;
        }
        if tecla(&[KeyCode::Right, KeyCode::D]) || mando.pulsado(InputFrame::RIGHT) {
            self.col = 1;
        }
        if (fila, col) != (self.fila, self.col) {
            return Some(Sfx::Graze);
        }
        if !(tecla(&[KeyCode::Z]) || mando.pulsado(InputFrame::JUMP)) {
            return None;
        }
        let a = CATALOGO[self.col][self.fila];
        let (dicho, sfx) = match pedir(g, a) {
            Pedido::YaPuesto => (format!("{} ya lo llevas puesto", a.nombre()), Sfx::Graze),
            Pedido::Puesto => (format!("Te pones {}", minuscula(a.nombre())), Sfx::Empezar),
            Pedido::Comprado => (format!("{}, para ti", a.nombre()), Sfx::Victoria),
            Pedido::Falta(n) => (
                format!(
                    "Te {} {n} ficha{}",
                    if n == 1 { "falta" } else { "faltan" },
                    plural(n)
                ),
                Sfx::Derrota,
            ),
        };
        if matches!(sfx, Sfx::Empezar | Sfx::Victoria) {
            g.guardar();
        }
        self.aviso = Some((dicho, get_time()));
        Some(sfx)
    }

    /// La tienda entera, en la lamina de la arena.
    pub fn dibujar(&self, g: &Guardado, l: &Layout, mando: Option<[&str; 5]>) {
        let t = get_time() as f32;
        clear_background(PAPEL);
        let o = l.to_screen(0.0, 0.0);
        draw_rectangle(o.x, o.y, l.len(ARENA_W), l.len(ARENA_H), FACHADA);
        // El filete de oro de la fachada, por dentro del marco.
        let p = |x: f32, y: f32| l.to_screen(x, y);
        let a = p(14.0, 14.0);
        draw_rectangle_lines(
            a.x,
            a.y,
            l.len(612.0),
            l.len(772.0),
            l.len(1.5),
            fade(ORO, 0.7),
        );

        rotulo(l);
        toldo(l, t);
        let elegido = CATALOGO[self.col][self.fila];
        escaparate(l, t, elegido, g);
        // Lo ultimo que ha dicho la modista, en una banda de papel sobre el
        // cristal: dos segundos, y medio mas apagandose.
        if let Some((texto, cuando)) = &self.aviso {
            let k = 1.0 - (((get_time() - cuando) as f32 - 2.0) / 0.5).clamp(0.0, 1.0);
            if k > 0.0 {
                let tam = l.len(17.0);
                let w = fuentes::medir(texto, tam, Cara::Cuerpo).width + l.len(28.0);
                let c = p(320.0, 214.0);
                let (x, y, h) = (c.x - w * 0.5, c.y - l.len(15.0), l.len(26.0));
                draw_rectangle(x, y, w, h, fade(PAPEL, k));
                draw_rectangle_lines(x, y, w, h, l.len(2.0), fade(TINTA, k));
                fuentes::centrado(
                    texto,
                    c.x,
                    c.y + l.len(4.0),
                    tam,
                    Cara::Cuerpo,
                    fade(TINTA, k),
                );
            }
        }

        // El catalogo, a dos columnas.
        for (c, titulo) in ["LOS TIROS", "LOS AMULETOS"].iter().enumerate() {
            let x = 24.0 + c as f32 * 304.0;
            let s = p(x + 144.0, 372.0);
            fuentes::centrado(titulo, s.x, s.y, l.len(20.0), Cara::Titulo, ORO);
            let (i, d) = (p(x + 20.0, 380.0), p(x + 268.0, 380.0));
            draw_line(i.x, i.y, d.x, d.y, l.len(1.0), fade(ORO, 0.6));
            for (f, art) in CATALOGO[c].iter().enumerate() {
                let elegida = (c, f) == (self.col, self.fila);
                etiqueta(l, x, 392.0 + f as f32 * 88.0, *art, g, elegida, t);
            }
        }

        // El pie: el bolsillo a la izquierda y los botones a la derecha.
        let y = 766.0;
        let b = p(40.0, y - 6.0);
        ficha(b, l.len(11.0), t * 2.0);
        let n = saldo(g);
        let bolsillo = format!("{n} ficha{} en el bolsillo", plural(n));
        let s = p(58.0, y);
        fuentes::texto(&bolsillo, s.x, s.y, l.len(19.0), Cara::Cuerpo, PAPEL);
        let (ok, atras) = match mando {
            Some(b) => (b[4], b[2]),
            None => ("Z", "ESC"),
        };
        let ayuda = format!("{ok} comprar o ponerse    {atras} salir");
        let s = p(616.0, y);
        fuentes::derecha(
            &ayuda,
            s.x,
            s.y,
            l.len(16.0),
            Cara::Cuerpo,
            fade(PAPEL, 0.8),
        );

        paspartu(l);
    }
}

/// El verde botella de la fachada: una tienda de 1900 se pintaba oscura para
/// que el escaparate encendido fuese lo que se viera desde la acera.
const FACHADA: Color = color_u8!(30, 54, 46, 255);
const FACHADA_LUZ: Color = color_u8!(46, 78, 64, 255);
/// El carmin de las rayas del toldo y del sello de "puesto".
const CARMIN: Color = color_u8!(176, 44, 58, 255);
const CRISTAL: Color = color_u8!(40, 28, 26, 255);

fn plural(n: u32) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn minuscula(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|p| p.to_lowercase().chain(c).collect())
        .unwrap_or_default()
}

/// El rotulo: una tabla negra con letras de oro, como las de las tiendas de
/// los bulevares.
fn rotulo(l: &Layout) {
    let p = |x: f32, y: f32| l.to_screen(x, y);
    let (a, w, h) = (p(40.0, 30.0), l.len(560.0), l.len(92.0));
    draw_rectangle(
        a.x - l.len(4.0),
        a.y - l.len(4.0),
        w + l.len(8.0),
        h + l.len(8.0),
        ORO,
    );
    draw_rectangle(a.x, a.y, w, h, TINTA);
    draw_rectangle_lines(
        a.x + l.len(6.0),
        a.y + l.len(6.0),
        w - l.len(12.0),
        h - l.len(12.0),
        l.len(1.0),
        fade(ORO, 0.6),
    );
    let c = p(320.0, 86.0);
    for (dx, color) in [(l.len(2.0), CARMIN), (0.0, ORO)] {
        fuentes::centrado(
            "LA MODISTA",
            c.x + dx,
            c.y + dx,
            l.len(56.0),
            Cara::Titulo,
            color,
        );
    }
    let c = p(320.0, 110.0);
    fuentes::centrado(
        "ALTA COSTURA  -  SE ADMITEN FICHAS DE BAILE",
        c.x,
        c.y,
        l.len(13.0),
        Cara::Cuerpo,
        fade(ORO, 0.85),
    );
}

/// El toldo a rayas, con el borde festoneado. Se mece un pelo, que un toldo
/// quieto parece pintado en la pared.
fn toldo(l: &Layout, t: f32) {
    let (y0, y1) = (134.0, 164.0);
    let rayas = 14;
    let ancho = 580.0 / rayas as f32;
    let brisa = (t * 1.3).sin() * 1.5;
    let a = l.to_screen(26.0, y0 - 4.0);
    draw_rectangle(a.x, a.y, l.len(588.0), l.len(y1 - y0 + 12.0), TINTA);
    for i in 0..rayas {
        let x = 30.0 + i as f32 * ancho;
        let color = if i % 2 == 0 { PAPEL } else { CARMIN };
        let s = l.to_screen(x, y0);
        draw_rectangle(s.x, s.y, l.len(ancho), l.len(y1 - y0), color);
        // El festón: medio circulo por raya, colgando.
        let c = l.to_screen(x + ancho * 0.5, y1 + brisa * 0.3);
        draw_circle(c.x, c.y, l.len(ancho * 0.5 + 1.8), TINTA);
        draw_circle(c.x, c.y, l.len(ancho * 0.5), color);
    }
    let s = l.to_screen(30.0, y0);
    draw_line(s.x, s.y, s.x + l.len(580.0), s.y, l.len(3.0), TINTA);
}

/// El escaparate: el cristal encendido, el maniqui con el vestido de ella y,
/// al lado, en su peana, lo que tienes senalado en el catalogo, en grande.
/// Encima, el aviso de la modista.
fn escaparate(l: &Layout, t: f32, elegido: Articulo, g: &Guardado) {
    let p = |x: f32, y: f32| l.to_screen(x, y);
    let (a, w, h) = (p(60.0, 186.0), l.len(520.0), l.len(160.0));
    draw_rectangle(
        a.x - l.len(6.0),
        a.y - l.len(6.0),
        w + l.len(12.0),
        h + l.len(12.0),
        TINTA,
    );
    draw_rectangle(a.x, a.y, w, h, CRISTAL);
    // La luz de dentro: un foco calido detras del maniqui.
    let f = p(190.0, 262.0);
    for (r, k) in [(72.0, 0.08), (52.0, 0.10), (34.0, 0.12)] {
        draw_circle(f.x, f.y, l.len(r), fade(LUZ, k));
    }
    // El reflejo del cristal: dos rayas en diagonal, que es lo que dice que
    // hay un cristal delante.
    for (x, ancho) in [(420.0, 26.0), (460.0, 10.0)] {
        let (b, c) = (p(x, 186.0), p(x - 70.0, 346.0));
        draw_line(b.x, b.y, c.x, c.y, l.len(ancho), fade(WHITE, 0.05));
    }
    // El suelo del escaparate.
    let s = p(60.0, 322.0);
    draw_rectangle(s.x, s.y, w, l.len(24.0), FACHADA_LUZ);
    draw_line(s.x, s.y, s.x + w, s.y, l.len(2.0), TINTA);

    maniqui(l, p(190.0, 322.0), t);

    // La peana con lo elegido.
    let pe = p(390.0, 322.0);
    let (pw, ph) = (l.len(90.0), l.len(18.0));
    draw_rectangle(
        pe.x - pw * 0.5 - 2.0,
        pe.y - ph - 2.0,
        pw + 4.0,
        ph + 4.0,
        TINTA,
    );
    draw_rectangle(pe.x - pw * 0.5, pe.y - ph, pw, ph, ORO);
    let flota = (t * 2.0).sin() * l.len(3.0);
    icono(elegido, p(390.0, 262.0) + vec2(0.0, flota), l.len(34.0), t);
    let precio = elegido.precio();
    let cartel = if lo_lleva(g, elegido) {
        "puesto".to_owned()
    } else if lo_tiene(g, elegido) {
        "es tuyo".to_owned()
    } else {
        format!("{precio} ficha{}", plural(precio))
    };
    let c = p(390.0, 318.0);
    fuentes::centrado(&cartel, c.x, c.y, l.len(14.0), Cara::Cuerpo, TINTA);
}

/// Un maniqui de modista con el vestido de ella y su lazo: el escaparate
/// vende lo que ella lleva.
fn maniqui(l: &Layout, pie: Vec2, t: f32) {
    let k = l.len(1.0);
    let q = |x: f32, y: f32| pie + vec2(x, y) * k;
    let g = (2.2 * k).max(1.5);
    // El pie de hierro y el palo.
    let (a, b) = (q(-22.0, 0.0), q(22.0, 0.0));
    draw_line(a.x, a.y, b.x, b.y, 4.0 * k, TINTA);
    let (a, b) = (q(0.0, 0.0), q(0.0, -40.0));
    draw_line(a.x, a.y, b.x, b.y, 3.0 * k, TINTA);
    // La falda: un trapecio que se mece un poco, como si alguien la hubiera
    // rozado al pasar.
    let vuelo = (t * 1.1).sin() * 2.0;
    let falda = [
        q(-12.0, -84.0),
        q(12.0, -84.0),
        q(34.0 + vuelo, -34.0),
        q(-34.0 + vuelo, -34.0),
    ];
    for (color, grueso) in [(TINTA, g), (protagonista::VESTIDO, 0.0)] {
        let d = |v: Vec2, sx: f32, sy: f32| v + vec2(sx, sy) * grueso;
        draw_triangle(
            d(falda[0], -1.0, -1.0),
            d(falda[1], 1.0, -1.0),
            d(falda[2], 1.0, 1.0),
            color,
        );
        draw_triangle(
            d(falda[0], -1.0, -1.0),
            d(falda[2], 1.0, 1.0),
            d(falda[3], -1.0, 1.0),
            color,
        );
    }
    // Los pliegues.
    for x in [-14.0, 0.0, 14.0] {
        let (a, b) = (q(x * 0.4, -80.0), q(x + vuelo, -36.0));
        draw_line(a.x, a.y, b.x, b.y, 1.2 * k, fade(TINTA, 0.35));
    }
    // El busto: un ovalo de lino con la cintura del vestido.
    let c = q(0.0, -100.0);
    draw_ellipse(c.x, c.y, 17.0 * k + g, 22.0 * k + g, 0.0, TINTA);
    draw_ellipse(c.x, c.y, 17.0 * k, 22.0 * k, 0.0, protagonista::VESTIDO);
    let (a, b) = (q(-13.0, -86.0), q(13.0, -86.0));
    draw_line(a.x, a.y, b.x, b.y, 4.0 * k, protagonista::LAZO);
    // El cuello torneado y el remate.
    let c = q(0.0, -126.0);
    draw_circle(c.x, c.y, 6.0 * k + g, TINTA);
    draw_circle(c.x, c.y, 6.0 * k, color_u8!(196, 168, 128, 255));
    // Y el lazo, que es por lo que se la reconoce.
    let c = q(0.0, -86.0);
    for lado in [-1.0, 1.0] {
        let e = c + vec2(lado * 7.0, 0.0) * k;
        draw_ellipse(e.x, e.y, 7.0 * k, 4.5 * k, lado * 20.0, TINTA);
        draw_ellipse(e.x, e.y, 5.5 * k, 3.2 * k, lado * 20.0, protagonista::LAZO);
    }
    draw_circle(c.x, c.y, 2.8 * k, TINTA);
}

/// Una etiqueta del catalogo: papel con tinta, el dibujo, el nombre, lo que
/// hace y el precio o su sello.
fn etiqueta(l: &Layout, x: f32, y: f32, a: Articulo, g: &Guardado, elegida: bool, t: f32) {
    let (w, h) = (288.0, 78.0);
    let tuyo = lo_tiene(g, a);
    let puesto = lo_lleva(g, a);
    let alcanza = tuyo || saldo(g) >= a.precio();
    let s = l.to_screen(x, y);
    let (sw, sh) = (l.len(w), l.len(h));
    // La elegida se levanta un poco y lleva el filete de oro latiendo.
    let alza = if elegida { l.len(3.0) } else { 0.0 };
    let s = s - vec2(0.0, alza);
    if elegida {
        draw_rectangle(s.x + l.len(4.0), s.y + l.len(5.0), sw, sh, fade(TINTA, 0.5));
    }
    // Lo que no alcanza se imprime en un papel mas apagado, pero se lee igual:
    // saber lo que hace es lo que dice si merece la pena ir a por fichas.
    let papel = if alcanza {
        PAPEL
    } else {
        color_u8!(200, 186, 158, 255)
    };
    draw_rectangle(s.x, s.y, sw, sh, papel);
    draw_rectangle_lines(s.x, s.y, sw, sh, l.len(2.5), TINTA);
    if elegida {
        let pulso = 0.6 + 0.4 * (t * 5.0).sin();
        let m = l.len(5.0);
        draw_rectangle_lines(
            s.x - m,
            s.y - m,
            sw + m * 2.0,
            sh + m * 2.0,
            l.len(2.5),
            fade(ORO, pulso),
        );
    }

    // El dibujo, en su medallon.
    let c = s + vec2(l.len(38.0), sh * 0.5);
    draw_circle(c.x, c.y, l.len(27.0), TINTA);
    draw_circle(
        c.x,
        c.y,
        l.len(25.0),
        if tuyo { FACHADA_LUZ } else { FACHADA },
    );
    icono(a, c, l.len(18.0), if elegida { t } else { 0.0 });

    // El nombre y lo que hace.
    let tx = s.x + l.len(76.0);
    let tinta = if alcanza { TINTA } else { CARMIN };
    fuentes::texto(
        a.nombre(),
        tx,
        s.y + l.len(24.0),
        l.len(19.0),
        Cara::Cuerpo,
        TINTA,
    );
    for (i, linea) in a.lineas().iter().enumerate() {
        let y = s.y + l.len(44.0 + 16.0 * i as f32);
        fuentes::texto(linea, tx, y, l.len(14.0), Cara::Cuerpo, TINTA_TENUE);
    }

    // Arriba a la derecha: el sello si es tuyo, el precio si no.
    let d = s + vec2(sw - l.len(10.0), l.len(22.0));
    if puesto {
        sello(l, d, "PUESTO", CARMIN);
    } else if tuyo {
        sello(l, d, "TUYO", TINTA_TENUE);
    } else {
        let n = a.precio().to_string();
        fuentes::derecha(&n, d.x, d.y + l.len(2.0), l.len(22.0), Cara::Cuerpo, tinta);
        let ancho = fuentes::medir(&n, l.len(22.0), Cara::Cuerpo).width;
        ficha(d - vec2(ancho + l.len(12.0), l.len(6.0)), l.len(9.0), 0.0);
    }
}

/// Un sello de tampon: el texto en un recuadro del mismo color.
fn sello(l: &Layout, derecha: Vec2, texto: &str, color: Color) {
    let tam = l.len(13.0);
    let m = fuentes::medir(texto, tam, Cara::Cuerpo);
    let (w, h) = (m.width + l.len(10.0), l.len(18.0));
    let (x, y) = (derecha.x - w, derecha.y - l.len(13.0));
    draw_rectangle_lines(x, y, w, h, l.len(1.8), color);
    fuentes::texto(
        texto,
        x + l.len(5.0),
        y + l.len(13.5),
        tam,
        Cara::Cuerpo,
        color,
    );
}

/// El dibujo de cada cosa, en un circulo de radio `r` alrededor de `c`. `t`
/// lo anima un poco cuando esta senalado.
fn icono(a: Articulo, c: Vec2, r: f32, t: f32) {
    let g = (r * 0.12).max(1.5);
    let q = |x: f32, y: f32| c + vec2(x, y) * r;
    let linea = |a: Vec2, b: Vec2, w: f32, color: Color| {
        draw_line(a.x, a.y, b.x, b.y, w + g * 2.0, TINTA);
        draw_line(a.x, a.y, b.x, b.y, w, color);
    };
    match a {
        Articulo::Tiro(Tiro::Aguja) => {
            // La aguja en diagonal, con su ojo, y un hilo rojo que se riza.
            linea(q(-0.8, 0.8), q(0.8, -0.8), r * 0.12, PAPEL);
            let ojo = q(0.62, -0.62);
            draw_circle_lines(ojo.x, ojo.y, r * 0.09, g * 0.6, TINTA);
            let mut prev = ojo;
            for i in 1..=10 {
                let f = i as f32 / 10.0;
                let pt = q(
                    0.6 - f * 1.2,
                    -0.6 + (f * 6.0 + t * 3.0).sin() * 0.18 + f * 0.1,
                );
                draw_line(prev.x, prev.y, pt.x, pt.y, g, protagonista::LAZO);
                prev = pt;
            }
        }
        Articulo::Tiro(Tiro::Abanico) => {
            // Un abanico abierto: varillas alternas y el clavo de oro.
            let eje = q(0.0, 0.7);
            let n = 7;
            let abre = 0.1 * (t * 3.0).sin();
            for i in 0..n {
                let a0 = std::f32::consts::PI * (1.1 + 0.8 * i as f32 / n as f32) - abre;
                let a1 = std::f32::consts::PI * (1.1 + 0.8 * (i + 1) as f32 / n as f32) + abre;
                let (p0, p1) = (
                    eje + vec2(a0.cos(), a0.sin()) * r * 1.5,
                    eje + vec2(a1.cos(), a1.sin()) * r * 1.5,
                );
                let color = if i % 2 == 0 { CARMIN } else { PAPEL };
                draw_triangle(eje, p0, p1, color);
                draw_line(eje.x, eje.y, p0.x, p0.y, g * 0.7, TINTA);
            }
            draw_circle(eje.x, eje.y, g * 1.6, ORO);
        }
        Articulo::Tiro(Tiro::Castanuela) => {
            // Dos conchas de madera con su cordon.
            let golpe = (t * 8.0).sin().abs() * 0.12;
            for (dx, dy) in [(-0.28 - golpe, 0.1), (0.28 + golpe, -0.05)] {
                let e = q(dx, dy);
                draw_ellipse(e.x, e.y, r * 0.48 + g, r * 0.58 + g, 0.0, TINTA);
                draw_ellipse(
                    e.x,
                    e.y,
                    r * 0.48,
                    r * 0.58,
                    0.0,
                    color_u8!(132, 70, 40, 255),
                );
                draw_circle(e.x - r * 0.1, e.y - r * 0.15, r * 0.1, fade(WHITE, 0.35));
            }
            linea(q(-0.2, -0.55), q(0.2, -0.6), g * 0.5, protagonista::LAZO);
        }
        Articulo::Tiro(Tiro::Serpentina) => {
            // Una cinta de papel rizandose en dos colores.
            let mut prev = q(-0.9, 0.6);
            for i in 1..=18 {
                let f = i as f32 / 18.0;
                let pt = q(
                    -0.9 + f * 1.8,
                    0.6 - f * 1.2 + (f * 9.0 + t * 4.0).sin() * 0.28,
                );
                let color = if (i / 3) % 2 == 0 { ORO } else { CARMIN };
                draw_line(prev.x, prev.y, pt.x, pt.y, r * 0.2 + g * 2.0, TINTA);
                draw_line(prev.x, prev.y, pt.x, pt.y, r * 0.2, color);
                prev = pt;
            }
        }
        Articulo::Amuleto(Amuleto::Ninguno) => {
            // El cuello vacio: una cadena sin nada colgando.
            for i in 0..9 {
                let a = std::f32::consts::PI * (0.15 + 0.7 * i as f32 / 8.0);
                let e = q(a.cos() * 0.7, a.sin() * 0.7 - 0.3);
                draw_circle_lines(e.x, e.y, r * 0.08, g * 0.6, fade(PAPEL, 0.7));
            }
        }
        Articulo::Amuleto(Amuleto::Relicario) => {
            // Un corazon de oro colgando de su cadena.
            let c = q(0.0, 0.1);
            let lat = 1.0 + 0.06 * (t * 6.0).sin();
            let rr = r * 0.36 * lat;
            for (color, extra) in [(TINTA, g), (ORO, 0.0)] {
                for dx in [-1.0, 1.0] {
                    draw_circle(c.x + dx * rr * 0.75, c.y - rr * 0.3, rr + extra, color);
                }
                draw_triangle(
                    c + vec2(-rr * 1.72 - extra, -rr * 0.05),
                    c + vec2(rr * 1.72 + extra, -rr * 0.05),
                    c + vec2(0.0, rr * 1.8 + extra),
                    color,
                );
            }
            draw_circle(c.x, c.y + rr * 0.2, rr * 0.28, CARMIN);
            linea(q(0.0, -0.4), q(0.0, -0.9), g * 0.4, ORO);
        }
        Articulo::Amuleto(Amuleto::Guante) => {
            // Un guante largo de noche: la mano y la cana hasta el codo.
            let color = PAPEL;
            linea(q(0.05, 0.9), q(0.0, 0.05), r * 0.46, color);
            let palma = q(0.0, -0.1);
            draw_ellipse(palma.x, palma.y, r * 0.34 + g, r * 0.3 + g, 0.0, TINTA);
            for (i, dx) in [-0.24_f32, -0.08, 0.08, 0.24].iter().enumerate() {
                let largo = if i == 1 || i == 2 { 0.62 } else { 0.5 };
                linea(q(*dx, -0.2), q(*dx * 1.2, -0.2 - largo), r * 0.12, color);
            }
            linea(q(0.28, 0.0), q(0.6, -0.25), r * 0.13, color);
            draw_ellipse(palma.x, palma.y, r * 0.34, r * 0.3, 0.0, color);
            linea(q(-0.2, 0.55), q(0.2, 0.55), g * 0.4, ORO);
        }
        Articulo::Amuleto(Amuleto::Corse) => {
            // Un corse: reloj de arena con sus ballenas y el cordon cruzado.
            let cintura = 0.3 + 0.03 * (t * 4.0).sin();
            let contorno = [
                q(-0.55, -0.75),
                q(0.55, -0.75),
                q(cintura, 0.0),
                q(0.5, 0.75),
                q(-0.5, 0.75),
                q(-cintura, 0.0),
            ];
            for (color, extra) in [(TINTA, g), (CARMIN, 0.0)] {
                let d = |v: Vec2| c + (v - c) * (1.0 + extra / r);
                draw_triangle(d(contorno[0]), d(contorno[1]), d(contorno[2]), color);
                draw_triangle(d(contorno[0]), d(contorno[2]), d(contorno[5]), color);
                draw_triangle(d(contorno[5]), d(contorno[2]), d(contorno[3]), color);
                draw_triangle(d(contorno[5]), d(contorno[3]), d(contorno[4]), color);
            }
            for x in [-0.3, 0.3] {
                let (a, b) = (q(x, -0.7), q(x * 0.6, 0.7));
                draw_line(a.x, a.y, b.x, b.y, g * 0.5, fade(TINTA, 0.5));
            }
            for i in 0..4 {
                let y = -0.55 + i as f32 * 0.36;
                let w = 0.14;
                let (a, b) = (q(-w, y), q(w, y + 0.18));
                draw_line(a.x, a.y, b.x, b.y, g * 0.5, PAPEL);
                let (a, b) = (q(w, y), q(-w, y + 0.18));
                draw_line(a.x, a.y, b.x, b.y, g * 0.5, PAPEL);
            }
        }
    }
}

/// Una ficha de baile: un disco de laton con su canto y una estrella
/// troquelada. `giro` la hace dar vueltas sobre su eje, que en plano es
/// estrecharla con el coseno, el mismo truco que el giro del vals.
///
/// Es la misma en la calle, en el mapa y en la tienda: una moneda que cambia
/// de dibujo segun donde se mire deja de ser la misma moneda.
pub(crate) fn ficha(c: Vec2, r: f32, giro: f32) {
    let ancho = giro.cos().abs().max(0.18);
    let g = (r * 0.18).max(1.2);
    draw_ellipse(c.x, c.y, r * ancho + g, r + g, 0.0, TINTA);
    draw_ellipse(c.x, c.y, r * ancho, r, 0.0, ORO);
    draw_ellipse_lines(
        c.x,
        c.y,
        r * 0.72 * ancho,
        r * 0.72,
        0.0,
        g * 0.6,
        color_u8!(176, 118, 44, 255),
    );
    // La estrella troquelada, que se estrecha con el giro como todo lo demas.
    let e = r * 0.36;
    for (dx, dy) in [(1.0, 0.0), (0.0, 1.0)] {
        let (a, b) = (
            c + vec2(-dx * e * ancho, -dy * e),
            c + vec2(dx * e * ancho, dy * e),
        );
        draw_line(a.x, a.y, b.x, b.y, g * 0.9, color_u8!(150, 96, 34, 255));
    }
    // El brillo: una raya blanca que cruza con el giro.
    let brillo = c + vec2(-r * 0.35 * ancho, -r * 0.4);
    draw_circle(brillo.x, brillo.y, r * 0.14, fade(WHITE, 0.6 * ancho));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn con_fichas(n: usize) -> Guardado {
        let mut g = Guardado::default();
        for i in 0..n {
            g.marcar_ficha("El paseo de Viena", i);
        }
        g
    }

    #[test]
    fn el_catalogo_entero_cuesta_las_dieciseis_fichas() {
        let total: u32 = CATALOGO.iter().flatten().map(|a| a.precio()).sum();
        assert_eq!(total, 16, "tantas como hay en los cuatro paseos");
    }

    #[test]
    fn comprar_descuenta_se_pone_y_se_guarda() {
        let mut g = con_fichas(4);
        let abanico = Articulo::Tiro(Tiro::Abanico);
        assert_eq!(saldo(&g), 4);
        assert_eq!(pedir(&mut g, abanico), Pedido::Comprado);
        assert_eq!(saldo(&g), 1, "cuesta tres");
        assert_eq!(g.equipo.tiro, Tiro::Abanico, "comprado es puesto");
        // Y sobrevive a guardar y volver a leer.
        let leido = Guardado::desde_texto(&g.a_texto());
        assert_eq!(saldo(&leido), 1);
        assert!(lo_tiene(&leido, abanico));
        assert_eq!(leido.equipo.tiro, Tiro::Abanico);
    }

    #[test]
    fn sin_fichas_no_se_compra_y_lo_tuyo_no_se_vuelve_a_pagar() {
        let mut g = con_fichas(2);
        let relicario = Articulo::Amuleto(Amuleto::Relicario);
        assert_eq!(pedir(&mut g, relicario), Pedido::Falta(1));
        assert!(g.compras.is_empty() && g.equipo.amuleto == Amuleto::Ninguno);

        let guante = Articulo::Amuleto(Amuleto::Guante);
        assert_eq!(pedir(&mut g, guante), Pedido::Comprado);
        assert_eq!(saldo(&g), 0);
        // Quitarselo es gratis, y volver a ponerselo tambien.
        assert_eq!(
            pedir(&mut g, Articulo::Amuleto(Amuleto::Ninguno)),
            Pedido::Puesto
        );
        assert_eq!(pedir(&mut g, guante), Pedido::Puesto);
        assert_eq!(pedir(&mut g, guante), Pedido::YaPuesto);
        assert_eq!(g.compras.len(), 1, "no se cobra dos veces");
    }

    #[test]
    fn lo_de_serie_es_tuyo_desde_el_principio() {
        let g = Guardado::default();
        assert!(lo_lleva(&g, Articulo::Tiro(Tiro::Aguja)));
        assert!(lo_lleva(&g, Articulo::Amuleto(Amuleto::Ninguno)));
        assert_eq!(saldo(&g), 0);
    }
}
