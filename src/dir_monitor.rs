use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::Result;

#[derive(Debug)]
pub struct DirectoryMonitor {
    directory: PathBuf,

    files_set_before: HashSet<PathBuf>,
    files_set_after: HashSet<PathBuf>,
}

fn dir_files_set<P>(dir: P) -> Result<HashSet<PathBuf>>
where
    P: AsRef<Path>,
{
    Ok(fs::read_dir(dir.as_ref())?
        .filter_map(|entry| match entry {
            Ok(dir) => Some(dir.path()),
            Err(err) => {
                log::error!("Reading failed: {:?}", err);
                None
            }
        })
        .collect())
}

impl DirectoryMonitor {
    pub fn new<P>(dir: P) -> Result<DirectoryMonitor>
    where
        P: AsRef<Path>,
    {
        Ok(DirectoryMonitor {
            directory: dir.as_ref().to_path_buf(),
            files_set_before: dir_files_set(dir)?,
            files_set_after: HashSet::new(),
        })
    }

    pub fn check(&mut self) -> Result<impl Iterator<Item = &PathBuf>> {
        self.files_set_after = dir_files_set(&self.directory)?;
        Ok(self.files_set_after.difference(&self.files_set_before))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hygeia_test_helpers::create_test_temp_dir;
    use std::{
        fs::{create_dir_all, remove_dir_all, File},
        io::Write,
    };

    #[test]
    fn nothing_changed() {
        let mut d = DirectoryMonitor::new(".").unwrap();
        assert_eq!(d.check().unwrap().count(), 0);
    }

    #[test]
    fn one_file() {
        let tmp_dir = create_test_temp_dir!();
        let _ = remove_dir_all(&tmp_dir);
        create_dir_all(&tmp_dir).unwrap();

        let mut d = DirectoryMonitor::new(&tmp_dir).unwrap();
        assert!(d.files_set_before.is_empty());
        assert!(d.files_set_after.is_empty());

        let tmp_filename = tmp_dir.join("test");
        let mut tmp_file = File::create(tmp_filename).unwrap();
        tmp_file.write_all(b"test").unwrap();

        assert_eq!(d.check().unwrap().count(), 1);
        assert!(d.files_set_before.is_empty());
        assert!(!d.files_set_after.is_empty());

        let _ = remove_dir_all(&tmp_dir);
    }
}
