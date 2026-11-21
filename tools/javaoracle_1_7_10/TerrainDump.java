import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.world.World;
import net.minecraft.world.chunk.WorldChunk;
import net.minecraft.world.gen.chunk.OverworldChunkGenerator;

/** Pre-population terrain + carving for one chunk, in save order (y, z, x). */
public class TerrainDump {
    public static void main(String[] a) throws Exception {
        long seed = Long.parseLong(a[0]);
        int cx = Integer.parseInt(a[1]);
        int cz = Integer.parseInt(a[2]);
        Block.init();
        Block[] blocks = new Block[65536];
        byte[] meta = new byte[65536];
        java.util.Arrays.fill(blocks, Blocks.AIR);
        World world = MiniWorld.create(seed, blocks, meta);
        OverworldChunkGenerator g = new OverworldChunkGenerator(world, seed, false);
        WorldChunk chunk = g.getChunk(cx, cz);
        StringBuilder sb = new StringBuilder();
        for (int y = 0; y < 256; y++) for (int z = 0; z < 16; z++) for (int x = 0; x < 16; x++) {
            if (y != 0 || z != 0 || x != 0) sb.append(',');
            sb.append(Block.REGISTRY.getId(chunk.getBlockAt(x, y, z)));
        }
        System.out.println(sb);
    }
}
