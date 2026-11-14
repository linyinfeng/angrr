// Each test binary compiles this module separately and only uses part of it.
#![allow(dead_code)]

use std::{
    cell::Cell,
    collections::BTreeMap,
    fs,
    os::unix::{ffi::OsStrExt, fs::symlink},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use angrr::{
    command::{Commands, Options, RunOptions},
    config::Config,
    run::RunContext,
};
use clap::Parser;
use nix::{
    fcntl::AT_FDCWD,
    sys::{
        stat::{UtimensatFlags, utimensat},
        time::TimeSpec,
    },
};
use serde_json::json;
use tempfile::TempDir;

pub(crate) const DAY: u64 = 24 * 60 * 60;

/// Layout of a fixture; says nothing about which locations angrr scans.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Location {
    External,
    GcRoots,
    Profiles,
}

impl Location {
    fn directory(self) -> &'static str {
        match self {
            Location::External => "external",
            Location::GcRoots => "gcroots",
            Location::Profiles => "profiles",
        }
    }
}

pub(crate) struct StorePath {
    pub(crate) path: PathBuf,
}

pub(crate) struct RootLink {
    pub(crate) path: PathBuf,
    now: SystemTime,
}

pub(crate) struct Profile {
    pub(crate) path: PathBuf,
    now: SystemTime,
}

pub(crate) struct Fixture {
    /// Owns the sandbox directory; dropping it removes the whole tree.
    root: TempDir,
    /// State sandbox holding `store`, `gcroots` (with `auto`), `profiles`, `external`.
    pub(crate) path: PathBuf,
    /// Output file next to `path`, so snapshots of the sandbox stay valid.
    pub(crate) output: PathBuf,
    now: SystemTime,
    registrations: Cell<usize>,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        // Darwin temp dirs sit behind the `/var` -> `/private/var` symlink, so the
        // canonical path is what angrr itself resolves fixture paths to.
        let base = fs::canonicalize(root.path()).unwrap();
        let path = base.join("state");
        fs::create_dir(&path).unwrap();
        for directory in ["store", "gcroots", "profiles", "external"] {
            fs::create_dir(path.join(directory)).unwrap();
        }
        fs::create_dir(path.join("gcroots/auto")).unwrap();
        let output = base.join("output");
        Self {
            root,
            path,
            output,
            now: SystemTime::now(),
            registrations: Cell::new(0),
        }
    }

    pub(crate) fn directory(&self, location: Location) -> PathBuf {
        self.path.join(location.directory())
    }

    pub(crate) fn auto_directory(&self) -> PathBuf {
        self.path.join("gcroots/auto")
    }

    pub(crate) fn store_path(&self, name: &str) -> StorePath {
        let path = self.path.join("store").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("store item {name}")).unwrap();
        StorePath { path }
    }

    /// Creates a symlink into the store; registration is a separate step.
    pub(crate) fn root_link(&self, location: Location, name: &str, store: &StorePath) -> RootLink {
        let path = self.directory(location).join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        symlink(&store.path, &path).unwrap();
        RootLink {
            path,
            now: self.now,
        }
    }

    /// Creates the generation directory and handle, but no current link.
    pub(crate) fn profile(&self, location: Location, name: &str) -> Profile {
        let path = self.directory(location).join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        Profile {
            path,
            now: self.now,
        }
    }

    /// Registers a root in the auto directory; every call gets a unique link.
    pub(crate) fn register_auto(&self, link: &RootLink) -> PathBuf {
        let index = self.registrations.get();
        self.registrations.set(index + 1);
        let path = self.auto_directory().join(format!("root-{index}"));
        symlink(&link.path, &path).unwrap();
        path
    }

    /// Complete baseline configuration pointing at this sandbox.
    ///
    /// The policy configs keep `enable` in a private `common` field, so serde
    /// is the only way to build them. This factory is the single place that
    /// does it; tests only touch the returned public fields.
    ///
    /// The policies here are baselines only: tests set the priority, period and
    /// retention settings their assertions depend on explicitly, in the body.
    pub(crate) fn config(&self) -> Config {
        serde_json::from_value(json!({
            "store": self.path.join("store"),
            // Keep fixture filtering independent of whether tests run as root.
            "owned-only": "false",
            // TODO(#64): auto-only is the phase-1 baseline behaviour. Once recursive
            // root discovery, direct/indirect handling and source dedup land, the default
            // scans gcroots and profiles too; only auto cases then pin the scope explicitly.
            "directory": [self.auto_directory()],
            "temporary-root-policies": {
                "results": {
                    "path-regex": "/result[^/]*$",
                    "period": "7d",
                },
                // `enable` is not publicly reachable and the disabled test needs a
                // disabled policy, so the baseline config is the only place that can
                // build one. Tests that want it set its periods explicitly.
                "disabled": {
                    "enable": false,
                    "priority": 0,
                    "path-regex": ".*",
                    "period": "100d",
                },
            },
            "profile-policies": {
                "user": {
                    "profile-paths": [self.directory(Location::Profiles).join("profile")],
                    "keep-latest-n": 1,
                },
            },
        }))
        .unwrap()
    }
}

