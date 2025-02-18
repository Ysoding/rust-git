use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{bail, Result};
use ini::Ini;

pub struct Repository {
    worktree: PathBuf,
    gitdir: PathBuf,
    conf: Ini,
}

impl Repository {
    fn new(path: PathBuf, force: bool) -> Result<Self> {
        let gitdir = path.join(".git");

        if !(force || gitdir.is_dir()) {
            bail!("Not a Git Repository {:?}", path)
        }

        let conf = if gitdir.join("config").exists() {
            Ini::load_from_file("conf.ini").unwrap()
        } else if !force {
            bail!("Configuration file missing");
        } else {
            Ini::new()
        };

        if !force {
            let vers = conf
                .section(Some("core"))
                .unwrap()
                .get("repositoryformatversion")
                .unwrap()
                .parse::<i64>()
                .unwrap();
            if vers != 0 {
                bail!("Unsupported repositoryformatversion:{}", vers);
            }
        }

        Ok(Self {
            gitdir,
            worktree: path,
            conf,
        })
    }

    fn repo_path(&self, p: PathBuf) -> PathBuf {
        self.gitdir.join(p)
    }
}

pub fn repo_create(path: PathBuf) -> Result<Repository> {
    let repo = Repository::new(path, true)?;

    if repo.worktree.exists() {
        if !repo.worktree.is_dir() {
            bail!("{:?} is not a directory!", repo.gitdir);
        }

        if repo.gitdir.exists() {
            if let Ok(entries) = fs::read_dir(&repo.gitdir) {
                if entries.count() > 0 {
                    bail!("{:?} is not empty!", repo.gitdir);
                }
            } else {
                bail!("Unable to read directory: {:?}", repo.gitdir);
            }
        }
    } else {
        fs::create_dir_all(repo.worktree.clone())?;
    }

    assert!(repo_dir(&repo, PathBuf::from("branches"), true).is_ok_and(|x| x.is_some()));
    assert!(repo_dir(&repo, PathBuf::from("objects"), true).is_ok_and(|x| x.is_some()));
    assert!(repo_dir(&repo, PathBuf::from("refs/tags"), true).is_ok_and(|x| x.is_some()));
    assert!(repo_dir(&repo, PathBuf::from("refs/heads"), true).is_ok_and(|x| x.is_some()));

    let p = repo_file(&repo, PathBuf::from("description"), false).expect("create description: ");
    let mut f = File::create(p)?;
    f.write_all(b"Unnamed repository; edit this file 'description' to name the repository.\n")
        .expect("write description: ");

    let p = repo_file(&repo, PathBuf::from("HEAD"), false).expect("create HEAD: ");
    let mut f = File::create(p).expect("write HEAD: ");
    f.write_all(b"ref: refs/heads/master\n")?;

    let p = repo_file(&repo, PathBuf::from("config"), false).expect("create config: ");
    let conf = repo_default_config();
    conf.write_to_file(p).expect("write config: ");

    Ok(repo)
}

fn repo_default_config() -> Ini {
    let mut conf = Ini::new();
    conf.with_section(Some("core"))
        .set("repositoryformatversion", "0")
        .set("filemode", "false")
        .set("bare", "false");
    conf
}

fn repo_file(repo: &Repository, path: PathBuf, mkdir: bool) -> Result<PathBuf> {
    if let Some(parent) = path.parent().map(|p| p.to_path_buf()) {
        if parent != PathBuf::from("") {
            repo_dir(repo, parent, mkdir)?;
        }
    }
    Ok(repo.repo_path(path))
}

fn repo_dir(repo: &Repository, path: PathBuf, mkdir: bool) -> Result<Option<PathBuf>> {
    let p = repo.repo_path(path);

    if p.exists() {
        if p.is_dir() {
            return Ok(Some(p));
        } else {
            bail!("Not a directory {:?}", p);
        }
    }

    if mkdir {
        fs::create_dir_all(p.clone())?;
        Ok(Some(p))
    } else {
        Ok(None)
    }
}
