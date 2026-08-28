//! Guardar y cargar replays en disco.
//!
//! Aislado del resto de la app porque es lo unico del modulo de replays que
//! toca el sistema de ficheros, y en web no existe. `vals-core` no sabe nada de
//! esto: alli un replay son bytes y punto.

/// Carpeta donde se guardan los replays de las partidas.
#[cfg(not(target_arch = "wasm32"))]
const DIR: &str = "replays";

/// Guarda un replay y devuelve la ruta.
#[cfg(not(target_arch = "wasm32"))]
pub fn save(bytes: &[u8]) -> Result<String, String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    std::fs::create_dir_all(DIR).map_err(|e| format!("no se pudo crear {DIR}: {e}"))?;
    // Segundos desde epoch como nombre: ordena solo y no colisiona salvo que
    // guardes dos veces en el mismo segundo.
    let marca = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let ruta = format!("{DIR}/{marca}.valsrpl");
    std::fs::write(&ruta, bytes).map_err(|e| format!("no se pudo escribir {ruta}: {e}"))?;
    Ok(ruta)
}

#[cfg(target_arch = "wasm32")]
pub fn save(_bytes: &[u8]) -> Result<String, String> {
    Err("en web no hay sistema de ficheros".to_owned())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("no se pudo leer {path}: {e}"))
}

#[cfg(target_arch = "wasm32")]
pub fn load(_path: &str) -> Result<Vec<u8>, String> {
    Err("en web no hay sistema de ficheros".to_owned())
}
