//! El trazo que hierve: la linea viva de los dibujos animados de 1930.
//!
//! En la animacion a mano cada fotograma se vuelve a dibujar, y por muy quieto
//! que este el personaje la linea nunca cae dos veces en el mismo sitio:
//! **hierve**. Es de lo que mas delata un dibujo de los anos treinta, y aqui
//! sale casi gratis porque todo se dibuja por codigo.
//!
//! Tres reglas, sacadas de como se hacia de verdad:
//!
//! - **En doses**: un dibujo nuevo cada dos fotogramas de pelicula, 12 por
//!   segundo. Por reloj, igual que `pelicula.rs`, y no por fotograma del
//!   monitor (a 144 Hz seria ruido) ni por tick (el temblor no es juego).
//! - **Tres dibujos que se repiten**. Un temblor sorteado cada vez se lee como
//!   ruido de video; tres dibujos en bucle se leen como un animador que ha
//!   calcado la misma pose tres veces.
//! - **Poco**: de medio pixel a uno, sea cual sea el tamano de la ventana.
//!
//! **Dos ganchos, y ninguno en los jefes uno a uno.** `hervir` envuelve una
//! figura entera —la protagonista, un jefe, un enemigo del paseo, un marco— y
//! la tuerce un pelo alrededor de su centro con la matriz de modelo de
//! macroquad: ni una funcion de dibujo se entera, igual que no se entera de la
//! sacudida. `tinta` va dentro, en las pasadas de tinta de los ayudantes
//! compartidos (`con_tinta`, `draw_figura`, el `Pincel` de la protagonista), y
//! mueve el contorno un poco mas que el relleno: el borde engorda de un lado y
//! adelgaza del otro, y cambia en cada dibujo, que es exactamente lo que hace
//! una linea repasada a mano. Los jefes pintan su tinta a trozos con sus
//! propios lapices; ahi solo hierve la figura entera, y basta.
//!
//! **Las balas no hierven nunca**: van por su propio render y
//! ninguna pasa por aqui. Tampoco la hitbox ni el aro de focus, que se dibujan
//! fuera de la figura, ni los fondos, que son decorado y estan quietos como el
//! grano del papel.

use std::cell::Cell;

use macroquad::prelude::*;
use macroquad::window::get_internal_gl;
use vals_core::rng::Pcg32;

/// Dibujos por segundo: en doses, la mitad de los 24 de la pelicula.
const DIBUJOS: f64 = 12.0;
/// Cuantos dibujos distintos hay antes de volver al primero.
const VARIANTES: u64 = 3;
/// Lo mas que se mueve un punto de una figura, en pixeles. Mas de uno ya no es
/// una linea que hierve, es una figura que tirita.
pub const AMPLITUD_MAX: f32 = 1.0;
/// Pixeles de temblor por pixel de radio: una figura grande hierve algo mas
/// que una pequena, como en papel, hasta el tope.
const POR_RADIO: f32 = 0.012;
/// Lo que la tinta se separa del relleno, en fraccion de la amplitud.
const SEPARA_TINTA: f32 = 0.5;

thread_local! {
    /// Si hierve. Se apaga con la T: a quien le marea el movimiento no le
    /// sirve de nada que sea bonito.
    static ACTIVO: Cell<bool> = const { Cell::new(true) };
    /// La figura que se esta dibujando: su semilla, su amplitud y cuantas
    /// pasadas de tinta lleva, para que cada pieza se mueva a su aire.
    static FIGURA: Cell<Option<(u64, f32, u64)>> = const { Cell::new(None) };
}

/// Enciende o apaga el hervor. Devuelve como queda.
pub fn alternar() -> bool {
    ACTIVO.with(|a| {
        a.set(!a.get());
        a.get()
    })
}

/// El dibujo en curso: cuenta a 12 por segundo desde el arranque.
fn dibujo() -> u64 {
    (get_time() * DIBUJOS) as u64
}

/// Un sorteo por figura y por dibujo. Solo cuenta el dibujo modulo
/// `VARIANTES`, asi que a los tres dibujos se repite.
fn sorteo(dibujo: u64, semilla: u64) -> Pcg32 {
    let variante = dibujo % VARIANTES;
    Pcg32::new(
        semilla.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (variante + 1).wrapping_mul(0xD1B5_4A32_D192_ED03),
    )
}

/// Cuanto hierve una figura de `radio` pixeles. Cero si esta apagado.
pub fn amplitud(radio: f32, activo: bool) -> f32 {
    if activo {
        (radio * POR_RADIO).min(AMPLITUD_MAX)
    } else {
        0.0
    }
}

/// La matriz de un dibujo: un desplazamiento, un giro y un estiramiento de
/// nada alrededor de `centro`. Esta repartida para que ningun punto a menos de
/// `radio` del centro se mueva mas de `amp` pixeles: 0.42 de desplazamiento,
/// 0.3 de giro y 0.2 de estiramiento, que suman menos de uno.
pub fn matriz(centro: Vec2, radio: f32, amp: f32, dibujo: u64, semilla: u64) -> Mat4 {
    let mut rng = sorteo(dibujo, semilla);
    let mut uno = || rng.range_f32(-1.0, 1.0);
    let (dx, dy, giro, ex, ey) = (uno(), uno(), uno(), uno(), uno());
    let r = radio.max(1.0);
    let c = centro.extend(0.0);
    Mat4::from_translation(c + vec3(dx, dy, 0.0) * amp * 0.3)
        * Mat4::from_rotation_z(giro * amp * 0.3 / r)
        * Mat4::from_scale(vec3(
            1.0 + ex * amp * 0.2 / r,
            1.0 + ey * amp * 0.2 / r,
            1.0,
        ))
        * Mat4::from_translation(-c)
}

