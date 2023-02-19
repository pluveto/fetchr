use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkStatus {
    Created,
    Ready,
    Running,
    Paused,
    Finished,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub id: usize,
    pub block_path: PathBuf,
    pub start: usize,
    pub end: usize,
    pub status: ChunkStatus,
}

impl ToString for ChunkStatus {
    fn to_string(&self) -> String {
        match self {
            ChunkStatus::Created => "Created".to_string(),
            ChunkStatus::Ready => "Ready".to_string(),
            ChunkStatus::Running => "Running".to_string(),
            ChunkStatus::Paused => "Paused".to_string(),
            ChunkStatus::Finished => "Finished".to_string(),
            ChunkStatus::Failed => "Failed".to_string(),
        }
    }
}
