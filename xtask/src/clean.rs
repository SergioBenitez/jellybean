use crate::expand::PackExpander;

pub fn main(_: &[&str]) -> std::io::Result<()> {
    let artifacts_dir = crate::crate_path!("artifacts");
    if artifacts_dir.exists() {
        println!("- removing {}", artifacts_dir.display());
        std::fs::remove_dir_all(artifacts_dir)?;
    } else {
        println!(":: nothing to clean");
    }

    for entry in PackExpander::pack_dirs()? {
        let path = entry?.path();
        println!("- removing {}", path.display());
        std::fs::remove_dir_all(&path)?;
    }

    crate::sync::main(&[])
}
