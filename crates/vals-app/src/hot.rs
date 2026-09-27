//! Recarga en caliente del patron del jefe.
//!
//! Guardas el RON y el cambio entra sin recompilar. Es la funcion que convierte
//! disenar un patron en algo que se itera en segundos en vez de en minutos, y
//! por eso estaba en el plan desde el principio: un proyecto que se toca a
//! rachas no puede permitirse un ciclo de prueba lento.
//!
//! Se implementa comprobando la fecha del fichero cada pocos frames, no con un
//! observador del sistema de ficheros. Es una dependencia menos y un hilo
//! menos, y para un fichero que edita una persona a mano sobra de largo.
//!
//! En web no hay sistema de ficheros: alli el patron es el que quedo embebido
//! en el binario y esto se convierte en un no-op.

use vals_core::boss::BossDef;
use vals_core::paseo::PaseoDef;

/// Ruta del patron, resuelta al compilar.
///
/// Relativa al manifiesto y no al directorio de trabajo, para que funcione
/// tanto con `cargo run` desde la raiz como lanzando el ejecutable a pelo.
///
/// El tamano sale de `DEFAULT_BOSS_RONS`, igual que el de los paseos: estuvo
/// escrito a mano como 3, y cuando entro el cuarto baile (primero el dembow y
/// luego el cancan) su RON no se recargaba. Asi, olvidarse de uno no compila.
#[cfg(not(target_arch = "wasm32"))]
const RUTAS: [&str; vals_core::boss::DEFAULT_BOSS_RONS.len()] = [
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/patterns/boss1.ron"
    ),
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/patterns/boss2.ron"
    ),
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/patterns/boss3.ron"
    ),
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/patterns/boss4.ron"
    ),
];

/// Los paseos, en el orden de `PASEO_RONS`. El tamano sale de esa lista, asi
/// que anadir un paseo sin su ruta aqui no compila.
#[cfg(not(target_arch = "wasm32"))]
const RUTAS_PASEOS: [&str; vals_core::paseo::PASEO_RONS.len()] = [concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/paseos/viena.ron"
)];

/// Cuantos frames se muestra el aviso de recarga.
#[cfg(not(target_arch = "wasm32"))]
const AVISO_FRAMES: u32 = 180;

pub struct HotReload {
    #[cfg(not(target_arch = "wasm32"))]
    last: [Option<std::time::SystemTime>; RUTAS.len()],
    /// La fecha del paseo que se esta andando, y cual es.
    #[cfg(not(target_arch = "wasm32"))]
    last_paseo: Option<(usize, std::time::SystemTime)>,
    aviso: Option<(String, bool)>,
    aviso_frames: u32,
}

impl HotReload {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            last: [None; RUTAS.len()],
            #[cfg(not(target_arch = "wasm32"))]
            last_paseo: None,
            aviso: None,
            aviso_frames: 0,
        }
    }

    /// Mensaje a mostrar y si es un error.
    pub fn aviso(&self) -> Option<(&str, bool)> {
        self.aviso.as_ref().map(|(m, e)| (m.as_str(), *e))
    }

    /// Comprueba si cambio algun patron. Devuelve los jefes si todos parsean.
    ///
    /// Si cualquiera tiene un error de sintaxis **no se recarga nada**: se
    /// avisa y se sigue jugando con la ultima version buena. Que un parentesis
    /// mal puesto te eche del juego a media sesion de afinado seria lo
    /// contrario de lo que busca esta funcion.
    ///
    /// Se recargan los tres de golpe aunque solo cambie uno. Es mas simple y no
    /// cuesta nada: son tres ficheros de texto.
    pub fn poll(&mut self) -> Option<Vec<BossDef>> {
        self.aviso_frames = self.aviso_frames.saturating_sub(1);
        if self.aviso_frames == 0 {
            self.aviso = None;
        }

        #[cfg(target_arch = "wasm32")]
        {
            None
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut cambio = false;
            let mut primera_vez = false;
            for (i, ruta) in RUTAS.iter().enumerate() {
                let Ok(m) = std::fs::metadata(ruta).and_then(|m| m.modified()) else {
                    continue;
                };
                if self.last[i] != Some(m) {
                    primera_vez |= self.last[i].is_none();
                    self.last[i] = Some(m);
                    cambio = true;
                }
            }
            // En el primer sondeo solo se toma nota de las fechas; no es una
            // recarga, y avisar de ella al arrancar seria ruido.
            if !cambio || primera_vez {
                return None;
            }

            let mut defs = Vec::with_capacity(RUTAS.len());
            for ruta in RUTAS {
                let src = match std::fs::read_to_string(ruta) {
                    Ok(s) => s,
                    Err(e) => {
                        self.set_aviso(format!("no se pudo leer {ruta}: {e}"), true);
                        return None;
                    }
                };
                match BossDef::from_ron(&src) {
                    Ok(d) => defs.push(d),
                    Err(e) => {
                        self.set_aviso(format!("RON invalido: {e}"), true);
                        return None;
                    }
                }
            }
            let nombres: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
            self.set_aviso(
                format!("patrones recargados: {}", nombres.join(", ")),
                false,
            );
            Some(defs)
        }
    }

    /// Lo mismo para el paseo `i`, que es el que se esta andando: solo se
    /// vigila ese, porque es el unico que se puede estar afinando. Un RON roto
    /// se avisa y se sigue con el nivel de antes.
    pub fn poll_paseo(&mut self, i: usize) -> Option<PaseoDef> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = i;
            None
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let ruta = RUTAS_PASEOS.get(i)?;
            let m = std::fs::metadata(ruta).and_then(|m| m.modified()).ok()?;
            let antes = self.last_paseo.replace((i, m));
            // Como con los jefes, el primer vistazo solo toma nota.
            if antes.is_none_or(|(j, f)| j != i || f == m) {
                return None;
            }
            let src = match std::fs::read_to_string(ruta) {
                Ok(s) => s,
                Err(e) => {
                    self.set_aviso(format!("no se pudo leer {ruta}: {e}"), true);
                    return None;
                }
            };
            match PaseoDef::from_ron(&src) {
                Ok(d) => {
                    self.set_aviso(format!("paseo recargado: {}", d.nombre), false);
                    Some(d)
                }
                Err(e) => {
                    self.set_aviso(format!("RON invalido: {e}"), true);
                    None
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn set_aviso(&mut self, msg: String, error: bool) {
        // Tambien por consola: el banner se ve mientras juegas, pero al afinar
        // un patron sueles tener la terminal al lado, y ahi queda el historial
        // de recargas y de errores de sintaxis.
        println!("[hot-reload] {msg}");
        self.aviso = Some((msg, error));
        self.aviso_frames = AVISO_FRAMES;
    }
}
