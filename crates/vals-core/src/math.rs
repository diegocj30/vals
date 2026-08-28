//! Trigonometria determinista.
//!
//! Rust delega `sin`, `cos` y `atan2` en la libm del sistema, y glibc y la CRT
//! de MSVC **no devuelven exactamente los mismos bits**. Diferencias de un ULP,
//! irrelevantes para dibujar, pero letales para una simulacion determinista: el
//! jefe llama a `from_angle` en cada disparo, asi que al segundo de partida las
//! trayectorias ya se han separado.
//!
//! Lo descubrio el CI: el replay dorado, grabado en Windows, divergia en el
//! tick 60 al reproducirse en Linux. El plan inicial decia que el
//! determinismo cruzado entre plataformas quedaba fuera de alcance; el CI
//! demostro que si hacia falta, asi que aqui esta.
//!
//! Todo lo que hay dentro usa **solo** operaciones que IEEE-754 obliga a
//! redondear igual en cualquier maquina: sumar, restar, multiplicar, dividir,
//! comparar, `floor` y `round`. Ni una llamada a libm. Por eso el resultado es
//! identico bit a bit en Windows, en Linux y en el navegador.
//!
//! Precision medida frente a `std`: mejor que 5e-6 para seno y coseno en
//! `[-20, 20]`, y mejor que 2e-5 para el arco tangente. En pantalla eso es una
//! milesima de pixel. Aqui se apuntan balas, no se calculan orbitas.

use glam::Vec2;

pub const TAU: f32 = std::f32::consts::TAU;
pub const PI: f32 = std::f32::consts::PI;
const FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2;
/// 2/pi: cuantos cuadrantes hay en un angulo.
const FRAC_2_PI: f32 = std::f32::consts::FRAC_2_PI;

// pi/2 partido en dos: el f32 mas cercano, y lo que le falta. Restar por partes
// (Cody-Waite) evita perder cifras significativas al reducir angulos grandes.
const PI_2_HI: f32 = 1.5707964;
const PI_2_LO: f32 = -4.371139e-8;

/// Envuelve un angulo en `[0, TAU)`.
///
/// Con `floor` y no con el operador `%`, que en flotantes acaba llamando a
/// `fmod` de la libm y volveria a meter dependencia del sistema justo en el
/// sitio que estamos blindando.
pub fn wrap_tau(a: f32) -> f32 {
    let w = a - TAU * (a / TAU).floor();
    // Por si el redondeo deja el resultado justo en el borde.
    if w < 0.0 {
        w + TAU
    } else if w >= TAU {
        0.0
    } else {
        w
    }
}

/// Seno y coseno a la vez, que es como casi siempre hacen falta.
pub fn sin_cos(x: f32) -> (f32, f32) {
    // Reduccion al cuadrante: k cuartos de vuelta mas un resto pequeno.
    let k = (x * FRAC_2_PI).round();
    let r = x - k * PI_2_HI - k * PI_2_LO;

    let s = poly_sin(r);
    let c = poly_cos(r);

    match (k as i64) & 3 {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    }
}

pub fn sin(x: f32) -> f32 {
    sin_cos(x).0
}

pub fn cos(x: f32) -> f32 {
    sin_cos(x).1
}

/// Vector unitario en la direccion `a`.
pub fn from_angle(a: f32) -> Vec2 {
    let (s, c) = sin_cos(a);
    Vec2::new(c, s)
}

/// Angulo de un vector, en `(-PI, PI]`.
pub fn to_angle(v: Vec2) -> f32 {
    atan2(v.y, v.x)
}

/// Minimax en `[-pi/4, pi/4]`. Coeficientes clasicos de Cephes.
fn poly_sin(r: f32) -> f32 {
    let r2 = r * r;
    r * (1.0 + r2 * (-0.16666667 + r2 * (0.008332824 + r2 * -0.00019587841)))
}

fn poly_cos(r: f32) -> f32 {
    let r2 = r * r;
    1.0 + r2 * (-0.5 + r2 * (0.04166642 + r2 * -0.0013888378))
}

/// Arco tangente para `|z| <= 1`.
///
/// Minimax de grado 9, la aproximacion clasica para f32. Error maximo ~1e-5,
/// que en pantalla es una milesima de pixel.
fn atan_unit(z: f32) -> f32 {
    let z2 = z * z;
    z * (0.999866 + z2 * (-0.3302995 + z2 * (0.180141 + z2 * (-0.085133 + z2 * 0.0208351))))
}

