use crate::{File, FILES, Path, PATHS, Uid};
use crate::print::ColumnKind;
use std::path::PathBuf;
use std::str::FromStr;

pub fn get_file_by_uid<'a>(uid: Uid) -> Option<&'a mut File> {
    let files = unsafe { FILES.as_mut().unwrap() };

    files.get_mut(&uid)
}

// It returns `Some` if `uid` is valid.
pub fn get_path_by_uid<'a>(uid: Uid) -> Option<&'a Path> {
    let paths = unsafe { PATHS.as_mut().unwrap() };

    match paths.get(&uid) {
        Some(path) => unsafe { Some(std::mem::transmute(path)) },
        None => {
            let files = unsafe { FILES.as_mut().unwrap() };

            match files.get(&uid) {
                Some(file) => {
                    let path = match get_path_by_file(file) {
                        Some(path) => path,
                        None => {
                            return None;
                        },
                    };
                    paths.insert(uid, path.clone());
                    paths.get(&uid)
                },
                None => None,
            }
        },
    }
}

fn get_path_by_file(file: &File) -> Option<String> {
    match file.parent {
        Some(parent) => {
            let parent_path = get_path_by_uid(parent).unwrap();
            let mut parent_path = PathBuf::from_str(parent_path).unwrap();  // infallible
            let child_path = PathBuf::from_str(&file.name).unwrap();  // infallible

            parent_path.push(child_path);

            Some(parent_path.to_str().unwrap().to_string())
        },
        None if file.uid == Uid::ROOT => Some(String::from("/")),
        None => None,
    }
}

pub fn sort_files(files: &mut Vec<&File>, sort_by: ColumnKind, reverse: bool) {
    match sort_by {
        ColumnKind::Index => unreachable!(),
        ColumnKind::Name => {
            files.sort_by_key(|file| &file.name);
        },
        ColumnKind::Size => {
            files.sort_by_key(|file| file.size);
        },
        ColumnKind::TotalSize => {
            files.sort_by_key(|file| file.get_recursive_size());
        },
        ColumnKind::Modified => {
            files.sort_by_key(|file| file.last_modified);
        },
        ColumnKind::FileType => {
            files.sort_by_key(|file| file.file_type);
        },
        ColumnKind::FileExt => {
            files.sort_by_key(|file| file.file_ext.clone().unwrap_or(String::new()));
        },
    }

    if reverse {
        files.reverse();
    }
}
