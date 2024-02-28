use std::{fs, path::Path};

pub fn remove_dir_contents<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            remove_dir_contents(&path)?;
            fs::remove_dir(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}
