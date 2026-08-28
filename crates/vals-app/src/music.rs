//! La musica, tambien sintetizada. Ni un fichero de audio en el repo.
//!
//! Sin musica, "cada jefe es un baile" no se entiende: el compas es la mitad
//! de la identidad de un baile, y se oye antes que se ve.
//!
//! Un tema aqui no es un fichero ni un secuenciador: es una **lista de voces
//! con retardo**, exactamente las mismas que hacen los efectos de sonido. Poner
//! una nota en el compas 3, tiempo 2, es una `Voz` con `delay` calculado. Todo
//! el motor de audio ya estaba escrito; esto solo lo usa para mas cosas.
//!
//! El bucle se renderiza entero una vez al arrancar y se reproduce en bucle. A
//! ocho compases sale menos de un megabyte, y a cambio no hay que programar
//! notas en tiempo real ni preocuparse de la latencia.

use crate::audio::{Voz, Wave};

/// El baile de cada jefe. Es lo que define compas, tempo y armonia.
///
/// De momento los tres son de la familia del vals, distinguidos por tempo y
/// armonia. Cuando llegue el tango, sera una variante mas aqui: cambia el
/// compas a 4/4 y el patron de acompanamiento, y el resto del modulo sirve.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Baile {
    /// 3/4, oom-pah-pah. El vals de toda la vida.
    Vals,
    /// Un vals mas oscuro y mas lento, en tono menor grave.
    ValsEspejo,
    /// Rapido y agitado: el ultimo.
    ValsCoda,
}

impl Baile {
    /// El baile de cada jefe, por orden de aparicion.
    pub fn del_jefe(indice: usize) -> Self {
        match indice {
            0 => Baile::Vals,
            1 => Baile::ValsEspejo,
            _ => Baile::ValsCoda,
        }
    }

    pub fn indice(self) -> usize {
        match self {
            Baile::Vals => 0,
            Baile::ValsEspejo => 1,
            Baile::ValsCoda => 2,
        }
    }

    pub fn por_indice(i: usize) -> Self {
        Self::del_jefe(i)
    }

    fn bpm(self) -> f32 {
        match self {
            Baile::Vals => 168.0,
            Baile::ValsEspejo => 144.0,
            Baile::ValsCoda => 196.0,
        }
    }

    /// Grados de la escala menor sobre los que va cada compas.
    ///
    /// Se escriben como grados y no como notas sueltas para que transportar el
    /// tema entero sea cambiar un numero.
    fn progresion(self) -> &'static [i32] {
        match self {
            // i - VI - iv - V : el giro clasico, melancolico y con salida.
            Baile::Vals => &[0, 5, 3, 4],
            // i - iv - i - V, mas cerrado sobre si mismo. Es El Espejo.
            Baile::ValsEspejo => &[0, 3, 0, 4],
            // i - VII - VI - V : la bajada que no para. El final.
            Baile::ValsCoda => &[0, 6, 5, 4],
        }
    }

    /// Nota base, en semitonos sobre La2 (110 Hz).
    fn tonica(self) -> i32 {
        match self {
            Baile::Vals => 0,
            Baile::ValsEspejo => -3,
            Baile::ValsCoda => 2,
        }
    }
}

/// Compases que dura el bucle.
const COMPASES: usize = 8;
/// Tiempos por compas. Tres: es un vals.
const TIEMPOS: usize = 3;

/// Semitonos de la escala menor natural, por grado.
const MENOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

/// Frecuencia de una nota en semitonos sobre La2 (110 Hz).
fn nota(semitonos: i32) -> f32 {
    // 2^(n/12), sin `powf`: se hace con multiplicaciones para no depender de
    // la libm, igual que el resto del proyecto.
    let mut f = 110.0f32;
    let (mut n, paso) = if semitonos >= 0 {
        (semitonos, 1.059_463_1f32)
    } else {
        (-semitonos, 0.943_874_3f32)
    };
    while n > 0 {
        f *= paso;
        n -= 1;
    }
    f
}

/// Grado de la escala menor, con octavas.
fn grado(g: i32) -> i32 {
    let octava = g.div_euclid(7);
    let idx = g.rem_euclid(7) as usize;
    MENOR[idx] + octava * 12
}

/// Duracion de un tiempo, en segundos.
fn tiempo(b: Baile) -> f32 {
    60.0 / b.bpm()
}

/// Duracion del bucle completo.
pub fn duracion(b: Baile) -> f32 {
    tiempo(b) * (COMPASES * TIEMPOS) as f32
}

