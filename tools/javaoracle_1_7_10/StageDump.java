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

/** Replays populateChunk stage by stage, printing a hash of the centre chunk after each. */
public class StageDump {
    public static void main(String[] a) throws Exception {
        long seed = Long.parseLong(a[0]);
        int cx = Integer.parseInt(a[1]);
        int cz = Integer.parseInt(a[2]);
        int radius = 1;

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
        for (int dz = -(radius + 1); dz <= radius + 1; dz++) {
            for (int dx = -(radius + 1); dx <= radius + 1; dx++) {
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

        if (a.length > 3 && a[3].equals("heights")) {
            StringBuilder hs = new StringBuilder();
            for (int dz = 8; dz < 24; dz++) {
                for (int dx = 8; dx < 24; dx++) {
                    if (hs.length() > 0) hs.append(',');
                    hs.append(world.getHeight(bx + dx, bz + dz));
                }
            }
            System.out.println("HEIGHTS " + hs);
            return;
        }
        StringBuilder out = new StringBuilder();
        System.out.println("biome " + biome.getClass().getName() + " grass=" + biome.decorator.grassAttempts + " trees=" + biome.decorator.treeAttempts + " flowers=" + biome.decorator.flowerAttempts + " sand=" + biome.decorator.sandPatchAttempts);

        out.append("start ").append(hash(world, cx, cz)).append('\n');

        if (biome != Biome.DESERT && biome != Biome.DESERT_HILLS && random.nextInt(4) == 0) {
            int x = bx + random.nextInt(16) + 8;
            int y = random.nextInt(256);
            int z = bz + random.nextInt(16) + 8;
            new LakeFeature(Blocks.WATER).place(world, random, x, y, z);
        }
        out.append("water ").append(hash(world, cx, cz)).append('\n');

        if (random.nextInt(8) == 0) {
            int x = bx + random.nextInt(16) + 8;
            int y = random.nextInt(random.nextInt(248) + 8);
            int z = bz + random.nextInt(16) + 8;
            if (y < 63 || random.nextInt(10) == 0) {
                new LakeFeature(Blocks.LAVA).place(world, random, x, y, z);
            }
        }
        out.append("lava ").append(hash(world, cx, cz)).append('\n');

        for (int i = 0; i < 8; i++) {
            int x = bx + random.nextInt(16) + 8;
            int y = random.nextInt(256);
            int z = bz + random.nextInt(16) + 8;
            new DungeonFeature().place(world, random, x, y, z);
        }
        out.append("dungeon ").append(hash(world, cx, cz)).append('\n');

        if (a.length > 3 && a[3].equals("rng")) {
            java.util.Random probe = (java.util.Random) random;
            System.out.println("predecorate " + probe.nextLong() + " " + probe.nextLong());
            return;
        }
        biome.decorate(world, random, bx, bz);
        out.append("decorate ").append(hash(world, cx, cz)).append('\n');
        // The draw stream after the whole decorator, overrides included.
        out.append("rngafter ").append(random.nextLong()).append(' ').append(random.nextLong()).append('\n');
        if (a.length > 3 && a[3].equals("blocks")) {
            WorldChunk cc = world.getChunkSource().getChunk(cx, cz);
            StringBuilder sb = new StringBuilder();
            for (int y = 0; y < 256; y++) for (int z = 0; z < 16; z++) for (int x = 0; x < 16; x++) {
                if (y != 0 || z != 0 || x != 0) sb.append(',');
                sb.append(Block.REGISTRY.getId(cc.getBlockAt(x, y, z)));
            }
            System.out.println("BLOCKS " + sb);
            return;
        }

        int sx = bx + 8;
        int sz = bz + 8;
        for (int i = 0; i < 16; i++) {
            for (int j = 0; j < 16; j++) {
                int h = world.getPrecipitationHeight(sx + i, sz + j);
                if (world.canFreeze(sx + i, h - 1, sz + j)) {
                    world.setBlockWithMetadata(sx + i, h - 1, sz + j, Blocks.ICE, 0, 2);
                }
                if (world.canSnowFall(sx + i, h, sz + j, true)) {
                    world.setBlockWithMetadata(sx + i, h, sz + j, Blocks.SNOW_LAYER, 0, 2);
                }
            }
        }
        out.append("snow ").append(hash(world, cx, cz)).append('\n');
        System.out.print(out);
    }

    private static String hash(World world, int cx, int cz) {
        WorldChunk chunk = world.getChunkSource().getChunk(cx, cz);
        long h = 0xcbf29ce484222325L;
        int nonAir = 0;
        for (int x = 0; x < 16; x++) for (int z = 0; z < 16; z++) for (int y = 0; y < 256; y++) {
            int id = Block.REGISTRY.getId(chunk.getBlockAt(x, y, z));
            if (id != 0) nonAir++;
            h ^= id;
            h *= 0x100000001b3L;
        }
        return Long.toHexString(h) + "/" + nonAir;
    }

    private static long key(int x, int z) {
        return ((long) x << 32) ^ (z & 0xffffffffL);
    }
}
