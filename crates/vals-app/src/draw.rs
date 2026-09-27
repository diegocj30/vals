//! Todo el dibujado. Es la unica parte del proyecto que sabe que macroquad
//! existe, y esta escrita para poder tirarse a la basura y rehacerse sobre
//! miniquad puro o wgpu sin tocar `vals-core`.

use macroquad::prelude::*;
use vals_core::bullets::{BULLET_KINDS, Bullets, FLAG_PARRYABLE};
use vals_core::{ARENA_H, ARENA_W, Mode, World, player};

use crate::bailarines;
use crate::bullet_renderer::BulletRenderer;
use crate::cartela;
use crate::fuentes::{self, Cara};
use crate::jefes;
use crate::music::Tema;
use crate::paleta;
use crate::particulas::Particulas;
use crate::protagonista;
use crate::salon;
use crate::skeleton::{self, HUESOS, N_CINTA, N_FALDA, Pose, REMATES};
use crate::stats::FrameStats;
use vals_core::rng::Pcg32;

// El arte del juego es procedural: no se dibuja nada a mano. Lo que cambia
// respecto al neon de antes no es el detalle, es el material: esto es un cartel
// impreso, y por eso hay papel, tinta y colores de epoca. Ver `paleta.rs`.
const PLAYER_GLOW: Color = color_u8!(255, 226, 170, 40);
/// Lo que se agranda la protagonista respecto a su esqueleto.
const TALLA: f32 = 1.6;
const TRAIL: Color = color_u8!(214, 198, 164, 255);
const TRAIL_DASH: Color = color_u8!(232, 96, 142, 255);
const HITBOX: Color = color_u8!(226, 58, 92, 255);

/// **La tinta.** Un negro calido y no un agujero: en un cartel el contorno esta
/// impreso, no recortado.
///
/// Es el color que no habia. Este juego tenia 27 constantes y ninguna era un
/// contorno, y por eso se veia a marcianitos: sin tinta todo brilla, y lo que
/// brilla flota.
const TINTA: Color = color_u8!(18, 13, 20, 255);
/// Lo que engorda el contorno, en unidades logicas. Se **suma** al grosor en
/// vez de multiplicarlo, que es lo que hace que la linea salga del mismo ancho
/// en un antebrazo que en un muslo. Una plumilla no se ensancha con el hueso.
///
/// Va en unidades logicas y no en pixeles a proposito: asi el jefe, que se
/// dibuja a 2.1, lleva un trazo mas gordo que la protagonista. Es lo que hace
/// un dibujante cuando algo esta mas cerca. Ojo con bajarlo: los huesos mas
/// finos miden 0.7, asi que por debajo de 1 el contorno se queda en menos de un
/// pixel y desaparece —que es exactamente lo que paso en el primer intento—.
const TINTA_GRUESA: f32 = 1.35;
// Un color por tipo de bala. Que cada tipo se lea de un vistazo importa mas
// que que sea bonito: con la pantalla llena, el color es la unica pista de a
// que velocidad viene algo.
//
// **Estos no llevan la tinta del baile, y es a proposito.** Todo lo demas de la
// pantalla cambia de color con el baile; las balas no, porque su color no es
// decoracion, es la ficha tecnica: dice el tamano y la velocidad de lo que
// viene. Si cambiase con el baile habria que reaprenderlo cuatro veces.
const BULLET_COLORS: [Color; 4] = [
    color_u8!(240, 228, 196, 255), // pequena: hueso
    color_u8!(240, 150, 62, 255),  // media: naranja quemado
    color_u8!(96, 176, 168, 255),  // grande: verdigris
    color_u8!(255, 240, 170, 255), // aguja: amarillo de foco
];

// El jefe ya no tiene color propio: lo saca de la tinta de su baile, en
// `paleta::del_baile`. Lo unico que queda aqui es el disco de debajo, que es
// sombra y no color.
const BOSS_CORE: Color = color_u8!(20, 12, 14, 255);
const BOSS_FLASH: Color = color_u8!(255, 255, 255, 255);
const HP_BAR: Color = color_u8!(206, 66, 74, 255);
const HP_BAR_BG: Color = color_u8!(52, 34, 32, 255);
const SHOT: Color = color_u8!(248, 238, 206, 255);

// Las parryables. Rosa brillante y con un anillo que late: tienen que gritar
// "ven a por mi" desde el otro lado de la pantalla, porque en eso consiste el
// juego que anaden.
const PARRYABLE: Color = color_u8!(238, 112, 158, 255);
const PARRY_RING: Color = color_u8!(255, 255, 255, 255);
const METER: Color = color_u8!(110, 182, 172, 255);
pub(crate) const METER_FULL: Color = color_u8!(246, 206, 104, 255);

const VEIL: Color = color_u8!(20, 13, 15, 205);
pub(crate) const TITLE: Color = color_u8!(244, 232, 204, 255);
const DEFEAT: Color = color_u8!(214, 78, 84, 255);

const TEXT: Color = color_u8!(216, 200, 170, 255);
pub(crate) const TEXT_DIM: Color = color_u8!(142, 124, 102, 255);

// Y los mismos, para lo que se imprime **sobre el papel**: los avisos, las
// chapas y el overlay de F1, que viven en coordenadas de pantalla y por tanto
// caen fuera de la arena. Con los de arriba —pensados para la tarima oscura—
// serian ilegibles: es el precio de que el marco haya dejado de ser negro.
const TEXT_PAPEL: Color = paleta::TINTA_TENUE;
const TEXT_PAPEL_FUERTE: Color = paleta::TINTA;
const TEXT_PAPEL_ROJO: Color = color_u8!(158, 40, 44, 255);

/// Correspondencia entre las unidades logicas del mundo y los pixeles de la
/// ventana. El mundo nunca sabe cuantos pixeles mide nada.
pub struct Layout {
    origin: Vec2,
    scale: f32,
}

impl Layout {
    pub fn compute() -> Self {
        // Se ajusta por el eje mas apretado, no solo por el alto: en una
        // ventana estrecha —o en el navegador de un movil en vertical— escalar
        // solo por la altura saca la arena por los lados y recorta el campo de
        // juego, que en un danmaku es directamente injugable.
        // El margen es el papel, y ahora el papel se ve: la arena es una lamina
        // impresa en una pagina, no una pantalla a sangre. Antes eran 24 px
        // porque el marco era negro y daba igual.
        const MARGEN: f32 = 76.0;
        let scale = ((screen_height() - MARGEN) / ARENA_H)
            .min((screen_width() - MARGEN) / ARENA_W)
            .max(0.05);
        let size = vec2(ARENA_W * scale, ARENA_H * scale);
        let origin = ((vec2(screen_width(), screen_height()) - size) * 0.5).round();
        Self { origin, scale }
    }

    pub(crate) fn to_screen(&self, x: f32, y: f32) -> Vec2 {
        self.origin + vec2(x, y) * self.scale
    }

