//! Los jefes, que son bailarines.
//!
//! Hasta ahora un jefe era un hexagono, un triangulo y dos circulos girando.
//! Funcionaba, pero decia "marcianitos" a gritos. El proyecto ya lo tenia
//! resuelto desde hace tiempo —los personajes son esqueletos, no sprites— y la
//! protagonista lo demuestra desde H9: solo faltaba aplicarlo.
//!
//! Se reutiliza el esqueleto de `skeleton`: las mismas once articulaciones, los
//! mismos huesos y el mismo dibujo. Lo que cambia por jefe son dos cosas:
//!
//! - **El cuerpo** (`Cuerpo`): proporciones. Uno alto y de vestido largo, otro
//!   compacto y anguloso, otro bajo y ancho con flecos.
//! - **El baile**: como se mueven esas articulaciones. Y aqui esta la gracia,
//!   porque **cada uno se mueve como su gramatica de balas**. El vals gira, el
//!   tango se para en seco, el charleston rebota en la clave 3-3-2.
//!
//! Como en la protagonista, **la pose es funcion pura** del tick, la figura y
//! la vida que le queda. Ni estado de animacion, ni maquina de estados, ni
//! riesgo de que la animacion se desfase de la simulacion.
//!
//! El tango devuelve **dos figuras**: es una pareja. Por eso todo esto devuelve
//! una lista y no una pose suelta.

use macroquad::prelude::*;
use vals_core::math::{PI, TAU, sin_cos};

use crate::skeleton::{
    CABEZA, CADERA, CODO_D, CODO_I, MANO_D, MANO_I, N_CINTA, N_FALDA, N_JOINTS, PECHO, PIE_D,
    PIE_I, Pose, RADIO_CABEZA, RADIO_MONO, RODILLA_D, RODILLA_I, Tocado,
};

/// Cuanto mas grande que la protagonista se dibuja un jefe.
///
/// La arena mide 640x800 y el radio de golpeo del jefe es 46, o sea un aro de
/// 92 de diametro. Una bailarina mide unas 32 unidades de pies a cabeza, asi
/// que a 3.2 el cuerpo **llena** el aro: lo que ves es lo que le das.
///
/// Antes estaba a 2.1 con este mismo comentario, y era falso: la figura se
/// quedaba en dos tercios del aro. Un jefe que no llena su propio aro se lee
/// como un marcianito dentro de un circulo, que es justo lo que se estaba
/// intentando dejar de parecer.
pub const ESCALA: f32 = 3.2;

/// Arriba, en coordenadas de pantalla (la y crece hacia abajo).
const ARRIBA: f32 = -PI / 2.0;

fn desde(p: Vec2, ang: f32, largo: f32) -> Vec2 {
    let (s, c) = sin_cos(ang);
    p + vec2(c * largo, s * largo)
}

/// Las proporciones de un jefe.
#[derive(Clone, Copy)]
pub struct Cuerpo {
    pub torso: f32,
    pub cuello: f32,
    pub brazo: f32,
    pub antebrazo: f32,
    pub muslo: f32,
    pub pantorrilla: f32,
    pub hombros: f32,
    pub cintura: f32,
    /// Largo de la falda. Cero es sin falda.
    pub falda: f32,
    /// Lo que se abre la falda. Un vestido de vals cae; unos flecos vuelan.
    pub vuelo: f32,
    /// Lo que lleva en la cabeza. Sin caras, el tocado es lo unico que
    /// distingue una silueta de otra estando quieta.
    pub tocado: Tocado,
}

/// Alto y estrecho, de vestido largo.
const CUERPO_VALS: Cuerpo = Cuerpo {
    torso: 8.2,
    cuello: 6.0,
    brazo: 6.4,
    antebrazo: 5.8,
    muslo: 7.6,
    pantorrilla: 7.0,
    hombros: 3.2,
    cintura: 2.0,
    falda: 13.0,
    vuelo: 0.30,
    tocado: Tocado::Mono,
};

