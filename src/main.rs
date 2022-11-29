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
};

#[derive(Parser, Debug)]
struct Args {
    /// The path to your FFXIV installation.
    ffxiv_dir: PathBuf,

    /// The directory to output files to.
    output: PathBuf,
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
    let out_dir = args.output;
    let game_path = args.ffxiv_dir;

    println!("Initializing ironworks...");
    let mut ironworks = Ironworks::new();
    ironworks.add_resource(SqPack::new(Install::at(&game_path)));

    println!("Fetching file list...");
    let file_list = get_file_list()?;
    let file_count = file_list.len();
    println!("Found {} files to export!", file_count);

    for (files_done, file_name) in file_list.into_iter().enumerate() {
        let file = ironworks.file::<Vec<u8>>(&file_name);

        if let Ok(file) = file {
            let file_path = out_dir.join(Path::new(&file_name));

            let progress = (files_done as f64 / file_count as f64) * 100.;
            println!(
                "{} / {} ({:.2}%) - {}",
                files_done, file_count, progress, file_name
            );

            fs::create_dir_all(file_path.parent().unwrap())?;
            fs::write(file_path, file)?;
        }
    }

    println!("All files exported!");

    Ok(())
}