/// Lo que se separa la pasada de tinta numero `n` de una figura.
pub fn desvio(amp: f32, dibujo: u64, semilla: u64, n: u64) -> Vec2 {
    let mut rng = sorteo(
        dibujo,
        semilla ^ (n + 1).wrapping_mul(0x2545_F491_4F6C_DD1D),
    );
    Vec2::from_angle(rng.next_f32() * std::f32::consts::TAU) * amp * SEPARA_TINTA
}

fn con_matriz(m: Mat4, pinta: impl FnOnce()) {
    // Cambiar la matriz corta el lote de macroquad: una llamada de dibujo mas
    // por figura, que con una docena de figuras en pantalla no se nota.
    unsafe { get_internal_gl() }.quad_gl.push_model_matrix(m);
    pinta();
    unsafe { get_internal_gl() }.quad_gl.pop_model_matrix();
}

/// Dibuja una figura hirviendo alrededor de `centro`. `radio` es lo que mide
/// mas o menos en pixeles, y `semilla` la distingue de sus vecinas: con la
/// misma semilla dos figuras hervirian a la vez y a la par, y se notaria.
pub fn hervir(centro: Vec2, radio: f32, semilla: u64, pinta: impl FnOnce()) {
    let amp = amplitud(radio, ACTIVO.with(Cell::get));
    if amp <= 0.0 {
        return pinta();
    }
    let antes = FIGURA.with(|f| f.replace(Some((semilla, amp, 0))));
    con_matriz(matriz(centro, radio, amp, dibujo(), semilla), pinta);
    FIGURA.with(|f| f.set(antes));
}

/// Una pasada de tinta dentro de una figura que hierve: se separa un pelo del
/// relleno, cada pasada hacia su lado. Fuera de `hervir` (los fondos) o con el
/// hervor apagado, no hace nada.
pub fn tinta(pinta: impl FnOnce()) {
    let Some((semilla, amp, n)) = FIGURA.with(Cell::get) else {
        return pinta();
    };
    FIGURA.with(|f| f.set(Some((semilla, amp, n + 1))));
    let d = desvio(amp, dibujo(), semilla, n);
    con_matriz(Mat4::from_translation(d.extend(0.0)), pinta);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mueve(m: Mat4, p: Vec2) -> f32 {
        (m.transform_point3(p.extend(0.0)).truncate() - p).length()
    }

    #[test]
    fn el_mismo_dibujo_es_el_mismo_temblor() {
        let c = vec2(300.0, 200.0);
        assert_eq!(matriz(c, 80.0, 1.0, 5, 7), matriz(c, 80.0, 1.0, 5, 7));
        assert_eq!(desvio(1.0, 5, 7, 2), desvio(1.0, 5, 7, 2));
    }

    #[test]
    fn cambia_en_cada_dibujo_y_vuelve_a_los_tres() {
        // En doses y en bucle: tres dibujos distintos y el cuarto es el
        // primero. Si se repitiera antes, no herviria; si no se repitiera, se
        // leeria como ruido.
        let c = vec2(300.0, 200.0);
        let m = |d| matriz(c, 80.0, 1.0, d, 7);
        assert_ne!(m(0), m(1));
        assert_ne!(m(1), m(2));
        assert_ne!(m(0), m(2));
        assert_eq!(m(0), m(3));
        assert_eq!(m(1), m(4));
        assert_ne!(desvio(1.0, 0, 7, 0), desvio(1.0, 1, 7, 0));
        assert_eq!(desvio(1.0, 0, 7, 0), desvio(1.0, 3, 7, 0));
    }

    #[test]
    fn dos_figuras_no_hierven_a_la_par() {
        let c = vec2(300.0, 200.0);
        assert_ne!(matriz(c, 80.0, 1.0, 0, 1), matriz(c, 80.0, 1.0, 0, 2));
        assert_ne!(desvio(1.0, 0, 7, 0), desvio(1.0, 0, 7, 1));
    }

    #[test]
    fn ningun_punto_se_mueve_mas_de_la_amplitud() {
        for radio in [10.0, 60.0, 150.0, 900.0] {
            let amp = amplitud(radio, true);
            assert!(amp <= AMPLITUD_MAX);
            let c = vec2(512.0, 384.0);
            for semilla in 0..20 {
                for d in 0..VARIANTES {
                    let m = matriz(c, radio, amp, d, semilla);
                    for i in 0..32 {
                        let p = c + Vec2::from_angle(i as f32 * 0.196) * radio;
                        let x = mueve(m, p);
                        assert!(x <= amp + 1e-3, "se mueve {x} con radio {radio}");
                    }
                    assert!(desvio(amp, d, semilla, 3).length() <= amp);
                }
            }
        }
    }

    #[test]
    fn apagado_no_se_mueve_nada() {
        assert_eq!(amplitud(120.0, false), 0.0);
        let c = vec2(512.0, 384.0);
        let m = matriz(c, 120.0, amplitud(120.0, false), 1, 9);
        assert!(mueve(m, c + vec2(120.0, 0.0)) < 1e-4);
        assert_eq!(desvio(0.0, 1, 9, 0), Vec2::ZERO);
    }
}
