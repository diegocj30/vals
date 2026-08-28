//! Herramienta de replays.
//!
//! Vive en `vals-core` y no en la app a proposito: no necesita ventana, ni
//! GPU, ni macroquad. Verificar un replay es logica pura, y poder hacerlo
//! desde la terminal es lo que lo convierte en algo que se puede meter en CI.
//!
//! ```text
//! cargo run -p vals-core --example replay_tool -- verify <fichero>
//! cargo run -p vals-core --example replay_tool -- info <fichero>
//! cargo run -p vals-core --example replay_tool -- record-golden [fichero]
//! ```

use std::path::Path;
use std::process::ExitCode;

use vals_core::replay::{self, Replay};

const GOLDEN_PATH: &str = "assets/replays/golden.valsrpl";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("verify") => match args.get(1) {
            Some(p) => verify(Path::new(p)),
            None => uso(),
        },
        Some("info") => match args.get(1) {
            Some(p) => info(Path::new(p)),
            None => uso(),
        },
        Some("record-golden") => {
            let destino = args.get(1).map(String::as_str).unwrap_or(GOLDEN_PATH);
            record_golden(Path::new(destino))
        }
        _ => uso(),
    }
}

fn uso() -> ExitCode {
    eprintln!("uso: replay_tool <verify|info> <fichero>");
    eprintln!("     replay_tool record-golden [fichero]");
    ExitCode::FAILURE
}

fn leer(path: &Path) -> Option<Replay> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("no se pudo leer {}: {e}", path.display());
            return None;
        }
    };
    match Replay::from_bytes(&bytes) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            None
        }
    }
}

fn verify(path: &Path) -> ExitCode {
    let Some(r) = leer(path) else {
        return ExitCode::FAILURE;
    };
    match r.verify() {
        Ok(rep) => {
            println!(
                "OK  {} ticks ({:.1} s), {} huellas comprobadas, hash final {:#018x}",
                rep.ticks,
                r.seconds(),
                rep.checkpoints,
                rep.final_hash
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("FALLO  {e}");
            ExitCode::FAILURE
        }
    }
}

fn info(path: &Path) -> ExitCode {
    let Some(r) = leer(path) else {
        return ExitCode::FAILURE;
    };
    println!("semilla        {:#018x}", r.seed);
    println!("patron         {:#018x}", r.pattern_hash);
    println!(
        "patron actual  {:#018x}{}",
        replay::default_pattern_hash(),
        if r.pattern_hash == replay::default_pattern_hash() {
            ""
        } else {
            "   <-- NO COINCIDE"
        }
    );
    println!("ticks          {} ({:.1} s)", r.ticks(), r.seconds());
    println!(
        "huellas        {} (cada {})",
        r.checkpoints.len(),
        r.checkpoint_every
    );
    println!("tamano         {} bytes", r.to_bytes().len());
    ExitCode::SUCCESS
}

fn record_golden(path: &Path) -> ExitCode {
    let r = replay::record_golden();
    if let Some(dir) = path.parent()
        && let Err(e) = std::fs::create_dir_all(dir)
    {
        eprintln!("no se pudo crear {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    let bytes = r.to_bytes();
    if let Err(e) = std::fs::write(path, &bytes) {
        eprintln!("no se pudo escribir {}: {e}", path.display());
        return ExitCode::FAILURE;
    }
    println!(
        "grabado {} - {} ticks, {} huellas, {} bytes",
        path.display(),
        r.ticks(),
        r.checkpoints.len(),
        bytes.len()
    );
    ExitCode::SUCCESS
}
