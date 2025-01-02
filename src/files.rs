use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, io::Write};
use std::{fs, io, path::Path};
use walkdir::WalkDir;

struct ImageList {
    images: Vec<String>,
}

impl ImageList {
    /// Gibt die Bilder als kommagetrennte Liste in Anführungszeichen zurück.
    fn to_comma_separated(&self) -> String {
        self.images
            .iter()
            .map(|image| format!("\"{}\"", image)) // Setzt jeden Eintrag in Anführungszeichen
            .collect::<Vec<_>>() // Sammelt die Einträge in einen Vektor
            .join(", ") // Verbindet sie mit Kommas
    }
}

fn is_image_file(entry: &Path) -> bool {
    entry
        .extension()
        .and_then(|ext| ext.to_str())
        .map_or(false, |ext| {
            matches!(
                ext.to_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "webp"
            )
        })
}

fn collect_image_files(dir: &Path) -> io::Result<ImageList> {
    let images: Result<Vec<_>, _> = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file() && is_image_file(entry.path()))
        .map(|entry| {
            entry
                .file_name()
                .to_str()
                .map(String::from)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid filename"))
        })
        .collect();

    images.map(|mut images| {
        images.reverse();
        ImageList { images }
    })
}

pub fn to_js_list(save_dir: &Path) -> io::Result<String> {
    let image_list = collect_image_files(&save_dir)?;
    Ok(image_list.to_comma_separated())
}

pub fn save_image_to_directory(
    image_data: Vec<u8>,
    haiku_text: &str,
) -> Result<(), std::io::Error> {
    let save_dir = env::var("IMAGE_SAVE_DIR").expect("IMAGE_SAVE_DIR muss in .env definiert sein!");
    let dir_path = Path::new(&save_dir);

    // Verzeichnis erstellen, falls es nicht existiert
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)?;
    }

    // Eindeutigen Dateinamen generieren
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Systemzeit konnte nicht ermittelt werden")
        .as_secs(); // Zeitstempel in Sekunden
    let filename = format!("haiku_{}_{}.webp", timestamp, haiku_text.len());
    let file_path = dir_path.join(filename);

    // Bild in die Datei schreiben
    let mut file = fs::File::create(file_path)?;
    file.write_all(&image_data)?;

    Ok(())
}
