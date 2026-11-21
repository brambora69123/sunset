import java.util.HashMap;
import java.util.Map;
import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.world.World;
import net.minecraft.world.chunk.WorldChunk;
import net.minecraft.world.gen.chunk.OverworldChunkGenerator;

/** Multi-chunk world with generated terrain, for population probes. */
public final class WorldPrep {
    public static Map<Long, WorldChunk> chunks;

    public static World prepare(long seed, int cx, int cz, int radius) throws Exception {
        Block.init();
        Block[] blocks = new Block[65536];
        byte[] meta = new byte[65536];
        java.util.Arrays.fill(blocks, Blocks.STONE);
        World world = MiniWorld.create(seed, blocks, meta);
        OverworldChunkGenerator generator = new OverworldChunkGenerator(world, seed, false);
        chunks = new HashMap<>();
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
        return world;
    }

    public static long key(int x, int z) {
        return ((long) x << 32) ^ (z & 0xffffffffL);
    }
}