pub fn atan2(y: f32, x: f32) -> f32 {
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }

    let ax = x.abs();
    let ay = y.abs();
    // Se divide siempre el pequeno entre el grande: el cociente cae en [0, 1],
    // que es donde vale la aproximacion, y nunca desborda.
    let (num, den, invertir) = if ay > ax {
        (ax, ay, true)
    } else {
        (ay, ax, false)
    };

    let mut a = atan_unit(num / den);
    if invertir {
        a = FRAC_PI_2 - a;
    }
    if x < 0.0 {
        a = PI - a;
    }
    if y < 0.0 {
        a = -a;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Error maximo aceptado frente a `std`.
    ///
    /// No son cifras aspiracionales: son las medidas, con un poco de margen. Si
    /// alguien toca los coeficientes y empeora, el test lo dice.
    const TOL_SIN: f32 = 5e-6;
    const TOL_ATAN: f32 = 2e-5;

    #[test]
    fn el_seno_y_el_coseno_coinciden_con_std() {
        let mut peor: f32 = 0.0;
        let mut x = -20.0f32;
        while x < 20.0 {
            let (s, c) = sin_cos(x);
            peor = peor.max((s - x.sin()).abs()).max((c - x.cos()).abs());
            x += 0.0013;
        }
        assert!(peor < TOL_SIN, "error maximo {peor}");
    }

    #[test]
    fn siguen_valiendo_con_angulos_grandes() {
        // Un patron que gira mucho rato acumula angulo; la reduccion tiene que
        // aguantarlo sin desmoronarse.
        for x in [100.0f32, -100.0, 500.0, -1234.5, 5000.0] {
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() < 1e-3, "sin({x}) = {s} vs {}", x.sin());
            assert!((c - x.cos()).abs() < 1e-3, "cos({x}) = {c} vs {}", x.cos());
        }
    }

    #[test]
    fn se_mantiene_la_identidad_fundamental() {
        let mut x = -10.0f32;
        while x < 10.0 {
            let (s, c) = sin_cos(x);
            assert!((s * s + c * c - 1.0).abs() < 1e-5, "en {x}");
            x += 0.007;
        }
    }

    #[test]
    fn el_arcotangente_coincide_con_std() {
        let mut peor: f32 = 0.0;
        for i in -60..60 {
            for j in -60..60 {
                let (y, x) = (i as f32 * 0.17, j as f32 * 0.23);
                if x == 0.0 && y == 0.0 {
                    continue;
                }
                peor = peor.max((atan2(y, x) - y.atan2(x)).abs());
            }
        }
        assert!(peor < TOL_ATAN, "error maximo {peor}");
    }

    #[test]
    fn ida_y_vuelta_entre_angulo_y_vector() {
        let mut a = -PI + 0.001;
        while a < PI {
            let v = from_angle(a);
            assert!((v.length() - 1.0).abs() < 1e-5);
            let vuelta = to_angle(v);
            assert!((vuelta - a).abs() < 3e-5, "{a} -> {vuelta}");
            a += 0.01;
        }
    }

    #[test]
    fn los_ejes_dan_los_angulos_exactos() {
        assert_eq!(atan2(0.0, 1.0), 0.0);
        assert!((atan2(1.0, 0.0) - FRAC_PI_2).abs() < 1e-6);
        assert!((atan2(0.0, -1.0) - PI).abs() < 1e-6);
        assert!((atan2(-1.0, 0.0) + FRAC_PI_2).abs() < 1e-6);
        assert_eq!(atan2(0.0, 0.0), 0.0, "el vector nulo no debe dar NaN");
    }

    #[test]
    fn wrap_tau_deja_todo_dentro_del_rango() {
        for a in [-100.0f32, -TAU, -0.001, 0.0, 0.5, TAU, TAU + 0.5, 1000.0] {
            let w = wrap_tau(a);
            assert!((0.0..TAU).contains(&w), "wrap({a}) = {w}");
        }
        assert!((wrap_tau(0.5) - 0.5).abs() < 1e-6);
        assert!((wrap_tau(TAU + 0.5) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn el_resultado_es_estable_entre_llamadas() {
        // Trivial, pero deja constancia de la propiedad que importa: sin
        // llamadas a libm, el resultado no depende de nada del entorno.
        let mut x = -5.0f32;
        while x < 5.0 {
            assert_eq!(sin_cos(x), sin_cos(x));
            assert_eq!(atan2(x, 1.7), atan2(x, 1.7));
            x += 0.031;
        }
    }
}