/// Compacto y anguloso. Sin falda el que lleva, con falda corta el que sigue.
const CUERPO_TANGO: Cuerpo = Cuerpo {
    torso: 6.6,
    cuello: 5.0,
    brazo: 6.0,
    antebrazo: 5.6,
    muslo: 6.8,
    pantorrilla: 6.4,
    hombros: 4.0,
    cintura: 2.6,
    falda: 0.0,
    vuelo: 0.0,
    // Pelo pegado y partido: el tango se baila con la cabeza quieta.
    tocado: Tocado::Liso,
};

const CUERPO_TANGO_PAREJA: Cuerpo = Cuerpo {
    falda: 8.0,
    vuelo: 0.55,
    hombros: 3.0,
    ..CUERPO_TANGO
};

/// Bajo, ancho y con flecos.
const CUERPO_CHARLESTON: Cuerpo = Cuerpo {
    torso: 6.2,
    cuello: 4.6,
    brazo: 6.2,
    antebrazo: 5.4,
    muslo: 6.0,
    pantorrilla: 5.6,
    hombros: 4.4,
    cintura: 3.4,
    falda: 7.0,
    vuelo: 0.85,
    // El casquete con cinta de los anos veinte.
    tocado: Tocado::Casquete,
};

/// Piernas larguisimas y falda enorme: en el cancan la pierna es el numero y la
/// falda es lo que la ensena.
const CUERPO_CANCAN: Cuerpo = Cuerpo {
    torso: 5.4,
    cuello: 4.0,
    brazo: 6.2,
    antebrazo: 5.4,
    // Es el unico jefe donde la ropa hace tanto trabajo como el esqueleto.
    muslo: 7.4,
    pantorrilla: 7.0,
    hombros: 4.4,
    cintura: 3.6,
    falda: 7.5,
    vuelo: 0.75,
    // El penacho de plumas del Moulin Rouge.
    tocado: Tocado::Penacho,
};

/// Los angulos que decide un baile. Lo que `montar` convierte en una figura.
struct Postura {
    /// Desplazamiento de la cadera respecto al centro del jefe.
    centro: Vec2,
    /// Inclinacion del tronco.
    lean: f32,
    /// Angulo de hombro y flexion de codo, por brazo.
    brazo_i: (f32, f32),
    brazo_d: (f32, f32),
    /// Angulo de cadera y flexion de rodilla, por pierna.
    pierna_i: (f32, f32),
    pierna_d: (f32, f32),
    /// Hacia donde se va la tela.
    arrastre: Vec2,
    /// Cuanto se estrecha la figura a lo ancho. 1 es de frente, 0 es de perfil:
    /// **es lo que hace que una figura plana parezca que gira sobre si misma**.
    giro: f32,
    /// Ondulacion de la tela, en ticks.
    fase: f32,
}

impl Default for Postura {
    fn default() -> Self {
        Self {
            centro: Vec2::ZERO,
            lean: 0.0,
            brazo_i: (1.0, 0.4),
            brazo_d: (-1.0, 0.4),
            pierna_i: (0.25, 0.1),
            pierna_d: (-0.25, 0.1),
            arrastre: Vec2::ZERO,
            giro: 1.0,
            fase: 0.0,
        }
    }
}

/// De angulos a figura. Cinematica directa, igual que en la protagonista.
/// El cuerpo de una figura: el de siempre, con la falda mas grande y mas
/// suelta cuanto mas avanzado va el baile.
///
/// Un jefe de Cuphead cambia de cuerpo entre fases, no solo de pose. Aqui es el
/// vestido el que se enciende: con cada figura la falda crece y vuela mas.
/// **Solo la falda**: el esqueleto no se toca, y la hitbox es un circulo del
/// core, asi que nada de esto cambia donde hay que disparar.
fn crece(c: Cuerpo, fase: usize) -> Cuerpo {
    let f = fase.min(3) as f32;
    Cuerpo {
        falda: c.falda * (1.0 + 0.15 * f),
        vuelo: c.vuelo + 0.12 * f,
        ..c
    }
}