impl RootLink {
    /// Ages the out-link only; angrr measures a root's age by the link's mtime.
    pub(crate) fn set_age(&self, age: Duration) {
        let modified = self.now - age;
        let modified = TimeSpec::from(modified.duration_since(UNIX_EPOCH).unwrap());
        utimensat(
            AT_FDCWD,
            &self.path,
            &TimeSpec::UTIME_OMIT,
            &modified,
            UtimensatFlags::NoFollowSymlink,
        )
        .unwrap();
    }
}

impl Profile {
    pub(crate) fn generation_path(&self, number: usize) -> PathBuf {
        let mut name = self.path.file_name().unwrap().to_os_string();
        name.push(format!("-{number}-link"));
        self.path.with_file_name(name)
    }

    pub(crate) fn generation(&self, number: usize, store: &StorePath) -> RootLink {
        let path = self.generation_path(number);
        symlink(&store.path, &path).unwrap();
        RootLink {
            path,
            now: self.now,
        }
    }

    /// Points the profile at a generation through a relative link.
    pub(crate) fn set_current(&self, number: usize) {
        let target = self.generation_path(number);
        match fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("failed to clear current link {:?}: {error}", self.path),
        }
        symlink(target.file_name().unwrap(), &self.path).unwrap();
    }
}

pub(crate) fn run_options(args: &[&str]) -> RunOptions {
    let parsed =
        Options::try_parse_from(["angrr", "run"].into_iter().chain(args.iter().copied())).unwrap();
    let Commands::Run(options) = parsed.command else {
        panic!("expected run command");
    };
    options
}

pub(crate) fn run(options: RunOptions, config: Config) {
    let context = RunContext::new(options, config).unwrap();
    context.run().unwrap();
    context.finish().unwrap();
}

pub(crate) fn assert_present(path: &Path) {
    assert!(entry_exists(path), "{path:?} should exist");
}

pub(crate) fn assert_absent(path: &Path) {
    assert!(!entry_exists(path), "{path:?} should not exist");
}

fn entry_exists(path: &Path) -> bool {
    // exists() follows symlinks and would hide a surviving dangling registration.
    match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => panic!("failed to inspect {path:?}: {error}"),
    }
}

/// Compares the paths angrr recorded, ignoring order because directory
/// iteration order is unspecified.
pub(crate) fn assert_output_paths(output: &Path, delimiter: u8, expected: &[&PathBuf]) {
    let recorded = fs::read(output).unwrap();
    let mut recorded: Vec<&[u8]> = recorded.split(|byte| *byte == delimiter).collect();
    let mut expected: Vec<&[u8]> = expected
        .iter()
        .map(|path| path.as_os_str().as_bytes())
        .collect();
    recorded.sort();
    expected.sort();
    assert_eq!(recorded, expected);
}

#[derive(Debug, PartialEq, Eq)]
enum EntryKind {
    Directory,
    File,
    Symlink,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    modified: SystemTime,
    contents: Vec<u8>,
    kind: EntryKind,
}

pub(crate) fn snapshot(path: &Path) -> BTreeMap<PathBuf, Entry> {
    fn visit(path: &Path, entries: &mut BTreeMap<PathBuf, Entry>) {
        let metadata = fs::symlink_metadata(path).unwrap();
        let (kind, contents) = if metadata.is_symlink() {
            (
                EntryKind::Symlink,
                fs::read_link(path).unwrap().as_os_str().as_bytes().to_vec(),
            )
        } else if metadata.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                visit(&entry.unwrap().path(), entries);
            }
            (EntryKind::Directory, Vec::new())
        } else {
            (EntryKind::File, fs::read(path).unwrap())
        };
        entries.insert(
            path.to_path_buf(),
            Entry {
                modified: metadata.modified().unwrap(),
                contents,
                kind,
            },
        );
    }
    let mut entries = BTreeMap::new();
    visit(path, &mut entries);
    entries
}
