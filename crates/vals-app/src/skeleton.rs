//! La bailarina: un esqueleto de articulaciones animado por codigo.
//!
//! Nada de sprites. Se define la figura por sus huesos y se calculan las
//! articulaciones con cinematica directa, igual que las balas se dibujan con
//! una formula en vez de con una textura. Un personaje nuevo son unas
//! longitudes y una funcion de pose.
//!
//! La pose es **funcion pura del tick, del estado del jugador y de la gracia**.
//! No hay estado de animacion que mantener, ni maquina de estados que
//! sincronizar, ni riesgo de que la animacion y la simulacion se desfasen. Y
//! por eso se muestrea en ticks y no en frames: la figura se mueve igual a 60
//! que a 144 Hz, como la estela.
//!
//! La **gracia** es lo que hace que baile mejor segun caen los jefes. Es un
//! solo numero en `[0, 1]` que recorre toda la funcion: mas altura de brazos,
//! mas apertura de piernas, mas punta de pie, mas porte. Es puramente
//! cosmetico, asi que no hay nada que balancear — y solo sale barato porque la
//! figura es codigo: con sprites serian tres juegos de arte.

use macroquad::prelude::*;
use vals_core::math::{PI, sin_cos};
use vals_core::player::{self, Player};

pub const CADERA: usize = 0;
pub const PECHO: usize = 1;
pub const CABEZA: usize = 2;
pub const CODO_I: usize = 3;
pub const MANO_I: usize = 4;
pub const CODO_D: usize = 5;
pub const MANO_D: usize = 6;
pub const RODILLA_I: usize = 7;
pub const PIE_I: usize = 8;
pub const RODILLA_D: usize = 9;
pub const PIE_D: usize = 10;
pub const N_JOINTS: usize = 11;

/// Que articulaciones une cada hueso, y su grosor en cada extremo.
///
/// Los huesos se estrechan hacia la punta. Es la diferencia entre una figura
/// que parece un cuerpo y una que parece un monigote de palos.
pub const HUESOS: [(usize, usize, f32, f32); 10] = [
    (CADERA, PECHO, 2.3, 1.9),
    (PECHO, CABEZA, 1.7, 1.2),
    (PECHO, CODO_I, 1.5, 1.1),
    (CODO_I, MANO_I, 1.1, 0.7),
    (PECHO, CODO_D, 1.5, 1.1),
    (CODO_D, MANO_D, 1.1, 0.7),
    (CADERA, RODILLA_I, 1.9, 1.3),
    (RODILLA_I, PIE_I, 1.3, 0.8),
    (CADERA, RODILLA_D, 1.9, 1.3),
    (RODILLA_D, PIE_D, 1.3, 0.8),
];

/// Extremos que llevan un remate redondo: manos y pies.
pub const REMATES: [(usize, f32); 4] = [(MANO_I, 1.0), (MANO_D, 1.0), (PIE_I, 1.1), (PIE_D, 1.1)];

// Proporciones, en unidades logicas.
const TORSO: f32 = 7.0;
const CUELLO: f32 = 5.5;
pub const RADIO_CABEZA: f32 = 3.2;
/// El mono, encima de la cabeza. Un detalle diminuto que dice "bailarina" mas
/// rapido que cualquier otra cosa de la figura.
pub const RADIO_MONO: f32 = 1.6;
const BRAZO: f32 = 6.0;
const ANTEBRAZO: f32 = 5.5;
const MUSLO: f32 = 7.0;
const PANTORRILLA: f32 = 6.5;

/// Puntos del bajo de la falda.
pub const N_FALDA: usize = 9;
const FALDA_LARGO: f32 = 9.0;
/// Cuanto se queda atras la falda al moverse.
const FALDA_ARRASTRE: f32 = 7.0;

/// Arriba, en coordenadas de pantalla (la y crece hacia abajo).
const ARRIBA: f32 = -PI / 2.0;

/// La figura completa.
pub struct Pose {
    pub joints: [Vec2; N_JOINTS],
    /// Bajo de la falda, de un lado al otro. El vertice de arriba es la cadera.
    pub falda: [Vec2; N_FALDA],
    /// Centro del mono.
    pub mono: Vec2,
}

fn desde(p: Vec2, ang: f32, largo: f32) -> Vec2 {
    let (s, c) = sin_cos(ang);
    p + vec2(c * largo, s * largo)
}

