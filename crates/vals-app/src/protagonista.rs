//! La protagonista: la bailarina con cara y con lazo.
//!
//! Hasta aqui se dibujaba con `draw_figura`, igual que los jefes: huesos, una
//! falda de abanico y ninguna cara. Con los fondos ya
//! hechos, ella era lo que mas flojo se veia. Y la
//! causa es concreta: a la protagonista se la mira el combate entero, y una
//! figura sin cara no tiene a nadie dentro. Un cartel resuelve una bailarina
//! con la silueta porque se mira una vez; un personaje se mira mil.
//!
//! Lo que la convierte en personaje, en el orden en que se lee desde lejos:
//!
//! - **El lazo**: enorme, escarlata, encima de la cabeza. Es a ella lo que la
//!   taza a Cuphead: se reconoce desde la otra punta de la pantalla y ningun
//!   jefe lleva nada parecido. El escarlata tambien es suyo: los zapatos y el
//!   bajo del vestido, y nada mas en la pantalla es de ese color.
//! - **La cara**: cabeza grande, ojos que miran hacia donde va, cejas, boca y
//!   parpadeo. Y **el gesto sale del estado del jugador**, asi que la cara
//!   ademas informa: guina en el parry, aprieta en el dash, se asusta al
//!   recibir y sonrie chula con el super cargado.
//! - **El vestido de campana**, relleno y con enaguas, que se queda atras y se
//!   abre al moverse. En vez del abanico de triangulos.
//!
//! El esqueleto no cambia: la pose sigue saliendo de `skeleton::pose`, pura del
//! tick y del estado, y los miembros siguen siendo las mismas
//! mangueras. Lo nuevo es como se viste. Y todo lo de aqui es igual de puro:
//! el gesto, el parpadeo y el vuelo del vestido salen del jugador y de la fase,
//! sin ningun estado de animacion.
//!
//! **Cada pieza lleva su propio contorno**, pintada de atras a delante. En
//! `draw_figura` la silueta entera se pintaba primero en tinta y luego en color,
//! que da un solo contorno por fuera; aqui se quiere la linea de dentro —el brazo
//! sobre el vestido, el flequillo sobre la frente—, que es lo que hace que un
//! personaje parezca dibujado y no recortado.

use macroquad::prelude::*;
use vals_core::math::{PI, sin_cos};
use vals_core::player::{self, Player};

use crate::draw::Layout;
use crate::paleta::TINTA;
use crate::skeleton::{self, CABEZA, CADERA, CODO_D, CODO_I, HUESOS, MANO_D, MANO_I, PECHO, Pose};

// --- Tintas ---
// Tres planchas y poco mas, como el cartel: crema, escarlata y tinta. La piel y
// el pelo son lo unico que se sale, porque una cara necesita las dos.

/// El lazo y todo lo que es suyo: los zapatos y el bajo del vestido.
pub(crate) const LAZO: Color = color_u8!(222, 34, 52, 255);
/// El pliegue del lazo, para que tenga volumen sin dibujar una sombra.
const PLIEGUE: Color = color_u8!(140, 16, 36, 255);
/// El vestido va en crema por lo mismo que antes el cuerpo: es lo que mas
/// tiene que destacar sobre la tarima despues de las balas. Y porque la hitbox,
/// que es roja, cae en la cintura: sobre un vestido rojo desapareceria.
pub(crate) const VESTIDO: Color = color_u8!(244, 236, 216, 255);
const ENAGUA: Color = color_u8!(255, 252, 244, 255);
const MEDIA: Color = color_u8!(232, 222, 204, 255);
const PIEL: Color = color_u8!(250, 224, 196, 255);
const MEJILLA: Color = color_u8!(238, 120, 118, 150);
/// Pelirroja, como La Goulue en el cartel del Moulin Rouge. Y sirve: un pelo
/// negro se perderia contra la madera oscura.
const PELO: Color = color_u8!(150, 64, 40, 255);
const OJO: Color = color_u8!(255, 255, 255, 255);
const BOCA: Color = color_u8!(112, 20, 34, 255);
const LENGUA: Color = color_u8!(232, 104, 112, 255);
/// El fantasma: todo de un color, menos la tinta.
const ALMA: Color = color_u8!(250, 244, 226, 255);

// --- Medidas, en unidades de la pose ---

/// El contorno de las piezas. Menos que `TINTA_GRUESA` porque aqui hay lineas
/// por dentro, y con el mismo grosor la figura se empastaria.
const TINTA_ANCHO: f32 = 0.9;
/// Las lineas de la cara: cejas, boca, parpados.
const TRAZO: f32 = 0.45;
/// Por debajo de este ancho de ojo, en pixeles, el ojo se dibuja solo con la
/// pupila. Ver `dibujar_ojo`.
const OJO_MINIMO: f32 = 5.5;
/// La cabeza de dibujo animado: mas de un tercio de la figura. Es lo que hace
/// que la cara se lea a tamano de juego.
const R_CABEZA: f32 = 5.6;
/// Del pecho al centro de la cabeza. Deja un cuello corto, que es de dibujo.
const CUELLO: f32 = 6.8;
/// Cuanto sube la cintura del vestido desde la cadera. Alta, de dibujo: con la
/// cintura en la cadera el corpino salia largo como un delantal.
const CINTURA: f32 = 3.0;
/// Largo del vestido, de la cintura al bajo.
const LARGO_VESTIDO: f32 = 10.3;
/// Cuanto se queda atras el bajo a toda velocidad.
const VUELO: f32 = 4.5;

