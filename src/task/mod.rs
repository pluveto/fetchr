mod chunk;

pub mod controller;

use std::path::PathBuf;

use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub uuid: Uuid,
    pub original_url: Url, // url can be changed by redirect, so we need to keep the original url
    pub url: Url,
    pub path: PathBuf, // save to this path
    pub nthread: usize,
    pub size: usize,
    pub downloaded_size: usize,
    pub status: TaskStatus,
}

impl Task {
    pub fn new(url: Url, path: PathBuf, nthread: usize) -> Self {
        let uuid = Uuid::new_v4();
        let size = 0;
        let downloaded_size = 0;
        let status = TaskStatus::Created;
        Task { uuid, original_url: url.clone(), url, path, nthread, size, downloaded_size, status }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Created,
    Ready,
    Running,
    Paused,
    Finished,
    Failed,
}
