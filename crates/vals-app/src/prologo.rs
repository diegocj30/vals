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
//!
//! **El final es lo mismo con otras laminas**: ella vuelve al salon con las
//! cuatro musicas, los cuatro tocan otra vez, baila por fin, y los creditos con
//! lo que se lleva de la temporada. Sale solo al volver a la pista con los
//! cuatro bailes ganados, el ultimo que sea, y desde el menu se vuelve a ver.

use macroquad::prelude::*;
use vals_core::boss::BossDef;
use vals_core::nota::Nota;
use vals_core::paseo::{VISTA_H, VISTA_W};
use vals_core::pista::Pista;
use vals_core::player::PLAYER_SPEED;
use vals_core::{Player, World};

use crate::acciones::Nombres;
use crate::draw::{self, Layout, fade};
use crate::escenarios::{circulo, linea, rect, tri};
use crate::fuentes::{self, Cara};
use crate::idioma::{Texto, nombre, tx, txf};
use crate::libro::{self, Marcas};
use crate::music::Tema;
use crate::paleta::{self, LUZ, ORO, PAPEL, PARED, TARIMA, TINTA, VETA};
use crate::{cartela, glifos};

/// La luz del amanecer, que entra por los ventanales al final.
const ALBA: Color = color_u8!(255, 160, 128, 255);
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

/// Las del final, igual que las del prologo.
const FINAL: [[Texto; 3]; 4] = [
    [Texto::Final1, Texto::Final1A, Texto::Final1B],
    [Texto::Final2, Texto::Final2A, Texto::Final2B],
    [Texto::Final3, Texto::Final3A, Texto::Final3B],
    [Texto::Final4, Texto::Final4A, Texto::Final4B],
];