fn montar(c: &Cuerpo, p: &Postura) -> Pose {
    let cadera = p.centro;
    let pecho = desde(cadera, ARRIBA + p.lean, c.torso);
    let cabeza = desde(pecho, ARRIBA + p.lean * 1.4, c.cuello);
    let mono = desde(
        cabeza,
        ARRIBA + p.lean * 1.4,
        RADIO_CABEZA + RADIO_MONO * 0.7,
    );

    let (hi, ci) = p.brazo_i;
    let (hd, cd) = p.brazo_d;
    let hombro_i = ARRIBA + hi + p.lean;
    let hombro_d = ARRIBA + hd + p.lean;
    let codo_i = desde(pecho, hombro_i, c.brazo);
    let mano_i = desde(codo_i, hombro_i + ci, c.antebrazo);
    let codo_d = desde(pecho, hombro_d, c.brazo);
    let mano_d = desde(codo_d, hombro_d - cd, c.antebrazo);

    let (ai, fi) = p.pierna_i;
    let (ad, fd) = p.pierna_d;
    let cadera_i = ARRIBA + PI + ai + p.lean * 0.5;
    let cadera_d = ARRIBA + PI + ad + p.lean * 0.5;
    let rodilla_i = desde(cadera, cadera_i, c.muslo);
    let pie_i = desde(rodilla_i, cadera_i - fi, c.pantorrilla);
    let rodilla_d = desde(cadera, cadera_d, c.muslo);
    let pie_d = desde(rodilla_d, cadera_d - fd, c.pantorrilla);

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
    let mut falda = [cadera; N_FALDA];
    if c.falda > 0.0 {
        for (i, punto) in falda.iter_mut().enumerate() {
            let t = i as f32 / (N_FALDA - 1) as f32;
            let ang = PI * (0.13 + 0.74 * t);
            let (forma, _) = sin_cos(t * PI);
            let (onda, _) = sin_cos(p.fase * 0.22 + t * 3.1);
            let largo = c.falda * (0.72 + 0.28 * forma) * (1.0 + c.vuelo * onda.abs());
            *punto = desde(cadera, ang, largo)
                + p.arrastre * (0.35 + 0.65 * forma)
                + vec2(0.0, onda * 0.6);
        }
    }

    // --- Corpino ---
    let eje = (pecho - cadera).normalize_or_zero();
    let perp = vec2(-eje.y, eje.x);
    let corpino = [
        pecho + perp * c.hombros,
        pecho - perp * c.hombros,
        cadera - perp * c.cintura,
        cadera + perp * c.cintura,
    ];

    let mut pose = Pose {
        joints,
        falda,
        mono,
        corpino,
        // Las cintas son de la protagonista: los jefes no llevan.
        cintas: [[Vec2::ZERO; N_CINTA]; 2],
        tocado: c.tocado,
    };

    // El giro. Aplastar la figura a lo ancho es la forma barata de que una
    // figura plana parezca que gira sobre su eje, y en un vals es media
    // animacion.
    if p.giro != 1.0 {
        let aplastar = |v: &mut Vec2| v.x = p.centro.x + (v.x - p.centro.x) * p.giro;
        for j in pose.joints.iter_mut() {
            aplastar(j);
        }
        for j in pose.falda.iter_mut() {
            aplastar(j);
        }
        for j in pose.corpino.iter_mut() {
            aplastar(j);
        }
        aplastar(&mut pose.mono);
    }
    pose
}

/// Las figuras de un jefe en este instante.
///
/// `t` son ticks continuos (`tick + alpha`), `fase` la figura del baile y
/// `vida` lo que le queda en `[0, 1]`. Devuelve una lista porque **el tango es
/// una pareja**.
pub fn poses(jefe: usize, fase: usize, t: f32, vida: f32) -> Vec<Pose> {
    match jefe {
        0 => vec![vals(fase, t, vida)],
        1 => tango(fase, t, vida),
        2 => vec![charleston(fase, t, vida)],
        _ => vec![cancan(fase, t, vida)],
    }
}

