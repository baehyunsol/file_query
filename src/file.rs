use crate::{FILES, PATHS};
use crate::utils::{get_file_by_uid, get_path_by_uid};
use crate::uid::Uid;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FileType {
    File,
    Dir,
    Symlink,
}

impl fmt::Display for FileType {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            fmt, "{}",
            match self {
                FileType::File => "file",
                FileType::Dir => "dir",
                FileType::Symlink => "link",
            }
        )
    }
}

pub struct File {
    pub parent: Option<Uid>,
    pub uid: Uid,
    pub name: String,  // not path, just name
    pub last_modified: SystemTime,
    pub size: u64,
    pub recursive_size: Option<u64>,  // if it's not calculated yet, it's None
    pub file_type: FileType,
    pub file_ext: Option<String>,
    pub children: Option<Vec<Uid>>,
}

impl File {
    // it registers the instance to the cache, and only returns its uid
    pub fn new_from_path_buf(path: PathBuf, uid: Option<Uid>, parent: Option<Uid>) -> Uid {
        let name = match path.file_name() {
            Some(s) => match s.to_str() {
                Some(s) => s.to_string(),
                None => {
                    return File::from_error_msg(String::new());
                },
            },
            None if uid == Some(Uid::ROOT) => String::new(),
            None => {
                return File::from_error_msg(String::new());
            },
        };
        let (last_modified, size) = match path.metadata() {
            Ok(metadata) => match metadata.modified() {
                Ok(last_modified) => (last_modified, metadata.len()),
                Err(e) => {
                    return File::from_io_error(e);
                },
            }
            Err(e) => {
                return File::from_io_error(e);
            },
        };
        let file_type = if path.is_symlink() {
            FileType::Symlink
        } else if path.is_dir() {
            FileType::Dir
        } else {
            FileType::File
        };
        let file_ext = match path.extension() {
            Some(ext) => match ext.to_str() {
                Some(s) => Some(s.to_string()),
                None => None,
            },
            None => None,
        };

        let result = File {
            parent,
            uid: uid.unwrap_or_else(|| Uid::normal_file()),
            name,
            last_modified,
            size,
            recursive_size: if file_type == FileType::File { Some(size) } else { None },
            file_type,
            file_ext,
            children: None,
        };

        let result_uid = result.uid;

        let files = unsafe { FILES.as_mut().unwrap() };
        files.insert(result_uid, result);

        let paths = unsafe { PATHS.as_mut().unwrap() };
        paths.insert(result_uid, path.to_str().unwrap().to_string());

        result_uid
    }

    // it registers the instance to the cache, and only returns its uid
    pub fn new_from_dir_path(path: String, uid: Option<Uid>, parent: Option<Uid>) -> Uid {
        let path = PathBuf::from_str(&path).unwrap();  // infallible

        File::new_from_path_buf(path, uid, parent)
    }

    // it registers the instance to the cache, and only returns its uid
    pub fn new_from_dir_entry(dir_entry: fs::DirEntry, parent: Option<Uid>) -> Uid {
        let (last_modified, size, file_type) = match dir_entry.metadata() {
            Ok(metadata) => {
                let file_type = if metadata.is_symlink() {
                    FileType::Symlink
                } else if metadata.is_dir() {
                    FileType::Dir
                } else {
                    FileType::File
                };
                let size = metadata.len();
                let last_modified = match metadata.modified() {
                    Ok(last_modified) => last_modified,
                    Err(e) => {
                        return File::from_io_error(e);
                    },
                };
    
                (last_modified, size, file_type)
            },
            Err(e) => {
                return File::from_io_error(e);
            },
        };
        let name = match dir_entry.file_name().to_str() {
            Some(s) => s.to_string(),
            None => {
                return File::from_error_msg(String::new());
            },
        };
        let file_ext = match dir_entry.path().extension() {
            Some(ext) => match ext.to_str() {
                Some(s) => Some(s.to_string()),
                None => None,
            },
            None => None,
        };

        let result = File {
            parent,
            uid: Uid::normal_file(),
            name,
            last_modified,
            size,
            recursive_size: if file_type == FileType::File { Some(size) } else { None },
            file_type,
            file_ext,
            children: None,
        };

        let result_uid = result.uid;

        let files = unsafe { FILES.as_mut().unwrap() };
        files.insert(result_uid, result);

        result_uid
    }

    // it registers the instance to the cache, and only returns its uid
    pub fn from_io_error(e: io::Error) -> Uid {
        let message = match e.kind() {
            io::ErrorKind::PermissionDenied => String::from("Permission Denied"),
            e => panic!("{e:?}"),
        };
        let message = format!("<<Error: {message}>>");
        let uid = Uid::error();

        let result = File {
            parent: None,
            uid,
            name: message,
            ..File::dummy()
        };

        let files = unsafe { FILES.as_mut().unwrap() };
        files.insert(uid, result);

        uid
    }

    // it registers the instance to the cache, and only returns its uid
    pub fn from_error_msg(e: String) -> Uid {
        let message = if e.is_empty() {
            String::from("<<Error>>")
        } else {
            format!("<<Error: {e}>>")
        };
        let uid = Uid::error();

        let result = File {
            parent: None,
            uid,
            name: message,
            ..File::dummy()
        };

        let files = unsafe { FILES.as_mut().unwrap() };
        files.insert(uid, result);

        uid
    }

