use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

fn collect_source_files(dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let mut content = Vec::new();
            if let Ok(mut file) = fs::File::open(&path) {
                let _ = file.read_to_end(&mut content);
                files.insert(path.to_string_lossy().into_owned(), content);
            }
        }
    }
}

fn main() {
    println!("cargo::rerun-if-changed=src");

    let mut files = BTreeMap::new();
    collect_source_files(Path::new("src"), &mut files);

    let mut hash: u64 = 0xcbf29ce484222325;
    for (_, content) in &files {
        for &byte in content {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }

    println!("cargo::rustc-env=REGLER_SOURCE_HASH={hash:016x}");
}