/// **El Vals**: gira y no para. Es su verbo, el mismo que el de sus balas.
///
/// El giro se acelera con cada figura y con la vida que le queda: cuanto peor
/// lo lleva, mas rapido gira.
fn vals(fase: usize, t: f32, vida: f32) -> Pose {
    let ritmo = 0.030 + fase as f32 * 0.010 + (1.0 - vida) * 0.020;
    let a = t * ritmo;
    let (giro_s, giro_c) = sin_cos(a);
    // Vaiven de tres tiempos por encima del giro.
    let (bal, _) = sin_cos(t * 0.075);

    montar(
        &crece(CUERPO_VALS, fase),
        &Postura {
            centro: vec2(0.0, bal * 0.8),
            lean: bal * 0.14,
            // Los brazos, en posicion de salon: uno arriba y otro abierto.
            brazo_i: (0.95 + giro_s * 0.20, -0.55),
            brazo_d: (-1.25 + giro_s * 0.20, 0.30),
            pierna_i: (0.30 + giro_s * 0.10, 0.12),
            pierna_d: (-0.34 + giro_s * 0.10, 0.08),
            // La falda sale disparada hacia fuera del giro.
            arrastre: vec2(giro_s * 5.0, 1.0),
            giro: 0.35 + 0.65 * giro_c.abs(),
            fase: t,
        },
    )
}

/// **El Tango**: una pareja, y no se mueve, se **para**.
///
/// Los angulos no interpolan: saltan de una postura a otra cada medio compas.
/// Es el mismo verbo que sus balas —salir, frenar, volver— llevado al cuerpo.
/// En la segunda figura, los ochos, la pareja se separa.
fn tango(fase: usize, t: f32, vida: f32) -> Vec<Pose> {
    // El tema va a 120 negras en 4/4: 30 ticks por tiempo. Los pasos caen en
    // los tiempos, como las balas.
    let paso = (t / 60.0).floor() as i32;
    let alterna = if paso.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
    // Un golpe corto justo despues del paso, que es lo que hace que se lea
    // como un acento y no como un cambio de postura sin mas.
    let golpe = (1.0 - (t.rem_euclid(60.0)) / 10.0).max(0.0);

    // Se separan en los ochos y se vuelven a juntar en el molinete.
    let apertura = match fase {
        0 => 5.0,
        1 => 5.0 + 16.0 * (1.0 - vida),
        _ => 7.0,
    };

    let lleva = montar(
        &crece(CUERPO_TANGO, fase),
        &Postura {
            centro: vec2(-apertura, 0.0),
            lean: 0.16 * alterna + golpe * 0.10,
            brazo_i: (1.35, -0.20),
            brazo_d: (-0.30 - golpe * 0.25, 0.55),
            pierna_i: (0.16 + 0.34 * alterna.max(0.0), 0.06),
            pierna_d: (-0.42 * alterna.max(0.0) - 0.10, 0.04),
            arrastre: Vec2::ZERO,
            giro: 0.85,
            fase: t,
        },
    );
    let sigue = montar(
        &crece(CUERPO_TANGO_PAREJA, fase),
        &Postura {
            centro: vec2(apertura, 0.0),
            lean: -0.22 * alterna - golpe * 0.12,
            brazo_i: (1.15, -0.45),
            brazo_d: (-1.30 + golpe * 0.20, 0.15),
            pierna_i: (0.42 * alterna.max(0.0) + 0.10, 0.05),
            pierna_d: (-0.18 - 0.30 * alterna.max(0.0), 0.06),
            arrastre: vec2(-alterna * 3.0, 0.5),
            giro: 0.85,
            fase: t,
        },
    );
    vec![lleva, sigue]
}

