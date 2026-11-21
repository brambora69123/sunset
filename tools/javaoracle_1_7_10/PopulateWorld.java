import java.util.HashMap;
import java.util.Map;
import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.world.World;
import net.minecraft.world.chunk.WorldChunk;
import net.minecraft.world.gen.chunk.OverworldChunkGenerator;

/**
 * Generates a terrain area, populates it the way a fresh world does (centre
 * chunk first, then ring by ring), and dumps one chunk in save order.
 */
public class PopulateWorld {
    public static void main(String[] a) throws Exception {
        long seed = Long.parseLong(a[0]);
        int cx = Integer.parseInt(a[1]);
        int cz = Integer.parseInt(a[2]);
        int radius = a.length > 3 ? Integer.parseInt(a[3]) : 1;

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
                if (chunk == null) {
                    chunk = generator.getChunk(x, z);
                    chunks.put(key(x, z), chunk);
                }
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

        java.util.List<int[]> order = new java.util.ArrayList<>();
        for (int dz = -radius; dz <= radius; dz++) {
            for (int dx = -radius; dx <= radius; dx++) {
                order.add(new int[] {dx, dz});
            }
        }
        order.sort((p, q) -> {
            int da = Math.max(Math.abs(p[0]), Math.abs(p[1]));
            int db = Math.max(Math.abs(q[0]), Math.abs(q[1]));
            if (da != db) return Integer.compare(da, db);
            if (p[1] != q[1]) return Integer.compare(p[1], q[1]);
            return Integer.compare(p[0], q[0]);
        });
        for (int[] p : order) {
            generator.populateChunk(world.getChunkSource(), cx + p[0], cz + p[1]);
        }

        WorldChunk centre = chunks.get(key(cx, cz));
        StringBuilder sb = new StringBuilder();
        for (int y = 0; y < 256; y++) for (int z = 0; z < 16; z++) for (int x = 0; x < 16; x++) {
            if (y != 0 || z != 0 || x != 0) sb.append(',');
            sb.append(Block.REGISTRY.getId(centre.getBlockAt(x, y, z)));
        }
        System.out.println(sb);
    }

    private static long key(int x, int z) {
        return ((long) x << 32) ^ (z & 0xffffffffL);
    }
}
