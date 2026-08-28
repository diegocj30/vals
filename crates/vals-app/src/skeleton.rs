//! El personaje: un esqueleto de articulaciones animado por codigo.
//!
//! Nada de sprites. Se define una figura por sus huesos y se calculan las
//! articulaciones con cinematica directa, igual que las balas se dibujan con
//! una formula en vez de con una textura. Un personaje nuevo es un puñado de
//! longitudes y una funcion de pose.
//!
//! La pose es **funcion pura del tick y del estado del jugador**. No hay estado
//! de animacion que mantener, ni maquina de estados que sincronizar, ni riesgo
//! de que la animacion y la simulacion se desfasen: si el mundo esta en el tick
//! 900 y el jugador va a tal velocidad, la pose es esa y no hay otra.
//!
//! Y por eso se muestrea en ticks y no en frames: la figura se mueve igual a 60
//! que a 144 Hz, como la estela.

use macroquad::prelude::*;
use vals_core::math::{PI, sin_cos};
use vals_core::player::{self, Player};

/// Articulaciones, en unidades logicas y relativas al centro del jugador.
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

/// Que articulaciones une cada hueso.
pub const HUESOS: [(usize, usize); 10] = [
    (CADERA, PECHO),
    (PECHO, CABEZA),
    (PECHO, CODO_I),
    (CODO_I, MANO_I),
    (PECHO, CODO_D),
    (CODO_D, MANO_D),
    (CADERA, RODILLA_I),
    (RODILLA_I, PIE_I),
    (CADERA, RODILLA_D),
    (RODILLA_D, PIE_D),
];

// Proporciones, en unidades logicas. La figura mide unos 29 de alto, algo mas
// que el circulo que sustituye (radio 11), lo justo para leerse como una
// persona sin taparte media pantalla.
const TORSO: f32 = 7.0;
const CUELLO: f32 = 5.5;
pub const RADIO_CABEZA: f32 = 3.2;
const BRAZO: f32 = 6.0;
const ANTEBRAZO: f32 = 5.5;
const MUSLO: f32 = 7.0;
const PANTORRILLA: f32 = 6.5;

/// Arriba, en coordenadas de pantalla (la y crece hacia abajo).
const ARRIBA: f32 = -PI / 2.0;

/// La figura completa: solo articulaciones. La inclinacion ya esta metida en
/// ellas, no hace falta guardarla aparte.
pub struct Pose {
    pub joints: [Vec2; N_JOINTS],
}

fn desde(p: Vec2, ang: f32, largo: f32) -> Vec2 {
    let (s, c) = sin_cos(ang);
    p + vec2(c * largo, s * largo)
}

