// build.rs — conditionally add locally-extracted dev libraries to the linker
// search path. The GTK4/Libadwaita dev packages are not installed system-wide
// on this machine; the .deb files are extracted to `deps/` (which is
// gitignored). PKG_CONFIG_PATH must still point at the deps pkgconfig dir so
// pkg-config can find the .pc files, but this script removes the need for a
// manual RUSTFLAGS="-C link-arg=-L..." flag.
//
// The check is conditional so that CI (which installs system packages and has
// no deps/ dir) is unaffected.

use std::path::Path;

fn main() {
    let deps = Path::new(env!("CARGO_MANIFEST_DIR")).join("deps/usr/lib/x86_64-linux-gnu");
    if deps.is_dir() {
        println!("cargo:rustc-link-search={}", deps.display());
        println!("cargo:rerun-if-changed={}", deps.display());
    }
}
