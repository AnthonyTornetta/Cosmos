//! Used to backup the current world's save file. This does NOT save any new items, only creates a
//! backup of all currently saved data.

use bevy::{prelude::*, time::common_conditions::on_timer};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    path::Path,
};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

use super::saving::SavingSystemSet;

#[derive(Event, Default)]
/// Send this event to trigger a world backup
pub struct CreateWorldBackup;

const DATE_FORMAT: &str = "%Y_%m_%d_%H_%M_%S";

fn backup_world(mut evr_create_backup: EventReader<CreateWorldBackup>) {
    if evr_create_backup.is_empty() {
        return;
    }

    evr_create_backup.clear();

    info!("Backing up existing save data");
    let date_time = Utc::now();

    let formatted = format!("{}", date_time.format(DATE_FORMAT));
    let _ = std::fs::create_dir("./backups");
    if let Err(e) = zip_directory(Path::new("./world"), Path::new(&format!("./backups/{formatted}_world_backup.zip"))) {
        error!("Error backing up world!!!\n{e:?}");
    }
}

fn cleanup_backups() {
    let mut backups = vec![];
    for backup in WalkDir::new("backups").max_depth(1) {
        let Ok(backup) = backup else {
            continue;
        };

        let path = backup.path();
        if path.extension() != Some(OsStr::new("zip")) {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|x| x.to_str()) else {
            continue;
        };

        let Ok(date_time_parsed) =
            NaiveDateTime::parse_from_str(&file_name[0..file_name.len() - ".zip".len()], DATE_FORMAT).map(|x| x.and_utc())
        else {
            continue;
        };

        info!("{date_time_parsed:?}");

        backups.push((date_time_parsed, path.to_string_lossy().to_string()));
    }

    backups.sort_by_key(|x| x.0);
    backups.reverse();

    let now = Utc::now();

    // Keep one backup every 5 minutes for the last hour
    prune_by_interval(&mut backups, now, Duration::minutes(5), Duration::hours(1));

    // Keep one backup every hour for the last 24 hours
    prune_by_interval(&mut backups, now, Duration::hours(1), Duration::hours(24));

    // Keep one backup per day for the last 7 days
    prune_by_interval(&mut backups, now, Duration::days(1), Duration::days(7));

    // Keep one backup per week for the last 4 weeks
    prune_by_interval(&mut backups, now, Duration::weeks(1), Duration::weeks(4));

    // Remove all other backups
    for (_, path) in backups {
        info!("Pruning old backup {path}");
        std::fs::remove_file(&path).unwrap_or_else(|e| panic!("failed to remove file @ {path}!\n{e:?}"));
    }
}

fn prune_by_interval(backups: &mut Vec<(DateTime<Utc>, String)>, now: DateTime<Utc>, interval: Duration, max_age: Duration) {
    let mut next_cutoff = now - interval;
    let max_cutoff = now - max_age;

    backups.retain(|(timestamp, _)| {
        if *timestamp > max_cutoff && *timestamp <= next_cutoff {
            next_cutoff -= interval;
            false
        } else {
            true
        }
    });
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
        .add_systems(Update, cleanup_backups.run_if(on_timer(std::time::Duration::from_mins(5))))
        .add_event::<CreateWorldBackup>();
}
