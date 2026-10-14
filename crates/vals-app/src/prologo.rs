//! El prologo: la historia del juego, contada en cinco laminas.
//!
//! El juego no tenia historia: un menu, un mapa y cuatro jefes. Funcionaba,
//! pero nadie sabia por que una caja de musica le dispara a una bailarina. Esto
//! lo cuenta en medio minuto, como un programa de mano ilustrado: hacia 1900, un
//! salon de baile que viaja de ciudad en ciudad; a medianoche sus cosas cobran
//! vida y se escapan con la musica, cada una a su ciudad; y la bailarina va a
//! por ella, calle a calle (los paseos) y baile a baile (los jefes).
//!
//! **Sale solo la primera vez**, entre el menu y la pista, y se guarda que ya
//! se vio. Desde el menu se puede volver a ver. Se salta entero o lamina a
//! lamina: una historia que no se puede saltar se odia a la segunda.
//!
//! Va con el reloj de pared y no con los ticks, como las cartelas: es
//! presentacion y tiene que durar lo mismo a 60 que a 144 Hz.
//!
//! Y nada de dibujo propio que ya exista: la protagonista es la de
//! `protagonista`, los jefes los de `jefes` y las ciudades los decorados de
//! `escenarios`. Lo unico nuevo es como se colocan en cada lamina.

use macroquad::prelude::*;
use vals_core::paseo::{VISTA_H, VISTA_W};
use vals_core::player::PLAYER_SPEED;
use vals_core::{Player, World};

use crate::acciones::Nombres;
use crate::draw::{self, Layout, fade};
use crate::escenarios::{circulo, linea, rect, tri};
use crate::fuentes::{self, Cara};
use crate::idioma::{Texto, nombre, tx};
use crate::music::Tema;
use crate::paleta::{self, LUZ, ORO, PAPEL, PARED, TARIMA, TINTA, VETA};
use crate::protagonista::{self, Expresion};
use crate::{escenarios, jefes, modista, salon, skeleton};

// ---------------------------------------------------------------------------
// Los textos: estan en `idioma`, y aqui en que orden salen.
// ---------------------------------------------------------------------------

/// Cada lamina: el titulo y dos lineas, como un intertitulo de cine mudo.
const LAMINAS: [[Texto; 3]; 5] = [
    [Texto::Lamina1, Texto::Lamina1A, Texto::Lamina1B],
    [Texto::Lamina2, Texto::Lamina2A, Texto::Lamina2B],
    [Texto::Lamina3, Texto::Lamina3A, Texto::Lamina3B],
    [Texto::Lamina4, Texto::Lamina4A, Texto::Lamina4B],
    [Texto::Lamina5, Texto::Lamina5A, Texto::Lamina5B],
];

/// Adonde se fue cada baile, en el orden de `paleta::TINTAS`. Se traducen con
/// `idioma::nombre`, como los bailes.
const CIUDADES: [&str; 4] = ["VIENA", "BUENOS AIRES", "CHICAGO", "PARIS"];
/// Los numeros del programa, en el mismo orden.
const NUMEROS: [&str; 4] = ["El Vals", "El Tango", "El Charleston", "El Cancan"];
const ROMANOS: [&str; 4] = ["I.", "II.", "III.", "IV."];

/// Una linea por jefe, para su cartela de entrada.
const LEMAS_BAILE: [Texto; 4] = [
    Texto::LemaVals,
    Texto::LemaTango,
    Texto::LemaCharleston,
    Texto::LemaCancan,
];

/// Una linea por paseo, por el jefe al que lleva.
const LEMAS_PASEO: [Texto; 4] = [
    Texto::LemaViena,
    Texto::LemaArrabal,
    Texto::LemaChicago,
    Texto::LemaMontmartre,
];

/// El lema de un jefe. Uno que no exista se queda sin lema: es adorno.
pub fn lema_del_baile(jefe: usize) -> &'static str {
    LEMAS_BAILE.get(jefe).map_or("", |t| tx(*t))
}

/// El lema del paseo que va antes del jefe `jefe`.
pub fn lema_del_paseo(jefe: usize) -> &'static str {
    LEMAS_PASEO.get(jefe).map_or("", |t| tx(*t))
}

// ---------------------------------------------------------------------------
// El reloj
// ---------------------------------------------------------------------------