/// Que cara pone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Expresion {
    Normal,
    /// Dash: ojos entornados, cejas en uve, boca apretada.
    Decidida,
    /// Parry: guina un ojo. Es un gesto de siete ticks y tiene que leerse.
    Guino,
    /// Soltando el super: boca abierta de par en par.
    Grito,
    /// Recien golpeada: ojos apretados.
    Dolor,
    /// El resto de la invulnerabilidad tras el golpe: cejas arriba, boca en o.
    Susto,
    /// El super cargado: parpados a media asta y sonrisa de lado.
    Chula,
    /// El fantasma: ojos cerrados y en paz.
    Alma,
}

/// Todo lo que la cara y el vestido sacan del jugador, ademas de la pose.
#[derive(Clone, Copy, Debug)]
pub struct Gesto {
    pub cara: Expresion,
    /// Hacia donde miran las pupilas, de largo como mucho 1.
    pub mirada: Vec2,
    /// 0 con los ojos abiertos, 1 cerrados.
    pub parpado: f32,
    /// Hacia donde gira la cara: -1 a la izquierda, 1 a la derecha.
    pub giro: f32,
    /// Cuanto se queda atras el bajo del vestido, en unidades de la pose.
    pub vuelo: Vec2,
    /// La fase de la pose, para el vaiven del bajo y del lazo.
    pub fase: f32,
}

/// La invulnerabilidad mas larga que no viene de un golpe. Por encima de esto,
/// los i-frames son de haber muerto (`RESPAWN_IFRAME_TICKS`).
const SIN_GOLPE: u32 = mayor(
    mayor(player::DASH_IFRAME_TICKS, player::PARRY_IFRAME_TICKS),
    player::SUPER_TICKS,
);
/// Los ticks de ojos apretados al principio del golpe; el resto es susto.
const DOLOR: u32 = 40;

const fn mayor(a: u32, b: u32) -> u32 {
    if a > b { a } else { b }
}

/// El gesto que toca ahora.
///
/// **De lo que dura menos a lo que dura mas.** El parry son siete ticks y el
/// dash nueve: si no ganasen a todo, con los i-frames de un golpe o el super
/// cargado no se verian nunca, y son justo los gestos que dicen que el boton
/// ha funcionado.
pub fn expresion(j: &Player) -> Expresion {
    let golpe = j.iframes.saturating_sub(SIN_GOLPE);
    if j.is_parrying() {
        Expresion::Guino
    } else if j.is_dashing() {
        Expresion::Decidida
    } else if j.super_ticks > 0 {
        Expresion::Grito
    } else if golpe > player::RESPAWN_IFRAME_TICKS - SIN_GOLPE - DOLOR {
        Expresion::Dolor
    } else if golpe > 0 {
        Expresion::Susto
    } else if j.meter_full() {
        Expresion::Chula
    } else {
        Expresion::Normal
    }
}

/// Cuanto tiene cerrados los ojos, de 0 a 1.
///
/// Dos relojes que no se sincronizan: el parpadeo sale irregular, que es como
/// parpadea alguien vivo, sin azar ni estado. Un solo periodo se nota enseguida
/// y parece un metronomo.
pub fn parpadeo(fase: f32) -> f32 {
    const DURA: f32 = 8.0;
    [233.0f32, 347.0]
        .iter()
        .map(|periodo| {
            let t = fase.rem_euclid(*periodo);
            (1.0 - (t - DURA * 0.5).abs() / (DURA * 0.5)).max(0.0)
        })
        .fold(0.0, f32::max)
}

impl Gesto {
    pub fn de(j: &Player, fase: f32) -> Self {
        // `p.vel` viene del core (otro glam), asi que se cruza a mano, igual
        // que en la pose.
        let v = vec2(j.vel.x, j.vel.y) / player::PLAYER_SPEED;
        Self {
            cara: expresion(j),
            // Mira hacia donde va, y al frente si esta quieta.
            mirada: v.clamp_length_max(1.0),
            parpado: parpadeo(fase),
            giro: v.x.clamp(-1.0, 1.0),
            // Contra la marcha, como la falda de antes; el dash (900) satura.
            vuelo: -v.clamp_length_max(1.4) * VUELO,
            fase,
        }
    }
}

/// Centro y radio de la cabeza, en unidades de la pose.
///
/// Sale del cuello de la pose, pero mas lejos y mas grande que la del
/// esqueleto: la de `skeleton` es la de una figura de proporciones de persona.
pub fn cabeza(p: &Pose) -> (Vec2, f32) {
    let pecho = p.joints[PECHO];
    let arriba = (p.joints[CABEZA] - pecho).normalize_or_zero();
    (pecho + arriba * CUELLO, R_CABEZA)
}

/// Dibuja a la protagonista con los pies y la cintura donde dice la pose.
///
/// `alfa` es cuanto se ve (el parpadeo de los i-frames, el fantasma que se
/// desvanece) y `alma` la pinta de fantasma: todo de un color salvo la tinta.
pub fn dibujar(p: &Pose, g: &Gesto, centro: Vec2, l: &Layout, alfa: f32, alma: bool) {
    let pin = Pincel {
        centro,
        k: l.scale(),
        alma,
    };
    if alfa >= 1.0 {
        pintar(pin, p, g);
    } else {
        en_lamina(alfa, || pintar(pin, p, g));
    }
}

