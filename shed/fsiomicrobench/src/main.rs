/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use anyhow::anyhow;
use blake3::Hash;
use clap::Parser;
use rand::RngCore;

const BENCH_DIR_NAME: &str = "__fsiomicrobench__";
const COMBINED_DATA_FILE_NAME: &str = "__combined_data__";
const DEFAULT_NUMBER_OF_FILES: usize = 64 * 1024;
const DEFAULT_CHUNK_SIZE: usize = 4 * 1024;
const NUMBER_OF_SUB_DIRS: usize = 256;
const BYTES_IN_KILOBYTE: usize = 1024;
const BYTES_IN_MEGABYTE: usize = 1024 * BYTES_IN_KILOBYTE;
const BYTES_IN_GIGABYTE: usize = 1024 * BYTES_IN_MEGABYTE;

/// Cross-platform micro benchmark for File System I/O, benchmarking the
/// create(), open(), read(), write() and sync() system calls.
#[derive(Parser, Debug)]
#[command(bin_name = "fsiomicrobench")]
struct Args {
    /// Directory to use for testing
    #[arg(long, default_value_t = std::env::temp_dir().to_str().unwrap().to_string())]
    test_dir: String,

    /// Number of randomly generated files to use for benchmarking
    #[arg(long, default_value_t = DEFAULT_NUMBER_OF_FILES)]
    number_of_files: usize,

    /// Size of each chunk in bytes
    #[arg(long, default_value_t = DEFAULT_CHUNK_SIZE)]
    chunk_size: usize,
}

struct RandomData {
    // Directory to use for testing.
    test_dir: PathBuf,

    // Number of randomly generated files.
    number_of_files: usize,

    // Size of each chunk in bytes.
    chunk_size: usize,

    // Random content that will be written to files.
    chunks: Vec<Vec<u8>>,

    // Hashes to verify the data written to files.
    // Also used for generate file paths contents will be written to.
    hashes: Vec<Hash>,
}

impl RandomData {
    fn new(test_dir: PathBuf, number_of_files: usize, chunk_size: usize) -> Self {
        let mut rng = rand::rng();
        let mut chunks = Vec::with_capacity(number_of_files);
        let mut hashes = Vec::with_capacity(number_of_files);
        for _ in 0..number_of_files {
            let mut chunk = vec![0u8; chunk_size];
            rng.fill_bytes(&mut chunk);
            let hash = blake3::hash(&chunk);
            chunks.push(chunk);
            hashes.push(hash);
        }
        RandomData {
            test_dir,
            number_of_files,
            chunk_size,
            chunks,
            hashes,
        }
    }

    fn paths(&self) -> Vec<PathBuf> {
        self.hashes
            .iter()
            .map(|hash| hash_to_path(&self.test_dir, hash))
            .collect()
    }

    fn total_size(&self) -> usize {
        self.number_of_files * self.chunk_size
    }

    fn combined_data_path(&self) -> PathBuf {
        self.test_dir.join(COMBINED_DATA_FILE_NAME)
    }
}

fn prepare_directories(root: &Path) -> Result<()> {
    for i in 0..NUMBER_OF_SUB_DIRS {
        let sub_dir = format!("{:02x}", i);
        let sub_dir_path = root.join(sub_dir);
        fs::create_dir_all(&sub_dir_path)?;
    }
    Ok(())
}

fn validate_test_dir(test_dir: &str) -> Result<PathBuf> {
    let test_dir_path = Path::new(test_dir);
    if !test_dir_path.exists() {
        return Err(anyhow!("The directory {} does not exist.", test_dir));
    }
    let bench_dir_path = test_dir_path.join(BENCH_DIR_NAME);
    if bench_dir_path.exists() {
        fs::remove_dir_all(&bench_dir_path)?;
    }
    fs::create_dir(&bench_dir_path)?;
    prepare_directories(&bench_dir_path)?;
    Ok(bench_dir_path)
}

fn remove_test_dir(test_dir: &PathBuf) -> Result<()> {
    if test_dir.exists() {
        fs::remove_dir_all(test_dir)?;
    }
    Ok(())
}

fn hash_to_path(root: &Path, hash: &Hash) -> PathBuf {
    let hash_str = hash.to_hex().to_string();
    let sub_dir = &hash_str[0..2];
    root.join(sub_dir).join(hash_str)
}

fn bench_write_mfmd(random_data: &RandomData) -> Result<()> {
    let mut agg_create_dur = std::time::Duration::new(0, 0);
    let mut agg_write_dur = std::time::Duration::new(0, 0);
    for (chunk, path) in random_data.chunks.iter().zip(random_data.paths().iter()) {
        let start = Instant::now();
        let mut file = File::create(path)?;
        agg_create_dur += start.elapsed();

        let start = Instant::now();
        file.write_all(chunk)?;
        agg_write_dur += start.elapsed();
    }

    let mut agg_sync_dur = std::time::Duration::new(0, 0);
    for path in random_data.paths() {
        let start = Instant::now();
        let file = File::options().write(true).open(path)?;
        file.sync_all()?;
        agg_sync_dur += start.elapsed();
    }

    let avg_create_dur = agg_create_dur.as_secs_f64() / random_data.number_of_files as f64;
    let avg_write_dur = agg_write_dur.as_secs_f64() / random_data.number_of_files as f64;
    let avg_sync_dur = agg_sync_dur.as_secs_f64() / random_data.number_of_files as f64;
    let avg_e2e_dur = avg_create_dur + avg_write_dur + avg_sync_dur;
    let avg_create_write_dur = avg_create_dur + avg_write_dur;
    let mb_per_second_e2e = random_data.chunk_size as f64 / avg_e2e_dur / BYTES_IN_MEGABYTE as f64;
    let mb_per_second_create_write =
        random_data.chunk_size as f64 / avg_create_write_dur / BYTES_IN_MEGABYTE as f64;
    println!("MFMD Write");
    println!(
        "\t- {:.2} MiB/s create() + write() + sync()",
        mb_per_second_e2e
    );
    println!(
        "\t- {:.2} MiB/s create() + write()",
        mb_per_second_create_write
    );
    println!("\t- {:.4} ms create() latency", avg_create_dur * 1000.0);
    println!(
        "\t- {:.4} ms write() {:.0} KiB bytes latency",
        avg_write_dur * 1000.0,
        random_data.chunk_size as f64 / BYTES_IN_KILOBYTE as f64
    );
    println!(
        "\t- {:.4} ms sync() {:.0} KiB latency",
        avg_sync_dur * 1000.0,
        random_data.chunk_size as f64 / BYTES_IN_KILOBYTE as f64
    );
    Ok(())
}