/// Lo que dura cada lamina si no se toca nada: lo justo para leer dos lineas
/// y ver que pasa. Cinco laminas, medio minuto escaso.
const DURA: f32 = 5.5;
/// Lo que tarda en poder pasarse. Se entra pulsando confirmar, y ese mismo
/// boton no puede pasar ya la primera lamina (lo mismo que en las cartelas).
const GRACIA: f32 = 0.6;
/// El fundido a negro de entrada y salida de cada lamina.
const FUNDE: f32 = 0.45;

/// El prologo en marcha.
pub struct Prologo {
    lamina: usize,
    /// Segundos dentro de la lamina.
    t: f32,
    /// La bailarina, quieta: la pose y la cara salen de un `Player`, y aqui se
    /// le cambia solo la velocidad para que ande.
    ella: Player,
}

impl Prologo {
    pub fn nuevo() -> Self {
        Self {
            lamina: 0,
            t: 0.0,
            ella: World::new(0).player,
        }
    }

    /// Un frame. Al acabarse la lamina se pasa sola a la siguiente.
    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        if self.t >= DURA {
            self.siguiente();
        }
    }

    /// Confirmar: la siguiente lamina, pasada la gracia.
    pub fn avanzar(&mut self) {
        if self.t >= GRACIA {
            self.siguiente();
        }
    }

    /// Saltarlo entero. Sin gracia: saltar no se pulsa sin querer al entrar.
    pub fn saltar(&mut self) {
        self.lamina = LAMINAS.len();
    }

    pub fn acabado(&self) -> bool {
        self.lamina >= LAMINAS.len()
    }

    fn siguiente(&mut self) {
        self.lamina = (self.lamina + 1).min(LAMINAS.len());
        self.t = 0.0;
    }

    /// La lamina en curso, en la lamina apaisada del paseo. `teclas` son los
    /// nombres de lo que se este usando (ver `acciones::Nombres`).
    pub fn dibujar(&self, l: &Layout, teclas: &Nombres) {
        clear_background(PAPEL);
        let Some(textos) = LAMINAS.get(self.lamina) else {
            return;
        };
        let s = self.t;
        // Un zoom lento hacia el centro: la camara de un cine mudo no esta
        // quieta, y una lamina quieta parece una pantalla de carga.
        let z = 1.0 + 0.045 * (s / DURA);
        let lz = l
            .sacudido(vec2(VISTA_W, VISTA_H) * 0.5 * (1.0 - z))
            .escalado(z);
        match self.lamina {
            0 => el_salon(&lz, s, &self.ella),
            1 => medianoche(&lz, s),
            2 => ciudades(&lz, s),
            3 => ella_sola(&lz, s, &self.ella),
            _ => programa(&lz, s),
        }
        intertitulo(l, textos, s);
        let negro = 1.0 - (s / FUNDE).min((DURA - s) / FUNDE).clamp(0.0, 1.0);
        rect(l, 0.0, 0.0, VISTA_W, VISTA_H, fade(TINTA, negro));
        draw::paspartu_de(l, vec2(VISTA_W, VISTA_H));
        self.ayuda(l, teclas);
    }

    /// Debajo de la lamina, en el papel: por donde vamos y como se pasa.
    fn ayuda(&self, l: &Layout, teclas: &Nombres) {
        let (ok, esc) = (teclas.aceptar, teclas.atras);
        let o = l.to_screen(0.0, 0.0);
        let y = o.y + l.len(VISTA_H) + 26.0;
        let c = paleta::TINTA_TENUE;
        let n = format!("{} / {}", self.lamina + 1, LAMINAS.len());
        fuentes::texto(&n, o.x, y, 16.0, Cara::Cuerpo, c);
        let (siguiente, saltar) = (tx(Texto::Siguiente), tx(Texto::SaltarHistoria));
        let ayuda = format!("{ok}  {siguiente}      {esc}  {saltar}");
        fuentes::derecha(&ayuda, o.x + l.len(VISTA_W), y, 16.0, Cara::Cuerpo, c);
    }
}

/// La invitacion del menu a volver a verlo, arriba a la izquierda de la
/// arena: discreta, que lo que se viene a hacer al menu es jugar.
pub fn en_el_menu(l: &Layout, teclas: &Nombres) {
    let tecla = teclas.historia;
    let o = l.to_screen(0.0, 0.0);
    let u = l.scale().clamp(0.55, 1.2);
    let texto = format!("{tecla}   {}", tx(Texto::LaHistoria));
    fuentes::texto(
        &texto,
        o.x + 22.0 * u,
        o.y + 64.0 * u,
        17.0 * u,
        Cara::Cuerpo,
        draw::TEXT_DIM,
    );
}

