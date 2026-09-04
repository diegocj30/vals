//! La paleta: el cartel de la Belle Epoque.
//!
//! Este juego se veia a marcianitos y la causa no era el detalle, era el
//! material: neon cian y rosa sobre negro azulado, que es la paleta de un
//! vectorial de los ochenta. Un salon de baile de 1900 no se ve asi.
//!
//! La referencia no es Cuphead —eso seria 1930, manguera de goma y guantes
//! blancos— sino **el cartel de la Belle Epoque**: Toulouse-Lautrec, Cheret,
//! Mucha. Es literalmente el cartel del Moulin Rouge donde se bailaba el
//! cancan. De Cuphead se coge el metodo —tinta gruesa, fondo de material,
//! siluetas rotundas— y no el dibujo.
//!
//! Y lo que aporta el cartel y Cuphead no tiene: **un cartel de epoca se
//! imprimia a dos o tres tintas**. De ahi sale que cada baile tenga la suya,
//! igual que ya tiene su verbo y su compas.
//!
//! **El reparto es papel fuera, tarima dentro.** El marco es la pagina; la
//! arena es la lamina impresa encima, y sigue siendo oscura porque ahi es donde
//! tienen que brillar las balas. Un danmaku sobre fondo claro se lee peor, y en
//! esto la legibilidad no se negocia.

use macroquad::prelude::*;
use vals_core::rng::Pcg32;

/// El papel. Crema envejecido y no blanco: un cartel de 1900 lleva un siglo
/// amarilleando, y ese amarillo es la mitad de la epoca.
pub const PAPEL: Color = color_u8!(224, 211, 182, 255);
/// La tinta. Un negro calido, no un agujero: en un cartel el contorno esta
/// impreso, no recortado.
pub const TINTA: Color = color_u8!(26, 20, 24, 255);
/// Tinta aguada, para lo secundario impreso sobre el papel.
pub const TINTA_TENUE: Color = color_u8!(112, 96, 84, 255);

/// La pared del fondo del salon.
pub const PARED: Color = color_u8!(33, 23, 25, 255);
/// La tarima. Oscura, pero **madera**: lo que cambia no es el brillo, es que
/// deja de ser azul.
pub const TARIMA: Color = color_u8!(54, 37, 33, 255);
/// Las vetas de la tarima.
pub const VETA: Color = color_u8!(92, 62, 50, 255);
/// La luz de los focos: ocre de bombilla, no azul de luna.
pub const LUZ: Color = color_u8!(255, 212, 148, 255);
/// El dorado de la lampara.
pub const ORO: Color = color_u8!(255, 205, 128, 255);

/// Las dos tintas de cada baile, en el orden de `DEFAULT_BOSS_RONS`.
///
/// La primera es el cuerpo del jefe y la segunda su ropa. Un cartel se imprimia
/// a dos o tres planchas, y esto es exactamente eso: dos tintas por baile.
///
/// Es lo que hace que los cuatro se distingan **a un vistazo, con el sonido
/// quitado**, que era lo unico que faltaba despues de darles un verbo y un
/// compas propios.
pub const TINTAS: [(Color, Color); 4] = [
    // El Vals: azul de Prusia y hueso. Es el que ensena, y el mas frio.
    (color_u8!(88, 132, 190, 255), color_u8!(226, 216, 198, 255)),
    // El Tango: carmin y negro. Los dos colores de un tango, sin discusion.
    (color_u8!(196, 58, 66, 255), color_u8!(52, 34, 38, 255)),
    // El Charleston: mostaza y verde botella. Los anos veinte enteros.
    (color_u8!(226, 178, 66, 255), color_u8!(78, 116, 92, 255)),
    // El Cancan: rosa Moulin Rouge y granate. El cartel, tal cual.
    (color_u8!(232, 96, 142, 255), color_u8!(122, 40, 62, 255)),
];

/// La tinta de un baile. Un indice raro cae en el primero, que es el que
/// ensena: mejor el vals de mas que una pantalla en negro.
pub fn del_baile(i: usize) -> (Color, Color) {
    TINTAS[i.min(TINTAS.len() - 1)]
}

/// Cuantas motas tiene el grano del papel.
const MOTAS: usize = 520;
/// Semilla del grano. Constante a proposito: ver `grano`.
const SEMILLA: u64 = 0x5A10_4E15;

/// El grano del papel: motas en coordenadas normalizadas, con su radio.
///
/// **Fijas, no sorteadas cada frame.** Un grano que parpadea se lee como ruido
/// de video; uno quieto se lee como papel. Salen de la misma `Pcg32` del nucleo
/// con semilla constante, asi que son las mismas en cada arranque y en cada
/// maquina, y por eso hay un test que lo vigila.
///
/// Van normalizadas de 0 a 1 para que sobrevivan a que se cambie el tamano de
/// la ventana sin recalcular nada.
pub fn grano() -> Vec<(f32, f32, f32)> {
    let mut rng = Pcg32::new(SEMILLA);
    (0..MOTAS)
        .map(|_| {
            let x = rng.next_f32();
            let y = rng.next_f32();
            // Casi todas diminutas y unas pocas mas gordas: asi el papel tiene
            // grano fino y alguna imperfeccion, que es lo que tiene el de
            // verdad.
            let r = 0.4 + rng.next_f32().powi(3) * 1.1;
            (x, y, r)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_grano_del_papel_no_parpadea() {
        // Un grano sorteado cada frame se lee como ruido de video y marea; uno
        // quieto se lee como papel. La semilla es constante justo para eso, asi
        // que dos llamadas tienen que dar exactamente lo mismo.
        let a = grano();
        let b = grano();
        assert_eq!(a.len(), MOTAS);
        assert_eq!(a, b, "el grano ha cambiado entre dos llamadas");
    }

    #[test]
    fn las_motas_caen_dentro_del_papel() {
        for (x, y, r) in grano() {
            assert!((0.0..=1.0).contains(&x), "mota fuera: x={x}");
            assert!((0.0..=1.0).contains(&y), "mota fuera: y={y}");
            assert!(r > 0.0 && r < 1.6, "mota de radio raro: {r}");
        }
    }

    #[test]
    fn cada_baile_tiene_su_tinta_y_no_la_del_vecino() {
        // La misma regla que los verbos, aplicada al color: si dos bailes
        // comparten tinta, se acabo distinguirlos de un vistazo, que es
        // justamente para lo que esta.
        for (i, (a, _)) in TINTAS.iter().enumerate() {
            for (k, (b, _)) in TINTAS.iter().enumerate().skip(i + 1) {
                let d = (a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs();
                assert!(d > 0.35, "los bailes {i} y {k} tienen la misma tinta");
            }
        }
    }

    #[test]
    fn un_baile_que_no_existe_cae_en_el_ultimo() {
        // Un indice raro no puede dejar la pantalla en negro.
        assert_eq!(del_baile(99).0.r, TINTAS[TINTAS.len() - 1].0.r);
    }
}