/// Calcula la pose para este instante.
///
/// `phase` es `tick + alpha`: continuo, en ticks, para que la animacion vaya al
/// ritmo de la simulacion y no al de los fotogramas. `gracia` va de 0 a 1.
pub fn pose(p: &Player, phase: f32, gravity: bool, gracia: f32) -> Pose {
    let g = gracia.clamp(0.0, 1.0);
    let (bal_s, _) = sin_cos(phase * 0.075);
    let (paso_s, paso_c) = sin_cos(phase * 0.28);

    let vel_x = p.vel.x / player::PLAYER_SPEED;
    let rapidez = (p.vel.length() / player::PLAYER_SPEED).min(1.6);
    let focus = p.focus_t;
    let quieto = (1.0 - rapidez).max(0.0);

    // Inclinarse hacia donde se va. Es el gesto que mas hace que una figura
    // parezca que se mueve, por encima de mover los pies.
    let mut lean = (-vel_x * 0.30).clamp(-0.5, 0.5);
    // Vaiven de vals al estar parada: 3/4 lento, y mas amplio con la gracia.
    lean += bal_s * (0.10 + 0.06 * g) * quieto;

    // Con gracia se yergue: la cadera sube un poco y el porte cambia entero.
    let cadera = vec2(0.0, 2.0 - g * 1.2 + bal_s * (0.7 + 0.4 * g) * quieto);
    let pecho = desde(cadera, ARRIBA + lean, TORSO + g * 0.6);
    let cabeza = desde(pecho, ARRIBA + lean * 1.4, CUELLO);
    let mono = desde(cabeza, ARRIBA + lean * 1.4, RADIO_CABEZA + RADIO_MONO * 0.7);

    // --- Brazos ---
    // En reposo abiertos y curvados, como en segunda. Con focus se recogen. Y
    // el parry es un port de bras: suben por encima de la cabeza en un barrido.
    //
    // El parry se escribe como pose propia y no como un delta sobre la de
    // reposo: alli los brazos ya estan casi rectos, asi que "abrir mas" no
    // cambiaba nada visible. Lo que se lee es que **suban**.
    let (apertura, alzado) = if p.is_parrying() {
        (0.62, 1.10)
    } else {
        (
            1.05 - focus * 0.75 - g * 0.18,
            0.25 + focus * 0.55 + g * 0.30,
        )
    };

    let hombro_i = ARRIBA + apertura + lean;
    let hombro_d = ARRIBA - apertura + lean;
    let codo_extra = 0.55 - g * 0.20;
    let onda_brazo = bal_s * (0.10 + 0.08 * g) * quieto;

    let codo_i = desde(pecho, hombro_i + onda_brazo, BRAZO);
    let mano_i = desde(codo_i, hombro_i + codo_extra - alzado, ANTEBRAZO);
    let codo_d = desde(pecho, hombro_d - onda_brazo, BRAZO);
    let mano_d = desde(codo_d, hombro_d - codo_extra + alzado, ANTEBRAZO);

    // --- Piernas ---
    let (ang_i, ang_d, flex_i, flex_d) = if gravity && !p.on_ground {
        // En el aire: una recogida y otra estirada. Se lee al vuelo si subes o
        // caes. Con gracia, mas cerca de un grand jete.
        if p.vel.y < 0.0 {
            (0.55 + g * 0.25, -0.15 - g * 0.25, 0.95 - g * 0.35, 0.15)
        } else {
            (0.30, -0.35 - g * 0.20, 0.35, 0.55 - g * 0.25)
        }
    } else if rapidez > 0.15 {
        let amp = (0.55 + g * 0.20) * rapidez.min(1.0);
        (
            paso_s * amp,
            -paso_s * amp,
            0.30 + paso_c.max(0.0) * 0.5,
            0.30,
        )
    } else {
        // Parada: en punta y con la apertura que da la gracia.
        (0.26 + g * 0.22, -0.30 - g * 0.22, 0.10, 0.05)
    };

    let cadera_i = ARRIBA + PI + ang_i + lean * 0.5;
    let cadera_d = ARRIBA + PI + ang_d + lean * 0.5;
    let rodilla_i = desde(cadera, cadera_i, MUSLO);
    // La punta del pie se estira con la gracia: es el detalle que mas dice
    // "esto es baile" y no "esto camina".
    let pie_i = desde(rodilla_i, cadera_i - flex_i + g * 0.30, PANTORRILLA);
    let rodilla_d = desde(cadera, cadera_d, MUSLO);
    let pie_d = desde(rodilla_d, cadera_d - flex_d - g * 0.30, PANTORRILLA);

    let mut joints = [Vec2::ZERO; N_JOINTS];
    joints[CADERA] = cadera;
    joints[PECHO] = pecho;
    joints[CABEZA] = cabeza;
    joints[CODO_I] = codo_i;
    joints[MANO_I] = mano_i;
    joints[CODO_D] = codo_d;
    joints[MANO_D] = mano_d;
    joints[RODILLA_I] = rodilla_i;
    joints[PIE_I] = pie_i;
    joints[RODILLA_D] = rodilla_d;
    joints[PIE_D] = pie_d;

    // --- Falda ---
    // El arrastre se saca de la velocidad actual, no de un historial: asi la
    // pose sigue siendo una funcion pura y no hay estado de tela que mantener.
    // Conversion explicita: `p.vel` viene del core (glam 0.33) y aqui se
    // trabaja con el Vec2 de macroquad (glam 0.27). Es la frontera de la
    // arquitectura, y es a proposito que haya que cruzarla a mano.
    let v = vec2(p.vel.x, p.vel.y);
    let modulo = v.length().max(1e-4);
    let arrastre = -(v / modulo) * (modulo / player::PLAYER_SPEED).min(1.4) * FALDA_ARRASTRE;
    let mut falda = [Vec2::ZERO; N_FALDA];
    for (i, punto) in falda.iter_mut().enumerate() {
        let t = i as f32 / (N_FALDA - 1) as f32;
        let ang = PI * (0.13 + 0.74 * t);
        // Mas larga por el centro, como cae la tela.
        let (forma, _) = sin_cos(t * PI);
        let largo = FALDA_LARGO * (0.72 + 0.28 * forma) * (1.0 + g * 0.22);
        let (onda, _) = sin_cos(phase * 0.20 + t * 3.1);
        *punto = desde(cadera, ang, largo)
            + arrastre * (0.35 + 0.65 * forma)
            + vec2(0.0, onda * (0.5 + 0.5 * quieto));
    }

    // Durante el dash la figura se estira en la direccion del movimiento. No es
    // una pose nueva: es la misma, aplastada. Sale casi gratis y se lee al
    // instante.
    if p.is_dashing() {
        let d = p.dash.dir;
        let (s, c) = (d.y, d.x);
        let estirar = |j: &mut Vec2| {
            let a_lo_largo = j.x * c + j.y * s;
            let cruz = -j.x * s + j.y * c;
            let (nx, ny) = (a_lo_largo * 1.45, cruz * 0.62);
            *j = vec2(nx * c - ny * s, nx * s + ny * c);
        };
        for j in joints.iter_mut() {
            estirar(j);
        }
        for j in falda.iter_mut() {
            estirar(j);
        }
    }

    Pose {
        joints,
        falda,
        mono,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::{InputFrame, Mode, World};

    fn jugador() -> Player {
        World::new(0).player
    }

    fn largo(p: &Pose, a: usize, b: usize) -> f32 {
        (p.joints[a] - p.joints[b]).length()
    }

    #[test]
    fn los_huesos_no_cambian_de_longitud() {
        // Es la propiedad que hace que una figura articulada parezca una figura
        // y no una mancha: los huesos son rigidos.
        let esperado: Vec<f32> = {
            let p = pose(&jugador(), 0.0, false, 0.0);
            HUESOS.iter().map(|h| largo(&p, h.0, h.1)).collect()
        };

        let mut w = World::sandbox(0);
        for i in 0..400 {
            w.step(InputFrame::from_bits((i * 37 % 512) as u16));
            if w.player.is_dashing() {
                continue; // el dash deforma a proposito
            }
            let p = pose(&w.player, i as f32, false, 0.0);
            for (k, h) in HUESOS.iter().enumerate() {
                assert!(
                    (largo(&p, h.0, h.1) - esperado[k]).abs() < 0.01,
                    "el hueso {k} cambio de longitud en el tick {i}"
                );
            }
        }
    }

    #[test]
    fn la_figura_cabe_en_su_sitio() {
        let mut w = World::sandbox(0);
        for i in 0..400 {
            w.step(InputFrame::from_bits((i * 53 % 512) as u16));
            for gravity in [false, true] {
                for gracia in [0.0, 0.5, 1.0] {
                    let p = pose(&w.player, i as f32, gravity, gracia);
                    for j in p.joints.iter().chain(p.falda.iter()) {
                        assert!(j.length() < 45.0, "articulacion disparada a {j}");
                        assert!(j.is_finite(), "articulacion no finita: {j}");
                    }
                }
            }
        }
    }

    #[test]
    fn la_pose_es_funcion_pura_de_sus_entradas() {
        let p = jugador();
        let a = pose(&p, 123.5, false, 0.5);
        let b = pose(&p, 123.5, false, 0.5);
        assert_eq!(a.joints, b.joints);
        assert_eq!(a.falda, b.falda);
    }

    #[test]
    fn el_parry_levanta_los_brazos() {
        let mut normal = World::sandbox(0);
        let mut parriando = World::sandbox(0);
        normal.step(InputFrame::NONE);
        parriando.step(InputFrame::from_bits(InputFrame::PARRY));
        assert!(parriando.player.is_parrying());

        let a = pose(&normal.player, 10.0, false, 0.0);
        let b = pose(&parriando.player, 10.0, false, 0.0);
        let alto = |p: &Pose| (p.joints[MANO_I].y + p.joints[MANO_D].y) * 0.5;
        assert!(
            alto(&b) < alto(&a) - 4.0,
            "parriando las manos deberian subir: {} vs {}",
            alto(&b),
            alto(&a)
        );
    }

    #[test]
    fn el_focus_recoge_los_brazos() {
        let mut w = World::sandbox(0);
        let abierta = pose(&w.player, 0.0, false, 0.0);
        for _ in 0..30 {
            w.step(InputFrame::from_bits(InputFrame::FOCUS));
        }
        let recogida = pose(&w.player, 30.0, false, 0.0);
        let ancho = |p: &Pose| (p.joints[MANO_I].x - p.joints[MANO_D].x).abs();
        assert!(ancho(&recogida) < ancho(&abierta));
    }

    #[test]
    fn en_el_aire_las_piernas_no_estan_como_en_el_suelo() {
        let mut w = World::sandbox_with_mode(0, Mode::Platform);
        for _ in 0..40 {
            w.step(InputFrame::NONE);
        }
        assert!(w.player.on_ground);
        let suelo = pose(&w.player, 40.0, true, 0.0);

        for _ in 0..7 {
            w.step(InputFrame::from_bits(InputFrame::JUMP));
        }
        assert!(!w.player.on_ground);
        let aire = pose(&w.player, 47.0, true, 0.0);

        assert!((suelo.joints[PIE_I] - aire.joints[PIE_I]).length() > 1.5);
    }

    // --- Gracia ---

    #[test]
    fn con_gracia_se_yergue_y_abre_mas() {
        let p = jugador();
        let torpe = pose(&p, 0.0, false, 0.0);
        let elegante = pose(&p, 0.0, false, 1.0);

        // Mas porte: la cabeza queda mas alta.
        assert!(
            elegante.joints[CABEZA].y < torpe.joints[CABEZA].y - 1.5,
            "con gracia deberia erguirse: {} vs {}",
            elegante.joints[CABEZA].y,
            torpe.joints[CABEZA].y
        );

        // Mas apertura de piernas.
        let apertura = |q: &Pose| (q.joints[PIE_I].x - q.joints[PIE_D].x).abs();
        assert!(
            apertura(&elegante) > apertura(&torpe) + 1.0,
            "con gracia deberian abrirse mas los pies"
        );
    }

    #[test]
    fn la_gracia_no_rompe_los_huesos() {
        // Cambiar de gracia mueve las poses, pero los huesos siguen midiendo
        // lo mismo: es lo unico que no puede ceder.
        let p = jugador();
        let a = pose(&p, 0.0, false, 0.0);
        for gracia in [0.25, 0.5, 0.75, 1.0] {
            let b = pose(&p, 0.0, false, gracia);
            for h in HUESOS {
                // El torso crece a proposito con la gracia; el resto, no.
                if (h.0, h.1) == (CADERA, PECHO) {
                    continue;
                }
                assert!(
                    (largo(&b, h.0, h.1) - largo(&a, h.0, h.1)).abs() < 0.01,
                    "la gracia {gracia} deformo un hueso"
                );
            }
        }
    }

    #[test]
    fn la_falda_se_queda_atras_al_moverse() {
        let mut w = World::sandbox(0);
        let quieta = pose(&w.player, 0.0, false, 0.0);
        for _ in 0..20 {
            w.step(InputFrame::from_bits(InputFrame::RIGHT));
        }
        let corriendo = pose(&w.player, 20.0, false, 0.0);

        let centro = |q: &Pose| q.falda.iter().map(|f| f.x).sum::<f32>() / N_FALDA as f32;
        assert!(
            centro(&corriendo) < centro(&quieta) - 2.0,
            "yendo a la derecha la falda deberia quedarse a la izquierda: {} vs {}",
            centro(&corriendo),
            centro(&quieta)
        );
    }

    #[test]
    fn el_mono_va_encima_de_la_cabeza() {
        let p = pose(&jugador(), 0.0, false, 0.0);
        assert!(p.mono.y < p.joints[CABEZA].y);
        assert!((p.mono - p.joints[CABEZA]).length() < RADIO_CABEZA + RADIO_MONO + 0.5);
    }
}
