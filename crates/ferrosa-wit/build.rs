fn main() {
    let wit = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ferrosa-wit/wit");

    println!("cargo::rerun-if-changed={}", wit.display());

    for file in std::fs::read_dir(&wit).into_iter().flatten().flatten() {
        println!("cargo::rerun-if-changed={}", file.path().display());
    }
}
