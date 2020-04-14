#[macro_use]
extern crate nom;

mod blame;

use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt)]
#[allow(non_snake_case)]
#[structopt(global_settings = &[AppSettings::ColoredHelp])]
struct Args {
    #[structopt(name = "path", parse(from_os_str))]
    arg_path: PathBuf,
    #[structopt(short = "M")]
    /// find line moves within and across files
    flag_M: bool,
    #[structopt(short = "C")]
    /// find line copies within and across files
    flag_C: bool,
    #[structopt(short = "F")]
    /// follow only the first parent commits
    flag_F: bool,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Author {
    name: String,
    mail: String,
}

impl Author {
    fn new(name: &str, mail: &str) -> Self {
        Author {
            name: name.to_string(),
            mail: mail.to_string(),
        }
    }
}

#[derive(Debug)]
struct Commit {
    hash: String,
    author: String,
    author_mail: String,
    num_lines: usize,
}

struct TrackedFile {
    #[allow(dead_code)]
    path: PathBuf,
    commits: Vec<Commit>,
}

impl TrackedFile {
    fn from_path(path: &Path) -> Result<Self> {
        // Generate blame.
        let txt = blame::generate_blame(&path)?;
        let lines = blame::parse_blame(&txt);

        let mut commits: HashMap<String, Commit> = HashMap::new();
        lines.iter().for_each(|line| {
            if let Some(extra) = &line.header.extra {
                // We only see extra header details each time we encounter a new commit.
                commits.insert(
                    line.header.hash.to_string(),
                    Commit {
                        hash: line.header.hash.to_string(),
                        author: extra.author.to_string(),
                        author_mail: extra.author_mail.to_string(),
                        num_lines: 0,
                    },
                );
            }

            if let Some(commit) = commits.get_mut(line.header.hash) {
                commit.num_lines += 1;
            } else {
                unreachable!();
            }
        });

        let commits = commits.into_iter().map(|(_, v)| v).collect();
        Ok(TrackedFile {
            path: path.to_owned(),
            commits,
        })
    }
}

/*
impl Author {
    fn from_blame_header(header: &blame::Header) {
        Author {
            name: header.author.to_string(),
            mail: header.author_mail.to_string(),
            commits: Vec::new(),
            lines: Vec::new(),
        }
    }

    fn lines(&self) -> usize {
        self.commits.values().sum::<usize>()
    }
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} <{}>: Lines: {} Count: {}",
            self.name,
            self.email,
            self.lines(),
            self.commits.len()
        )
    }
}
*/

fn main() -> Result<()> {
    let args = Args::from_args();

    let path = args.arg_path.canonicalize()?;

    let tracked_file = TrackedFile::from_path(&path).context(format!(
        "Failure to generate blame details for: {}",
        path.display()
    ))?;

    let mut author_commits: HashMap<Author, Vec<Commit>> = HashMap::new();
    tracked_file.commits.into_iter().for_each(|commit| {
        let author = Author::new(&commit.author, &commit.author_mail);
        author_commits.entry(author).or_default().push(commit);
    });
    let mut author_commits: Vec<(usize, Author, Vec<Commit>)> = author_commits
        .into_iter()
        .map(|(author, commits)| {
            let num_lines = commits.iter().map(|commit| commit.num_lines).sum();
            (num_lines, author, commits)
        })
        .collect();
    author_commits.sort_by(|a, b| b.0.cmp(&a.0));

    println!("File: {}", tracked_file.path.display());

    author_commits.iter().for_each(|(lines, author, commits)| {
        println!(
            "  {} {}: Lines: {} Count: {}",
            author.name,
            author.mail,
            lines,
            commits.len()
        );
    });

    Ok(())
}