    /// Esquina de la arena en pixeles. La necesita el shader instanciado.
    pub fn origin_px(&self) -> Vec2 {
        self.origin
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Region logica que cabe en la ventana, para descartar lo que no se ve.
    pub fn visible_bounds(&self) -> (Vec2, Vec2) {
        let min = -self.origin / self.scale;
        let max = (vec2(screen_width(), screen_height()) - self.origin) / self.scale;
        (min, max)
    }

    pub(crate) fn len(&self, logical: f32) -> f32 {
        logical * self.scale
    }

    /// Una copia desplazada, en unidades de arena.
    ///
    /// Asi es como sacude la pantalla: **moviendo el origen**. Ni una funcion
    /// de dibujo se entera de que existe una sacudida, igual que no se entera
    /// de que existe una perspectiva.
    pub fn sacudido(&self, d: Vec2) -> Self {
        Self {
            origin: self.origin + d * self.scale,
            scale: self.scale,
        }
    }

    /// Una copia con la escala multiplicada.
    ///
    /// La pista la usa para la perspectiva: dibujar una figura al fondo es
    /// dibujarla con un `Layout` mas pequeno, sin que ni `draw_figura` ni nada
    /// de lo que llama tenga que enterarse de que existe una perspectiva.
    /// Un layout sin origen y con la escala que se pida: para dibujar figuras
    /// alrededor de un punto de pantalla, que es lo que hacen los jefes y el
    /// cartel lateral.
    pub(crate) fn con_escala(scale: f32) -> Self {
        Self {
            origin: Vec2::ZERO,
            scale,
        }
    }

    pub(crate) fn escalado(&self, k: f32) -> Self {
        Self {
            origin: self.origin,
            scale: self.scale * k,
        }
    }
}

pub(crate) fn fade(c: Color, a: f32) -> Color {
    Color { a: c.a * a, ..c }
}

/// Pinta un texto dos veces: desplazado en tinta y encima en su color. Es el
/// rotulo de un cartel, que lleva sombra para leerse sobre cualquier fondo.
fn con_sombra(pinta: impl Fn(f32, Color), color: Color) {
    pinta(1.6, fade(TINTA, 0.85));
    pinta(0.0, color);
}

/// Dibuja un frame de juego.
///
/// `bullets_gpu` decide el camino de render: con `Some` va el instanciado de un
/// solo draw call, con `None` el viejo a base de primitivas de macroquad. Los
/// dos conviven a proposito, porque es lo que hace reproducible la comparacion
/// de `docs/PERF.md` dentro de un ano.
pub fn frame(world: &World, alpha: f32, layout: &Layout, bullets_gpu: Option<&mut BulletRenderer>) {
    papel();
    draw_arena(
        layout,
        world.baile,
        Tema::de(world.baile, world.boss.phase),
        world.tick as f32 + alpha,
    );
    if world.mode == Mode::Platform {
        draw_ground(layout, world.tick as f32 + alpha);
    }
    draw_boss(world, alpha, layout);
    draw_player_shots(world, layout);
    draw_trail(world, layout);
    // Las balas van encima del jefe pero debajo del jugador: taparte tu propia
    // hitbox seria exactamente lo contrario de lo que hace falta.
    let phase = world.tick as f32 + alpha;
    match bullets_gpu {
        Some(r) => r.draw(&world.bullets, layout, pulse(phase), bullet_style),
        None => draw_bullets_at(&world.bullets, layout, phase),
    }
    draw_player(world, alpha, layout);
    paspartu(layout);
    paneles(world, alpha, layout);
    draw_hp_bar(world, layout);
    draw_cartas(world, layout);
    draw_hud(world, layout);
    draw_super_flash(world, layout);
}

/// El proscenio: el borde del escenario, con candilejas.
///
/// En el modo con salto el suelo era una linea azul. Ahora es el canto de un
/// escenario de 1900, una tabla con su fila de candilejas alumbrando hacia
/// arriba: es donde se planta una bailarina que no vuela.
fn draw_ground(l: &Layout, t: f32) {
    let o = l.to_screen(0.0, player::GROUND_Y + player::PLAYER_SPRITE_RADIUS);
    let w = l.len(ARENA_W);
    let alto = l.to_screen(0.0, ARENA_H).y - o.y;

    // La tabla del proscenio, con sus vetas y el canto en tinta.
    draw_rectangle(o.x, o.y, w, alto, color_u8!(62, 38, 32, 255));
    for k in 1..3 {
        let y = o.y + alto * k as f32 / 3.0;
        draw_line(o.x, y, o.x + w, y, 1.0, fade(TINTA, 0.35));
    }
    draw_line(o.x, o.y, o.x + w, o.y, 3.0, TINTA);

    // Las candilejas: una fila de luces en el borde que alumbran hacia arriba.
    // Tiemblan un poco cada una a su aire, que es lo que hace una llama de gas.
    let n = 9;
    for i in 0..n {
        let x = o.x + w * (i as f32 + 0.5) / n as f32;
        let brillo = 0.75 + 0.25 * (t * 0.05 + i as f32 * 1.3).sin();
        let (ancho, subida) = (l.len(26.0), l.len(70.0));
        draw_triangle(
            vec2(x, o.y),
            vec2(x - ancho, o.y - subida),
            vec2(x + ancho, o.y - subida),
            fade(paleta::LUZ, 0.05 * brillo),
        );
        draw_rectangle(x - l.len(9.0), o.y + 2.0, l.len(18.0), l.len(7.0), TINTA);
        draw_circle(x, o.y + 2.0, l.len(5.0), fade(paleta::LUZ, brillo));
    }
}

/// Fogonazo del super. Es la unica cosa que ocupa la pantalla entera, y por eso
/// se lee como "ha pasado algo gordo" sin necesidad de explicarlo.
fn draw_super_flash(world: &World, l: &Layout) {
    if world.player.super_ticks == 0 {
        return;
    }
    let k = world.player.super_ticks as f32 / player::SUPER_TICKS as f32;
    let o = l.to_screen(0.0, 0.0);
    draw_rectangle(
        o.x,
        o.y,
        l.len(ARENA_W),
        l.len(ARENA_H),
        fade(METER, k * k * 0.5),
    );
}

/// Nombre del jefe, cual de cuantos es, y las vidas que quedan.
fn draw_hud(world: &World, l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let w = l.len(ARENA_W);

    if !world.boss.defeated {
        let etiqueta = format!(
            "{}  -  {}  ({}/{})",
            world.boss.name,
            world.boss.phase_name(),
            world.boss.phase + 1,
            world.boss.phase_count()
        );
        // Con sombra de tinta: encima ya no hay una pared lisa sino un
        // decorado, y el sol dorado del club se comia el texto crema.
        con_sombra(
            |dx, c| {
                fuentes::derecha(
                    &etiqueta,
                    o.x + w - 8.0 + dx,
                    o.y + 34.0 + dx,
                    19.0,
                    Cara::Titulo,
                    c,
                )
            },
            TEXT,
        );

        // Y debajo, que vals suena. "Cada jefe es un baile" se entiende mejor
        // si el baile tiene nombre y autor.
        let baile = Tema::de(world.baile, world.boss.phase).titulo();
        con_sombra(
            |dx, c| {
                fuentes::derecha(
                    baile,
                    o.x + w - 8.0 + dx,
                    o.y + 52.0 + dx,
                    14.0,
                    Cara::Cuerpo,
                    c,
                )
            },
            TEXT,
        );
    }
}

/// Menu. De fondo corre el replay dorado, que es el modo atractor.
/// El menu, con los controles del mando que haya puesto.
///
/// Si hay mando, la lista es **la del mando y solo la del mando**. Enterarse de
/// que el super era el triangulo probando los cuatro botones es exactamente lo
/// que una pantalla de controles existe para evitar, y una lista doble se lee
/// peor que la que toca.
pub fn menu(l: &Layout, world: &World, intentos: u32, mando: Option<[&str; 4]>) {
    let cola = if intentos == 0 {
        String::new()
    } else {
        format!("   ({intentos} intentos)")
    };

    // La primera linea es como se entra; el resto, los controles. Van
    // separadas porque no pesan lo mismo: lo primero es una invitacion y lo
    // segundo una chuleta. Antes eran dos (volando o con salto), pero desde la
    // ultima version el modo lo trae cada baile y no se elige aqui.
    let lineas: Vec<String> = match mando {
        Some(b) => vec![
            format!("{}   entrar a la pista{cola}", b[0]),
            "stick o cruceta   mover".to_owned(),
            format!("{}  disparar     {}  dash", b[0], b[1]),
            format!("{}  parry        {}  super", b[2], b[3]),
            "L2  focus        M  mudo".to_owned(),
            "Parriar las balas ROSAS llena la barra SUPER;".to_owned(),
            format!("llena, {} limpia la pantalla y hace mucho dano.", b[3]),
        ],
        None => vec![
            format!("Z   entrar a la pista{cola}"),
            "flechas mover    Z disparar    X dash".to_owned(),
            "C parry    SHIFT focus    M mudo".to_owned(),
            "Parriar las balas ROSAS llena la barra SUPER.".to_owned(),
            "Llena, ESPACIO limpia la pantalla y hace mucho dano.".to_owned(),
        ],
    };

    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(ARENA_W), l.len(ARENA_H));
    let cx = o.x + w * 0.5;
    draw_rectangle(o.x, o.y, w, h, fade(VEIL, 0.8));

    // La protagonista, la estrella del cartel. Antes era la bailarina del vals
    // en penumbra, pero el vals es un jefe, y el cartel de un espectaculo lo
    // protagoniza quien lo baila. Va a todo color y **debajo** del titulo, no
    // detras: con las letras cruzandole la cara no se leia ni ella ni VALS.
    // Quieta, en su mejor porte, mirando alrededor, y cada cinco segundos le
    // guina un ojo a quien mira.
    let t = world.tick as f32;
    let mut ella = world.player.clone();
    ella.vel *= 0.0;
    ella.dash.ticks_left = 0;
    ella.parry_window = 0;
    ella.focus_t = 0.0;
    let pose = skeleton::pose(&ella, t, false, 1.0);
    let gesto = protagonista::Gesto {
        cara: if t.rem_euclid(300.0) < 24.0 {
            protagonista::Expresion::Guino
        } else {
            protagonista::Expresion::Normal
        },
        mirada: vec2((t * 0.013).sin() * 0.6, 0.1),
        ..protagonista::Gesto::de(&ella, t)
    };
    // Mide unas 37 unidades del lazo a los zapatos, 15 de ellas por debajo del
    // centro de la pose: asi los pies caen justo encima de la carta de controles.
    let estrella = Layout::con_escala(h * 0.32 / 37.0);
    let pies = o.y + h * 0.80;
    protagonista::dibujar(
        &pose,
        &gesto,
        vec2(cx, pies - estrella.len(15.0)),
        &estrella,
        1.0,
        false,
    );

