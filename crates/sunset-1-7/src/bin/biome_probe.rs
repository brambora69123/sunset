use sunset_1_7::biome;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: i64 = args[1].parse().expect("seed");
    let cx: i32 = args[2].parse().expect("chunk x");
    let cz: i32 = args[3].parse().expect("chunk z");

    let [mut mixer, mut layer, _] = biome::init(seed, false);
    let area = if args.len() > 4 && args[4] == "noise" {
        mixer.area(cx * 16, cz * 16, 16, 16)
    } else {
        layer.area(cx * 16, cz * 16, 16, 16)
    };
    println!(
        "{}",
        area.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
}