// ---------------------------------------------------------------------------
// Las laminas. Todas en unidades de la lamina apaisada (960 x 540); la franja
// de abajo es del intertitulo.
// ---------------------------------------------------------------------------

/// Donde empieza el intertitulo.
const PIE: f32 = 430.0;

/// La bailarina con los pies en `pies` y `talla` veces su esqueleto.
fn bailarina(l: &Layout, ella: &Player, pies: Vec2, talla: f32, t: f32, cara: Option<Expresion>) {
    let pose = skeleton::pose(ella, t, false, 1.0);
    let ls = l.escalado(talla);
    let bajo = pose.joints.iter().map(|q| q.y).fold(f32::MIN, f32::max);
    let centro = l.to_screen(pies.x, pies.y) - vec2(0.0, bajo * ls.scale());
    let mut gesto = protagonista::Gesto::de(ella, t);
    if let Some(c) = cara {
        gesto.cara = c;
    }
    protagonista::dibujar(&pose, &gesto, centro, &ls, 1.0, false);
}

/// La tarima de una lamina, desde `y` hasta abajo, con sus vetas.
fn tarima(l: &Layout, y: f32) {
    rect(l, 0.0, y, VISTA_W, VISTA_H - y, TARIMA);
    for k in 1..6 {
        let v = y + (VISTA_H - y) * (k as f32 / 6.0).powf(1.6);
        linea(l, (0.0, v), (VISTA_W, v), 1.2, VETA);
    }
    linea(l, (0.0, y), (VISTA_W, y), 3.0, TINTA);
}

/// Un foco en el suelo y su cono de luz desde arriba.
fn foco(l: &Layout, x: f32, suelo: f32, ancho: f32, brillo: f32) {
    tri(
        l,
        (x - ancho * 0.25, 0.0),
        (x + ancho * 0.25, 0.0),
        (x + ancho, suelo),
        fade(LUZ, 0.07 * brillo),
    );
    tri(
        l,
        (x - ancho * 0.25, 0.0),
        (x - ancho, suelo),
        (x + ancho, suelo),
        fade(LUZ, 0.07 * brillo),
    );
    let p = l.to_screen(x, suelo);
    draw_ellipse(
        p.x,
        p.y,
        l.len(ancho),
        l.len(ancho * 0.18),
        0.0,
        fade(LUZ, 0.35 * brillo),
    );
}

/// El pulso de la musica de la sala en el segundo `s`.
fn latido(tema: Tema, s: f32) -> f32 {
    salon::latido(tema, s * 60.0).0
}

/// Un jefe de pie en una caja de `alto` unidades con el centro en `c`.
fn jefe(l: &Layout, baile: usize, c: Vec2, alto: f32, t: f32, colores: (Color, Color)) {
    let (pulso, fuerte) = salon::latido(Tema::de(baile, 0), t);
    jefes::dibujar(
        baile,
        &jefes::Escena {
            centro: l.to_screen(c.x, c.y),
            escala: l.len(alto / 44.0),
            t,
            fase: 0,
            vida: 1.0,
            pulso,
            fuerte,
            tinta: colores.0,
            ropa: colores.1,
            tablas: None,
        },
    );
}

/// I. El salon de Viena, de noche, y ella entrando por la izquierda hasta el
/// centro del foco. Al llegar, le guina un ojo a quien mira.
fn el_salon(l: &Layout, s: f32, ella: &Player) {
    let suelo = 300.0;
    escenarios::dibujar(
        &l.escalado(1.5),
        0,
        suelo / 1.5,
        s * 60.0,
        latido(Tema::Sala, s),
    );
    tarima(l, suelo);

    let andar = jefes::suave(s / 3.2);
    let x = -80.0 + (VISTA_W * 0.5 + 80.0) * andar;
    foco(l, VISTA_W * 0.5, 405.0, 130.0, 0.4 + 0.6 * andar);
    let mut ella = ella.clone();
    ella.vel.x = PLAYER_SPEED * 0.6 * (1.0 - andar);
    let guino = (4.0..4.4).contains(&s).then_some(Expresion::Guino);
    bailarina(l, &ella, vec2(x, 405.0), 4.6, s * 60.0, guino);
}

