use hyper::{body, Error, Request};
use quicli::prelude::debug;
use tokio::net::TcpStream;

use crate::task::{chunk::ChunkStatus, Task, TaskStatus};

use super::chunk::Chunk;

#[derive(Debug, Clone, PartialEq)]
pub struct StateChangeEvent<T> {
    pub previous_state: T,
    pub current_state: T,
}

pub enum Phase {
    Init,
    Download,
    Merge,
}

pub struct DownloadError {
    pub phase: Phase,
    pub message: String,
    pub inner_message: String,
}

type StateChangeHandler = fn(StateChangeEvent<TaskStatus>, &Task);
pub struct TaskController {
    task: Box<Task>,
    chunks: Vec<Chunk>,
    on_state_changed: Option<Box<StateChangeHandler>>,
}

impl std::fmt::Debug for TaskController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("TaskHandler");
        debug_struct.field("task", &self.task);
        debug_struct.field("task_parts", &self.chunks);
        debug_struct.field("on_state_changed", &"omitted");
        debug_struct.finish()
    }
}
/// Task and TaskPart are plain data. TaskHandler is responsible for controlling the task.
impl TaskController {
    pub fn new(task: Box<Task>, on_state_changed: Option<Box<StateChangeHandler>>) -> Self {
        assert!(task.nthread > 0);
        let mut task_parts = Vec::new();
        TaskController { task, chunks: task_parts, on_state_changed: on_state_changed }
    }

    fn next_chunk_id(&mut self) -> usize {
        self.chunks.len()
    }

    /// 初始化阶段
    pub async fn init(&mut self) -> Result<(), DownloadError> {
        let (content_length, use_range) = match self.head_task().await {
            Ok((content_length, use_range)) => (content_length, use_range),
            Err(err) => {
                self.task.status = TaskStatus::Failed;
                return Err(err);
            }
        };

        debug!("Size: {}", content_length);
        debug!("Use Range: {}", use_range);

        if use_range {
            let chunk_size = content_length / self.task.nthread;
            let mut start = 0;
            let mut end = chunk_size;
            for _ in 0..self.task.nthread {
                let chunk = self.create_chunk(start, end);
                self.chunks.push(chunk);
                start = end + 1;
                end += chunk_size;
            }
            let last_chunk_index = self.chunks.len() - 1;
            self.chunks[last_chunk_index].end = content_length - 1;
        } else {
            let chunk = self.create_chunk(0, content_length - 1);
            self.chunks.push(chunk);
        }

        self.task.size = content_length;
        self._print_parts();

        Ok(())
    }

    fn create_chunk(&mut self, start: usize, end: usize) -> Chunk {
        let id = self.next_chunk_id();
        Chunk {
            id: id,
            block_path: self.create_chunk_path(id),
            start,
            end,
            status: ChunkStatus::Created,
        }
    }

    fn create_chunk_path(&self, id: usize) -> std::path::PathBuf {
        let path = self.task.path.clone();
        // use filename.part.N as chunk file name
        let dir = path.parent().unwrap();
        let filename = path.file_name().unwrap();
        let chunk_filename = format!("{}.part.{}", filename.to_str().unwrap(), id);

        let chunk_path = dir.join(chunk_filename);
        chunk_path
    }

    async fn head_task(&mut self) -> Result<(usize, bool), DownloadError> {
        debug!("HEAD {}", self.task.url);
        let mut use_range = false;
        let mut content_length = 0;

        let url = self.task.url.clone();
        let addr = url.socket_addrs(|| Some(80)).unwrap().into_iter().next().unwrap();
        let stream = TcpStream::connect(&addr).await.unwrap();
        let (mut sender, conn) = match hyper::client::conn::http1::handshake(stream).await {
            Ok((sender, conn)) => (sender, conn),
            Err(err) => {
                return Err(DownloadError {
                    phase: Phase::Init,
                    message: "Failed to connect to server".to_string(),
                    inner_message: err.to_string(),
                });
            }
        };

        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });
        let hyper_url: hyper::Uri = url.as_str().parse().unwrap();
        let authority = hyper_url.authority().unwrap().clone();

        debug!("authority: {}", authority.as_str());

        let req = match Request::builder()
            .uri(hyper_url)
            .header(hyper::header::HOST, authority.as_str())
            .body(http_body_util::Empty::<body::Bytes>::new())
        {
            Ok(req) => req,
            Err(err) => {
                return Err(DownloadError {
                    phase: Phase::Init,
                    message: "Failed to build request".to_string(),
                    inner_message: err.to_string(),
                });
            }
        };

        let res = match sender.send_request(req).await {
            Ok(res) => res,
            Err(err) => {
                return Err(DownloadError {
                    phase: Phase::Init,
                    message: "Failed to send request".to_string(),
                    inner_message: err.to_string(),
                });
            }
        };

        let headers = res.headers();
        if let Some(content_length_header) = headers.get("Content-Length") {
            content_length = content_length_header.to_str().unwrap().parse().unwrap();
        }
        if let Some(accept_ranges_header) = headers.get("Accept-Ranges") {
            use_range = accept_ranges_header.to_str().unwrap() == "bytes";
        }

        Ok((content_length, use_range))
    }
    pub fn start(&mut self) {
        assert!(self.task.size > 0);
        self.set_status(TaskStatus::Running);
    }

    fn _print_parts(&self) {
        macro_rules! table_fmt {
            () => {
                "{:<8} {:<12} {:<12} {:<12} {}"
            };
        }
        debug!(table_fmt!(), "CHUNK_ID", "STATUS", "START", "END", "PATH");
        for part in &self.chunks {
            debug!(
                table_fmt!(),
                part.id,
                part.status.to_string(),
                part.start,
                part.end,
                part.block_path.to_string_lossy()
            );
        }
    }

    fn set_status(&mut self, status: TaskStatus) {
        let previous_state = self.task.status.clone();
        self.task.status = status.clone();
        if let Some(handler) = &self.on_state_changed {
            handler(StateChangeEvent { previous_state, current_state: status }, &self.task);
        }
    }
}