    // El titulo, con sombra de tinta y una orla Deco debajo.
    let grande = 150.0 * l.scale().min(1.2);
    let y = o.y + h * 0.34;
    con_sombra(
        |dx, c| fuentes::centrado("VALS", cx + dx * 2.0, y + dx * 2.0, grande, Cara::Titulo, c),
        TITLE,
    );
    let ry = y + 26.0;
    let media = w * 0.26;
    draw_line(cx - media, ry, cx - 12.0, ry, 1.5, METER_FULL);
    draw_line(cx + 12.0, ry, cx + media, ry, 1.5, METER_FULL);
    draw_poly(cx, ry, 4, 6.0, 45.0, METER_FULL);
    con_sombra(
        |dx, c| {
            fuentes::centrado(
                "cuatro bailes, cuatro jefes",
                cx + dx,
                ry + 34.0 + dx,
                20.0,
                Cara::Cuerpo,
                c,
            )
        },
        METER_FULL,
    );

    // La invitacion, latiendo: es lo unico que hay que hacer en esta pantalla.
    let latido = 0.65 + 0.35 * (get_time() as f32 * 3.0).sin();
    // Justo bajo el subtitulo: mas abajo esta ella.
    let mut y = o.y + h * 0.46;
    for linea in &lineas[..1] {
        con_sombra(
            |dx, c| fuentes::centrado(linea, cx + dx, y + dx, 21.0, Cara::Cuerpo, c),
            fade(TITLE, latido),
        );
        y += 28.0;
    }

    // Y la chuleta, en una carta abajo, pequena.
    let (cw, ch) = (w * 0.86, 24.0 + 21.0 * (lineas.len() - 1) as f32);
    let (x0, y0) = (cx - cw * 0.5, o.y + h - ch - 40.0);
    draw_rectangle(x0, y0, cw, ch, fade(paleta::TINTA, 0.55));
    draw_rectangle_lines(x0, y0, cw, ch, 1.5, fade(METER_FULL, 0.7));
    let mut y = y0 + 26.0;
    for linea in &lineas[1..] {
        fuentes::centrado(linea, cx, y, 16.0, Cara::Cuerpo, TEXT);
        y += 21.0;
    }
}

pub fn fin_de_partida(l: &Layout, world: &World) {
    if world.victory {
        sello_de_victoria(l, world);
    } else {
        sello_de_derrota(l, world);
    }
}

/// La victoria: el sello, el nombre del baile y el publico tirando flores.
///
/// Antes decia "FIN DEL VALS" hasta cuando se ganaba el tango: era el texto de
/// cuando el juego tenia un solo baile. Ahora el sello lleva la tinta del baile
/// ganado y su nombre, y el publico hace lo que se hace al acabar un numero.
fn sello_de_victoria(l: &Layout, world: &World) {
    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(ARENA_W), l.len(ARENA_H));
    draw_rectangle(o.x, o.y, w, h, fade(VEIL, 0.75));
    lluvia_de_flores(l, world.baile);

    let (tinta, _) = paleta::del_baile(world.baile);
    let cx = o.x + w * 0.5;
    let mut y = o.y + h * 0.36;
    let ancho = fuentes::medir("¡BRAVO!", 72.0, Cara::Titulo).width;
    let (rw, rh) = (ancho * 0.5 + 40.0, 50.0);
    draw_rectangle(
        cx - rw,
        y - rh,
        rw * 2.0,
        rh * 2.0,
        fade(paleta::TINTA, 0.6),
    );
    for (d, grosor, alfa) in [(0.0, 4.0, 1.0), (8.0, 1.5, 0.6)] {
        draw_rectangle_lines(
            cx - rw - d,
            y - rh - d,
            (rw + d) * 2.0,
            (rh + d) * 2.0,
            grosor,
            fade(tinta, alfa),
        );
    }
    con_sombra(
        |dx, c| fuentes::centrado("¡BRAVO!", cx + dx, y + 20.0 + dx, 72.0, Cara::Titulo, c),
        tinta,
    );
    y += rh + 48.0;
    let nombre = format!("{}, bailado entero", world.boss.name);
    con_sombra(
        |dx, c| fuentes::centrado(&nombre, cx + dx, y + dx, 24.0, Cara::Cuerpo, c),
        TITLE,
    );
    y += 44.0;
    con_sombra(
        |dx, c| {
            fuentes::centrado(
                "R otra vez    ESC volver a la pista",
                cx + dx,
                y + dx,
                19.0,
                Cara::Cuerpo,
                c,
            )
        },
        TEXT,
    );
}

/// Confeti en las tintas del baile y alguna rosa: lo que le tira el publico a
/// quien acaba de bailar.
///
/// Sin particulas ni estado: cada pieza tiene sus numeros fijos —sacados de una
/// `Pcg32` con semilla constante, asi que son los mismos en cada frame— y su
/// posicion es funcion del reloj. Cae, se mece y gira, y al llegar abajo vuelve
/// a salir por arriba.
fn lluvia_de_flores(l: &Layout, baile: usize) {
    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(ARENA_W), l.len(ARENA_H));
    let t = get_time() as f32;
    let (tinta, ropa) = paleta::del_baile(baile);
    let colores = [tinta, ropa, METER_FULL, paleta::PAPEL];
    let mut rng = Pcg32::new(0xB4A_0F1E);
    for i in 0..110 {
        let (a, b, c) = (rng.next_f32(), rng.next_f32(), rng.next_f32());
        let rosa = i % 9 == 0;
        let caida = if rosa { 55.0 } else { 70.0 + b * 90.0 };
        let y = o.y + (a * h + t * caida).rem_euclid(h + 40.0) - 20.0;
        let x = o.x + b * w + (t * (1.2 + c) + i as f32).sin() * 14.0;
        // Nada se sale de la lamina, tampoco las flores (ver `paspartu`).
        if x < o.x + 6.0 || x > o.x + w - 6.0 || y < o.y + 6.0 || y > o.y + h - 6.0 {
            continue;
        }
        if rosa {
            // Una rosa: el tallo y la flor, con su tinta.
            draw_line(x, y, x - 4.0, y + 13.0, 2.0, color_u8!(64, 110, 60, 255));
            draw_circle(x, y, 6.5, paleta::TINTA);
            draw_circle(x, y, 5.0, color_u8!(196, 36, 52, 255));
            draw_circle(x + 1.0, y - 1.0, 2.0, color_u8!(140, 20, 36, 255));
        } else {
            draw_rectangle_ex(
                x,
                y,
                8.0,
                4.5,
                DrawRectangleParams {
                    offset: vec2(0.5, 0.5),
                    rotation: t * (1.5 + c * 4.0) + i as f32,
                    color: colores[i % colores.len()],
                },
            );
        }
    }
}

/// Cuanto del combate te habias comido, de 0 a 1.
///
/// Las figuras anteriores cuentan enteras y la de ahora, por la vida que le
/// quedaba al jefe. No es exacto —las figuras no duran lo mismo— y da igual:
/// esto no es una estadistica, es **saber si te quedaste cerca**.
fn recorrido(world: &World) -> f32 {
    let total = world.boss.phase_count().max(1) as f32;
    let hechas = world.boss.phase as f32;
    ((hechas + (1.0 - world.boss.hp_ratio())) / total).clamp(0.0, 1.0)
}

/// El sello del KO y la barra de lo que te falto.
///
/// La barra es lo mas Cuphead que hay aqui, y no por el dibujo: es que **un
/// juego duro tiene que decirte si te acercaste**. Si llevas cien intentos en
/// la coda; cien intentos con esta barra son cien intentos con informacion, y
/// sin ella son cien intentos a ciegas.
fn sello_de_derrota(l: &Layout, world: &World) {
    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(ARENA_W), l.len(ARENA_H));
    draw_rectangle(o.x, o.y, w, h, VEIL);

    let cx = o.x + w * 0.5;
    let mut y = o.y + h * 0.30;

    // El sello: el titulo con una orla alrededor, como un cuno de tinta.
    let ancho = fuentes::medir("SE ACABO", 64.0, Cara::Titulo).width;
    let (rw, rh) = (ancho * 0.5 + 34.0, 44.0);
    for (grosor, alfa) in [(4.0, 1.0), (1.5, 0.6)] {
        let d = if grosor > 2.0 { 0.0 } else { 7.0 };
        draw_rectangle_lines(
            cx - rw - d,
            y - rh - d,
            (rw + d) * 2.0,
            (rh + d) * 2.0,
            grosor,
            fade(DEFEAT, alfa),
        );
    }
    fuentes::centrado("SE ACABO", cx, y + 16.0, 64.0, Cara::Titulo, DEFEAT);
    y += rh + 46.0;

    // Donde caiste.
    let quien = format!(
        "{}  ({}/{})",
        world.boss.phase_name(),
        world.boss.phase + 1,
        world.boss.phase_count()
    );
    fuentes::centrado(&quien, cx, y, 22.0, Cara::Cuerpo, TEXT);
    y += 34.0;

    // Y la barra: cuanto del baile te habias comido.
    let hecho = recorrido(world);
    let barra = w * 0.56;
    let x0 = cx - barra * 0.5;
    draw_rectangle(x0, y, barra, 9.0, HP_BAR_BG);
    draw_rectangle(x0, y, barra * hecho, 9.0, HP_BAR);
    draw_rectangle_lines(x0, y, barra, 9.0, 1.5, fade(TEXT_DIM, 0.9));
    // Las marcas de cada figura, para que la barra diga algo y no solo llene.
    let figuras = world.boss.phase_count().max(1);
    for i in 1..figuras {
        let x = x0 + barra * i as f32 / figuras as f32;
        draw_line(x, y, x, y + 9.0, 1.5, fade(TEXT_DIM, 0.9));
    }
    y += 26.0;
    let pct = (hecho * 100.0).round() as u32;
    fuentes::centrado(
        &format!("te comiste el {pct}% del baile"),
        cx,
        y,
        17.0,
        Cara::Cuerpo,
        TEXT_DIM,
    );
    y += 40.0;

    for linea in ["R reintentar este baile", "ESC volver a la pista"] {
        fuentes::centrado(linea, cx, y, 19.0, Cara::Cuerpo, TEXT);
        y += 25.0;
    }
}