/// Pinta algo opaco en una lamina aparte y la pone encima con transparencia.
///
/// Con cada pieza transparente por separado se veia todo lo de debajo —la
/// tinta de las piernas a traves del vestido, el pelo a traves de la cara— y
/// la figura se volvia una mancha justo cuando mas tiene que leerse: los gestos
/// del golpe solo salen durante los i-frames, que es cuando parpadea. Un
/// fantasma de verdad es una figura entera que se transparenta, no un monton
/// de piezas.
///
/// La lamina es del tamano de la ventana y se guarda entre frames: se rehace
/// solo si cambia la ventana.
fn en_lamina(alfa: f32, pinta: impl FnOnce()) {
    thread_local! {
        static LAMINA: std::cell::RefCell<Option<RenderTarget>> = const { std::cell::RefCell::new(None) };
    }
    let (w, h) = (screen_width(), screen_height());
    let dpi = screen_dpi_scale();
    let (pw, ph) = ((w * dpi) as u32, (h * dpi) as u32);
    let lamina = LAMINA.with_borrow_mut(|l| {
        let vale = l
            .as_ref()
            .is_some_and(|r| (r.texture.width() as u32, r.texture.height() as u32) == (pw, ph));
        if !vale {
            let r = render_target(pw, ph);
            r.texture.set_filter(FilterMode::Nearest);
            *l = Some(r);
        }
        l.clone()
    });
    let Some(lamina) = lamina else { return };

    let mut camara = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w, h));
    camara.render_target = Some(lamina.clone());
    set_camera(&camara);
    clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
    pinta();
    set_default_camera();
    draw_texture_ex(
        &lamina.texture,
        0.0,
        0.0,
        Color::new(1.0, 1.0, 1.0, alfa),
        DrawTextureParams {
            dest_size: Some(vec2(w, h)),
            // Una lamina de macroquad sale boca abajo: la y de la textura
            // crece hacia arriba y la de la pantalla hacia abajo.
            flip_y: true,
            ..Default::default()
        },
    );
}

fn pintar(pin: Pincel, p: &Pose, g: &Gesto) {
    let j = &p.joints;
    let eje = (j[PECHO] - j[CADERA]).normalize_or_zero();
    let rapidez = g.vuelo.length() / VUELO;

    // --- Piernas ---
    // Medias blancas y zapatos de baile escarlata. Primero toda la tinta de las
    // dos, luego el color: asi la rodilla no deja una raya a media pierna.
    let piernas = [
        (miembro(p, [HUESOS[6], HUESOS[7]], 1.05, 0.8), 1.0),
        (miembro(p, [HUESOS[8], HUESOS[9]], 1.05, 0.8), -1.0),
    ];
    for (m, _) in &piernas {
        pin.tubo(m, TINTA_ANCHO, TINTA);
    }
    for (m, _) in &piernas {
        pin.tubo(m, 0.0, MEDIA);
    }
    for (m, hacia) in &piernas {
        // Un zapato alargado y cruzado a la espinilla, apuntando hacia fuera.
        let (pie, antes) = (m[m.len() - 1].0, m[m.len() - 3].0);
        let d = (pie - antes).normalize_or_zero();
        let delante = vec2(-d.y, d.x) * *hacia;
        let ang = delante.y.atan2(delante.x);
        pin.pieza(&ovalo(pie + delante * 0.8, 2.4, 1.3, ang), LAZO);
    }

    // --- Vestido ---
    let (silueta, bajo) = vestido(g, j[CADERA], eje, rapidez);
    let abajo = (vec2(0.0, 1.0) - eje).normalize_or_zero();
    // Las enaguas asoman por debajo, una onda por cada punto del bajo: es el
    // volante del cancan y dice "falda" sin dibujar ni un pliegue.
    for q in bajo.iter().step_by(2) {
        pin.pieza(&ovalo(*q + abajo * 0.5, 1.15, 1.0, 0.0), ENAGUA);
    }
    pin.pieza(&silueta, VESTIDO);
    // La franja escarlata del bajo, un poco por dentro del borde.
    let cintura = j[CADERA] + eje * CINTURA;
    let franja: Vec<Vec2> = bajo[1..bajo.len() - 1]
        .iter()
        .map(|q| q.lerp(cintura, 0.1) - abajo * 0.9)
        .collect();
    pin.linea(&franja, 1.0, LAZO);

    // --- Brazos y guantes ---
    // Despues del vestido, porque caen por delante de la falda; antes del
    // corpino, que tapa donde nacen.
    let brazos = [
        (
            miembro(p, [HUESOS[2], HUESOS[3]], 0.85, 0.72),
            CODO_I,
            MANO_I,
        ),
        (
            miembro(p, [HUESOS[4], HUESOS[5]], 0.85, 0.72),
            CODO_D,
            MANO_D,
        ),
    ];
    for (m, ..) in &brazos {
        pin.tubo(m, TINTA_ANCHO, TINTA);
        pin.tubo(m, 0.0, PIEL);
    }
    for (_, codo, mano) in brazos {
        // Guante blanco con su puno: un extremo grande es la mitad de la
        // lectura de un personaje dibujado.
        let d = (j[mano] - j[codo]).normalize_or_zero();
        let ang = d.y.atan2(d.x);
        pin.pieza(&ovalo(j[mano] - d * 1.2, 0.8, 1.35, ang), ENAGUA);
        pin.pieza(&ovalo(j[mano], 1.75, 1.6, ang), ENAGUA);
    }

    // --- Corpino y mangas de farol ---
    // El corpino de la pose llega a la cadera; aqui se corta en la cintura del
    // vestido, que va mas alta.
    let c = &p.corpino;
    let f = CINTURA / (j[PECHO] - j[CADERA]).length().max(1.0);
    let corpino = [c[0], c[1], c[2].lerp(c[1], f), c[3].lerp(c[0], f)];
    pin.pieza(&corpino, VESTIDO);
    for codo in [CODO_I, CODO_D] {
        let d = (j[codo] - j[PECHO]).normalize_or_zero();
        pin.pieza(
            &ovalo(j[PECHO] + d * 2.5, 2.3, 1.9, d.y.atan2(d.x)),
            VESTIDO,
        );
    }

    // --- Cuello, cabeza y lazo ---
    let (cabeza, r) = cabeza(p);
    let cuello = [(j[PECHO], 0.95), (cabeza, 0.95)];
    pin.tubo(&cuello, TINTA_ANCHO, TINTA);
    pin.tubo(&cuello, 0.0, PIEL);
    // La cabeza se inclina la mitad que el cuello: con lo que pesa, ladeada
    // del todo parecia que se caia.
    let arriba = ((cabeza - j[PECHO]).normalize_or_zero() + vec2(0.0, -1.0)).normalize_or_zero();
    dibujar_cabeza(&pin, g, Marco::nuevo(cabeza, arriba), r, rapidez);
}

