use std::path::{Path, PathBuf};

use ini::Ini;

pub struct Repository {
    worktree: PathBuf,
    gitdir: PathBuf,
    conf: Ini,
}

impl Repository {
    pub fn new(path: PathBuf, force: bool) -> Self {
        let gitdir = path.join(".git");
        // let res =

        if !(force || gitdir.is_dir()) {
            panic!("Not a Git Repository {:?}", path)
        }

        let conf = if gitdir.join("config").exists() {
            Ini::load_from_file("conf.ini").unwrap()
        } else if !force {
            panic!("Configuration file missing");
        } else {
            Ini::new()
        };

        if !force {
            conf.section(Some("core"))
                .unwrap()
                .get("repositoryformatversion");
        }

        Self {
            gitdir,
            worktree: path,
            conf,
        }
    }

    fn repo_file(&self, p: &str) -> PathBuf {
        self.gitdir.join(p)
    }
}