/// **El Charleston**: rebota, y rebota en la clave 3-3-2.
///
/// El tema va a 100 negras en 4/4 —36 ticks por tiempo, 144 por compas— y la
/// clave cae en 0, 54 y 108. Las rodillas se abren y se cierran en esos tres
/// golpes, que es literalmente el paso del charleston.
fn charleston(fase: usize, t: f32, vida: f32) -> Pose {
    const COMPAS: f32 = 144.0;
    let dentro = t.rem_euclid(COMPAS);
    // En cual de los tres golpes de la clave estamos.
    let golpe = if dentro < 54.0 {
        0
    } else if dentro < 108.0 {
        1
    } else {
        2
    };
    let desde_golpe = dentro - [0.0, 54.0, 108.0][golpe];
    // La patada: fuerte al caer el golpe y se relaja hasta el siguiente.
    let patada = (1.0 - desde_golpe / 26.0).max(0.0);
    let lado = if golpe == 1 { -1.0 } else { 1.0 };

    let brio = 1.0 + fase as f32 * 0.18 + (1.0 - vida) * 0.25;
    let (rebote, _) = sin_cos(t * 0.22);

    montar(
        &crece(CUERPO_CHARLESTON, fase),
        &Postura {
            centro: vec2(lado * patada * 2.0, -patada * 2.2 * brio + rebote * 0.6),
            lean: -lado * patada * 0.22,
            // Los brazos van al contrario que las piernas: es lo que hace que
            // se lea como charleston y no como saltar.
            brazo_i: (0.75 + lado * patada * 0.85 * brio, -0.70),
            brazo_d: (-0.75 + lado * patada * 0.85 * brio, 0.70),
            // Rodillas adentro y afuera, que es el paso.
            pierna_i: (0.20 + lado * patada * 0.75 * brio, 0.85 * patada),
            pierna_d: (-0.20 + lado * patada * 0.75 * brio, 0.85 * (1.0 - patada)),
            arrastre: vec2(-lado * patada * 4.0, 0.0),
            giro: 1.0,
            fase: t,
        },
    )
}

/// **El Cancan**: patea, y la patada se va sola.
///
/// Es lo contrario del dembow que hubo aqui: aquel se hundia en el bombo, este
/// **sube**. El galop va a 150 negras en 2/4 —24 ticks por tiempo, 48 por
/// compas— y en cada tiempo se levanta una pierna: la izquierda en el uno, la
/// derecha en el dos. Eso es la fila del music-hall, y es la misma reja en la
/// que caen sus balas.
fn cancan(fase: usize, t: f32, vida: f32) -> Pose {
    const TIEMPO: f32 = 24.0;
    const COMPAS: f32 = 48.0;
    let dentro = t.rem_euclid(COMPAS);
    let cual = (dentro / TIEMPO).floor();
    let desde = dentro - cual * TIEMPO;
    // Sube de golpe y baja despacio, que es como es una patada.
    let patada = (1.0 - desde / 14.0).max(0.0);
    let izquierda = cual == 0.0;
    let lado = if izquierda { 1.0 } else { -1.0 };

    let brio = 1.0 + fase as f32 * 0.20 + (1.0 - vida) * 0.30;
    let (vaiven, _) = sin_cos(t * 0.05);

    montar(
        &crece(CUERPO_CANCAN, fase),
        &Postura {
            // La y crece hacia abajo, asi que en la patada SUBE.
            centro: vec2(vaiven * 3.0 + lado * 1.5, -patada * 2.0 * brio),
            // Y se echa hacia atras, que es lo que contrapesa la pierna.
            lean: lado * 0.16 + patada * 0.26,
            // Un brazo en alto y el otro sujetando la falda.
            brazo_i: (1.30 + patada * 0.30, -1.05),
            brazo_d: (-0.55, 0.65),
            // Sube la pierna del lado que toca; la otra aguanta el peso.
            pierna_i: (0.15 + patada * 1.50 * brio * lado.max(0.0), 0.08),
            pierna_d: (-0.15 - patada * 1.50 * brio * (-lado).max(0.0), 0.08),
            arrastre: vec2(-lado * patada * 5.0, -1.0),
            giro: 1.0,
            fase: t,
        },
    )
}