/// Los ejes de la cabeza: `en(u, v)` es `u` a la derecha y `v` hacia abajo de
/// la cara, asi que todo lo de la cara se inclina con el cuello sin pensarlo.
struct Marco {
    c: Vec2,
    lado: Vec2,
    abajo: Vec2,
    giro: f32,
}

impl Marco {
    fn nuevo(c: Vec2, arriba: Vec2) -> Self {
        let lado = vec2(-arriba.y, arriba.x);
        Self {
            c,
            lado,
            abajo: -arriba,
            giro: lado.y.atan2(lado.x),
        }
    }

    fn en(&self, u: f32, v: f32) -> Vec2 {
        self.c + self.lado * u + self.abajo * v
    }

    fn ovalo(&self, u: f32, v: f32, rx: f32, ry: f32) -> Vec<Vec2> {
        ovalo(self.en(u, v), rx, ry, self.giro)
    }
}

/// Como esta cada ojo.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Ojo {
    /// Abierto, con el parpado bajado esta fraccion (0 del todo abierto).
    Abierto(f32),
    /// Cerrado en calma: una curva hacia abajo.
    Cerrado,
    /// Cerrado de gusto: una curva hacia arriba. El guino.
    Feliz,
    /// Apretado, en uve: el golpe.
    Apretado,
}

/// Como tiene los ojos y las cejas cada gesto, para el ojo del lado `s` (-1 el
/// de la izquierda de la pantalla, 1 el de la derecha).
///
/// Las cejas son `(alza, ceno)`: cuanto suben y cuanto baja la punta de dentro.
/// Un ceno positivo es enfado o empeno; negativo, pena o susto. Con dos numeros
/// salen todas las caras.
fn ojos(cara: Expresion, s: f32, parpado: f32) -> (Ojo, (f32, f32)) {
    let abierto = |tapa: f32| {
        let t = tapa.max(parpado);
        if t > 0.85 {
            Ojo::Cerrado
        } else {
            Ojo::Abierto(t)
        }
    };
    match cara {
        Expresion::Normal => (abierto(0.0), (0.0, 0.0)),
        Expresion::Decidida => (abierto(0.35), (-0.3, 0.8)),
        Expresion::Guino if s < 0.0 => (Ojo::Feliz, (-0.3, 0.3)),
        Expresion::Guino => (Ojo::Abierto(0.0), (0.6, -0.1)),
        Expresion::Grito => (Ojo::Abierto(0.0), (0.8, -0.2)),
        Expresion::Dolor => (Ojo::Apretado, (0.2, -0.7)),
        Expresion::Susto => (abierto(0.0), (0.7, -0.6)),
        Expresion::Chula if s > 0.0 => (abierto(0.4), (0.6, -0.1)),
        Expresion::Chula => (abierto(0.4), (0.0, 0.25)),
        Expresion::Alma => (Ojo::Cerrado, (0.3, -0.2)),
    }
}

/// La cabeza entera: pelo, cara, gesto y lazo.
fn dibujar_cabeza(pin: &Pincel, g: &Gesto, m: Marco, r: f32, rapidez: f32) {
    // La cara se gira un poco hacia donde va: los rasgos se corren a ese lado
    // y el pelo y el lazo al contrario. Es un tres cuartos de pobre, y basta.
    let gi = g.giro;
    let desvio = gi * 1.3;

    // --- Pelo y cara ---
    // Primero toda la tinta y luego los rellenos, para que el pelo de detras y
    // la cara salgan como una sola silueta.
    let pelo = m.ovalo(-gi * 0.6, -0.6, r * 1.1, r * 1.06);
    let rizos = [
        m.ovalo(-r * 0.93 - gi * 0.4, 1.4, 1.7, 2.1),
        m.ovalo(r * 0.93 - gi * 0.4, 1.4, 1.7, 2.1),
    ];
    let cara = m.ovalo(0.0, 0.0, r, r * 0.97);
    for pieza in [&pelo, &rizos[0], &rizos[1], &cara] {
        pin.contorno(pieza, TINTA_ANCHO * 2.0);
    }
    for pieza in [&pelo, &rizos[0], &rizos[1]] {
        pin.relleno(pieza, PELO);
    }
    pin.relleno(&cara, PIEL);
    // El flequillo, con su linea sobre la frente.
    pin.pieza(&m.ovalo(-gi * 0.9, -r * 0.84, r * 0.97, r * 0.36), PELO);

    // --- Mejillas ---
    for s in [-1.0, 1.0] {
        pin.relleno(&m.ovalo(s * 3.1 + desvio * 0.8, 2.4, 1.0, 0.62), MEJILLA);
    }

    // --- Ojos y cejas ---
    let (rx, ry) = (1.3, 1.9);
    for s in [-1.0f32, 1.0] {
        let (ojo, (alza, ceno)) = ojos(g.cara, s, g.parpado);
        // El ojo del lado contrario al giro queda al fondo y se estrecha.
        let rx = if s * gi < 0.0 {
            rx * (1.0 - 0.25 * gi.abs())
        } else {
            rx
        };
        let (u, v) = (s * 1.95 + desvio, 0.7);
        dibujar_ojo(pin, &m, (u, v), (rx, ry), s, ojo, g);

        let cv = v - ry - 0.95 - alza;
        let ceja = [
            m.en(u - s * 1.05, cv + ceno * 0.5),
            m.en(u, cv - 0.35),
            m.en(u + s * 1.1, cv - ceno * 0.5 + 0.15),
        ];
        pin.linea(&ceja, TRAZO * 1.3, TINTA);
    }

    // --- Boca ---
    dibujar_boca(pin, &m, desvio * 0.9, 3.6, g.cara);

    // --- El lazo ---
    // Encima de la cabeza y corrido hacia la nuca. Se queda un poco atras al
    // moverse y aletea, mas cuanto mas deprisa va.
    let (lento, _) = sin_cos(g.fase * 0.12);
    let (rapido, _) = sin_cos(g.fase * 0.45);
    let aleteo = lento * 0.06 + rapido * 0.18 * rapidez.min(1.0);
    let nudo = m.en(-gi * 1.4, -r * 0.95) + g.vuelo * 0.08;
    lazo(pin, &m, nudo, aleteo);
}