/// El jefe: poligonos concentricos girando a distintas velocidades.
///
/// Todo procedural. No hay ni un pixel dibujado a mano en este juego, y esa
/// decision es la que hace que anadir un jefe nuevo sea escribir un RON.
fn draw_boss(world: &World, alpha: f32, l: &Layout) {
    let b = &world.boss;
    if b.defeated {
        return;
    }
    let p = b.render_pos(alpha);
    let s = l.to_screen(p.x, p.y);
    let t = world.tick as f32 + alpha;
    let vida = b.hp_ratio();

    // El aro de golpeo. El cuerpo dice quien es; el aro dice donde darle.
    // Cuando el jefe era un hexagono las dos cosas eran la misma, y por eso se
    // leia bien pero no contaba nada.
    // Y va con la tinta de su baile: un cartel de epoca se imprimia a dos o
    // tres planchas, y aqui cada baile tiene las suyas. Es lo que hace que los
    // cuatro se distingan de un vistazo, con el sonido quitado.
    let (tinta_cuerpo, tinta_tela) = paleta::del_baile(world.baile);
    let (pulso, brillo) = bailarines::halo(t, vida);
    let r = l.len(b.radius) * pulso;
    draw_circle(s.x, s.y, r, fade(BOSS_CORE, 0.55));
    draw_poly_lines(s.x, s.y, 24, r, 0.0, 1.5, fade(tinta_cuerpo, 0.20 + brillo));

    // Y la figura. El jefe es un bailarin, y baila lo suyo.
    let golpeado = b.hit_flash > 0;
    let (cuerpo, tela) = if golpeado {
        (BOSS_FLASH, BOSS_FLASH)
    } else {
        (tinta_cuerpo, tinta_tela)
    };
    // El cuerpo lo pone cada jefe (`jefes/`). En el suelo no puede atravesar
    // el proscenio: el cancan baja al borde del escenario para patear a ras de
    // tablas, y colgado de su hitbox se hundia bajo las candilejas.
    let (pulso, fuerte) = salon::latido(Tema::de(world.baile, b.phase), t);
    let tablas = (world.mode == Mode::Platform).then(|| {
        l.to_screen(0.0, player::GROUND_Y + player::PLAYER_SPRITE_RADIUS)
            .y
    });
    jefes::dibujar(
        bailarin(world),
        &jefes::Escena {
            centro: s,
            escala: l.escalado(bailarines::ESCALA).scale(),
            t,
            fase: b.phase,
            vida,
            pulso,
            fuerte,
            tinta: cuerpo,
            ropa: tela,
            tablas,
        },
    );
}

/// Barra de vida de la fase, con una marca por fase superada.
fn draw_hp_bar(world: &World, l: &Layout) {
    let b = &world.boss;
    if b.defeated {
        return;
    }
    let o = l.to_screen(0.0, 0.0);
    let w = l.len(ARENA_W);
    let alto = 6.0;
    let y = o.y + 8.0;

    draw_rectangle(o.x, y, w, alto, HP_BAR_BG);
    draw_rectangle(o.x, y, w * b.hp_ratio(), alto, HP_BAR);

    // Un punto por fase: lleno el que se esta jugando, hueco el que queda.
    for i in 0..b.phase_count() {
        let cx = o.x + 8.0 + i as f32 * 12.0;
        let cy = y + alto + 10.0;
        if i <= b.phase {
            draw_circle(cx, cy, 3.0, HP_BAR);
        } else {
            draw_circle_lines(cx, cy, 3.0, 1.0, HP_BAR_BG);
        }
    }
}

/// Los disparos del jugador: trazos finos, para no confundirlos con las balas
/// que matan.
fn draw_player_shots(world: &World, l: &Layout) {
    for b in world.player_shots.iter_live() {
        let s = l.to_screen(b.pos.x, b.pos.y);
        let largo = l.len(14.0);
        draw_line(s.x, s.y - largo, s.x, s.y + largo, 2.0, fade(SHOT, 0.85));
        draw_circle(s.x, s.y, l.len(2.5), SHOT);
    }
}

/// Color de una bala y si lleva anillo de parryable.
///
/// La usan los dos caminos de render, el viejo y el instanciado, para que no
/// puedan divergir: la paleta vive aqui y el renderer de GPU no sabe de
/// estetica.
pub fn bullet_style(kind: u8, flags: u8) -> (Color, bool) {
    let parryable = flags & FLAG_PARRYABLE != 0;
    let c = if parryable {
        PARRYABLE
    } else {
        BULLET_COLORS[kind as usize]
    };
    (c, parryable)
}

/// Latido del anillo de las parryables, en `[0, 1]`.
pub fn pulse(phase: f32) -> f32 {
    0.5 + 0.5 * (phase * 0.18).sin()
}

/// Dibuja las balas.
///
/// Dos circulos por bala (halo y nucleo) con las primitivas normales de
/// macroquad. Es a proposito la version lenta: H6 la sustituye por un unico
/// draw call instanciado, y esto es el "antes" con el que se comparara.
pub fn draw_bullets(bullets: &Bullets, l: &Layout) {
    draw_bullets_at(bullets, l, 0.0)
}

/// `phase` anima el latido de las parryables. La escena de stress pasa 0.
pub fn draw_bullets_at(bullets: &Bullets, l: &Layout, phase: f32) {
    let latido = pulse(phase);
    for b in bullets.iter_live() {
        let s = l.to_screen(b.pos.x, b.pos.y);
        let (color, parryable) = bullet_style(b.kind, b.flags);
        let r = l.len(BULLET_KINDS[b.kind as usize].draw_radius);

        // Lo mismo que el shader: halo fuera, tinta en el canto, color dentro,
        // y las agujas alargadas hacia donde van.
        let (eje, [largo, ancho]) = crate::bullet_renderer::estiramiento(b.kind, b.vel.x, b.vel.y);
        let giro = eje[1].atan2(eje[0]).to_degrees();
        draw_circle(s.x, s.y, r * 1.9, fade(color, 0.16));
        draw_ellipse(s.x, s.y, r * largo, r * ancho, giro, TINTA);
        draw_ellipse(s.x, s.y, r * largo * 0.9, r * ancho * 0.9, giro, color);

        if parryable {
            // El anillo es lo que de verdad las distingue del resto a simple
            // vista, mas que el color: se mueve.
            draw_circle_lines(
                s.x,
                s.y,
                r * (1.45 + 0.35 * latido),
                1.5,
                fade(PARRY_RING, 0.35 + 0.45 * latido),
            );
        }
    }
}

thread_local! {
    /// El grano del papel. Se calcula una vez y no se vuelve a tocar.
    static GRANO: Vec<(f32, f32, f32)> = paleta::grano();
}

/// El papel: el fondo de todo, con su grano.
///
/// Esto es lo que antes era `clear_background` de un negro azulado, o sea nada.
/// Ahora el marco es **material**: una pagina crema con tooth, y la arena una
/// lamina impresa encima. Es el cambio que mas se ve del juego entero y son
/// diez lineas.
fn papel() {
    clear_background(paleta::PAPEL);
}

/// Los paneles de los lados, cuando la ventana deja sitio.
///
/// Un shmup vertical en una pantalla apaisada deja dos franjas de papel vacio,
/// y las recreativas las llenaban de arte. Aqui: a la izquierda el cartel del
/// baile, con su bailarina en grande; a la derecha el programa, con sus
/// figuras. En una ventana vertical no caben y no se dibujan: nada de esto hace
/// falta para jugar.
fn paneles(world: &World, alpha: f32, l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let margen = o.x;
    if margen < 170.0 {
        return;
    }
    let ancho = (margen - 44.0).min(250.0);
    let alto = l.len(ARENA_H);
    cartel_del_baile(world, alpha, margen * 0.5, o.y, ancho, alto);
    programa(world, o.x + l.len(ARENA_W) + margen * 0.5, o.y, ancho, alto);
}

