import java.lang.reflect.Field;
import java.util.List;
import net.minecraft.block.Block;
import net.minecraft.entity.living.mob.MobCategory;
import net.minecraft.util.ProgressListener;
import net.minecraft.util.math.BlockPos;
import net.minecraft.world.World;
import net.minecraft.world.biome.source.BiomeSource;
import net.minecraft.world.chunk.ChunkSource;
import net.minecraft.world.chunk.WorldChunk;
import net.minecraft.world.dimension.Dimension;
import net.minecraft.world.gen.WorldGeneratorType;
import sun.misc.Unsafe;

/** A single-chunk vanilla world, allocated without constructors, for parity probes. */
public class MiniWorld {
    static Unsafe unsafe;

    static {
        try {
            Field f = Unsafe.class.getDeclaredField("theUnsafe");
            f.setAccessible(true);
            unsafe = (Unsafe) f.get(null);
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }

    static void set(Object target, Class<?> owner, String field, Object value) throws Exception {
        long off = unsafe.objectFieldOffset(owner.getDeclaredField(field));
        unsafe.putObject(target, off, value);
    }

    public static World create(long seed, Block[] blocks, byte[] meta) throws Exception {
        World world = (World) unsafe.allocateInstance(net.minecraft.server.world.ServerWorld.class);
        Dimension dim = (Dimension) unsafe.allocateInstance(net.minecraft.world.dimension.OverworldDimension.class);
        set(world, World.class, "dimension", dim);
        set(dim, Dimension.class, "biomeSource", new BiomeSource(seed, WorldGeneratorType.DEFAULT));

        WorldChunk chunk = new WorldChunk(world, blocks, meta, 0, 0);
        ChunkSource source = new ChunkSource() {
            @Override public boolean hasChunk(int x, int z) { return true; }
            @Override public WorldChunk getChunk(int x, int z) { return chunk; }
            @Override public WorldChunk loadChunk(int x, int z) { return chunk; }
            @Override public void populateChunk(ChunkSource s, int x, int z) {}
            @Override public boolean save(boolean e, ProgressListener l) { return true; }
            @Override public boolean tick() { return false; }
            @Override public boolean shouldSave() { return false; }
            @Override public String getDebugInfo() { return "mini"; }
            @Override public List getSpawnEntries(MobCategory c, int x, int y, int z) { return java.util.Collections.emptyList(); }
            @Override public BlockPos findNearestStructure(World w, String n, int x, int y, int z) { return null; }
            @Override public int size() { return 1; }
            @Override public void placeStructures(int x, int z) {}
            @Override public void flush() {}
        };
        set(world, World.class, "chunkSource", source);
        set(world, World.class, "random", new java.util.Random(0));
        set(world, World.class, "profiler", new net.minecraft.util.profiler.Profiler());
        set(world, World.class, "lightPropagation", new int[1 << 20]);
        set(world, World.class, "eventListeners", new java.util.ArrayList());

        Object data = unsafe.allocateInstance(net.minecraft.world.WorldData.class);
        set(world, World.class, "data", data);
        long seedOff = unsafe.objectFieldOffset(net.minecraft.world.WorldData.class.getDeclaredField("seed"));
        unsafe.putLong(data, seedOff, seed);
        set(data, net.minecraft.world.WorldData.class, "generatorType", WorldGeneratorType.DEFAULT);

        set(world, net.minecraft.server.world.ServerWorld.class, "scheduledTicks", new java.util.HashSet());
        set(world, net.minecraft.server.world.ServerWorld.class, "scheduledTicksInOrder", new java.util.TreeSet());
        return world;
    }
}