/// II. Las doce en el reloj del salon, y los cuatro objetos despertandose uno
/// detras de otro: de silueta a su tinta, con un respingo de muelle.
fn medianoche(l: &Layout, s: f32) {
    rect(l, 0.0, 0.0, VISTA_W, VISTA_H, PARED);
    tarima(l, 330.0);

    // El reloj: la aguja de los minutos llega a las doce en el primer segundo.
    let (cx, cy, r) = (VISTA_W * 0.5, 82.0, 52.0);
    circulo(l, cx, cy, r + 5.0, TINTA);
    circulo(l, cx, cy, r, PAPEL);
    for h in 0..12 {
        let a = h as f32 / 12.0 * std::f32::consts::TAU;
        let d = vec2(a.sin(), -a.cos());
        let (p, q) = (d * (r - 8.0), d * (r - 2.0));
        linea(l, (cx + p.x, cy + p.y), (cx + q.x, cy + q.y), 2.0, TINTA);
    }
    let minutos = -0.9 * (1.0 - jefes::suave(s / 1.0));
    let d = vec2(minutos.sin(), -minutos.cos());
    linea(l, (cx, cy), (cx, cy - r * 0.55), 4.0, TINTA);
    linea(
        l,
        (cx, cy),
        (cx + d.x * r * 0.85, cy + d.y * r * 0.85),
        2.5,
        TINTA,
    );
    // Y al dar la hora, una onda dorada.
    let onda = ((s - 1.0) / 0.8).clamp(0.0, 1.0);
    if onda > 0.0 && onda < 1.0 {
        let p = l.to_screen(cx, cy);
        draw_circle_lines(p.x, p.y, l.len(r + 90.0 * onda), 3.0, fade(ORO, 1.0 - onda));
    }

    for baile in 0..4 {
        let despierta = 1.4 + 0.55 * baile as f32;
        let k = ((s - despierta) / 0.5).clamp(0.0, 1.0);
        let (cuerpo, ropa) = paleta::del_baile(baile);
        let colores = (mezcla(TINTA, cuerpo, k), mezcla(TINTA, ropa, k));
        // Dormido no se mueve: el reloj de su dibujo se para.
        let t = s.max(despierta) * 60.0;
        let alto = 170.0 * (0.82 + 0.18 * jefes::boing(k));
        let c = vec2(150.0 + 220.0 * baile as f32, 330.0 - alto * 0.44);
        jefe(l, baile, c, alto, t, colores);
    }
}

/// III. Las cuatro ciudades, cuatro carteles en fila que caen uno detras de
/// otro: el decorado de cada baile, su jefe bailando en el y el nombre.
fn ciudades(l: &Layout, s: f32) {
    rect(l, 0.0, 0.0, VISTA_W, VISTA_H, PARED);
    let (ancho, alto, hueco) = (214.0, 386.0, 18.0);
    let x_ini = (VISTA_W - 4.0 * ancho - 3.0 * hueco) * 0.5;
    for (baile, ciudad) in CIUDADES.iter().enumerate() {
        let k = jefes::bote(((s - 0.3 - 0.4 * baile as f32) / 0.7).clamp(0.0, 1.0));
        if k <= 0.0 {
            continue;
        }
        let x0 = x_ini + baile as f32 * (ancho + hueco);
        let y0 = 22.0 - (1.0 - k) * 440.0;
        // El decorado se dibuja para una franja de 640 de ancho; aqui se
        // agranda hasta llenar el alto del cartel y se recorta su centro.
        let e = alto / 240.0;
        let ox = x0 + ancho * 0.5 - 320.0 * e;
        recortado(l, (x0, y0, ancho, alto), || {
            let le = l.sacudido(vec2(ox, y0)).escalado(e);
            escenarios::dibujar(&le, baile, 240.0, s * 60.0, latido(Tema::de(baile, 0), s));
            let c = vec2(x0 + ancho * 0.5, y0 + alto * 0.62);
            jefe(l, baile, c, 190.0, s * 60.0, paleta::del_baile(baile));
        });
        let p = l.to_screen(x0, y0);
        let (w, h) = (l.len(ancho), l.len(alto));
        draw_rectangle_lines(p.x, p.y, w, h, l.len(4.0), TINTA);
        // El nombre en una cinta de tinta, como la cabecera de un cartel.
        let cinta = l.len(34.0);
        draw_rectangle(p.x, p.y, w, cinta, TINTA);
        let tam = l.len(22.0);
        let y = p.y + cinta * 0.5 + tam * 0.3;
        fuentes::centrado(nombre(ciudad), p.x + w * 0.5, y, tam, Cara::Titulo, PAPEL);
    }
}