/// El cartel: el nombre del baile arriba, la bailarina bailando en su tinta, y
/// la pieza al pie. Es la carta de presentacion de cada jefe en Cuphead, hecha
/// a la manera de un cartel de 1900.
fn cartel_del_baile(world: &World, alpha: f32, cx: f32, y0: f32, ancho: f32, alto: f32) {
    let x0 = cx - ancho * 0.5;
    let h = alto * 0.78;
    orla(x0, y0, ancho, h);

    // El nombre, encogido hasta que quepa: "EL CHARLESTON" no mide lo que
    // "EL VALS".
    let nombre = world.boss.name.to_uppercase();
    let mut tam = 40.0;
    while tam > 18.0 && fuentes::medir(&nombre, tam, Cara::Titulo).width > ancho - 24.0 {
        tam -= 2.0;
    }
    fuentes::centrado(&nombre, cx, y0 + 52.0, tam, Cara::Titulo, TINTA);

    // El escenario del cartel: un campo plano de la tinta del baile, y la
    // bailarina encima en silueta. Es el recurso de Toulouse-Lautrec: color
    // plano y una figura negra, y se lee desde el otro lado de la calle.
    let (tinta, ropa) = paleta::del_baile(world.baile);
    let (sx, sy, sw, sh) = (x0 + 14.0, y0 + 74.0, ancho - 28.0, h - 150.0);
    draw_rectangle(sx, sy, sw, sh, tinta);
    draw_rectangle_lines(sx, sy, sw, sh, 2.5, TINTA);

    let t = world.tick as f32 + alpha;
    let (pulso, fuerte) = salon::latido(Tema::de(world.baile, world.boss.phase), t);
    jefes::dibujar(
        bailarin(world),
        &jefes::Escena {
            centro: vec2(cx, sy + sh * 0.56),
            escala: (sh / 44.0).min(sw / 30.0),
            t,
            fase: world.boss.phase,
            vida: world.boss.hp_ratio(),
            pulso,
            fuerte,
            tinta: TINTA,
            ropa,
            tablas: None,
        },
    );

    // La pieza, partida por el guion si no cabe: "El Danubio azul" arriba y
    // "Johann Strauss II, 1866" debajo.
    let pieza = Tema::de(world.baile, world.boss.phase).titulo();
    let mut y = sy + sh + 28.0;
    for trozo in pieza.split(" - ") {
        fuentes::centrado(trozo, cx, y, 15.0, Cara::Cuerpo, TEXT_PAPEL_FUERTE);
        y += 19.0;
    }
}

/// El programa de mano: las figuras del baile, con la que suena marcada.
///
/// Es informacion de verdad y no adorno: en un jefe de cuatro figuras, saber
/// por cual vas es saber cuanto queda.
fn programa(world: &World, cx: f32, y0: f32, ancho: f32, alto: f32) {
    let x0 = cx - ancho * 0.5;
    let h = alto * 0.78;
    orla(x0, y0, ancho, h);

    fuentes::centrado("PROGRAMA", cx, y0 + 46.0, 28.0, Cara::Titulo, TINTA);
    draw_line(
        x0 + 30.0,
        y0 + 60.0,
        x0 + ancho - 30.0,
        y0 + 60.0,
        1.5,
        TINTA,
    );

    let b = &world.boss;
    let mut y = y0 + 100.0;
    for i in 0..b.phase_count() {
        let (color, tachada) = match i.cmp(&b.phase) {
            std::cmp::Ordering::Less => (TEXT_PAPEL, true),
            std::cmp::Ordering::Equal => (TEXT_PAPEL_FUERTE, false),
            std::cmp::Ordering::Greater => (fade(TEXT_PAPEL, 0.7), false),
        };
        fuentes::centrado(&cartela::ordinal(i), cx, y, 12.0, Cara::Cuerpo, color);
        let nombre = b.figura(i);
        fuentes::centrado(nombre, cx, y + 24.0, 22.0, Cara::Titulo, color);
        if tachada {
            // Lo ya bailado, tachado a pluma, como en un programa usado.
            let mitad = fuentes::medir(nombre, 22.0, Cara::Titulo).width * 0.5 + 6.0;
            draw_line(cx - mitad, y + 17.0, cx + mitad, y + 17.0, 2.0, color);
        }
        if i == b.phase {
            // Y la que suena, senalada con una mano de imprenta: un triangulo.
            let ax = x0 + 16.0;
            draw_triangle(
                vec2(ax, y + 8.0),
                vec2(ax, y + 24.0),
                vec2(ax + 11.0, y + 16.0),
                TINTA,
            );
        }
        y += 64.0;
    }
}

/// Una orla de cartel: papel un punto mas claro, tinta gruesa y un filete.
fn orla(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, fade(WHITE, 0.18));
    draw_rectangle_lines(x, y, w, h, 3.5, TINTA);
    draw_rectangle_lines(x - 6.0, y - 6.0, w + 12.0, h + 12.0, 1.2, fade(TINTA, 0.55));
}

/// El paspartu: el papel de alrededor, puesto **encima** de lo que se sale.
///
/// Lo que desborda la lamina —balas que escapan, el sol del club, los tejados
/// del arrabal, la bambalina del Moulin Rouge— manchaba el papel. En un cartel
/// nada se sale de la plancha, asi que aqui tampoco: despues de pintar el
/// mundo, el papel de alrededor se vuelve a poner encima y lo tapa, y luego el
/// grano y el marco. Mas barato que un scissor y no toca el render instanciado
/// de las balas, que va por su cuenta.
pub(crate) fn paspartu(l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(ARENA_W), l.len(ARENA_H));
    let (sw, sh) = (screen_width(), screen_height());
    let papel = paleta::PAPEL;
    draw_rectangle(0.0, 0.0, sw, o.y, papel);
    draw_rectangle(0.0, o.y + h, sw, sh - o.y - h, papel);
    draw_rectangle(0.0, o.y, o.x, h, papel);
    draw_rectangle(o.x + w, o.y, sw - o.x - w, h, papel);

    let dentro = |x: f32, y: f32| x > o.x && x < o.x + w && y > o.y && y < o.y + h;
    GRANO.with(|motas| {
        for (x, y, r) in motas {
            let (px, py) = (x * sw, y * sh);
            if !dentro(px, py) {
                draw_circle(px, py, *r, fade(paleta::TINTA, 0.07));
            }
        }
    });
    marco(l);
}

/// El marco de la lamina: tinta gruesa y una esquina Deco en cada canto.
///
/// El borde sigue siendo informacion —dice hasta donde se puede llegar— y por
/// eso sigue nitido. Lo que cambia es que ahora tambien dice que lo de dentro
/// esta impreso.
fn marco(l: &Layout) {
    let o = l.to_screen(0.0, 0.0);
    let (w, h) = (l.len(ARENA_W), l.len(ARENA_H));
    let gordo = l.len(5.0);
    let fino = l.len(1.2);
    let aire = l.len(4.5);

    draw_rectangle_lines(o.x, o.y, w, h, gordo, paleta::TINTA);
    // Un filete por fuera, separado: es lo que hace que parezca una orla
    // impresa y no un rectangulo de programador.
    draw_rectangle_lines(
        o.x - aire,
        o.y - aire,
        w + aire * 2.0,
        h + aire * 2.0,
        fino,
        fade(paleta::TINTA, 0.55),
    );

    // Las esquinas. Dos trazos en angulo por canto, hacia dentro.
    let brazo = l.len(26.0);
    for (cx, cy, sx, sy) in [
        (o.x, o.y, 1.0, 1.0),
        (o.x + w, o.y, -1.0, 1.0),
        (o.x, o.y + h, 1.0, -1.0),
        (o.x + w, o.y + h, -1.0, -1.0),
    ] {
        let d = l.len(9.0);
        let (a, b) = (cx + sx * d, cy + sy * d);
        draw_line(a, b, a + sx * brazo, b, fino * 2.0, paleta::TINTA);
        draw_line(a, b, a, b + sy * brazo, fino * 2.0, paleta::TINTA);
    }
}

fn draw_arena(l: &Layout, baile: usize, tema: Tema, t: f32) {
    salon::dibujar(l, baile, tema, t);
}

/// La estela. Se muestrea a ritmo de tick en el core, asi que se ve igual a 60
/// que a 144 Hz.
fn draw_trail(world: &World, l: &Layout) {
    for (point, age) in world.player.trail.iter_newest_first() {
        // Cuadratico: la cola se apaga rapido y no deja una serpiente sucia.
        let strength = (1.0 - age) * (1.0 - age);
        let s = l.to_screen(point.pos.x, point.pos.y);
        let r = l.len(player::PLAYER_SPRITE_RADIUS * 0.8 * (1.0 - age));
        let color = if point.dashing { TRAIL_DASH } else { TRAIL };
        draw_circle(s.x, s.y, r, fade(color, strength * 0.5));
    }
}