fn dibujar_ojo(
    pin: &Pincel,
    m: &Marco,
    (u, v): (f32, f32),
    (rx, ry): (f32, f32),
    s: f32,
    ojo: Ojo,
    g: &Gesto,
) {
    // Un punto en coordenadas del ojo, que son las de la cara.
    let en = |x: f32, y: f32| m.en(u + x, v + y);
    let arco = |a0: f32, a1: f32, achata: f32| -> Vec<Vec2> {
        (0..=8)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f32 / 8.0;
                en(rx * a.cos(), ry * achata * a.sin())
            })
            .collect()
    };
    match ojo {
        Ojo::Abierto(tapa) => {
            let susto = if g.cara == Expresion::Susto {
                0.75
            } else {
                1.0
            };
            if pin.k * rx < OJO_MINIMO {
                // A tamano de juego un ojo mide unos pocos pixeles, y blanco,
                // contorno y pupila se funden en una mancha gris. Un ovalo de
                // tinta con su brillo se lee siempre: es el ojo de los dibujos
                // de 1930, y por lo mismo, que se veian pequenos y en grano.
                let pupila = en(0.0, 0.0) + g.mirada * 0.3;
                pin.relleno(
                    &ovalo(pupila, rx * 0.72 * susto, ry * 0.8 * susto, m.giro),
                    TINTA,
                );
                pin.relleno(&ovalo(pupila + vec2(-0.3, -0.5), 0.35, 0.35, 0.0), OJO);
            } else {
                let blanco = m.ovalo(u, v, rx, ry);
                pin.contorno(&blanco, TRAZO * 2.0);
                pin.relleno(&blanco, OJO);
                // La pupila mira en coordenadas de pantalla, no de la cara:
                // hacia donde va ella, este como este la cabeza. Grande: con
                // las pequenas los ojos salian de loca.
                let (px, py) = (0.9 * susto, 1.25 * susto);
                let mirada = vec2(g.mirada.x * (rx - px), g.mirada.y * (ry - py)) * 0.85;
                let pupila = en(0.0, 0.1) + mirada;
                pin.relleno(&ovalo(pupila, px, py, m.giro), TINTA);
                pin.relleno(&ovalo(pupila + vec2(-0.25, -0.4), 0.3, 0.3, 0.0), OJO);
            }

            // El parpado: tapa lo de encima de una cuerda y deja la raya.
            let yc = -ry + 2.0 * ry * tapa;
            if tapa > 0.02 {
                let m0 = (yc / ry).clamp(-1.0, 1.0).asin();
                let tapado = arco(PI - m0, 2.0 * PI + m0, 1.0);
                pin.relleno(&tapado, PIEL);
                pin.linea(&[tapado[0], tapado[8]], TRAZO * 1.6, TINTA);
            }
            // Dos pestanas en la esquina de fuera, a la altura del parpado.
            let y = yc.max(-ry * 0.7);
            let x = s * rx * (1.0 - (y / ry).powi(2)).max(0.0).sqrt();
            let base = en(x, y);
            for (dx, dy) in [(0.8, -0.55), (0.95, 0.0)] {
                pin.linea(
                    &[base, base + m.lado * (s * dx) + m.abajo * dy],
                    TRAZO,
                    TINTA,
                );
            }
        }
        Ojo::Cerrado => pin.linea(&arco(0.0, PI, 0.4), TRAZO * 1.6, TINTA),
        Ojo::Feliz => pin.linea(&arco(PI, 2.0 * PI, 0.5), TRAZO * 1.6, TINTA),
        Ojo::Apretado => {
            // Una uve que apunta hacia la nariz: > <.
            let uve = [
                en(-s * rx, -ry * 0.55),
                en(s * rx * 0.7, 0.0),
                en(-s * rx, ry * 0.55),
            ];
            pin.linea(&uve, TRAZO * 1.7, TINTA);
        }
    }
}