/// IV. Ella sola en el foco, crecida, con las fichas de baile dando vueltas
/// alrededor: la que va a por la musica.
fn ella_sola(l: &Layout, s: f32, ella: &Player) {
    rect(l, 0.0, 0.0, VISTA_W, VISTA_H, PARED);
    tarima(l, 360.0);
    let suelo = 408.0;
    foco(l, VISTA_W * 0.5, suelo, 160.0, 1.0);

    // Las fichas: una elipse alrededor de la cintura. Las de detras van antes
    // que ella y las de delante despues, y asi la rodean de verdad.
    let fichas: Vec<(f32, Vec2)> = (0..6)
        .map(|i| {
            let a = s * 1.1 + i as f32 / 6.0 * std::f32::consts::TAU;
            let c = vec2(VISTA_W * 0.5 + a.cos() * 190.0, 290.0 + a.sin() * 34.0);
            (a.sin(), c)
        })
        .collect();
    let pinta = |delante: bool| {
        for (prof, c) in &fichas {
            if (*prof > 0.0) == delante {
                let p = l.to_screen(c.x, c.y);
                modista::ficha(p, l.len(13.0 + 3.0 * prof), s * 3.0 + c.x * 0.01);
            }
        }
    };
    pinta(false);
    bailarina(
        l,
        ella,
        vec2(VISTA_W * 0.5, suelo),
        6.2,
        s * 60.0,
        Some(Expresion::Chula),
    );
    pinta(true);
}

/// V. La pagina del programa: los cuatro numeros de la noche, que salen uno
/// detras de otro, cada uno con su jefe y su tinta. La letra va en tinta negra
/// y el color en un filete debajo: la Poiret es tan fina que en mostaza sobre
/// papel no se leia.
fn programa(l: &Layout, s: f32) {
    rect(l, 0.0, 0.0, VISTA_W, VISTA_H, PAPEL);
    let p = l.to_screen(24.0, 20.0);
    let (w, h) = (l.len(VISTA_W - 48.0), l.len(PIE - 36.0));
    draw_rectangle_lines(p.x, p.y, w, h, l.len(3.0), TINTA);
    let d = l.len(8.0);
    draw_rectangle_lines(p.x + d, p.y + d, w - d * 2.0, h - d * 2.0, 1.0, TINTA);

    let cx = l.to_screen(VISTA_W * 0.5, 0.0).x;
    let titulo = l.to_screen(0.0, 84.0).y;
    fuentes::centrado("VALS", cx, titulo, l.len(52.0), Cara::Titulo, TINTA);
    let ry = l.to_screen(0.0, 104.0).y;
    let media = l.len(150.0);
    draw_line(cx - media, ry, cx + media, ry, 1.5, TINTA);
    draw_poly(cx, ry, 4, l.len(5.0), 45.0, TINTA);

    for (i, numero) in NUMEROS.iter().enumerate() {
        let titulo = nombre(numero);
        let a = ((s - 0.5 - 0.45 * i as f32) / 0.4).clamp(0.0, 1.0);
        if a <= 0.0 {
            continue;
        }
        let y = 160.0 + 66.0 * i as f32;
        let (tinta, ropa) = paleta::del_baile(i);
        jefe(
            l,
            i,
            vec2(222.0, y - 2.0),
            62.0,
            s * 60.0,
            (fade(tinta, a), fade(ropa, a)),
        );
        let base = l.to_screen(0.0, y + 9.0).y;
        let tam = l.len(30.0);
        let x = l.to_screen(300.0, 0.0).x;
        let tenue = fade(paleta::TINTA_TENUE, a);
        fuentes::texto(ROMANOS[i], x, base, tam, Cara::Titulo, tenue);
        let x = l.to_screen(350.0, 0.0).x;
        fuentes::texto(titulo, x, base, tam, Cara::Titulo, fade(TINTA, a));
        let ancho = fuentes::medir(titulo, tam, Cara::Titulo).width;
        let filete = base + l.len(7.0);
        draw_line(x, filete, x + ancho, filete, l.len(3.0), fade(tinta, a));
        let fin = l.to_screen(720.0, 0.0).x;
        let ciudad = nombre(CIUDADES[i]);
        fuentes::derecha(ciudad, fin, base, l.len(18.0), Cara::Cuerpo, fade(TINTA, a));
        // La linea de puntos del programa, del nombre a la ciudad.
        let desde = x + ancho + l.len(12.0);
        let hasta = fin - fuentes::medir(ciudad, l.len(18.0), Cara::Cuerpo).width - l.len(12.0);
        let mut px = desde;
        while px < hasta {
            draw_circle(
                px,
                base - l.len(4.0),
                l.len(1.3),
                fade(paleta::TINTA_TENUE, a),
            );
            px += l.len(9.0);
        }
    }
}