fn draw_player(world: &World, alpha: f32, l: &Layout) {
    let p = world.player.render_pos(alpha);
    let s = l.to_screen(p.x, p.y);
    let sprite_r = l.len(player::PLAYER_SPRITE_RADIUS);

    // Fase continua en ticks: el parpadeo va al ritmo de la simulacion y no al
    // de los fps, asi que se ve igual en cualquier monitor.
    let t = world.tick as f32 + alpha;

    // Durante los i-frames el cuerpo palpita. Es la senal de "ahora mismo las
    // balas no te tocan", y tiene que leerse de un vistazo.
    let body_alpha = if world.player.is_invulnerable() {
        0.35 + 0.4 * ((t * 0.9).sin() * 0.5 + 0.5)
    } else {
        1.0
    };

    // El halo se queda: es lo que la separa del fondo cuando la pantalla se
    // llena. Lo que cambia es lo que hay dentro.
    draw_circle(s.x, s.y, sprite_r * 2.2, fade(PLAYER_GLOW, body_alpha));
    // La elegancia sube con cada figura superada: la bailarina baila mejor
    // segun avanza el vals. Es solo cosmetico, pero de un vistazo dice por
    // donde vas. Antes subia por jefe caido; ahora que el baile es un jefe,
    // sube por fase, que ademas se nota mas veces por partida.
    let figuras = world.boss.phase_count();
    let elegancia = if figuras > 1 {
        world.boss.phase as f32 / (figuras - 1) as f32
    } else {
        0.0
    };
    let figura = skeleton::pose(&world.player, t, world.mode == Mode::Platform, elegancia);
    // Un 60 % mas grande que el esqueleto. A tamano 1 media 27 pixeles, que es
    // un marcianito; a 1.4 ya era una bailarina, y a 1.6 ademas se le ve la
    // cara. La hitbox no cambia: es el punto de 2.5 que se dibuja encima de
    // todo, y ese es el que manda.
    let gesto = protagonista::Gesto::de(&world.player, t);
    protagonista::dibujar(&figura, &gesto, s, &l.escalado(TALLA), body_alpha, false);

    // Anillo de focus: se cierra sobre la hitbox conforme entras en modo lento.
    let ft = world.player.focus_t;
    if ft > 0.0 {
        let r = sprite_r * (2.0 - 0.8 * ft);
        draw_circle_lines(s.x, s.y, r, 1.5, fade(HITBOX, ft * 0.9));
    }

    // Radio de roce, visible solo en focus: es informacion util al aprender,
    // pero pintarla siempre ensuciaria la pantalla justo cuando mas limpia
    // tiene que estar.
    if ft > 0.0 {
        draw_circle_lines(
            s.x,
            s.y,
            l.len(player::GRAZE_RADIUS),
            1.0,
            fade(METER, ft * 0.25),
        );
    }

    // La ventana de parry abierta. Se dibuja el alcance real, ni mas ni menos:
    // que el jugador pueda aprender la distancia mirando.
    if world.player.is_parrying() {
        let r = l.len(player::PARRY_RADIUS);
        draw_circle(s.x, s.y, r, fade(PARRYABLE, 0.10));
        draw_circle_lines(s.x, s.y, r, 2.0, fade(PARRY_RING, 0.75));
    }

    // Fogonazo del super.
    if world.player.super_ticks > 0 {
        let k = world.player.super_ticks as f32 / player::SUPER_TICKS as f32;
        draw_circle(
            s.x,
            s.y,
            sprite_r * (3.0 + 28.0 * (1.0 - k)),
            fade(METER, k * 0.35),
        );
    }

    // La hitbox real. Siempre visible: aprender que casi todo el personaje es
    // decorativo es parte de aprender el juego.
    draw_circle(
        s.x,
        s.y,
        l.len(player::PLAYER_HITBOX_RADIUS) * (1.0 + 0.6 * ft),
        HITBOX,
    );
}

/// Dibuja la figura: falda, huesos arqueados, manos, zapatos, cabeza y tocado.
///
/// **Todo se pinta dos veces: engordado en tinta y luego del tamano real en
/// color.** Eso es un contorno de verdad —el mismo grosor en cada trazo, sin
/// post-proceso ni una segunda pasada de render— y es lo que separa un dibujo
/// de un monigote de neon. Antes eran estas mismas dos pasadas con la gorda
/// transparente, que es el truco contrario: en vez de recortar la figura contra
/// el fondo, la difuminaba.
pub(crate) fn draw_figura(
    p: &Pose,
    centro: Vec2,
    l: &Layout,
    alfa: f32,
    cuerpo: Color,
    tela: Color,
) {
    // Los jefes no llevan cintas y algunos no llevan falda. En vez de un campo
    // mas en la pose, se mira si la geometria es degenerada: si el ultimo punto
    // esta donde el primero, esa parte no existe.
    let hay_cintas = p.cintas[0][N_CINTA - 1] != p.cintas[0][0];
    let hay_falda = p.falda[0] != p.falda[N_FALDA - 1];
    let punto = |j: Vec2| centro + vec2(j.x, j.y) * l.scale();
    let cadera = punto(p.joints[skeleton::CADERA]);

    // --- Cintas ---
    // Van detras de todo: son lo mas lejano y lo mas tenue. No llevan contorno
    // porque son mas finas que la propia tinta.
    for cinta in p.cintas.iter().filter(|_| hay_cintas) {
        for k in 0..N_CINTA - 1 {
            let (a, b) = (punto(cinta[k]), punto(cinta[k + 1]));
            let t = 1.0 - k as f32 / (N_CINTA - 1) as f32;
            let g = l.len(0.8 * t).max(0.8);
            draw_line(a.x, a.y, b.x, b.y, g, fade(tela, alfa * 0.5 * t));
        }
    }

    // La silueta entera a un grosor. `engorde` la infla en unidades logicas:
    // con engorde y tinta sale el contorno, con cero y color sale el relleno.
    let silueta = |engorde: f32, c_cuerpo: Color, c_tela: Color| {
        // --- Falda ---
        // Plana y opaca, no dos capas translucidas. Un cartel se imprimia a
        // tintas planas, y ademas asi la falda tapa lo que hay detras en vez de
        // dejarlo entrever, que es la mitad de lo que hacia parecer esto un
        // esqueleto.
        if hay_falda {
            let bajo: Vec<Vec2> = p
                .falda
                .iter()
                .map(|q| {
                    let v = punto(*q) - cadera;
                    cadera + v.normalize_or_zero() * (v.length() + l.len(engorde))
                })
                .collect();
            for i in 0..N_FALDA - 1 {
                draw_triangle(cadera, bajo[i], bajo[i + 1], c_tela);
            }
            // Los costados del abanico son radios de la cadera: engordar hacia
            // fuera no los ensancha ni un pelo, asi que hay que trazarlos.
            if engorde > 0.0 {
                let g = l.len(engorde * 2.0);
                for extremo in [bajo[0], bajo[N_FALDA - 1]] {
                    draw_line(cadera.x, cadera.y, extremo.x, extremo.y, g, c_tela);
                }
            }
        }

        // --- Corpino ---
        // El torso deja de ser una linea y pasa a tener silueta. Se engorda
        // empujando cada esquina desde el centro, que en un cuadrilatero
        // convexo es exactamente un contorno.
        let q: Vec<Vec2> = p.corpino.iter().map(|v| punto(*v)).collect();
        let centroide = (q[0] + q[1] + q[2] + q[3]) * 0.25;
        let q: Vec<Vec2> = q
            .iter()
            .map(|v| *v + (*v - centroide).normalize_or_zero() * l.len(engorde))
            .collect();
        draw_triangle(q[0], q[1], q[2], c_cuerpo);
        draw_triangle(q[0], q[2], q[3], c_cuerpo);

        // --- Huesos ---
        for (a, b, r0, r1, arqueo) in HUESOS {
            hueso(
                punto(p.joints[a]),
                punto(p.joints[b]),
                r0 + engorde,
                r1 + engorde,
                arqueo,
                l,
                c_cuerpo,
            );
        }

        // --- Manos ---
        for (j, r) in REMATES {
            let m = punto(p.joints[j]);
            draw_circle(m.x, m.y, l.len(r + engorde), c_cuerpo);
        }

        // --- Zapatos ---
        // Un pie redondo se lee como una pelota; uno alargado y cruzado a la
        // espinilla se lee como un zapato. Apuntan hacia fuera, cada uno al
        // suyo, que es como se planta una bailarina.
        for (rodilla, pie, hacia) in [
            (skeleton::RODILLA_I, skeleton::PIE_I, 1.0),
            (skeleton::RODILLA_D, skeleton::PIE_D, -1.0),
        ] {
            let r = punto(p.joints[rodilla]);
            let f = punto(p.joints[pie]);
            let d = (f - r).normalize_or_zero();
            let delante = vec2(-d.y, d.x) * hacia;
            let ang = delante.y.atan2(delante.x).to_degrees();
            let c = f + delante * l.len(skeleton::ZAPATO.0 * 0.3);
            draw_ellipse(
                c.x,
                c.y,
                l.len(skeleton::ZAPATO.0 + engorde),
                l.len(skeleton::ZAPATO.1 + engorde),
                ang,
                c_cuerpo,
            );
        }

        // --- Cabeza y tocado ---
        let cabeza = punto(p.joints[skeleton::CABEZA]);
        draw_circle(
            cabeza.x,
            cabeza.y,
            l.len(skeleton::RADIO_CABEZA + engorde),
            c_cuerpo,
        );
        tocado(p.tocado, cabeza, punto(p.mono), engorde, l, c_cuerpo);
    };

    let tinta = fade(TINTA, alfa);
    silueta(TINTA_GRUESA, tinta, tinta);
    silueta(0.0, fade(cuerpo, alfa), fade(tela, alfa));
}

