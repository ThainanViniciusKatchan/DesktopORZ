use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=pt-br.json");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // OUT_DIR = target/<perfil>/build/<pkg>-<hash>/out -> sobe 3 níveis até target/<perfil>
    let exe_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR inesperado")
        .to_path_buf();
    fs::copy("pt-br.json", exe_dir.join("pt-br.json")).expect("falha ao copiar pt-br.json");
}
