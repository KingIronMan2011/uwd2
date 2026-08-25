use std::fs;
use std::io;
use std::path::Path;

use crate::constants::*;
use crate::fetch_pdb;
use crate::fetch_pdb::fetch;
use crate::parse_pdb::parse_pdb;

pub fn get_rva(guid: String) -> u32 {
    // quick way to get usable directory
    let dir = data_dir();
    // dbg!(dir);
    // .rva is arbitrary, im storing a single u32 so there isnt exactly a good extension for this
    let pdbpath = dir.join(&guid).with_extension("rva");
    if let Some(rva) = read_cached_rva(&pdbpath) {
        println!("PDB cached. Reading...");
        rva
    } else {
        println!("PDB not found. Fetching...");
        let url = fetch_pdb::build_url(&guid);
        let pdbfile = fetch(&url);
        println!("Fetched! Parsing...");
        let rva = parse_pdb(pdbfile);
        println!("Parsed! Caching...");
        write_cached_rva(&dir, &guid, rva).expect("failed to write the PDB RVA cache");
        println!("Cached!");
        rva
    }
}

fn read_cached_rva(path: &Path) -> Option<u32> {
    fs::read(path).ok()?.try_into().ok().map(u32::from_be_bytes)
}

fn write_cached_rva(dir: &Path, guid: &str, rva: u32) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let destination = dir.join(guid).with_extension("rva");
    let temporary = dir.join(format!("{guid}.{}.tmp", std::process::id()));
    fs::write(&temporary, rva.to_be_bytes())?;
    fs::rename(temporary, destination)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{read_cached_rva, write_cached_rva};

    fn test_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "uwd2-cache-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn writes_a_complete_rva_cache_entry() {
        let dir = test_dir();
        write_cached_rva(&dir, "TEST", 0x1234_5678).unwrap();
        assert_eq!(read_cached_rva(&dir.join("TEST.rva")), Some(0x1234_5678));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ignores_a_truncated_cache_entry() {
        let dir = test_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("TEST.rva");
        fs::write(&path, [0x12]).unwrap();
        assert_eq!(read_cached_rva(&path), None);
        fs::remove_dir_all(dir).unwrap();
    }
}
