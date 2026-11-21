import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.Arrays;

import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.block.FallingBlock;
import net.minecraft.server.world.ServerWorld;
import net.minecraft.world.World;
import net.minecraft.world.dimension.Dimension;
import net.minecraft.world.dimension.NetherDimension;
import sun.misc.Unsafe;

/**
 * Liquid and gravity parity probe: lays out a 1.7.10 scenario on one chunk,
 * places water/lava or sand/gravel the way a bucket, a world generation
 * feature or a chunk generator does, runs the scheduled block ticks with a
 * fixed world random, and prints every non-air block of the print box as
 * {@code x y z block-id metadata}.
 *
 * <p>The Rust counterpart is {@code crates/sunrise-world/src/level/tests/liquids.rs}.
 * Run: {@code tools/javaoracle_1_7_10/run.sh FluidOracle flat}
 */
public class FluidOracle {
    static final int HEIGHT = 16;
    static final long SEED = 1234;

    static Unsafe unsafe;

    static {
        try {
            Field field = Unsafe.class.getDeclaredField("theUnsafe");
            field.setAccessible(true);
            unsafe = (Unsafe) field.get(null);
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }

    static void putBoolean(Object target, Class<?> owner, String field, boolean value)
            throws Exception {
        long offset = unsafe.objectFieldOffset(owner.getDeclaredField(field));
        unsafe.putBoolean(target, offset, value);
    }

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.err.println("usage: FluidOracle "
                    + "<flat|hole|run15|two_sources|water_lava|lava_over_water|cave|"
                    + "lava_spring_under_water|sand_fall|spring_gen|lava_nether>");
            System.exit(2);
        }
        String scenario = args[0];

        Block.init();
        Block[] blocks = new Block[65536];
        byte[] meta = new byte[65536];
        Arrays.fill(blocks, Blocks.AIR);

        int ticks;
        boolean fallImmediately = false;
        boolean ticksImmediately = false;
        switch (scenario) {
            case "flat":
                plate(blocks, 4, 4, 12, 12);
                ticks = 80;
                break;
            case "hole":
                plate(blocks, 4, 4, 12, 12);
                fill(blocks, 8, 3, 8, 8, 3, 8, Blocks.AIR);
                ticks = 80;
                break;
            case "run15":
                fill(blocks, 0, 3, 7, 15, 3, 9, Blocks.STONE);
                fill(blocks, 0, 4, 7, 15, 4, 7, Blocks.STONE);
                fill(blocks, 0, 4, 9, 15, 4, 9, Blocks.STONE);
                fill(blocks, 0, 4, 8, 0, 4, 8, Blocks.STONE);
                fill(blocks, 15, 4, 8, 15, 4, 8, Blocks.STONE);
                ticks = 100;
                break;
            case "two_sources":
                plate(blocks, 2, 6, 14, 10);
                ticks = 120;
                break;
            case "water_lava":
                fill(blocks, 0, 3, 0, 15, 3, 15, Blocks.STONE);
                ticks = 60;
                break;
            case "lava_over_water":
                plate(blocks, 4, 4, 12, 12);
                ticks = 80;
                break;
            case "cave":
                fill(blocks, 3, 0, 3, 13, 4, 13, Blocks.STONE);
                fill(blocks, 5, 1, 5, 11, 3, 11, Blocks.AIR);
                fill(blocks, 8, 4, 8, 8, 4, 8, Blocks.AIR);
                ticks = 120;
                break;
            case "lava_spring_under_water":
                plate(blocks, 4, 4, 12, 12);
                ticks = 20;
                break;
            case "sand_fall":
                plate(blocks, 4, 4, 12, 12);
                fallImmediately = true;
                ticks = 10;
                break;
            case "spring_gen":
                plate(blocks, 4, 4, 12, 12);
                ticksImmediately = true;
                ticks = 20;
                break;
            case "lava_nether":
                plate(blocks, 4, 4, 12, 12);
                ticks = 40;
                break;
            default:
                throw new IllegalArgumentException("unknown scenario " + scenario);
        }

        World world = MiniWorld.create(SEED, blocks, meta);
        MiniWorld.set(world, ServerWorld.class, "scheduledTicksThisTick", new ArrayList<>());
        isolate(world);
        world.random.setSeed(SEED);
        FallingBlock.fallImmediately = fallImmediately;
        if (scenario.equals("lava_nether")) {
            // NetherDimension's constructor is skipped by Unsafe, so set the
            // two fields the liquid rules read.
            Object dimension = unsafe.allocateInstance(NetherDimension.class);
            putBoolean(dimension, Dimension.class, "yeetsWater", true);
            putBoolean(dimension, Dimension.class, "noSky", true);
            MiniWorld.set(world, World.class, "dimension", dimension);
        }

