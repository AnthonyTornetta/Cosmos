//! Used to backup the current world's save file. This does NOT save any new items, only creates a
//! backup of all currently saved data.

use bevy::{prelude::*, time::common_conditions::on_timer};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use cosmos_core::utils::timer::UtilsTimer;
use std::{
    ffi::OsStr,
    fs::File,
    io::{self, Read, Write},
    path::Path,
};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

use crate::persistence::{WorldRoot, saving::SAVING_SCHEDULE};

use super::saving::SavingSystemSet;

#[derive(Message, Default)]
/// Send this event to trigger a world backup
pub struct CreateWorldBackup;

const DATE_FORMAT: &str = "%Y_%m_%d_%H_%M_%S";
const BACKUP_ENDING: &str = "_world_backup.zip";

fn backup_world(mut evr_create_backup: MessageReader<CreateWorldBackup>, world_root: Res<WorldRoot>) {
    if evr_create_backup.is_empty() {
        return;
    }

    evr_create_backup.clear();

    info!("Backing up existing save data");
    let date_time = Utc::now();

    let timer = UtilsTimer::start();

    let formatted = format!("{}", date_time.format(DATE_FORMAT));
    let _ = std::fs::create_dir_all(world_root.path_for("backups/"));
    let dest_path = format!("{}/{formatted}{BACKUP_ENDING}", world_root.path_for("backups"));
    let dest_path = Path::new(&dest_path);
    if let Err(e) = zip_directory(Path::new(world_root.get()), &dest_path, &["backups"]) {
        error!("Error backing up world!!!\n{e:?}");
    } else {
        info!("Backup saved to {dest_path:?}");
    }

    timer.log_duration("Backup took");
}

fn cleanup_backups(world_root: Res<WorldRoot>) {
    info!("Initiating backup prune.");

    let now = Utc::now();

    let mut backups = vec![];

    for backup in WalkDir::new(world_root.path_for("backups")).max_depth(1) {
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

        if !file_name.ends_with(BACKUP_ENDING) {
            continue;
        }

        let Ok(date_time_parsed) =
            NaiveDateTime::parse_from_str(&file_name[0..file_name.len() - BACKUP_ENDING.len()], DATE_FORMAT).map(|x| x.and_utc())
        else {
            continue;
        };

        if now.signed_duration_since(date_time_parsed).num_milliseconds() < 0 {
            // Don't delete backups marked as being taken in the future, the system clock is
            // probably wrong in that case.
            continue;
        }
        backups.push((date_time_parsed, path.to_string_lossy().to_string()));
    }

    backups.sort_by_key(|x| x.0);

    backups.reverse();

    prune_by_interval(&mut backups, now, Duration::minutes(10), Duration::hours(1));
    prune_by_interval(&mut backups, now, Duration::hours(1), Duration::hours(24));
    prune_by_interval(&mut backups, now, Duration::days(1), Duration::days(7));
    prune_by_interval(&mut backups, now, Duration::weeks(1), Duration::weeks(4));

    if backups.is_empty() {
        info!("No backups to prune.");
    }

    // If any backups remain in this list, they don't meet our time-span criteria.
    for (_, path) in backups.into_iter() {
        info!("Pruning old backup {path}");
        std::fs::remove_file(&path).unwrap_or_else(|e| panic!("failed to remove file @ {path}!\n{e:?}"));
    }
}

/// Keep one backup ever `interval` timespan for the last `max_age`.
fn prune_by_interval(backups: &mut Vec<(DateTime<Utc>, String)>, now: DateTime<Utc>, interval: Duration, max_age: Duration) {
    let mut range_end = now;
    let range_start = now - max_age;

    while range_end > range_start {
        let range_begin = range_end - interval;
        // Find the most recent backup within the range
        if let Some(pos) = backups
            .iter()
            .position(|(timestamp, _)| *timestamp <= range_end && *timestamp > range_begin)
        {
            backups.remove(pos);
        }
        range_end = range_begin;
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
pub fn zip_directory(src_dir: &Path, dest_file: &Path, ignore: &[&str]) -> io::Result<()> {
    let file = File::create(dest_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let mut buffer = Vec::new();

    let options = SimpleFileOptions::default();

    for entry in walkdir::WalkDir::new(src_dir) {
        let entry = entry?;
        let path = entry.path();

        let name = path.strip_prefix(src_dir).unwrap().to_str().unwrap();

        if ignore.iter().any(|i| name.to_lowercase().starts_with(*i)) {
            continue;
        };

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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Responsible for taking a backup of the currently saved world
pub enum BackupSystemSet {
    /// Copies the currently saved world into a zip backup
    PerformBackup,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(SAVING_SCHEDULE, BackupSystemSet::PerformBackup.before(SavingSystemSet::BeginSaving));

    app.add_systems(SAVING_SCHEDULE, backup_world.in_set(BackupSystemSet::PerformBackup))
        .add_systems(FixedUpdate, cleanup_backups.run_if(on_timer(std::time::Duration::from_mins(20))))
        .add_message::<CreateWorldBackup>();
}