fn bench_read_mfmd(random_data: &RandomData) -> Result<()> {
    let mut agg_open_dur = std::time::Duration::new(0, 0);
    let mut agg_read_dur = std::time::Duration::new(0, 0);
    let mut read_data = vec![0u8; random_data.chunk_size];
    for path in random_data.paths() {
        let start = Instant::now();
        let mut file = File::open(path)?;
        agg_open_dur += start.elapsed();

        let start = Instant::now();
        file.read_exact(&mut read_data)?;
        agg_read_dur += start.elapsed();
    }
    let avg_open_dur = agg_open_dur.as_secs_f64() / random_data.number_of_files as f64;
    let avg_read_dur = agg_read_dur.as_secs_f64() / random_data.number_of_files as f64;
    let avg_dur = avg_open_dur + avg_read_dur;
    let mb_per_second = random_data.chunk_size as f64 / avg_dur / BYTES_IN_MEGABYTE as f64;
    println!("MFMD Read");
    println!("\t- {:.2} MiB/s open() + read()", mb_per_second);
    println!("\t- {:.4} ms open() latency", avg_open_dur * 1000.0);
    println!(
        "\t- {:.4} ms read() {:.0} KiB latency",
        avg_read_dur * 1000.0,
        random_data.chunk_size as f64 / BYTES_IN_KILOBYTE as f64
    );
    Ok(())
}

fn bench_write_sfmd(random_data: &RandomData) -> Result<()> {
    let start = Instant::now();
    let mut file = File::create(random_data.combined_data_path())?;
    for chunk in &random_data.chunks {
        file.write_all(chunk)?;
    }
    let write_dur = start.elapsed().as_secs_f64();
    let start = Instant::now();
    file.sync_all()?;
    let sync_dur = start.elapsed().as_secs_f64();
    let agg_dur = write_dur + sync_dur;
    let mb_per_second_e2e = random_data.total_size() as f64 / BYTES_IN_MEGABYTE as f64 / agg_dur;
    let mb_per_second_write =
        random_data.total_size() as f64 / BYTES_IN_MEGABYTE as f64 / write_dur;
    println!("SFMD Write");
    println!(
        "\t- {:.2} MiB/s create() + write() + sync()",
        mb_per_second_e2e
    );
    println!("\t- {:.2} MiB/s create() + write()", mb_per_second_write);
    Ok(())
}

fn bench_read_sfmd(random_data: &RandomData) -> Result<()> {
    let file_path = random_data.combined_data_path();
    let mut read_data = Vec::with_capacity(random_data.total_size());
    let start = Instant::now();
    let mut file = File::open(&file_path)?;
    file.read_to_end(&mut read_data)?;
    let agg_dur = start.elapsed().as_secs_f64();
    let mb_per_second = read_data.len() as f64 / BYTES_IN_MEGABYTE as f64 / agg_dur;
    println!("SFMD Read");
    println!("\t- {:.2} MiB/s open() + read()", mb_per_second);
    Ok(())
}

fn print_section_divider() {
    println!("-----------------------------------");
}

fn print_glossary() {
    println!("Glossary:");
    println!(
        "MFMD - Multiple Files Multiple Data - Writing and reading multiple files, each containing different data chunks."
    );
    println!(
        "SFMD - Single File Multiple Data - Writing and reading a single file containing multiple data chunks."
    );
}

fn main() -> Result<()> {
    print_section_divider();
    print_glossary();
    let args = Args::parse();
    match validate_test_dir(&args.test_dir) {
        Ok(path) => {
            print_section_divider();
            println!("Prepared the directory at {:?}", path);
            println!("Generating in-memory random data ...");
            let random_data = RandomData::new(path, args.number_of_files, args.chunk_size);
            println!(
                "The random data generated with {} chunks with {:.0} KiB each, with the total size of {:.2} GiB.",
                random_data.number_of_files,
                random_data.chunk_size as f64 / BYTES_IN_KILOBYTE as f64,
                random_data.total_size() as f64 / BYTES_IN_GIGABYTE as f64
            );
            print_section_divider();
            bench_write_mfmd(&random_data)?;
            bench_read_mfmd(&random_data)?;
            bench_write_sfmd(&random_data)?;
            bench_read_sfmd(&random_data)?;
            print_section_divider();
            println!("Removing the directory at {:?}", random_data.test_dir);
            remove_test_dir(&random_data.test_dir)?;
        }
        Err(e) => return Err(e),
    }

    Ok(())
}