fn dibujar_boca(pin: &Pincel, m: &Marco, u: f32, v: f32, cara: Expresion) {
    let en = |x: f32, y: f32| m.en(u + x, v + y);
    // Una curva de `x0` a `x1` que se comba `comba` por el medio (positiva,
    // hacia abajo: sonrisa) y se inclina `inclina` (la punta de la derecha).
    let curva = |x0: f32, x1: f32, comba: f32, inclina: f32| -> Vec<Vec2> {
        (0..=6)
            .map(|i| {
                let t = i as f32 / 6.0;
                let x = x0 + (x1 - x0) * t;
                en(x, comba * 4.0 * t * (1.0 - t) - inclina * (t - 0.5))
            })
            .collect()
    };
    let ancho = TRAZO * 1.4;
    match cara {
        Expresion::Normal | Expresion::Alma => {
            pin.linea(&curva(-0.9, 0.9, 0.45, 0.0), ancho, TINTA)
        }
        Expresion::Decidida => pin.linea(&curva(-0.8, 0.8, -0.2, 0.0), ancho, TINTA),
        Expresion::Guino => pin.linea(&curva(-0.7, 0.9, 0.3, 0.5), ancho, TINTA),
        Expresion::Susto => {
            let o = m.ovalo(u, v + 0.1, 0.45, 0.6);
            pin.contorno(&o, TRAZO * 2.0);
            pin.relleno(&o, BOCA);
        }
        Expresion::Dolor => {
            let o = m.ovalo(u, v + 0.1, 0.8, 0.55);
            pin.contorno(&o, TRAZO * 2.0);
            pin.relleno(&o, BOCA);
        }
        Expresion::Grito => {
            let o = m.ovalo(u, v + 0.3, 1.1, 1.15);
            pin.contorno(&o, TRAZO * 2.0);
            pin.relleno(&o, BOCA);
            pin.relleno(&m.ovalo(u, v + 0.95, 0.7, 0.4), LENGUA);
        }
        Expresion::Chula => {
            // Media luna ancha y ladeada, con los dientes arriba.
            let media = |hondo: f32| -> Vec<Vec2> {
                (0..=10)
                    .map(|i| {
                        let a = PI * i as f32 / 10.0;
                        en(1.45 * a.cos(), hondo * a.sin() - 0.25 * a.cos())
                    })
                    .collect()
            };
            let boca = media(1.1);
            pin.contorno(&boca, TRAZO * 2.0);
            pin.relleno(&boca, BOCA);
            pin.relleno(&media(0.4), OJO);
        }
    }
}

/// El lazo: dos lazadas, dos caidas y el nudo. Lo que es suyo y de nadie mas.
///
/// Las lazadas son ovalos ladeados hacia arriba, que se estrechan solos hacia
/// el nudo. Un pliegue oscuro junto al nudo le da volumen sin sombrear nada.
fn lazo(pin: &Pincel, m: &Marco, nudo: Vec2, aleteo: f32) {
    let arriba = -m.abajo;
    // Las caidas, detras de las lazadas.
    for s in [-1.0, 1.0] {
        let caida = [
            (nudo, 0.95),
            (nudo + m.lado * (s * 1.4) + m.abajo * 1.8, 0.85),
            (nudo + m.lado * (s * 2.2) + m.abajo * 3.4, 0.8),
        ];
        pin.tubo(&caida, TINTA_ANCHO, TINTA);
        pin.tubo(&caida, 0.0, LAZO);
    }
    let lazadas = [-1.0f32, 1.0].map(|s| {
        let c = nudo + m.lado * (s * 2.9) + arriba * 0.8;
        (s, ovalo(c, 3.2, 2.25, m.giro - s * (0.38 + aleteo)))
    });
    for (_, l) in &lazadas {
        pin.contorno(l, TINTA_ANCHO * 2.0);
    }
    for (s, l) in &lazadas {
        pin.relleno(l, LAZO);
        let pliegue = nudo + m.lado * (s * 1.5) + arriba * 0.4;
        pin.relleno(&ovalo(pliegue, 1.1, 1.0, m.giro), PLIEGUE);
        let raya = [
            nudo + m.lado * (s * 1.4),
            nudo + m.lado * (s * 3.6) + arriba * 1.3,
        ];
        pin.linea(&raya, TRAZO, TINTA);
    }
    pin.pieza(&ovalo(nudo, 1.2, 1.45, m.giro), LAZO);
}

/// Los puntos de un miembro de dos huesos, con su grosor, siguiendo la misma
/// curva que `draw_figura`: el arqueo sale de la tabla de huesos, asi que la
/// manguera es la de siempre.
fn miembro(
    p: &Pose,
    huesos: [(usize, usize, f32, f32, f32); 2],
    r0: f32,
    r1: f32,
) -> Vec<(Vec2, f32)> {
    const PASOS: usize = 8;
    let mut v = Vec::with_capacity(2 * PASOS + 2);
    for (k, (a, b, _, _, arqueo)) in huesos.into_iter().enumerate() {
        for i in 0..=PASOS {
            let t = i as f32 / PASOS as f32;
            let u = (k as f32 + t) * 0.5;
            let q = skeleton::trazo(p.joints[a], p.joints[b], arqueo, t);
            v.push((q, r0 + (r1 - r0) * u));
        }
    }
    v
}

