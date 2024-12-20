//! Used to backup the current world's save file. This does NOT save any new items, only creates a
//! backup of all currently saved data.

use bevy::prelude::*;
use chrono::Utc;
use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};
use zip::write::SimpleFileOptions;

use super::saving::SavingSystemSet;

#[derive(Event, Default)]
/// Send this event to trigger a world backup
pub struct CreateWorldBackup;

fn backup_world(mut evr_create_backup: EventReader<CreateWorldBackup>) {
    if evr_create_backup.is_empty() {
        return;
    }

    evr_create_backup.clear();

    info!("Backing up existing save data");
    let date_time = Utc::now();

    let formatted = format!("{}", date_time.format("%Y_%m_%d_%H_%M_%S"));
    let _ = std::fs::create_dir("./backups");
    if let Err(e) = zip_directory(Path::new("./world"), Path::new(&format!("./backups/{formatted}_world_backup.zip"))) {
        error!("Error backing up world!!!\n{e:?}");
    }
}

/// Zips the contents of a directory into a zip file.
///
/// # Arguments
///
/// * `src_dir` - The source directory to zip.
/// * `dest_file` - The path to the output zip file.
///
/// # Errors
///
/// Returns an error if the directory traversal or file writing fails.
pub fn zip_directory(src_dir: &Path, dest_file: &Path) -> io::Result<()> {
    let file = File::create(dest_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let mut buffer = Vec::new();

    let options = SimpleFileOptions::default();

    for entry in walkdir::WalkDir::new(src_dir) {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(src_dir).unwrap().to_str().unwrap();

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut f = File::open(path)?;
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if path.is_dir() {
            zip.add_directory(name, options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

pub(super) fn register(app: &mut App) {
    app.add_systems(First, backup_world.before(SavingSystemSet::BeginSaving))
        .add_event::<CreateWorldBackup>();
}
