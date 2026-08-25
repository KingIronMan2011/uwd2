use std::io::Cursor;

use pdb::FallibleIterator;

pub fn parse_pdb(pdbfile: Vec<u8>) -> u32 {
    let pdbreader = Cursor::new(pdbfile);
    let mut shell32 = pdb::PDB::open(pdbreader).expect("invalid shell32.pdb file");
    let symbol_table = shell32
        .global_symbols()
        .expect("shell32.pdb has no global symbol table");
    let address_map = shell32
        .address_map()
        .expect("shell32.pdb has no address map");
    for symbol in symbol_table.iter().iterator().flatten() {
        let data = symbol
            .parse()
            .expect("failed to parse a shell32.pdb symbol");
        if let pdb::SymbolData::Public(d) = data {
            // i'd be surprised if there's any false positives here and i dont want to get cocky with name unmangling
            if d.name
                .as_bytes()
                .windows(b"s_DesktopBuildPaint".len())
                .any(|name| name == b"s_DesktopBuildPaint")
                && d.function
            {
                // dbg!(d);
                let rva = d
                    .offset
                    .to_rva(&address_map)
                    .expect("desktop paint symbol has no relative virtual address");
                // dbg!(rva);
                return rva.0;
            }
        }
    }
    panic!("desktop paint symbol is missing from shell32.pdb");
}