/// La campana del vestido: la silueta entera y, aparte, los puntos del bajo.
///
/// Cuelga a medio camino entre el tronco y la plomada, que es lo que hace la
/// tela. Los costados son curvas hinchadas —una campana, no un triangulo— y el
/// bajo se ondula con la fase. Al moverse se abre y **se queda atras**, mas
/// cuanto mas abajo: la cintura va pegada al cuerpo y el bajo arrastra.
fn vestido(g: &Gesto, cadera: Vec2, eje: Vec2, rapidez: f32) -> (Vec<Vec2>, Vec<Vec2>) {
    const COSTADO: usize = 6;
    const BAJO: usize = 14;
    let abajo = (vec2(0.0, 1.0) - eje).normalize_or_zero();
    let ancho = vec2(-abajo.y, abajo.x);
    let cintura = cadera + eje * CINTURA;
    let (wc, wh) = (2.4, 6.4 * (1.0 + 0.22 * rapidez.min(1.4)));
    let costado = |s: f32, t: f32| {
        let a = cintura + ancho * (wc * s);
        let control = cintura + abajo * (LARGO_VESTIDO * 0.3) + ancho * (wh * 0.95 * s);
        let b = cintura + abajo * LARGO_VESTIDO + ancho * (wh * s);
        let u = 1.0 - t;
        a * (u * u) + control * (2.0 * u * t) + b * (t * t)
    };

    let mut silueta = Vec::with_capacity(2 * COSTADO + BAJO + 1);
    for i in 0..=COSTADO {
        silueta.push(costado(1.0, i as f32 / COSTADO as f32));
    }
    for i in 1..BAJO {
        let t = i as f32 / BAJO as f32;
        let (onda, _) = sin_cos(g.fase * 0.2 + t * 3.1);
        let (combado, _) = sin_cos(t * PI);
        silueta.push(
            cintura
                + ancho * (wh * (1.0 - 2.0 * t))
                + abajo * (LARGO_VESTIDO + 0.9 * combado + onda * 0.45),
        );
    }
    for i in (0..=COSTADO).rev() {
        silueta.push(costado(-1.0, i as f32 / COSTADO as f32));
    }
    for q in silueta.iter_mut() {
        let hondo = ((*q - cintura).dot(abajo) / LARGO_VESTIDO).clamp(0.0, 1.2);
        *q += g.vuelo * hondo.powf(1.4);
    }
    let bajo = silueta[COSTADO..=COSTADO + BAJO].to_vec();
    (silueta, bajo)
}

/// Un ovalo como lista de puntos, girado `giro` radianes.
///
/// Propio y no `draw_ellipse` porque hace falta el contorno como linea, y
/// porque los veinte lados de macroquad se ven a la escala del titulo.
fn ovalo(c: Vec2, rx: f32, ry: f32, giro: f32) -> Vec<Vec2> {
    const LADOS: usize = 28;
    let (s, co) = giro.sin_cos();
    (0..LADOS)
        .map(|i| {
            let a = i as f32 / LADOS as f32 * std::f32::consts::TAU;
            let (x, y) = (rx * a.cos(), ry * a.sin());
            c + vec2(x * co - y * s, x * s + y * co)
        })
        .collect()
}

/// Lo que pinta, en unidades de la pose alrededor de `centro`.
#[derive(Clone, Copy)]
struct Pincel {
    centro: Vec2,
    k: f32,
    alma: bool,
}

impl Pincel {
    fn p(&self, v: Vec2) -> Vec2 {
        self.centro + v * self.k
    }

    fn tinte(&self, c: Color) -> Color {
        if self.alma && c != TINTA {
            Color { a: c.a, ..ALMA }
        } else {
            c
        }
    }

    /// Relleno en abanico desde el centroide. Vale para todo lo de aqui, que
    /// es convexo o casi.
    fn relleno(&self, pts: &[Vec2], c: Color) {
        let c = self.tinte(c);
        let cen = self.p(pts.iter().copied().sum::<Vec2>() / pts.len() as f32);
        for (i, a) in pts.iter().enumerate() {
            let b = pts[(i + 1) % pts.len()];
            draw_triangle(cen, self.p(*a), self.p(b), c);
        }
    }

    /// Una linea quebrada con las juntas redondas. Nunca por debajo de un
    /// pixel: una linea de cara que desaparece es una cara que desaparece.
    fn linea(&self, pts: &[Vec2], grosor: f32, c: Color) {
        let g = (grosor * self.k).max(1.0);
        let c = self.tinte(c);
        for w in pts.windows(2) {
            let (a, b) = (self.p(w[0]), self.p(w[1]));
            draw_line(a.x, a.y, b.x, b.y, g, c);
        }
        for q in pts {
            let q = self.p(*q);
            draw_circle(q.x, q.y, g * 0.5, c);
        }
    }

    /// El contorno cerrado de una pieza, en tinta.
    fn contorno(&self, pts: &[Vec2], grosor: f32) {
        let mut cerrado = pts.to_vec();
        cerrado.push(pts[0]);
        self.linea(&cerrado, grosor, TINTA);
    }

    /// Una pieza: contorno de tinta y el relleno encima, que se come la mitad
    /// de dentro de la linea.
    fn pieza(&self, pts: &[Vec2], c: Color) {
        self.contorno(pts, TINTA_ANCHO * 2.0);
        self.relleno(pts, c);
    }