/// El fantasma: al morir, la bailarina sube flotando, se mece y se desvanece.
///
/// Es lo que hace Cuphead, y es lo mas amable que se puede hacer con una
/// muerte: no es un castigo, es una salida de escena. Dura poco mas de un
/// segundo, es casi transparente y no tapa balas. Lleva aureola, porque un
/// fantasma de dibujo animado sin aureola es una sabana.
pub fn fantasma(world: &World, donde: (f32, f32), edad: f32, l: &Layout) {
    const DURA: f32 = 1.4;
    if !(0.0..DURA).contains(&edad) {
        return;
    }
    let k = edad / DURA;
    let alfa = (1.0 - k) * 0.6;
    let vaiven = (edad * 6.0).sin() * 7.0;
    let centro = l.to_screen(donde.0, donde.1 - k * 110.0) + vec2(vaiven, 0.0);
    let ls = l.escalado(TALLA);
    let t = world.tick as f32;
    let pose = skeleton::pose(&world.player, t, false, 1.0);
    let gesto = protagonista::Gesto {
        cara: protagonista::Expresion::Alma,
        ..protagonista::Gesto::de(&world.player, t)
    };
    protagonista::dibujar(&pose, &gesto, centro, &ls, alfa, true);

    // La aureola, por encima del lazo.
    let (cabeza, r) = protagonista::cabeza(&pose);
    let (cabeza, r) = (centro + cabeza * ls.scale(), ls.len(r));
    draw_ellipse_lines(
        cabeza.x,
        cabeza.y - r * 2.0,
        r * 1.1,
        r * 0.35,
        0.0,
        2.0,
        fade(METER_FULL, alfa * 1.4),
    );
}

/// Que bailarin sale en el combate.
///
/// **Es `baile`, no `boss_index`**, y la diferencia se paso por alto varios
/// trozos. Desde que se entra a los bailes por la pista, `World::empezar_en`
/// deja un solo jefe en la lista, asi que `boss_index` vale siempre 0: los
/// cuatro jefes se dibujaban con el cuerpo y la coreografia del vals, solo
/// cambiados de color. La pareja del tango y las patadas del cancan no salian
/// en ninguna partida. La musica ya leia `baile`; el cuerpo no.
fn bailarin(world: &World) -> usize {
    world.baile
}

/// Lo que lleva en la cabeza, dibujado alrededor del punto `mono`.
///
/// Un cartel de epoca no dibuja caras: Toulouse-Lautrec resuelve una bailarina
/// entera con la silueta y el sombrero. Aqui igual, y sale gratis porque la
/// pose ya calcula el punto de encima de la cabeza.
fn tocado(t: skeleton::Tocado, cabeza: Vec2, mono: Vec2, engorde: f32, l: &Layout, c: Color) {
    use skeleton::{RADIO_CABEZA as R, Tocado};

    let arriba = (mono - cabeza).normalize_or_zero();
    let lado = vec2(-arriba.y, arriba.x);
    let grados = lado.y.atan2(lado.x).to_degrees();

    match t {
        // El rodete alto de la protagonista y del vals.
        Tocado::Mono => {
            let r = l.len(skeleton::RADIO_MONO * 1.35 + engorde);
            draw_circle(mono.x, mono.y, r, c);
        }
        // Pelo pegado y partido: el tango se baila con la cabeza quieta.
        Tocado::Liso => {
            let cen = cabeza + arriba * l.len(R * 0.42);
            let (w, h) = (l.len(R * 1.02 + engorde), l.len(R * 0.60 + engorde));
            draw_ellipse(cen.x, cen.y, w, h, grados, c);
        }
        // El casquete de los anos veinte, con la cinta a un lado.
        Tocado::Casquete => {
            let cen = cabeza + arriba * l.len(R * 0.34);
            let (w, h) = (l.len(R * 1.20 + engorde), l.len(R * 0.88 + engorde));
            draw_ellipse(cen.x, cen.y, w, h, grados, c);
            let lazo = cen + lado * l.len(R * 1.05);
            draw_circle(lazo.x, lazo.y, l.len(1.1 + engorde), c);
        }
        // El penacho del Moulin Rouge: tres plumas abriendose. Se reusa
        // `hueso`, asi que salen estrechandose y curvadas sin escribir nada.
        Tocado::Penacho => {
            for (sep, alto) in [(-0.55, 3.4), (0.0, 4.6), (0.55, 3.6)] {
                let dir = (arriba + lado * sep).normalize_or_zero();
                let punta = mono + dir * l.len(alto);
                hueso(mono, punta, 1.0 + engorde, 0.3 + engorde, sep * 1.4, l, c);
            }
        }
    }
}

/// Un hueso que se estrecha hacia la punta y **se arquea**.
///
/// Se traza con circulos solapados en vez de con un poligono: a este tamano se
/// ve igual, las uniones salen redondeadas gratis y son cuatro lineas.
///
/// El arqueo desplaza el control de una Bezier cuadratica en perpendicular al
/// hueso. Es lo que convierte un palo en una manguera: el trazo se curva y el
/// codo deja de verse, que es como se dibuja un brazo y como no se dibuja un
/// esqueleto. Con arqueo cero sale exactamente la recta de siempre.
fn hueso(a: Vec2, b: Vec2, r0: f32, r1: f32, arqueo: f32, l: &Layout, color: Color) {
    const PASOS: usize = 9;
    let desvio = l.len(arqueo);
    for i in 0..=PASOS {
        let t = i as f32 / PASOS as f32;
        let p = skeleton::trazo(a, b, desvio, t);
        let r = l.len(r0 + (r1 - r0) * t).max(0.6);
        draw_circle(p.x, p.y, r, color);
    }
}

/// Las cartas del HUD: la de vida y las del super, como en Cuphead.
///
/// Tres puntos y una barra son un HUD de programador. Una carta con "HP 3"
/// impresa se lee igual de rapido y dice de que epoca es el juego. Cuando queda
/// una vida la carta se pone roja y parpadea, que es lo que hace la de Cuphead,
/// y por lo mismo: es el unico momento en que hay que mirarla.
fn draw_cartas(world: &World, l: &Layout) {
    let o = l.to_screen(0.0, ARENA_H);
    let (x0, y) = (o.x + 12.0, o.y - 42.0);

    let vidas = world.lives;
    let critico = vidas <= 1 && !world.is_over();
    let parpadea = critico && (get_time() * 3.0).fract() < 0.5;
    carta(
        x0,
        y,
        54.0,
        30.0,
        if parpadea { DEFEAT } else { paleta::PAPEL },
    );
    fuentes::centrado(
        &format!("HP {vidas}"),
        x0 + 27.0,
        y + 22.0,
        19.0,
        Cara::Titulo,
        TINTA,
    );

    // El super en cinco cartas que se llenan de abajo arriba. Una barra
    // continua dice lo mismo, pero cinco cartas dicen ademas cuanto falta de
    // un vistazo.
    let m = world.player.meter_ratio();
    let lleno = world.player.meter_full();
    let relleno = if lleno { METER_FULL } else { METER };
    let (cw, ch) = (17.0, 26.0);
    for i in 0..5 {
        let x = x0 + 64.0 + i as f32 * (cw + 5.0);
        let (cy, k) = (y + 2.0, (m * 5.0 - i as f32).clamp(0.0, 1.0));
        carta(x, cy, cw, ch, paleta::PAPEL);
        if k > 0.0 {
            let alto = (ch - 4.0) * k;
            draw_rectangle(x + 2.0, cy + ch - 2.0 - alto, cw - 4.0, alto, relleno);
        }
    }

    // La etiqueta se queda: en las primeras partidas de verdad nadie supo para
    // que servia la barra hasta que dijo "SUPER".
    let tx = x0 + 64.0 + 5.0 * (cw + 5.0) + 4.0;
    con_sombra(
        |dx, c| fuentes::texto("SUPER", tx + dx, y + 21.0 + dx, 15.0, Cara::Cuerpo, c),
        if lleno { METER_FULL } else { TEXT },
    );
    if lleno {
        // Latido y aviso explicito de la tecla: si no lo dice, no existe.
        let t = (get_time() as f32 * 6.0).sin() * 0.5 + 0.5;
        con_sombra(
            |dx, c| {
                fuentes::texto(
                    "ESPACIO",
                    tx + 50.0 + dx,
                    y + 21.0 + dx,
                    19.0,
                    Cara::Cuerpo,
                    c,
                )
            },
            fade(METER_FULL, 0.55 + 0.45 * t),
        );
    }
}

/// Una carta: papel con borde de tinta.
fn carta(x: f32, y: f32, w: f32, h: f32, fondo: Color) {
    draw_rectangle(x, y, w, h, fondo);
    draw_rectangle_lines(x, y, w, h, 2.0, TINTA);
}

const X: f32 = 14.0;

fn put(text: &str, color: Color, y: &mut f32) {
    fuentes::texto(text, X, *y, 17.0, Cara::Cuerpo, color);
    *y += 18.0;
}

