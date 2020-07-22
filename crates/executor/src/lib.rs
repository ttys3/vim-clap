//! This crates provides the feature of executing the system command both in
//! sync and dynamic manner.

use anyhow::Result;
use filter::{subprocess, Source};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const BASE_TAGS_CMD: &str = "ctags -R -x --output-format=json --fields=+n --exclude=.git --exclude=*.json --exclude=node_modules --exclude=target";

#[derive(Serialize, Deserialize, Debug)]
struct TagInfo {
    name: String,
    path: String,
    pattern: String,
    line: usize,
    kind: String,
}

pub enum Executor {
    ProjTags,
}

impl TagInfo {
    pub fn format(&self) -> String {
        let pat_len = self.pattern.len();
        let name_lnum = format!("{}:{}", self.name, self.line);
        let kind = format!("[{}@{}]", self.kind, self.path);
        format!(
            "{text:<width1$} {kind:<width2$} {pattern}",
            text = name_lnum,
            width1 = 30,
            kind = kind,
            width2 = 30,
            pattern = &self.pattern[2..pat_len - 2].trim(),
        )
    }
}

pub fn formatted_tags_stream(cmd: &str, dir: &PathBuf) -> Result<impl Iterator<Item = String>> {
    let stdout_stream = subprocess::Exec::shell(cmd).cwd(dir).stream_stdout()?;
    Ok(BufReader::new(stdout_stream).lines().filter_map(|line| {
        line.ok().and_then(|tag| {
            if let Ok(tag) = serde_json::from_str::<TagInfo>(&tag) {
                Some(tag.format())
            } else {
                None
            }
        })
    }))
}

pub fn default_formatted_tags_stream(dir: &PathBuf) -> Result<impl Iterator<Item = String>> {
    formatted_tags_stream(BASE_TAGS_CMD, dir)
}

pub fn execute_at(executor: Executor, dir: &PathBuf) -> Result<Vec<String>> {
    let lines = match executor {
        Executor::ProjTags => formatted_tags_stream(BASE_TAGS_CMD, dir)?.collect::<Vec<_>>(),
    };

    Ok(lines)
}

impl Executor {
    pub fn execute<P: AsRef<PathBuf>>(self, cmd: &str, dir: P) -> Result<Vec<String>> {
        let lines = match self {
            Self::ProjTags => {
                formatted_tags_stream(BASE_TAGS_CMD, dir.as_ref())?.collect::<Vec<_>>()
            }
        };

        Ok(lines)
    }

    pub fn execute_dyn(self) {}
}