        // Place the liquids the way a bucket or a generation feature does.
        switch (scenario) {
            case "flat":
            case "hole":
                world.setBlockWithMetadata(8, 4, 8, Blocks.WATER, 0, 3);
                kick(world, 8, 4, 8);
                break;
            case "run15":
                world.setBlockWithMetadata(8, 4, 8, Blocks.WATER, 0, 3);
                kick(world, 8, 4, 8);
                break;
            case "two_sources":
                world.setBlockWithMetadata(4, 4, 8, Blocks.WATER, 0, 3);
                world.setBlockWithMetadata(12, 4, 8, Blocks.WATER, 0, 3);
                kick(world, 4, 4, 8);
                kick(world, 12, 4, 8);
                break;
            case "water_lava":
                // Source lava beside water: obsidian.
                world.setBlockWithMetadata(3, 4, 3, Blocks.LAVA, 0, 3);
                world.setBlockWithMetadata(3, 4, 4, Blocks.WATER, 0, 3);
                // A flow of level 2 beside water: cobblestone.
                world.setBlockWithMetadata(12, 4, 3, Blocks.FLOWING_LAVA, 2, 3);
                world.setBlockWithMetadata(12, 4, 4, Blocks.WATER, 0, 3);
                // Water below, lava above: the flow turns the water to stone.
                world.setBlockWithMetadata(3, 4, 12, Blocks.WATER, 0, 3);
                world.setBlockWithMetadata(3, 5, 12, Blocks.LAVA, 0, 3);
                kick(world, 3, 5, 12);
                break;
            case "lava_over_water":
                world.setBlockWithMetadata(8, 4, 8, Blocks.WATER, 0, 3);
                world.setBlockWithMetadata(8, 6, 8, Blocks.LAVA, 0, 3);
                kick(world, 8, 6, 8);
                break;
            case "cave":
                world.setBlockWithMetadata(8, 5, 8, Blocks.WATER, 0, 3);
                kick(world, 8, 5, 8);
                break;
            case "lava_spring_under_water":
                world.setBlockWithMetadata(8, 4, 8, Blocks.LAVA, 0, 3);
                world.setBlockWithMetadata(8, 5, 8, Blocks.WATER, 0, 3);
                break;
            case "sand_fall":
                world.setBlockWithMetadata(8, 6, 8, Blocks.SAND, 0, 3);
                world.setBlockWithMetadata(9, 6, 8, Blocks.GRAVEL, 0, 3);
                world.setBlockWithMetadata(10, 6, 8, Blocks.SAND, 1, 3);
                break;
            case "spring_gen":
                // LiquidFallFeature: flags 2, then the immediate tick itself.
                world.doTicksImmediately = true;
                world.setBlockWithMetadata(8, 4, 8, Blocks.FLOWING_WATER, 0, 2);
                world.doTicksImmediately = false;
                break;
            case "lava_nether":
                world.setBlockWithMetadata(8, 4, 8, Blocks.LAVA, 0, 3);
                kick(world, 8, 4, 8);
                break;
            default:
                throw new AssertionError();
        }

        for (int t = 0; t < ticks; t++) {
            world.data.setTime(world.data.getTime() + 1L);
            world.doScheduledTicks(false);
        }

        print(world);
    }

    /** A stone plate at y = 3 spanning the given x/z rectangle. */
    static void plate(Block[] blocks, int x0, int z0, int x1, int z1) {
        fill(blocks, x0, 3, z0, x1, 3, z1, Blocks.STONE);
    }

    /**
     * Gives the world one real chunk and fresh empty chunks around it, so a
     * liquid near the border reads air outside the chunk instead of the same
     * chunk's wrapped coordinates (MiniWorld returns its one chunk for every
     * chunk position).
     */
    static void isolate(World world) throws Exception {
        java.util.Map<Long, net.minecraft.world.chunk.WorldChunk> chunks = new java.util.HashMap<>();
        chunks.put(0L, world.getChunk(0, 0));
        MiniWorld.set(world, World.class, "chunkSource",
                new net.minecraft.world.chunk.ChunkSource() {
                    @Override public boolean hasChunk(int x, int z) { return true; }
                    @Override public net.minecraft.world.chunk.WorldChunk getChunk(int x, int z) {
                        long key = ((long) x << 32) ^ (z & 0xffffffffL);
                        return chunks.computeIfAbsent(key,
                                k -> new net.minecraft.world.chunk.WorldChunk(world, x, z));
                    }
                    @Override public net.minecraft.world.chunk.WorldChunk loadChunk(int x, int z) {
                        return getChunk(x, z);
                    }
                    @Override public void populateChunk(net.minecraft.world.chunk.ChunkSource s, int x, int z) {}
                    @Override public boolean save(boolean e, net.minecraft.util.ProgressListener l) { return true; }
                    @Override public boolean tick() { return false; }
                    @Override public boolean shouldSave() { return false; }
                    @Override public String getDebugInfo() { return "isolated"; }
                    @Override public java.util.List getSpawnEntries(
                            net.minecraft.entity.living.mob.MobCategory c, int x, int y, int z) {
                        return java.util.Collections.emptyList();
                    }
                    @Override public net.minecraft.util.math.BlockPos findNearestStructure(
                            World w, String n, int x, int y, int z) { return null; }
                    @Override public int size() { return 1; }
                    @Override public void placeStructures(int x, int z) {}
                    @Override public void flush() {}
                });
    }

    /** blocks index: x << 12 | z << 8 | y (WorldChunk's 256-high order). */
    static void fill(Block[] blocks, int x0, int y0, int z0, int x1, int y1, int z1, Block block) {
        for (int x = x0; x <= x1; x++) {
            for (int z = z0; z <= z1; z++) {
                for (int y = y0; y <= y1; y++) {
                    blocks[x << 12 | z << 8 | y] = block;
                }
            }
        }
    }

    /** A neighbour change on the placed liquid, as the block beside it does. */
    static void kick(World world, int x, int y, int z) {
        world.getBlock(x, y, z).neighborChanged(world, x, y, z, Blocks.STONE);
    }

    static void print(World world) {
        for (int y = 0; y <= HEIGHT; y++) {
            for (int z = 0; z < 16; z++) {
                for (int x = 0; x < 16; x++) {
                    Block block = world.getBlock(x, y, z);
                    if (block == Blocks.AIR) {
                        continue;
                    }
                    System.out.println(x + " " + y + " " + z + " "
                            + Block.REGISTRY.getKey(block) + " " + world.getBlockMetadata(x, y, z));
                }
            }
        }
    }
}