/// Bloque de rendimiento, comun al juego y a la escena de stress.
fn stats_block(stats: &FrameStats, steps: u32, y: &mut f32) {
    put(&format!("{:>6.1} fps", stats.fps()), TEXT, y);
    put(&format!("p50 {:>5.2} ms", stats.p50_ms()), TEXT_DIM, y);
    put(
        &format!("p99 {:>5.2} ms", stats.p99_ms()),
        // Rojo por encima de 20 ms: ahi ya se nota el tiron.
        if stats.p99_ms() > 20.0 {
            HITBOX
        } else {
            TEXT_DIM
        },
        y,
    );
    *y += 6.0;
    put(&format!("sim  {:>5.3} ms", stats.sim_ms()), TEXT_DIM, y);
    put(&format!("draw {:>5.3} ms", stats.render_ms()), TEXT_DIM, y);
    put(
        &format!("steps/frame {steps}"),
        if steps > 1 { HITBOX } else { TEXT_DIM },
        y,
    );
}

/// Overlay de la escena de stress. Sin jugador: solo cuenta y coste.
pub fn bench_overlay(
    stats: &FrameStats,
    bullets: &Bullets,
    target: usize,
    steps: u32,
    gpu: Option<&BulletRenderer>,
) {
    let mut y = 26.0;
    put("ESCENA DE STRESS", TEXT_PAPEL_ROJO, &mut y);
    y += 6.0;
    stats_block(stats, steps, &mut y);
    y += 6.0;
    put(
        &format!("balas    {:>6}", bullets.live_count()),
        TEXT,
        &mut y,
    );
    put(&format!("objetivo {:>6}", target), TEXT_PAPEL, &mut y);
    put(
        &format!("slots    {:>6}", bullets.scanned_slots()),
        TEXT_PAPEL,
        &mut y,
    );
    y += 6.0;
    match gpu {
        Some(r) => {
            put("render  INSTANCIADO", TEXT_PAPEL_FUERTE, &mut y);
            put(&format!("instancias {:>6}", r.drawn), TEXT_PAPEL, &mut y);
            put(&format!("culling    {:>6}", r.culled), TEXT_PAPEL, &mut y);
            put("draw calls      1", TEXT_PAPEL_FUERTE, &mut y);
        }
        None => {
            put("render  MACROQUAD", TEXT_PAPEL_ROJO, &mut y);
            put(
                &format!("draw calls {:>6}", bullets.live_count() * 2),
                TEXT_PAPEL_ROJO,
                &mut y,
            );
        }
    }
    fuentes::texto(
        "estos numeros van a docs/PERF.md",
        X,
        screen_height() - 14.0,
        15.0,
        Cara::Cuerpo,
        TEXT_PAPEL,
    );
}

pub fn debug_overlay(
    world: &World,
    stats: &FrameStats,
    steps: u32,
    chispas: usize,
    mando: Option<[&str; 4]>,
) {
    let mut y = 26.0;
    stats_block(stats, steps, &mut y);

    y += 6.0;
    put(&format!("tick    {}", world.tick), TEXT_PAPEL, &mut y);
    put(
        &format!(
            "figura  {}/{}  {}",
            world.boss.phase + 1,
            world.boss.phase_count(),
            world.boss.phase_name()
        ),
        TEXT_PAPEL,
        &mut y,
    );
    put(&format!("vidas   {}", world.lives), TEXT_PAPEL, &mut y);
    put(&format!("chispas {chispas}"), TEXT_PAPEL, &mut y);
    put(
        &format!("modo    {}", world.mode.nombre()),
        TEXT_PAPEL,
        &mut y,
    );
    put(
        &format!("balas   {}", world.bullets.live_count()),
        TEXT_PAPEL,
        &mut y,
    );
    put(
        &format!("muertes {}", world.player.deaths),
        if world.player.deaths > 0 {
            TEXT_PAPEL_ROJO
        } else {
            TEXT_PAPEL
        },
        &mut y,
    );

    y += 6.0;
    put(
        &format!("medidor {:>3.0}%", world.player.meter_ratio() * 100.0),
        if world.player.meter_full() {
            TEXT_PAPEL_FUERTE
        } else {
            TEXT_PAPEL
        },
        &mut y,
    );
    put(
        &format!(
            "parry {}  graze {}",
            world.player.parries, world.player.grazes
        ),
        TEXT_PAPEL,
        &mut y,
    );

    // Frame data del dash: es lo que hay que mirar para afinar el feel.
    y += 6.0;
    let d = &world.player.dash;
    put(
        &format!("vel  {:>5.0}", world.player.vel.length()),
        TEXT_PAPEL,
        &mut y,
    );
    put(
        &format!(
            "dash {:>2}  iframes {:>3}",
            d.ticks_left, world.player.iframes
        ),
        if world.player.is_invulnerable() {
            TEXT_PAPEL_ROJO
        } else {
            TEXT_PAPEL
        },
        &mut y,
    );
    put(
        &format!("cd   {:>2}  buffer  {:>3}", d.cooldown, d.buffer),
        TEXT_PAPEL,
        &mut y,
    );

    // Con los nombres del mando si hay mando. Esta es la linea que esta puesta
    // todo el rato, asi que es la que de verdad se lee: tenerla en teclado
    // mientras el menu decia otra cosa era peor que no tener ninguna.
    let help = match mando {
        Some(b) => format!(
            "{} disparar  {} dash  {} parry  {} super  L2 focus  M mudo  F1  R  F3 saltar",
            b[0], b[1], b[2], b[3]
        ),
        None => "Z disparar  X dash  C parry  ESPACIO super  SHIFT focus  M mudo  F1  R  F3 saltar"
            .to_owned(),
    };
    fuentes::texto(
        &help,
        X,
        screen_height() - 14.0,
        15.0,
        Cara::Cuerpo,
        TEXT_PAPEL,
    );
}

/// Aviso de una linea. Verde si fue bien, rojo si no.
pub fn banner(msg: &str, error: bool) {
    let color = if error {
        TEXT_PAPEL_ROJO
    } else {
        TEXT_PAPEL_FUERTE
    };
    fuentes::texto(msg, X, screen_height() - 38.0, 17.0, Cara::Cuerpo, color);
}

/// Chivato de grabacion. Se graba siempre, asi que conviene que se vea.
pub fn recording_badge(ticks: u64) {
    let texto = format!("REC {:>5}t   F2 guardar", ticks);
    let m = fuentes::medir(&texto, 15.0, Cara::Cuerpo);
    draw_circle(screen_width() - m.width - 26.0, 20.0, 4.0, TEXT_PAPEL_ROJO);
    fuentes::derecha(
        &texto,
        screen_width() - 16.0,
        25.0,
        15.0,
        Cara::Cuerpo,
        TEXT_PAPEL,
    );
}

/// Estado de la reproduccion de un replay.
pub fn replay_badge(tick: usize, total: usize, divergencia: Option<u64>) {
    let acabado = tick >= total;
    let texto = if acabado {
        format!("REPLAY  fin ({total}t)   R repetir")
    } else {
        format!("REPLAY  {tick}/{total}")
    };
    fuentes::derecha(
        &texto,
        screen_width() - 16.0,
        25.0,
        17.0,
        Cara::Cuerpo,
        TEXT_PAPEL_FUERTE,
    );

    if let Some(t) = divergencia {
        let aviso = format!("DIVERGENCIA en el tick {t}");
        fuentes::derecha(
            &aviso,
            screen_width() - 16.0,
            48.0,
            19.0,
            Cara::Cuerpo,
            TEXT_PAPEL_ROJO,
        );
    }
}

/// Las chispas, encima de todo.
pub fn particulas(p: &Particulas, l: &Layout) {
    p.draw(|q| l.to_screen(q.x, q.y), l.scale());
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::{Mode, World};

    #[test]
    fn cada_baile_se_dibuja_con_su_bailarin() {
        // El bug que vigila: se elegia el cuerpo con `boss_index`, que al entrar
        // por la pista vale siempre 0, y los cuatro jefes salian como el vals.
        for baile in 0..4 {
            let w = World::empezar_en(7, Mode::Flight, baile);
            assert_eq!(w.boss_index, 0, "la premisa: la pista deja un solo jefe");
            assert_eq!(bailarin(&w), baile, "el baile {baile} sale con otro cuerpo");
        }
    }

    #[test]
    fn el_recorrido_dice_cuanto_te_comiste() {
        // La barra del KO es lo que convierte cien intentos a ciegas en cien
        // intentos con informacion, asi que tiene que ser honesta.
        let mut w = World::with_mode(7, Mode::Flight);
        let figuras = w.boss.phase_count();
        assert!(
            figuras >= 2,
            "este test necesita un jefe con varias figuras"
        );

        // Recien empezado, casi nada.
        assert!(recorrido(&w) < 0.05, "empieza cerca de cero");

        // Y cuanto mas lejos llegas, mas marca. Es lo unico que se le pide: no
        // es una estadistica, es saber si te quedaste cerca.
        let mut antes = recorrido(&w);
        for fase in 1..figuras {
            w.boss.phase = fase;
            let ahora = recorrido(&w);
            assert!(
                ahora > antes,
                "la figura {fase} no marca mas que la anterior"
            );
            antes = ahora;
        }

        // Y nunca se sale de la barra, pase lo que pase con los indices.
        w.boss.phase = figuras + 5;
        let r = recorrido(&w);
        assert!((0.0..=1.0).contains(&r), "se ha salido de la barra: {r}");
    }
}