/// Construye el tema de un baile como lista de voces.
pub fn tema(b: Baile) -> Vec<Voz> {
    let t = tiempo(b);
    let prog = b.progresion();
    let raiz = b.tonica();
    let mut v = Vec::with_capacity(256);

    for compas in 0..COMPASES {
        let acorde = prog[compas % prog.len()];
        let base = raiz + grado(acorde);
        let t0 = compas as f32 * TIEMPOS as f32 * t;

        // --- Bajo: el "oom". Solo en el primer tiempo, y grave.
        v.push(Voz::nota(Wave::Sine, nota(base - 12), t * 0.9, 0.30, 1.6).tras(t0));

        // --- Acompanamiento: los dos "pah". La triada, corta y bajita.
        // Este patron —grave al uno, acorde al dos y al tres— *es* el vals.
        // Es lo que hace que se reconozca antes de oir la melodia.
        for paso in 1..TIEMPOS {
            let cuando = t0 + paso as f32 * t;
            for tono in [0, 2, 4] {
                let f = nota(base + grado(tono));
                v.push(Voz::nota(Wave::Square, f, t * 0.35, 0.055, 3.2).tras(cuando));
            }
        }

        // --- Melodia: tres notas por compas sobre los tonos del acorde, una
        // octava arriba, con la frase subiendo y bajando a lo largo del bucle.
        let arco = [0, 2, 4, 2, 4, 2, 1, 0][compas];
        for paso in 0..TIEMPOS {
            let cuando = t0 + paso as f32 * t;
            let g = acorde + arco + [0, 2, 1][paso];
            let f = nota(raiz + grado(g) + 12);
            let vol = if paso == 0 { 0.20 } else { 0.13 };
            v.push(Voz::nota(Wave::Sine, f, t * 0.8, vol, 2.0).tras(cuando));
        }
    }

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODOS: [Baile; 3] = [Baile::Vals, Baile::ValsEspejo, Baile::ValsCoda];

    #[test]
    fn cada_baile_produce_notas() {
        for b in TODOS {
            let t = tema(b);
            assert!(t.len() > 50, "{b:?} apenas tiene notas: {}", t.len());
        }
    }

    #[test]
    fn ninguna_nota_se_sale_del_bucle() {
        // Si una nota terminase despues del bucle, el bucle daria un salto.
        for b in TODOS {
            let fin = duracion(b);
            for voz in tema(b) {
                assert!(
                    voz.fin() <= fin + 0.001,
                    "{b:?}: una nota acaba en {} y el bucle dura {fin}",
                    voz.fin()
                );
            }
        }
    }

    #[test]
    fn las_frecuencias_son_audibles() {
        for b in TODOS {
            for voz in tema(b) {
                let f = voz.frecuencia();
                assert!(
                    (40.0..2000.0).contains(&f),
                    "{b:?}: frecuencia rara, {f} Hz"
                );
            }
        }
    }

    #[test]
    fn los_tres_bailes_suenan_distinto() {
        let firma = |b: Baile| {
            tema(b)
                .iter()
                .map(|v| (v.frecuencia() as u32).wrapping_mul(31) ^ (v.fin() * 1000.0) as u32)
                .fold(0u32, |a, x| a.wrapping_mul(17).wrapping_add(x))
        };
        let f: Vec<u32> = TODOS.iter().map(|b| firma(*b)).collect();
        assert_ne!(f[0], f[1]);
        assert_ne!(f[1], f[2]);
        assert_ne!(f[0], f[2]);
    }

    #[test]
    fn el_compas_es_de_tres() {
        // El bajo cae solo en el primer tiempo de cada compas: es lo que hace
        // que se oiga como un vals y no como cualquier otra cosa.
        let b = Baile::Vals;
        let t = tiempo(b);
        let graves: Vec<f32> = tema(b)
            .iter()
            .filter(|v| v.frecuencia() < nota(0))
            .map(|v| v.inicio())
            .collect();
        assert_eq!(graves.len(), COMPASES);
        for (i, inicio) in graves.iter().enumerate() {
            let esperado = i as f32 * TIEMPOS as f32 * t;
            assert!(
                (inicio - esperado).abs() < 0.001,
                "el bajo {i} esta fuera de sitio"
            );
        }
    }

    #[test]
    fn el_bucle_dura_ocho_compases() {
        for b in TODOS {
            let esperado = 60.0 / b.bpm() * 24.0;
            assert!((duracion(b) - esperado).abs() < 0.001);
        }
    }

    #[test]
    fn la_afinacion_es_correcta() {
        assert!((nota(0) - 110.0).abs() < 0.01, "La2");
        assert!((nota(12) - 220.0).abs() < 0.2, "una octava arriba");
        assert!((nota(-12) - 55.0).abs() < 0.1, "una octava abajo");
        assert!((nota(24) - 440.0).abs() < 0.6, "dos octavas");
    }
}
