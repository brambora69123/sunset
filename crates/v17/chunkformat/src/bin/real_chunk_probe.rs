fn main() {
    let path = std::env::args().nth(1).expect("nbt path");
    let bytes = std::fs::read(path).expect("read nbt");
    let chunk = sunset_1_7_chunkformat::chunk::read_chunk(&bytes).expect("parse chunk");
    println!("pos={},{}", chunk.x_pos, chunk.z_pos);
    println!(
        "biomes={}",
        chunk
            .biomes
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "height={}",
        chunk
            .height_map
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    let out = sunset_1_7_chunkformat::chunk::write_chunk(&chunk, true).expect("write chunk");
    let again = sunset_1_7_chunkformat::chunk::read_chunk(&out).expect("reparse");
    assert_eq!(chunk.biomes, again.biomes);
    assert_eq!(chunk.height_map, again.height_map);
    println!("rewrite_roundtrip ok");
}