/// Calcula la pose del jugador para este instante.
///
/// `phase` es `tick + alpha`: continuo, en ticks, para que la animacion vaya al
/// ritmo de la simulacion y no al de los fotogramas.
pub fn pose(p: &Player, phase: f32, gravity: bool) -> Pose {
    let (bal_s, _) = sin_cos(phase * 0.075);
    let (paso_s, paso_c) = sin_cos(phase * 0.28);

    let vel_x = p.vel.x / player::PLAYER_SPEED;
    let rapidez = (p.vel.length() / player::PLAYER_SPEED).min(1.6);
    let focus = p.focus_t;

    // Inclinarse hacia donde se va. Es el gesto que mas hace que una figura
    // parezca que se mueve, por encima de mover los pies.
    let mut lean = (-vel_x * 0.30).clamp(-0.5, 0.5);
    // Vaiven de vals cuando esta parada: 3/4 lento. El juego se llama asi.
    let quieto = (1.0 - rapidez).max(0.0);
    lean += bal_s * 0.12 * quieto;

    let cadera = vec2(0.0, 2.0 + bal_s * 0.8 * quieto);
    let pecho = desde(cadera, ARRIBA + lean, TORSO);
    let cabeza = desde(pecho, ARRIBA + lean * 1.4, CUELLO);

    // --- Brazos ---
    // En reposo van abiertos y curvados, como una bailarina en segunda. Al
    // hacer focus se recogen. Y el parry es un port de bras: los brazos suben
    // por encima de la cabeza en un barrido.
    //
    // El parry se escribe como pose propia y no como un delta sobre la de
    // reposo: los brazos ya estan casi rectos ahi, asi que "abrir mas" no
    // cambiaba nada visible. Lo que se lee es que **suban**.
    let parry = p.is_parrying();
    let (apertura, alzado) = if parry {
        (0.62, 1.10)
    } else {
        (1.05 - focus * 0.75, 0.25 + focus * 0.55)
    };

    let hombro_i = ARRIBA + apertura + lean;
    let hombro_d = ARRIBA - apertura + lean;
    let codo_extra = 0.55;

    let codo_i = desde(pecho, hombro_i + bal_s * 0.10 * quieto, BRAZO);
    let mano_i = desde(codo_i, hombro_i + codo_extra - alzado, ANTEBRAZO);
    let codo_d = desde(pecho, hombro_d - bal_s * 0.10 * quieto, BRAZO);
    let mano_d = desde(codo_d, hombro_d - codo_extra + alzado, ANTEBRAZO);

    // --- Piernas ---
    let (ang_i, ang_d, flex_i, flex_d) = if gravity && !p.on_ground {
        // En el aire: una pierna recogida y otra estirada. Se lee al vuelo si
        // estas saltando o cayendo.
        let subiendo = p.vel.y < 0.0;
        if subiendo {
            (0.55, -0.15, 0.95, 0.15)
        } else {
            (0.30, -0.35, 0.35, 0.55)
        }
    } else if rapidez > 0.15 {
        // Zancada. La fase avanza con el reloj, no con la distancia recorrida:
        // mas simple y a esta escala no se nota.
        let amp = 0.55 * rapidez.min(1.0);
        (
            paso_s * amp,
            -paso_s * amp,
            0.30 + paso_c.max(0.0) * 0.5,
            0.30,
        )
    } else {
        // Parada: en punta, ligeramente abierta.
        (0.26, -0.30, 0.10, 0.05)
    };

    let cadera_i = ARRIBA + PI + ang_i + lean * 0.5;
    let cadera_d = ARRIBA + PI + ang_d + lean * 0.5;
    let rodilla_i = desde(cadera, cadera_i, MUSLO);
    let pie_i = desde(rodilla_i, cadera_i - flex_i, PANTORRILLA);
    let rodilla_d = desde(cadera, cadera_d, MUSLO);
    let pie_d = desde(rodilla_d, cadera_d - flex_d, PANTORRILLA);

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

    // Durante el dash la figura se estira en la direccion del movimiento. No es
    // una pose nueva: es la misma, aplastada. Sale casi gratis y se lee al
    // instante.
    if p.is_dashing() {
        let d = p.dash.dir;
        let (s, c) = (d.y, d.x);
        for j in joints.iter_mut() {
            let a_lo_largo = j.x * c + j.y * s;
            let cruz = -j.x * s + j.y * c;
            let (nx, ny) = (a_lo_largo * 1.45, cruz * 0.62);
            *j = vec2(nx * c - ny * s, nx * s + ny * c);
        }
    }

    Pose { joints }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vals_core::{InputFrame, World};

    fn jugador() -> Player {
        World::new(0).player
    }

    fn largo(p: &Pose, (a, b): (usize, usize)) -> f32 {
        (p.joints[a] - p.joints[b]).length()
    }

    #[test]
    fn los_huesos_no_cambian_de_longitud() {
        // Es la propiedad que hace que una figura articulada parezca una
        // figura y no una mancha: los huesos son rigidos.
        let esperado: Vec<f32> = {
            let p = pose(&jugador(), 0.0, false);
            HUESOS.iter().map(|h| largo(&p, *h)).collect()
        };

        let mut w = World::sandbox(0);
        for i in 0..400 {
            let input = InputFrame::from_bits((i * 37 % 512) as u16);
            w.step(input);
            let p = pose(&w.player, i as f32, false);
            for (k, h) in HUESOS.iter().enumerate() {
                // El dash deforma a proposito; ahi no aplica.
                if w.player.is_dashing() {
                    continue;
                }
                assert!(
                    (largo(&p, *h) - esperado[k]).abs() < 0.01,
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
                let p = pose(&w.player, i as f32, gravity);
                for j in p.joints {
                    assert!(
                        j.length() < 40.0,
                        "articulacion disparada a {j} en el tick {i}"
                    );
                    assert!(j.is_finite(), "articulacion no finita: {j}");
                }
            }
        }
    }

    #[test]
    fn la_pose_es_funcion_pura_del_tick_y_el_estado() {
        let p = jugador();
        let a = pose(&p, 123.5, false);
        let b = pose(&p, 123.5, false);
        assert_eq!(a.joints, b.joints);
    }

    #[test]
    fn el_parry_levanta_los_brazos() {
        let mut normal = World::sandbox(0);
        let mut parriando = World::sandbox(0);
        normal.step(InputFrame::NONE);
        parriando.step(InputFrame::from_bits(InputFrame::PARRY));
        assert!(parriando.player.is_parrying());

        let a = pose(&normal.player, 10.0, false);
        let b = pose(&parriando.player, 10.0, false);
        // Las manos suben: es el barrido por encima de la cabeza.
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
        let abierta = pose(&w.player, 0.0, false);
        for _ in 0..30 {
            w.step(InputFrame::from_bits(InputFrame::FOCUS));
        }
        let recogida = pose(&w.player, 30.0, false);
        let ancho = |p: &Pose| (p.joints[MANO_I].x - p.joints[MANO_D].x).abs();
        assert!(
            ancho(&recogida) < ancho(&abierta),
            "en focus deberia cerrarse: {} vs {}",
            ancho(&recogida),
            ancho(&abierta)
        );
    }

    #[test]
    fn en_el_aire_las_piernas_no_estan_como_en_el_suelo() {
        let mut w = World::sandbox_with_mode(0, vals_core::Mode::Platform);
        for _ in 0..40 {
            w.step(InputFrame::NONE);
        }
        assert!(w.player.on_ground);
        let suelo = pose(&w.player, 40.0, true);

        w.step(InputFrame::from_bits(InputFrame::JUMP));
        for _ in 0..6 {
            w.step(InputFrame::from_bits(InputFrame::JUMP));
        }
        assert!(!w.player.on_ground);
        let aire = pose(&w.player, 47.0, true);

        assert!(
            (suelo.joints[PIE_I] - aire.joints[PIE_I]).length() > 1.5,
            "saltando las piernas tienen que verse distintas"
        );
    }
}
