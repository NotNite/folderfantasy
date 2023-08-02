use clap::Parser;
use flate2::read::GzDecoder;
use ironworks::{
    sqpack::{Install, SqPack},
    Ironworks,
};
use std::{
    error::Error,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{atomic::AtomicUsize, Arc},
};

#[derive(Parser, Debug)]
struct Args {
    /// The path to your FFXIV installation.
    ffxiv_dir: PathBuf,

    /// The directory to output files to.
    output: PathBuf,

    /// Amount of threads to split extraction between.
    threads: Option<usize>,
}

fn get_file_list() -> Result<Vec<String>, Box<dyn Error>> {
    let body =
        reqwest::blocking::get("https://rl2.perchbird.dev/download/export/CurrentPathList.gz")?
            .bytes()?;
    let mut decoder = GzDecoder::new(&body[..]);
    let mut content = String::new();
    decoder.read_to_string(&mut content)?;

    Ok(content.split('\n').map(|s| s.trim().to_string()).collect())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let threads = args.threads.unwrap_or(1);

    println!("Fetching file list...");
    let file_list = get_file_list()?;
    let file_count = file_list.len();
    println!("Found {} files to export!", file_count);

    // split file list into chunks
    let chunks = file_list.chunks(file_count / threads);
    let mut handles = Vec::new();

    let done = Arc::new(AtomicUsize::new(0));
    for i in 0..threads {
        let chunk = chunks.clone().nth(i).unwrap().to_vec();

        let game_path = args.ffxiv_dir.clone();
        let out_dir = args.output.clone();
        let done = done.clone();

        let handle = std::thread::spawn(move || {
            let mut ironworks = Ironworks::new();
            ironworks.add_resource(SqPack::new(Install::at(&game_path)));

            for file_name in chunk {
                let file = ironworks.file::<Vec<u8>>(&file_name);
                if let Ok(file) = file {
                    let file_path = out_dir.join(Path::new(&file_name));
                    fs::create_dir_all(file_path.parent().unwrap())
                        .expect("Failed to create directory");
                    fs::write(file_path, file).expect("Failed to write file");

                    done.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    // print every 10k
                    if done.load(std::sync::atomic::Ordering::SeqCst) % 1000 == 0 {
                        println!(
                            "{}/{}",
                            done.load(std::sync::atomic::Ordering::SeqCst),
                            file_count
                        );
                    }
                }
            }
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("All files exported!");
    Ok(())
}
