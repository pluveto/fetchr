use std::path::{Path, PathBuf};

use fetchr::task::{Task, controller::{TaskController, StateChangeEvent}, TaskStatus};
use quicli::prelude::*;
use structopt::StructOpt;
use tokio::net::TcpStream;
use url::Url;
use uuid::Uuid;

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const PKG_NAME: Option<&str> = option_env!("CARGO_PKG_NAME");

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(long = "nthread", short = "n", default_value = "8")]
    nthread: usize,
    url: Url,
    #[structopt(long = "rename", short = "r", default_value = "")]
    rename: String, // A name[.ext] without full path
    #[structopt(flatten)]
    verbosity: Verbosity,
}


fn build_save_path(directory_path: PathBuf, rename: &str, download_url: &Url) -> PathBuf {
    let basename = if rename.is_empty() {
        let path = download_url.path();
        let basename = Path::new(path).file_name().expect("Failed to get basename");
        basename.to_string_lossy().into_owned()
    } else {
        rename.to_owned()
    };
    let mut path = directory_path;
    path.push(basename);
    path
}

fn main() -> CliResult {
    // let args = Cli::from_args();
    // args.verbosity.setup_env_logger("fetchr")?;
    // let cwd = std::env::current_dir().expect("Failed to get current directory");
    // let task = Box::new(Task::new(
    //     args.url.clone(),
    //     build_save_path(cwd, &args.rename, &args.url),
    //     args.nthread,
    // ));
    // let mut handler = TaskHandler::new(
    //     task,
    //     Some(|event: , task: &Task| {
    //         println!(
    //             "Task {} changed from {:?} to {:?}",
    //             task.uuid, event.previous_state, event.current_state
    //         );
    //     }),
    // );
    // handler.start();
    Ok(())
}

async fn test_main_impl() {
    let args = vec!["fetchr", "https://www.rust-lang.org/logos/rust-logo-512x512.png", "-vvv"];
    let args = Cli::from_iter(args.iter());
    args.verbosity.setup_env_logger(PKG_NAME.unwrap()).unwrap();
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let save_path = build_save_path(cwd, &args.rename, &args.url);

    debug!("save path: {}", save_path.to_string_lossy());
    let task = Box::new(Task::new(args.url.clone(), save_path, args.nthread));
    let mut handler = TaskController::new(
        task,
        Some(Box::new(|event: StateChangeEvent<TaskStatus>, task: &Task| {
            debug!(
                "Task {} changed from {:?} to {:?}",
                task.uuid, event.previous_state, event.current_state
            );
        })),
    );

    handler.init().await;
    handler.start();
}

#[test]
fn test_main() {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(test_main_impl());
}