    // it registers the instance to the cache, and only returns its uid
    pub fn message_for_truncated_rows(n: usize) -> Uid {
        let uid = Uid::message_for_truncated_rows(n);

        // for performance, it doesn't instantiate the same instance multiple times
        if get_file_by_uid(uid).is_some() {
            return uid;
        }

        let result = File {
            parent: None,
            uid,
            name: format!(
                "... (truncated {n} row{})",
                if n < 2 { "" } else { "s" },
            ),
            ..File::dummy()
        };

        let files = unsafe { FILES.as_mut().unwrap() };
        files.insert(uid, result);

        uid
    }

    // It's safe (and recommended) to call this function multiple times.
    pub fn init_children(&mut self) {
        if self.children.is_some() || !self.is_dir() {
            return;
        }

        let self_path = get_path_by_uid(self.uid).unwrap();

        match fs::read_dir(self_path) {
            Ok(entries) => {
                let mut result = vec![];

                for entry in entries {
                    match entry {
                        Ok(e) => {
                            result.push(File::new_from_dir_entry(e, Some(self.uid)));
                        },
                        Err(e) => {
                            result.push(File::from_io_error(e));
                        },
                    }
                }

                self.children = Some(result);
            },
            Err(e) => {
                self.children = Some(vec![File::from_io_error(e)]);
            },
        }
    }

    pub fn is_dir(&self) -> bool {
        !self.is_special_file() && matches!(self.file_type, FileType::Dir)
    }

    pub fn is_file(&self) -> bool {
        !self.is_special_file() && matches!(self.file_type, FileType::File)
    }

    pub fn is_hidden_file(&self) -> bool {
        !self.is_special_file() && self.name.starts_with(".")
    }

    // not a file
    // it's either an error or a system prompt
    pub fn is_special_file(&self) -> bool {
        self.uid.is_special()
    }

    pub fn get_children(&self, show_hidden_files: bool) -> Vec<&File> {
        if self.get_children_num(show_hidden_files) == 0 {
            vec![]
        }

        else {
            let mut child_iter = self.children.as_ref().unwrap().iter().map(
                |child| get_file_by_uid(*child).unwrap() as &File
            );

            if show_hidden_files {
                child_iter.collect()
            }

            else {
                child_iter.filter(
                    |child| !child.is_hidden_file()
                ).collect()
            }
        }
    }

    // it calls `init_children` if it has to
    pub fn get_children_num(&self, include_hidden_files: bool) -> usize {
        if self.is_dir() {
            match &self.children {
                Some(c) => if include_hidden_files {
                    c.len()
                } else {
                    c.iter().map(
                        |uid| get_file_by_uid(*uid).unwrap()
                    ).filter(
                        |c| !c.is_hidden_file()
                    ).count()
                },
                None => {
                    let very_unsafe_object = get_file_by_uid(self.uid).unwrap();
                    very_unsafe_object.init_children();

                    if include_hidden_files {
                        very_unsafe_object.children.as_ref().unwrap().len()
                    }

                    else {
                        very_unsafe_object.children.as_ref().unwrap().iter().map(
                            |uid| get_file_by_uid(*uid).unwrap() as &File
                        ).filter(
                            |c| !c.is_hidden_file()
                        ).count()
                    }
                },
            }
        } else {
            0
        }
    }

    pub fn get_parent_uid(&self) -> Uid {
        if !self.is_special_file() {
            match self.parent {
                Some(uid) => uid,
                None => {
                    let path = get_path_by_uid(self.uid).unwrap();
                    let std_path = Path::new(path);
                    let parent_path = std_path.parent().unwrap().to_string_lossy().to_string();

                    // TODO: better way to find the root dir
                    let parent_uid = if parent_path == "/" {
                        Uid::ROOT
                    } else {
                        Uid::normal_file()
                    };

                    let parent_uid = File::new_from_dir_path(parent_path, Some(parent_uid), None);

                    // what an unsafe operation
                    get_file_by_uid(self.uid).unwrap().parent = Some(parent_uid);

                    parent_uid
                },
            }
        }

        else {
            unreachable!()
        }
    }

    pub fn get_recursive_size(&self) -> u64 {
        match self.recursive_size {
            Some(s) => s,
            None => {
                let mut sum = 0;

                for child in self.get_children(true).iter() {
                    sum += child.get_recursive_size();
                }

                // what an unsafe operation
                get_file_by_uid(self.uid).unwrap().recursive_size = Some(sum);

                sum
            },
        }
    }

    // make sure that nobody reads these values
    pub fn dummy() -> Self {
        File {
            parent: None,
            uid: Uid::error(),
            name: String::new(),
            last_modified: SystemTime::now(),
            size: 0,
            recursive_size: None,
            file_type: FileType::File,
            file_ext: None,
            children: None,
        }
    }

    pub fn debug_info(&self) -> String {
        let parent_info = match self.parent {
            Some(p) => format!(
                "Some({:?})",
                get_path_by_uid(p),
            ),
            None => String::from("None"),
        };
        let uid_info = self.uid.debug_info();

        format!(
            "File {}parent: {parent_info}, uid: {uid_info}, name: {}, last_modified: {:?}, size: {}, recursive_size: {:?}, file_type: {:?}, file_ext: {:?}{}",
            '{',
            self.name,
            self.last_modified,
            self.size,
            self.recursive_size,
            self.file_type,
            self.file_ext,
            '}',
        )
    }
}
