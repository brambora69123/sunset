# 1.7.10 Java oracle

Runs the real Minecraft 1.7.10 jars so a Rust port can be diffed against them.
`build.sh` widens the mapped jar and compiles the probes; `run.sh` runs one.

```sh
sh tools/javaoracle_1_7_10/build.sh
sh tools/javaoracle_1_7_10/run.sh PopulateStages 1234 0 0 7
```

The mapped jar keeps `protected` members that were only legal in the obfuscated
default package, so `Widen.java` rewrites every protected field and method to
public before the probes compile against it.

## Probes

| Class | Prints |
|---|---|
| `MiniWorld` | not a probe: a constructor-less `World` around one chunk, for the others to build on |
| `WorldPrep` | not a probe: a multi-chunk generated world, for population probes |
| `TerrainDump` | one chunk after terrain and carving, save order |
| `PopulateWorld` | a chunk after the whole population pass over an area |
| `StageDump` | the chunk after each population stage (lakes, dungeons, decorate, snow) |
| `PopulateStages` | the next two RNG longs after a requested stage, one stage at a time |
| `DecorateStages` | the biome decorator's own passes, one at a time |
| `TreeDump` | one tree on a flat platform (`tree seed normal\|ground\|canopy`), save order |
| `RenderOracle` | the client's `BlockRenderer` vertex stream for a block case (needs the client jar first, which `run.sh` handles) |

The Rust side of each comparison lives beside the crate it checks:
`crates/v17/chunkgen/src/bin/{terrain_dump,carved_dump,world_dump,stage_dump}.rs`
for terrain and population, `crates/v17/features/src/bin/` for individual
features. `TreeDump`'s counterpart is
`crates/v17/features/src/bin/tree_parity.rs`, which takes the same arguments:
`cargo +1.99.1 run -p sunset-1-7-features --bin tree_parity -- acacia 101 normal`.
Save-order output is `index = y << 8 | z << 4 | x` so
`tools/terrain_parity_1_7_10.py` can diff a dump against a real world directory.

Two details that cost time: call `net.minecraft.block.Block.init()` before
touching `Blocks.*`, and set a field with `Unsafe.putLong`/`putInt` when it is
primitive (`putObject` corrupts it).