/// Los creditos de la musica, en el orden en que suena en el juego.
const PIEZAS: [Texto; 5] = [
    Texto::CreditoDanubio,
    Texto::CreditoOlas,
    Texto::CreditoCumparsita,
    Texto::CreditoRag,
    Texto::CreditoGalop,
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

/// Lo que dura cada lamina si no se toca nada: para leer dos lineas sin prisa,
/// aunque uno se despiste un momento. Con 5,5 s habia que leer deprisa; quien
/// lee rapido ya puede pasarla con confirmar.
const DURA: f32 = 9.0;
/// Lo que tarda en poder pasarse. Se entra pulsando confirmar, y ese mismo
/// boton no puede pasar ya la primera lamina (lo mismo que en las cartelas).
const GRACIA: f32 = 0.6;
/// El fundido a negro de entrada y salida de cada lamina.
const FUNDE: f32 = 0.45;
/// La de los creditos dura mas que las otras: hay mas que leer, y es la
/// ultima.
const DURA_CREDITOS: f32 = DURA + 6.0;
/// Lo que tarda en bajar el telon al final de los creditos.
const TELON: f32 = 1.6;

/// Si al volver a la pista toca el final: los cuatro bailes ganados, en el
/// orden que sea, y sin haberlo visto todavia.
pub fn toca_el_final(pista: &Pista, visto: bool) -> bool {
    pista.todo_vencido() && !visto
}

/// Lo que se ensena de la temporada en los creditos: cada baile, en el orden
/// de los jefes, con su nota y su mejor tiempo; y el tiempo jugado.
pub struct Resumen {
    bailes: Vec<(String, Option<Nota>, Option<u64>)>,
    jugado: u64,
}

impl Resumen {
    pub fn de(marcas: &Marcas, bailes: &[BossDef]) -> Self {
        Self {
            bailes: bailes
                .iter()
                .map(|b| {
                    let m = marcas.por_nombre.get(&b.name);
                    (
                        b.name.clone(),
                        m.and_then(|m| m.nota),
                        m.and_then(|m| m.mejor),
                    )
                })
                .collect(),
            jugado: marcas.jugado,
        }
    }
}

/// El prologo en marcha, o el final.
pub struct Prologo {
    lamina: usize,
    /// Segundos dentro de la lamina.
    t: f32,
    /// La bailarina, quieta: la pose y la cara salen de un `Player`, y aqui se
    /// le cambia solo la velocidad para que ande.
    ella: Player,
    /// Con algo, esto es el final, y eso es lo que sale en sus creditos.
    fin: Option<Resumen>,
}

impl Prologo {
    pub fn nuevo() -> Self {
        Self {
            lamina: 0,
            t: 0.0,
            ella: World::new(0).player,
            fin: None,
        }
    }

    /// El final, con la temporada para los creditos.
    pub fn el_final(resumen: Resumen) -> Self {
        Self {
            fin: Some(resumen),
            ..Self::nuevo()
        }
    }

    pub fn es_el_final(&self) -> bool {
        self.fin.is_some()
    }

    fn laminas(&self) -> &'static [[Texto; 3]] {
        if self.es_el_final() { &FINAL } else { &LAMINAS }
    }

    /// Lo que dura la lamina en curso.
    fn dura(&self) -> f32 {
        if self.es_el_final() && self.lamina + 1 == FINAL.len() {
            DURA_CREDITOS
        } else {
            DURA
        }
    }

    /// Un frame. Al acabarse la lamina se pasa sola a la siguiente.
    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        if self.t >= self.dura() {
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
        self.lamina = self.laminas().len();
    }

    pub fn acabado(&self) -> bool {
        self.lamina >= self.laminas().len()
    }

    fn siguiente(&mut self) {
        self.lamina = (self.lamina + 1).min(self.laminas().len());
        self.t = 0.0;
    }

    /// La lamina en curso, en la lamina apaisada del paseo. `teclas` son los
    /// nombres de lo que se este usando (ver `acciones::Nombres`).
    pub fn dibujar(&self, l: &Layout, teclas: &Nombres) {
        clear_background(PAPEL);
        let Some(textos) = self.laminas().get(self.lamina) else {
            return;
        };
        let (s, dura) = (self.t, self.dura());
        // Un zoom lento hacia el centro: la camara de un cine mudo no esta
        // quieta, y una lamina quieta parece una pantalla de carga.
        let z = 1.0 + 0.045 * (s / dura);
        let lz = l
            .sacudido(vec2(VISTA_W, VISTA_H) * 0.5 * (1.0 - z))
            .escalado(z);
        match (&self.fin, self.lamina) {
            (None, 0) => el_salon(&lz, s, &self.ella),
            (None, 1) => medianoche(&lz, s),
            (None, 2) => ciudades(&lz, s),
            (None, 3) => ella_sola(&lz, s, &self.ella),
            (None, _) => programa(&lz, s),
            (Some(_), 0) => de_vuelta(&lz, s, &self.ella),
            (Some(_), 1) => la_musica_vuelve(&lz, s),
            (Some(_), 2) => su_primer_baile(&lz, s, &self.ella),
            (Some(r), _) => creditos(&lz, s, r),
        }
        intertitulo(l, textos, s);
        let negro = 1.0 - (s / FUNDE).min((dura - s) / FUNDE).clamp(0.0, 1.0);
        rect(l, 0.0, 0.0, VISTA_W, VISTA_H, fade(TINTA, negro));
        draw::paspartu_de(l, vec2(VISTA_W, VISTA_H));
        self.ayuda(l, teclas);
        // Y la funcion acaba como en un teatro: con el telon bajado.
        if dura == DURA_CREDITOS {
            cartela::telon(jefes::suave((s - (dura - TELON - FUNDE)) / TELON));
        }
    }

    /// Debajo de la lamina, en el papel: por donde vamos y como se pasa.
    fn ayuda(&self, l: &Layout, teclas: &Nombres) {
        let o = l.to_screen(0.0, 0.0);
        let y = o.y + l.len(VISTA_H) + 26.0;
        let c = paleta::TINTA_TENUE;
        let n = format!("{} / {}", self.lamina + 1, self.laminas().len());
        fuentes::texto(&n, o.x, y, 16.0, Cara::Cuerpo, c);
        let (siguiente, saltar) = (tx(Texto::Siguiente), tx(Texto::SaltarHistoria));
        let ayuda = glifos::rellenar(
            "{}  {}      {}  {}",
            &[
                &teclas.aceptar,
                &glifos::txt(siguiente),
                &teclas.atras,
                &glifos::txt(saltar),
            ],
        );
        glifos::derecha(&ayuda, o.x + l.len(VISTA_W), y, 16.0, c);
    }
}