/// Un aro alrededor del jefe, para que se vea donde le entran los disparos.
///
/// El cuerpo dice quien es; el aro dice donde darle. Cuando el jefe era un
/// hexagono las dos cosas eran la misma, y por eso se leia bien pero no decia
/// nada.
pub fn halo(t: f32, vida: f32) -> (f32, f32) {
    let pulso = (t * 0.05).rem_euclid(TAU);
    let (s, _) = sin_cos(pulso);
    (1.0 + s * 0.03, 0.25 + 0.35 * (1.0 - vida))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_pose_es_funcion_pura_de_sus_entradas() {
        // La misma disciplina que la protagonista: sin esto, la animacion y la
        // simulacion se desfasan y nadie sabe por que.
        for jefe in 0..4 {
            let a = poses(jefe, 1, 123.5, 0.6);
            let b = poses(jefe, 1, 123.5, 0.6);
            for (x, y) in a.iter().zip(b.iter()) {
                assert_eq!(x.joints, y.joints, "jefe {jefe}");
                assert_eq!(x.falda, y.falda);
            }
        }
    }

    #[test]
    fn el_tango_es_una_pareja_y_los_demas_no() {
        assert_eq!(poses(0, 0, 10.0, 1.0).len(), 1);
        assert_eq!(
            poses(1, 0, 10.0, 1.0).len(),
            2,
            "el tango se baila entre dos"
        );
        assert_eq!(poses(2, 0, 10.0, 1.0).len(), 1);
        assert_eq!(poses(3, 0, 10.0, 1.0).len(), 1);
    }

    #[test]
    fn la_figura_sigue_siendo_un_cuerpo() {
        // No se comprueban longitudes de hueso: el escorzo aplasta la figura a
        // lo ancho a proposito —es lo que hace que parezca que gira— y eso
        // cambia las distancias. Lo que no puede cambiar nunca es la anatomia:
        // la cabeza arriba, los pies abajo y todo dentro de un tamano humano.
        for jefe in 0..4 {
            for paso in 0..120 {
                let t = paso as f32 * 3.7;
                for vida in [1.0, 0.5, 0.0] {
                    for pose in poses(jefe, paso as usize % 4, t, vida) {
                        let cadera = pose.joints[CADERA];
                        assert!(
                            pose.joints[CABEZA].y < cadera.y,
                            "jefe {jefe}: la cabeza no esta arriba"
                        );
                        // El cancan es la excepcion, y a proposito: su numero
                        // **es** el pie por encima de la cadera. Lo que si
                        // sigue cumpliendo es que un pie esta siempre en el
                        // suelo, que es lo que separa una patada de un salto.
                        let ambos =
                            pose.joints[PIE_I].y > cadera.y && pose.joints[PIE_D].y > cadera.y;
                        let alguno =
                            pose.joints[PIE_I].y > cadera.y || pose.joints[PIE_D].y > cadera.y;
                        assert!(
                            if jefe == 3 { alguno } else { ambos },
                            "jefe {jefe}: los pies no estan donde deben"
                        );
                        for (i, j) in pose.joints.iter().enumerate() {
                            assert!(j.is_finite(), "jefe {jefe}: la articulacion {i} es NaN");
                            assert!(
                                (*j - cadera).length() < 40.0,
                                "jefe {jefe}: la articulacion {i} se ha ido a {j}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn cada_jefe_tiene_su_silueta() {
        // Si los tres midieran lo mismo, serian el mismo munequito con otra
        // animacion, y la mitad de la gracia es que se distingan parados.
        let alto = |jefe: usize| {
            let pose = &poses(jefe, 0, 0.0, 1.0)[0];
            pose.joints[CADERA].y - pose.joints[CABEZA].y
        };
        let (v, t, c, d) = (alto(0), alto(1), alto(2), alto(3));
        assert!(
            v > t && t > c && c > d,
            "alturas iguales: vals {v}, tango {t}, charleston {c}, cancan {d}"
        );
    }

    #[test]
    fn el_tango_se_para_en_seco_y_el_vals_no() {
        // La diferencia que define a los dos bailes, llevada al cuerpo. El vals
        // cambia un poco en cada tick; el tango se queda quieto y salta.
        let mueve = |jefe: usize, a: f32, b: f32| {
            let (x, y) = (poses(jefe, 0, a, 1.0), poses(jefe, 0, b, 1.0));
            x.iter()
                .zip(y.iter())
                .map(|(p, q)| {
                    p.joints
                        .iter()
                        .zip(q.joints.iter())
                        .map(|(u, v)| (*u - *v).length())
                        .sum::<f32>()
                })
                .sum::<f32>()
        };
        // Dentro del mismo paso, el tango casi no se mueve.
        let tango_quieto = mueve(1, 20.0, 40.0);
        // Y al cruzar el paso, pega un salto.
        let tango_salto = mueve(1, 55.0, 65.0);
        assert!(
            tango_salto > tango_quieto * 3.0,
            "el tango deberia saltar de postura: quieto {tango_quieto}, salto {tango_salto}"
        );
    }

    #[test]
    fn el_cancan_levanta_una_pierna_en_cada_tiempo() {
        // Su verbo, llevado al cuerpo: el pie que sube alterna con el compas.
        // Izquierda en el uno, derecha en el dos, cada 24 ticks. Si esto se
        // desincroniza, la bailarina deja de bailar lo que suena.
        let pie = |t: f32, i: usize| poses(3, 0, t, 1.0)[0].joints[i].y;
        // En el uno sube la izquierda, y esta mas arriba que la derecha.
        assert!(
            pie(0.0, PIE_I) < pie(0.0, PIE_D),
            "el uno no levanta la izquierda"
        );
        // En el dos, al reves.
        assert!(
            pie(24.0, PIE_D) < pie(24.0, PIE_I),
            "el dos no levanta la derecha"
        );
        // Y entre patada y patada el pie vuelve abajo: la patada se va sola,
        // igual que sus balas.
        assert!(pie(20.0, PIE_I) > pie(0.0, PIE_I), "la patada no baja");
    }

    #[test]
    fn el_charleston_patea_en_la_clave_3_3_2() {
        // Los tres golpes caen en 0, 54 y 108 de cada compas de 144 ticks, que
        // es la misma reja que usan sus balas.
        let altura = |t: f32| poses(2, 0, t, 1.0)[0].joints[CADERA].y;
        // Justo en el golpe la cadera sube; a mitad de camino ha bajado.
        for golpe in [0.0, 54.0, 108.0] {
            let en_golpe = altura(golpe);
            let entre = altura(golpe + 40.0);
            assert!(
                en_golpe < entre,
                "no hay patada en el golpe {golpe}: {en_golpe} vs {entre}"
            );
        }
    }

    #[test]
    fn cada_jefe_lleva_un_tocado_distinto() {
        // Sin caras, el tocado es lo unico que distingue una silueta de otra
        // estando quieta: es lo que hace un cartel, que resuelve un personaje
        // con la forma y el sombrero. Si dos coinciden, dos jefes se convierten
        // en el mismo munequito con otra animacion.
        let suyo: Vec<Tocado> = (0..4).map(|j| poses(j, 0, 0.0, 1.0)[0].tocado).collect();
        for i in 0..suyo.len() {
            for k in i + 1..suyo.len() {
                assert_ne!(suyo[i], suyo[k], "los jefes {i} y {k} llevan lo mismo");
            }
        }
    }

    #[test]
    fn la_falda_crece_con_cada_figura_y_el_que_no_lleva_no_la_estrena() {
        // Lo lejos que llega el bajo de la falda desde la cadera.
        let vuelo = |jefe: usize, fase: usize| {
            let p = &poses(jefe, fase, 0.0, 1.0)[0];
            let cadera = p.joints[CADERA];
            p.falda
                .iter()
                .map(|q| (*q - cadera).length())
                .fold(0.0, f32::max)
        };
        assert!(
            vuelo(0, 3) > vuelo(0, 0) * 1.3,
            "el vals no se enciende en el fleckerl"
        );
        // El que lleva en el tango va sin falda, y crecer no puede ponersela.
        let p = &poses(1, 2, 0.0, 1.0)[0];
        assert_eq!(
            p.falda[0],
            p.falda[N_FALDA - 1],
            "al del tango le ha salido falda"
        );
    }
}
