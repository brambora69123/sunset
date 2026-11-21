use sunset_1_7::biome;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: i64 = args[1].parse().expect("seed");
    let x: i32 = args[2].parse().expect("x");
    let z: i32 = args[3].parse().expect("z");
    let w: i32 = args[4].parse().expect("w");
    let h: i32 = args[5].parse().expect("h");
    let which = args.get(6).map(String::as_str).unwrap_or("voronoi");

    let [mut mixer, mut layer, _] = biome::init(seed, false);
    let area = if which == "noise" {
        mixer.area(x, z, w, h)
    } else {
        layer.area(x, z, w, h)
    };
    println!(
        "{}",
        area.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
}
