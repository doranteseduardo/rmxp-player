pub mod audio;

use std::path::Path;
use crate::marshal;

#[tauri::command]
pub fn load_data(path: String) -> Result<marshal::RubyValue, String> {
    println!("Loading data from: {}", path);
    let data = marshal::load_file(Path::new(&path)).map_err(|e| e.to_string())?;
    Ok(data)
}
