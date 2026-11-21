import java.util.HashMap;
import java.util.Map;
import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.world.World;
import net.minecraft.world.biome.Biome;
import net.minecraft.world.chunk.WorldChunk;
import net.minecraft.world.gen.chunk.OverworldChunkGenerator;
import net.minecraft.world.gen.feature.DungeonFeature;
import net.minecraft.world.gen.feature.LakeFeature;

/**
 * Column heights of the populated chunk: the chunk height map `World.getHeight`
 * reads, the top non-air block, and every tall grass block. Used to explain
 * cases where a feature's `getHeight` differs between the oracle and a port
 * that scans for the highest non-air block.
 */
public class HeightsAfter {
    public static void main(String[] a) throws Exception {
        long seed = Long.parseLong(a[0]);
        int cx = Integer.parseInt(a[1]);
        int cz = Integer.parseInt(a[2]);

        Block.init();
        Block[] blocks = new Block[65536];
        byte[] meta = new byte[65536];
        java.util.Arrays.fill(blocks, Blocks.STONE);
        World world = MiniWorld.create(seed, blocks, meta);
        OverworldChunkGenerator generator = new OverworldChunkGenerator(world, seed, false);

        Map<Long, WorldChunk> chunks = new HashMap<>();
        MiniWorld.set(world, World.class, "chunkSource", new net.minecraft.world.chunk.ChunkSource() {
            @Override public boolean hasChunk(int x, int z) { return chunks.containsKey(key(x, z)); }
            @Override public WorldChunk getChunk(int x, int z) { return chunks.get(key(x, z)); }
            @Override public WorldChunk loadChunk(int x, int z) {
                WorldChunk chunk = chunks.get(key(x, z));
                if (chunk == null) { chunk = generator.getChunk(x, z); chunks.put(key(x, z), chunk); }
                return chunk;
            }
            @Override public void populateChunk(net.minecraft.world.chunk.ChunkSource s, int x, int z) {}
            @Override public boolean save(boolean e, net.minecraft.util.ProgressListener l) { return true; }
            @Override public boolean tick() { return false; }
            @Override public boolean shouldSave() { return false; }
            @Override public String getDebugInfo() { return "multi"; }
            @Override public java.util.List getSpawnEntries(net.minecraft.entity.living.mob.MobCategory c, int x, int y, int z) { return java.util.Collections.emptyList(); }
            @Override public net.minecraft.util.math.BlockPos findNearestStructure(World w, String n, int x, int y, int z) { return null; }
            @Override public int size() { return chunks.size(); }
            @Override public void placeStructures(int x, int z) {}
            @Override public void flush() {}
        });
        for (int dz = -2; dz <= 2; dz++) {
            for (int dx = -2; dx <= 2; dx++) {
                world.getChunkSource().loadChunk(cx + dx, cz + dz);
            }
        }

        int bx = cx * 16;
        int bz = cz * 16;
        Biome biome = world.getBiome(bx + 16, bz + 16);
        java.util.Random random = new java.util.Random(seed);
        random.setSeed(seed);
        long halfA = random.nextLong() / 2L * 2L + 1L;
        long halfB = random.nextLong() / 2L * 2L + 1L;
        random.setSeed((long) cx * halfA + (long) cz * halfB ^ seed);

        if (biome != Biome.DESERT && biome != Biome.DESERT_HILLS && random.nextInt(4) == 0) {
            int x = bx + random.nextInt(16) + 8;
            int y = random.nextInt(256);
            int z = bz + random.nextInt(16) + 8;
            new LakeFeature(Blocks.WATER).place(world, random, x, y, z);
        }
        if (random.nextInt(8) == 0) {
            int x = bx + random.nextInt(16) + 8;
            int y = random.nextInt(random.nextInt(248) + 8);
            int z = bz + random.nextInt(16) + 8;
            if (y < 63 || random.nextInt(10) == 0) {
                new LakeFeature(Blocks.LAVA).place(world, random, x, y, z);
            }
        }
        for (int i = 0; i < 8; i++) {
            int x = bx + random.nextInt(16) + 8;
            int y = random.nextInt(256);
            int z = bz + random.nextInt(16) + 8;
            new DungeonFeature().place(world, random, x, y, z);
        }

        System.out.println("pre");
        dump(world, chunks.get(key(cx, cz)), bx, bz);
        biome.decorate(world, random, bx, bz);
        System.out.println("post");
        dump(world, chunks.get(key(cx, cz)), bx, bz);
    }

    static void dump(World world, WorldChunk chunk, int bx, int bz) {
        for (int z = 0; z < 16; z++) {
            for (int x = 0; x < 16; x++) {
                int top = 0;
                for (int y = 255; y >= 0; y--) {
                    if (chunk.getBlockAt(x, y, z) != Blocks.AIR) {
                        top = y + 1;
                        break;
                    }
                }
                System.out.println("H " + (bx + x) + " " + (bz + z) + " " + chunk.getHeight(x, z) + " " + top);
            }
        }
        StringBuilder sb = new StringBuilder("TALLGRASS");
        for (int y = 0; y < 256; y++) for (int z = 0; z < 16; z++) for (int x = 0; x < 16; x++) {
            if (chunk.getBlockAt(x, y, z) == Blocks.TALLGRASS) {
                sb.append(' ').append(bx + x).append(',').append(y).append(',').append(bz + z);
            }
        }
        System.out.println(sb);
    }

    static long key(int x, int z) {
        return ((long) x << 32) ^ (z & 0xffffffffL);
    }
}
