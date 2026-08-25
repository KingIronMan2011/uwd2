use std::io::Read;

const MAX_PDB_SIZE: usize = 50 * 1024 * 1024;

pub fn build_url(guid: &str) -> String {
    format!("https://msdl.microsoft.com/download/symbols/shell32.pdb/{guid}/shell32.pdb")
}

pub fn fetch(url: &str) -> Vec<u8> {
    let resp = ureq::get(url)
        .call()
        .expect("failed to download the shell32.pdb symbol file");
    let len = if let Some(len) = resp.headers().get("Content-Length") {
        len.to_str()
            .ok()
            .and_then(|len| len.parse().ok())
            .unwrap_or(15_000_000)
    } else {
        // last time i checked, the file was about 11.6MB, so this should be fine
        15_000_000
    };
    assert!(len <= MAX_PDB_SIZE, "PDB download exceeds the 50 MiB limit");

    let mut buf: Vec<u8> = Vec::with_capacity(len);
    resp.into_body()
        .into_reader()
        .take((MAX_PDB_SIZE + 1) as u64)
        .read_to_end(&mut buf)
        .expect("failed to read the shell32.pdb download");
    assert!(
        buf.len() <= MAX_PDB_SIZE,
        "PDB download exceeds the 50 MiB limit"
    );
    buf
}

#[cfg(test)]
mod tests {
    use super::build_url;

    #[test]
    fn builds_the_expected_symbol_server_url() {
        assert_eq!(
            build_url("ABC123"),
            "https://msdl.microsoft.com/download/symbols/shell32.pdb/ABC123/shell32.pdb"
        );
    }
}
