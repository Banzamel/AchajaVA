// The version of Achaja comes from package.json (the single source of truth), so the hooks,
// the command line and the update check all report the version of the installer.
fn main() {
    let pkg = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../package.json");
    println!("cargo:rerun-if-changed={}", pkg.display());
    let text = std::fs::read_to_string(&pkg).unwrap_or_default();
    let version = text
        .lines()
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("\"version\":").map(|v| v.trim().trim_end_matches(',').trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "0.0.0".into());
    println!("cargo:rustc-env=ACHAJA_VERSION={version}");
}