    /// Una manguera de grosor variable, engordada `engorde`: con tinta y
    /// engorde sale el contorno, con cero y color el relleno.
    fn tubo(&self, pts: &[(Vec2, f32)], engorde: f32, c: Color) {
        let c = self.tinte(c);
        for w in pts.windows(2) {
            let ((a, ra), (b, rb)) = (w[0], w[1]);
            let d = (b - a).normalize_or_zero();
            let n = vec2(-d.y, d.x);
            let (ra, rb) = (ra + engorde, rb + engorde);
            let (a1, a2) = (self.p(a + n * ra), self.p(a - n * ra));
            let (b1, b2) = (self.p(b + n * rb), self.p(b - n * rb));
            draw_triangle(a1, a2, b1, c);
            draw_triangle(a2, b2, b1, c);
        }
        for (q, r) in pts {
            let q = self.p(*q);
            draw_circle(q.x, q.y, (r + engorde) * self.k, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::World;

    fn jugadora() -> Player {
        World::sandbox(0).player
    }

    #[test]
    fn cada_estado_pone_su_cara() {
        let base = jugadora();
        assert_eq!(expresion(&base), Expresion::Normal);

        let con = |poner: fn(&mut Player)| {
            let mut j = base.clone();
            poner(&mut j);
            expresion(&j)
        };
        assert_eq!(con(|j| j.parry_window = 3), Expresion::Guino);
        assert_eq!(con(|j| j.dash.ticks_left = 4), Expresion::Decidida);
        assert_eq!(con(|j| j.super_ticks = 5), Expresion::Grito);
        assert_eq!(
            con(|j| j.iframes = player::RESPAWN_IFRAME_TICKS),
            Expresion::Dolor
        );
        assert_eq!(con(|j| j.iframes = SIN_GOLPE + 1), Expresion::Susto);
        assert_eq!(con(|j| j.meter = player::METER_MAX), Expresion::Chula);
    }

    #[test]
    fn los_iframes_de_un_boton_no_son_un_golpe() {
        // El dash, el parry y el super tambien dan invulnerabilidad, y al
        // acabar dejan unos ticks colgando. Si eso contase como golpe, cada
        // dash acabaria con cara de susto.
        for iframes in [
            player::DASH_IFRAME_TICKS,
            player::PARRY_IFRAME_TICKS,
            player::SUPER_TICKS,
        ] {
            let mut j = jugadora();
            j.iframes = iframes;
            assert_eq!(expresion(&j), Expresion::Normal, "iframes {iframes}");
        }
    }

    #[test]
    fn tras_un_golpe_pasa_del_dolor_al_susto_y_a_la_calma() {
        // La cuenta atras de los i-frames del respawn, tick a tick: primero
        // ojos apretados, luego susto, y nunca se vuelve atras.
        let mut j = jugadora();
        let mut vistas = Vec::new();
        for iframes in (0..=player::RESPAWN_IFRAME_TICKS).rev() {
            j.iframes = iframes;
            let cara = expresion(&j);
            if vistas.last() != Some(&cara) {
                vistas.push(cara);
            }
        }
        assert_eq!(
            vistas,
            [Expresion::Dolor, Expresion::Susto, Expresion::Normal]
        );
    }

    #[test]
    fn lo_breve_gana_a_lo_largo() {
        // Con el super cargado y recien golpeada, el parry y el dash siguen
        // viendose: son los gestos que dicen que el boton ha funcionado.
        let mut j = jugadora();
        j.iframes = player::RESPAWN_IFRAME_TICKS;
        j.meter = player::METER_MAX;
        j.dash.ticks_left = 4;
        assert_eq!(expresion(&j), Expresion::Decidida);
        j.parry_window = 3;
        assert_eq!(expresion(&j), Expresion::Guino);
    }

    #[test]
    fn parpadea_de_vez_en_cuando_y_no_a_compas() {
        // Diez minutos a 60 Hz.
        let cerrados: Vec<u32> = (0..36_000u32)
            .filter(|t| parpadeo(*t as f32) > 0.85)
            .collect();
        let fraccion = cerrados.len() as f32 / 36_000.0;
        assert!(fraccion < 0.03, "se pasa el rato con los ojos cerrados");

        // Cada parpadeo es un tramo de ticks seguidos; mira los huecos.
        let inicios: Vec<u32> = cerrados
            .windows(2)
            .filter(|w| w[1] - w[0] > 1)
            .map(|w| w[1])
            .collect();
        let huecos: Vec<u32> = inicios.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(huecos.len() > 100, "casi no parpadea");
        assert!(
            huecos.iter().all(|h| *h < 6 * 60),
            "pasa mas de 6 s sin parpadear"
        );
        assert!(
            huecos.iter().any(|h| *h != huecos[0]),
            "parpadea como un metronomo"
        );
        for t in 0..1000 {
            let p = parpadeo(t as f32 * 0.37);
            assert!((0.0..=1.0).contains(&p));
        }
    }

    #[test]
    fn mira_y_se_gira_hacia_donde_va() {
        let mut j = jugadora();
        let quieta = Gesto::de(&j, 0.0);
        assert_eq!(quieta.mirada, Vec2::ZERO);
        assert_eq!(quieta.giro, 0.0);

        j.vel.x = player::PLAYER_SPEED;
        let derecha = Gesto::de(&j, 0.0);
        assert!(derecha.mirada.x > 0.9 && derecha.giro > 0.9);

        // El dash va al triple: la pupila no se sale del ojo.
        j.vel.x = -player::DASH_SPEED;
        j.vel.y = player::DASH_SPEED;
        let dash = Gesto::de(&j, 0.0);
        assert!(dash.mirada.length() <= 1.0 + 1e-5);
        assert_eq!(dash.giro, -1.0);
        assert!(dash.mirada.y > 0.0, "si baja, mira hacia abajo");
    }

    #[test]
    fn el_vestido_se_queda_atras_y_se_abre_al_moverse() {
        let mut j = jugadora();
        let (cadera, eje) = (vec2(0.0, 2.0), vec2(0.0, -1.0));
        let vestir = |j: &Player| {
            let g = Gesto::de(j, 0.0);
            vestido(&g, cadera, eje, g.vuelo.length() / VUELO).1
        };
        let centro = |b: &[Vec2]| b.iter().map(|q| q.x).sum::<f32>() / b.len() as f32;
        let ancho = |b: &[Vec2]| (b[0].x - b[b.len() - 1].x).abs();

        let quieta = vestir(&j);
        j.vel.x = player::PLAYER_SPEED;
        let corriendo = vestir(&j);
        assert!(
            centro(&corriendo) < centro(&quieta) - 2.0,
            "yendo a la derecha el bajo deberia quedarse a la izquierda"
        );
        assert!(ancho(&corriendo) > ancho(&quieta) + 1.0, "no se abre");
    }
}