/// La invitacion del menu a volver a verlo, arriba a la izquierda de la
/// arena: discreta, que lo que se viene a hacer al menu es jugar. Debajo, el
/// final, si ya se ha ganado.
pub fn en_el_menu(l: &Layout, teclas: &Nombres, hay_final: bool) {
    let o = l.to_screen(0.0, 0.0);
    let u = l.scale().clamp(0.55, 1.2);
    let lineas = [
        (&teclas.historia, Texto::LaHistoria),
        (&teclas.el_final, Texto::ElFinal),
    ];
    for (i, (tecla, t)) in lineas.iter().take(1 + hay_final as usize).enumerate() {
        let texto = glifos::rellenar("{}   {}", &[tecla, &glifos::txt(tx(*t))]);
        glifos::texto(
            &texto,
            o.x + 22.0 * u,
            o.y + (64.0 + 24.0 * i as f32) * u,
            17.0 * u,
            draw::TEXT_DIM,
        );
    }
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
        latido(Tema::Sala(0), s),
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
    hoja(l, 84.0);

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

/// La hoja de un programa de mano: papel, doble filete y VALS en la cabecera,
/// con la base del titulo en `y`.
fn hoja(l: &Layout, y: f32) {
    rect(l, 0.0, 0.0, VISTA_W, VISTA_H, PAPEL);
    let p = l.to_screen(24.0, 20.0);
    let (w, h) = (l.len(VISTA_W - 48.0), l.len(PIE - 36.0));
    draw_rectangle_lines(p.x, p.y, w, h, l.len(3.0), TINTA);
    let d = l.len(8.0);
    draw_rectangle_lines(p.x + d, p.y + d, w - d * 2.0, h - d * 2.0, 1.0, TINTA);

    let cx = l.to_screen(VISTA_W * 0.5, 0.0).x;
    let titulo = l.to_screen(0.0, y).y;
    fuentes::centrado("VALS", cx, titulo, l.len(52.0), Cara::Titulo, TINTA);
    let ry = l.to_screen(0.0, y + 20.0).y;
    let media = l.len(150.0);
    draw_line(cx - media, ry, cx + media, ry, 1.5, TINTA);
    draw_poly(cx, ry, 4, l.len(5.0), 45.0, TINTA);
}

// ---------------------------------------------------------------------------
// Las laminas del final.
// ---------------------------------------------------------------------------

/// Una corchea: la cabeza, la plica y el corchete. Es la musica que vuelve.
fn corchea(l: &Layout, x: f32, y: f32, tam: f32, c: Color) {
    let p = l.to_screen(x, y);
    let r = l.len(tam * 0.3);
    draw_ellipse(p.x, p.y, r * 1.3, r, -20.0, c);
    let (px, arriba) = (p.x + r * 1.15, p.y - l.len(tam));
    let g = l.len(tam * 0.1).max(1.0);
    draw_line(px, p.y, px, arriba, g, c);
    draw_line(px, arriba, px + r * 1.4, arriba + r * 1.8, g * 1.4, c);
}

/// I. Como la primera del prologo: ella entra en el salon de Viena, pero con
/// las cuatro musicas dando vueltas a su alrededor y los cuatro detras, en
/// fila y pequenos. No la persiguen: vuelven con ella.
fn de_vuelta(l: &Layout, s: f32, ella: &Player) {
    let suelo = 300.0;
    escenarios::dibujar(
        &l.escalado(1.5),
        0,
        suelo / 1.5,
        s * 60.0,
        latido(Tema::PasoBase, s),
    );
    tarima(l, suelo);

    let andar = jefes::suave(s / 3.2);
    let x = -80.0 + (VISTA_W * 0.5 + 80.0) * andar;
    foco(l, VISTA_W * 0.5, 405.0, 130.0, 0.4 + 0.6 * andar);
    for baile in 0..4 {
        let alto = 92.0;
        let c = vec2(x - 120.0 - 84.0 * baile as f32, 405.0 - alto * 0.44);
        jefe(l, baile, c, alto, s * 60.0, paleta::del_baile(baile));
    }
    let mut ella = ella.clone();
    ella.vel.x = PLAYER_SPEED * 0.6 * (1.0 - andar);
    let guino = (4.0..4.4).contains(&s).then_some(Expresion::Guino);
    bailarina(l, &ella, vec2(x, 405.0), 4.6, s * 60.0, guino);
    for i in 0..4 {
        let a = s * 1.6 + i as f32 / 4.0 * std::f32::consts::TAU;
        let (cx, cy) = (x + a.cos() * 80.0, 250.0 + a.sin() * 24.0);
        corchea(l, cx, cy, 26.0, paleta::del_baile(i).0);
    }
}

/// II. Los cuatro en fila en la tarima, como en la medianoche del prologo,
/// pero cada uno en su sitio: se le enciende el foco, se pone a tocar y le
/// salen corcheas de su tinta. Uno detras de otro, hasta que suenan todos.
fn la_musica_vuelve(l: &Layout, s: f32) {
    rect(l, 0.0, 0.0, VISTA_W, VISTA_H, PARED);
    tarima(l, 330.0);
    for baile in 0..4 {
        let empieza = 0.5 + 0.6 * baile as f32;
        let k = ((s - empieza) / 0.5).clamp(0.0, 1.0);
        let x = 150.0 + 220.0 * baile as f32;
        foco(l, x, 330.0, 95.0, k);
        // Callado no se mueve, como dormido en el prologo.
        let t = s.max(empieza) * 60.0;
        let alto = 170.0 * (0.82 + 0.18 * jefes::boing(k));
        let c = vec2(x, 330.0 - alto * 0.44);
        let colores = paleta::del_baile(baile);
        jefe(l, baile, c, alto, t, colores);
        for j in 0..3 {
            let sube = ((s - empieza) * 0.5 + j as f32 / 3.0).fract();
            let y = c.y - alto * 0.55 - sube * 150.0;
            let dx = (sube * 6.0 + j as f32 * 2.0).sin() * 22.0;
            corchea(l, x + dx, y, 22.0, fade(colores.0, k * (1.0 - sube)));
        }
    }
}

/// III. Su primer baile: ella en el centro del salon, yendo y viniendo bajo
/// el foco, con los cuatro tocando a los lados, dos y dos, como una orquesta.
/// Y por los ventanales va entrando el amanecer.
fn su_primer_baile(l: &Layout, s: f32, ella: &Player) {
    let suelo = 300.0;
    escenarios::dibujar(
        &l.escalado(1.5),
        0,
        suelo / 1.5,
        s * 60.0,
        latido(Tema::PasoBase, s),
    );
    let alba = jefes::suave((s - 1.0) / (DURA - 2.0));
    rect(l, 0.0, 0.0, VISTA_W, suelo, fade(ALBA, 0.38 * alba));
    tarima(l, suelo);

    for (baile, x) in [(0, 90.0), (1, 200.0), (2, 760.0), (3, 870.0)] {
        let alto = 100.0;
        let c = vec2(x, 400.0 - alto * 0.44);
        jefe(l, baile, c, alto, s * 60.0, paleta::del_baile(baile));
    }
    // El vaiven: va y viene por el centro, inclinada hacia donde va, y recoge
    // y abre los brazos al compas.
    let w = 1.3;
    let x = VISTA_W * 0.5 + 110.0 * (s * w).sin();
    let mut ella = ella.clone();
    ella.vel.x = PLAYER_SPEED * 0.45 * (s * w).cos();
    ella.focus_t = 0.4 - 0.4 * (s * w * 2.0).cos();
    foco(l, x, 405.0, 130.0, 1.0);
    let cara = Some(Expresion::Normal);
    bailarina(l, &ella, vec2(x, 405.0), 4.6, s * 60.0, cara);
}

/// IV. Los creditos, en la misma hoja que el programa del prologo: a la
/// izquierda la musica y las tipografias, a la derecha los bailes de quien
/// juega con su sello y su mejor tiempo, y abajo con que esta hecho.
fn creditos(l: &Layout, s: f32, r: &Resumen) {
    hoja(l, 70.0);
    // Cada bloque entra un poco despues que el anterior.
    let a = |i: f32| ((s - 0.4 - 0.4 * i) / 0.5).clamp(0.0, 1.0);
    let tinta = |a: f32| fade(TINTA, a);
    let tenue = |a: f32| fade(paleta::TINTA_TENUE, a);
    let centrado = |t: &str, x: f32, y: f32, tam: f32, cara: Cara, color: Color| {
        let p = l.to_screen(x, y);
        fuentes::centrado(t, p.x, p.y, l.len(tam), cara, color);
    };

    let izq = 262.0;
    centrado(
        tx(Texto::LaMusica),
        izq,
        134.0,
        21.0,
        Cara::Titulo,
        tinta(a(0.0)),
    );
    for (i, t) in PIEZAS.iter().enumerate() {
        let y = 162.0 + 24.0 * i as f32;
        centrado(tx(*t), izq, y, 16.0, Cara::Cuerpo, tinta(a(0.0)));
    }
    let tipos = tx(Texto::LasTipografias);
    centrado(tipos, izq, 300.0, 21.0, Cara::Titulo, tinta(a(1.0)));
    for (i, t) in [Texto::CreditoPoiret, Texto::CreditoBarlow]
        .iter()
        .enumerate()
    {
        let y = 328.0 + 24.0 * i as f32;
        centrado(tx(*t), izq, y, 16.0, Cara::Cuerpo, tinta(a(1.0)));
    }

    let der = 698.0;
    centrado(
        tx(Texto::TusBailes),
        der,
        134.0,
        21.0,
        Cara::Titulo,
        tinta(a(2.0)),
    );
    for (i, (baile, nota, mejor)) in r.bailes.iter().enumerate() {
        let y = 176.0 + 44.0 * i as f32;
        let k = a(2.0 + 0.3 * i as f32);
        let (cuerpo, ropa) = paleta::del_baile(i);
        jefe(
            l,
            i,
            vec2(540.0, y - 6.0),
            38.0,
            s * 60.0,
            (fade(cuerpo, k), fade(ropa, k)),
        );
        let p = l.to_screen(572.0, y + 7.0);
        fuentes::texto(nombre(baile), p.x, p.y, l.len(22.0), Cara::Titulo, tinta(k));
        let tiempo = mejor.map_or_else(|| "-".to_owned(), libro::reloj);
        let p = l.to_screen(830.0, y + 7.0);
        fuentes::derecha(&tiempo, p.x, p.y, l.len(16.0), Cara::Cuerpo, tenue(k));
        // Y el sello de su nota, que cae despues, uno detras de otro.
        if let Some(n) = nota {
            let golpe = (s - 2.2 - 0.3 * i as f32) / 0.35;
            libro::sello(l.to_screen(872.0, y), l.len(17.0), *n, false, golpe);
        }
    }
    let jugado = txf(Texto::Jugado, &[&libro::horas(r.jugado)]);
    centrado(&jugado, der, 352.0, 16.0, Cara::Cuerpo, tenue(a(3.0)));

    let hecho = tx(Texto::HechoEnRust);
    centrado(
        hecho,
        VISTA_W * 0.5,
        394.0,
        16.0,
        Cara::Cuerpo,
        tenue(a(4.0)),
    );
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
        // Entre 40 y 50 segundos: lo que se aguanta sin tocar nada.
        assert!((40.0..=50.0).contains(&segundos), "dura {segundos} s");
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
    fn el_final_pasa_solo_y_se_acaba_con_los_creditos() {
        let resumen = Resumen::de(&Marcas::default(), &BossDef::default_bosses());
        let mut p = Prologo::el_final(resumen);
        assert!(p.es_el_final() && !Prologo::nuevo().es_el_final());
        let mut segundos = 0.0;
        while !p.acabado() {
            p.step(FRAME);
            segundos += FRAME;
            assert!(segundos < 60.0, "no se acaba nunca");
        }
        // Tres laminas y los creditos, que se leen mas despacio.
        let esperado = (FINAL.len() - 1) as f32 * DURA + DURA_CREDITOS;
        assert!((segundos - esperado).abs() < 0.1, "dura {segundos} s");
        assert_eq!(p.lamina, FINAL.len());
    }

    #[test]
    fn el_final_toca_con_los_cuatro_ganados_el_ultimo_que_sea() {
        use vals_core::boss::Nivel;
        let pista = || {
            Pista::new(vec![
                ("El Vals".into(), Nivel::Facil),
                ("El Tango".into(), Nivel::Dificil),
                ("El Charleston".into(), Nivel::Media),
                ("El Cancan".into(), Nivel::Media),
            ])
        };
        // Lo normal, el tango el ultimo; y si no, el que sea.
        for orden in [[0, 2, 3, 1], [0, 3, 1, 2]] {
            let mut p = pista();
            for (k, i) in orden.iter().enumerate() {
                assert!(!toca_el_final(&p, false), "con {k} ganados no");
                p.marcar_vencido(*i);
            }
            assert!(toca_el_final(&p, false), "con los cuatro si");
            assert!(!toca_el_final(&p, true), "y una vez visto, ya no");
        }
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