/// El intertitulo: la franja de tinta de abajo con el titulo y sus dos lineas,
/// que entran un poco despues que la lamina para que se mire primero el dibujo.
fn intertitulo(l: &Layout, textos: &[Texto; 3], s: f32) {
    rect(l, 0.0, PIE, VISTA_W, VISTA_H - PIE, fade(TINTA, 0.94));
    linea(l, (0.0, PIE), (VISTA_W, PIE), 2.0, ORO);
    linea(
        l,
        (0.0, PIE + 5.0),
        (VISTA_W, PIE + 5.0),
        0.8,
        fade(ORO, 0.6),
    );
    let a = ((s - 0.35) / 0.4).clamp(0.0, 1.0);
    let cx = l.to_screen(VISTA_W * 0.5, 0.0).x;
    let y = |u: f32| l.to_screen(0.0, u).y;
    let [titulo, una, otra] = textos.map(tx);
    let claro = fade(draw::TITLE, a);
    fuentes::centrado(
        titulo,
        cx,
        y(466.0),
        l.len(30.0),
        Cara::Titulo,
        fade(ORO, a),
    );
    fuentes::centrado(una, cx, y(494.0), l.len(18.0), Cara::Cuerpo, claro);
    fuentes::centrado(otra, cx, y(518.0), l.len(18.0), Cara::Cuerpo, claro);
}

/// Dibuja `pinta` recortado a un rectangulo de la lamina.
///
/// Los decorados se salen de su franja a proposito (el sol del club, la
/// bambalina del Moulin Rouge) y con cuatro en fila se pisarian. El paspartu
/// no sirve aqui, porque entre cartel y cartel no hay papel sino pared. La
/// ovacion del super tambien lo usa: su onda llega a las esquinas y no puede
/// salirse al papel.
pub(crate) fn recortado(l: &Layout, (x, y, w, h): (f32, f32, f32, f32), pinta: impl FnOnce()) {
    let p = l.to_screen(x, y);
    // El recorte va en pixeles del framebuffer, no en los logicos de macroquad.
    let dpi = screen_dpi_scale();
    let caja = (
        (p.x * dpi) as i32,
        (p.y * dpi) as i32,
        (l.len(w) * dpi) as i32,
        (l.len(h) * dpi) as i32,
    );
    unsafe { get_internal_gl() }.quad_gl.scissor(Some(caja));
    pinta();
    unsafe { get_internal_gl() }.quad_gl.scissor(None);
}

fn mezcla(a: Color, b: Color, k: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * k,
        a.g + (b.g - a.g) * k,
        a.b + (b.b - a.b) * k,
        a.a + (b.a - a.a) * k,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f32 = 1.0 / 60.0;

    #[test]
    fn las_laminas_pasan_solas_y_se_acaba() {
        let mut p = Prologo::nuevo();
        let mut segundos = 0.0;
        while !p.acabado() {
            p.step(FRAME);
            segundos += FRAME;
            assert!(segundos < 60.0, "no se acaba nunca");
        }
        // Entre 20 y 30 segundos: lo que se aguanta sin tocar nada.
        assert!((20.0..=30.0).contains(&segundos), "dura {segundos} s");
    }

    #[test]
    fn confirmar_pasa_de_lamina_pero_no_nada_mas_entrar() {
        // Se entra pulsando confirmar: ese boton no puede pasar la primera.
        let mut p = Prologo::nuevo();
        p.avanzar();
        assert_eq!(p.lamina, 0, "se ha comido la primera lamina");
        for _ in 0..(GRACIA / FRAME) as u32 + 2 {
            p.step(FRAME);
        }
        p.avanzar();
        assert_eq!((p.lamina, p.t), (1, 0.0), "y pasada la gracia si pasa");
        // Y la gracia vale en cada lamina, no solo en la primera.
        p.avanzar();
        assert_eq!(p.lamina, 1);
    }

    #[test]
    fn saltar_lo_acaba_de_golpe_y_no_se_pasa_de_largo() {
        let mut p = Prologo::nuevo();
        p.saltar();
        assert!(p.acabado());
        p.step(DURA * 3.0);
        assert!(p.acabado());
        assert_eq!(p.lamina, LAMINAS.len(), "la cuenta se ha salido");
    }
}
