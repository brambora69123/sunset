import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Random;
import java.util.concurrent.atomic.AtomicLong;

import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.server.world.ServerWorld;
import net.minecraft.world.World;

import sun.misc.Unsafe;

/**
 * Liquid parity probe: builds one of the 1.7.10 layouts, places the liquids,
 * ticks the scheduled queue and prints the blocks that changed (plus every
 * liquid), in save order, and the world random's state.
 *
 * The Rust counterpart is crates/v17/liquids/src/bin/liquid_probe.rs.
 * Run: tools/javaoracle_1_7_10/run.sh LiquidProbe flat
 */
public class LiquidProbe {
    static final int SPAN = 16;
    static final int HEIGHT = 32;
    static final long SEED = 1234;

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

    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.err.println("usage: LiquidProbe <flat|hole|two_sources|water_lava|lava_over_water|cave|run15>");
            System.exit(2);
        }
        String name = args[0];

        Block.init();
        Block[] blocks = new Block[65536];
        byte[] meta = new byte[65536];
        Arrays.fill(blocks, Blocks.AIR);

        int[][] kicks;
        int ticks;

        switch (name) {
            case "flat":
                fill(blocks, 0, 0, 0, 15, 4, 15, Blocks.STONE);
                kicks = new int[][] {{8, 5, 8}};
                ticks = 80;
                break;
            case "hole":
                fill(blocks, 0, 0, 0, 15, 4, 15, Blocks.STONE);
                fill(blocks, 8, 1, 8, 8, 4, 8, Blocks.AIR);
                kicks = new int[][] {{8, 6, 8}};
                ticks = 80;
                break;
            case "two_sources":
                fill(blocks, 0, 0, 0, 15, 4, 15, Blocks.STONE);
                kicks = new int[][] {{4, 5, 8}, {12, 5, 8}};
                ticks = 100;
                break;
            case "water_lava":
                fill(blocks, 0, 0, 0, 15, 9, 15, Blocks.STONE);
                fill(blocks, 1, 5, 1, 5, 9, 5, Blocks.AIR);
                fill(blocks, 10, 5, 1, 14, 9, 5, Blocks.AIR);
                fill(blocks, 1, 5, 10, 5, 9, 14, Blocks.AIR);
                fill(blocks, 2, 4, 11, 2, 4, 11, Blocks.AIR);
                kicks = new int[][] {{2, 5, 11}};
                ticks = 120;
                break;
            case "lava_over_water":
                fill(blocks, 0, 0, 0, 15, 4, 15, Blocks.STONE);
                kicks = new int[][] {{8, 6, 8}};
                ticks = 80;
                break;
            case "cave":
                fill(blocks, 0, 0, 0, 15, 4, 15, Blocks.STONE);
                fill(blocks, 4, 1, 4, 11, 3, 11, Blocks.AIR);
                fill(blocks, 8, 4, 8, 8, 4, 8, Blocks.AIR);
                kicks = new int[][] {{8, 6, 8}};
                ticks = 120;
                break;
            case "run15":
                fill(blocks, 0, 0, 0, 15, 4, 15, Blocks.STONE);
                for (int x = 0; x <= 15; x++) {
                    fill(blocks, x, 5, 7, x, 5, 7, Blocks.STONE);
                    fill(blocks, x, 5, 9, x, 5, 9, Blocks.STONE);
                }
                fill(blocks, 0, 5, 8, 0, 5, 8, Blocks.STONE);
                kicks = new int[][] {{1, 5, 8}};
                ticks = 80;
                break;
            default:
                throw new IllegalArgumentException("unknown layout " + name);
        }

        World world = MiniWorld.create(SEED, blocks, meta);
        MiniWorld.set(world, ServerWorld.class, "scheduledTicksThisTick", new ArrayList<>());
        world.random.setSeed(SEED);

        // Place the liquids the way a generator or a player does: flags 3.
        if (name.equals("flat") || name.equals("hole") || name.equals("two_sources")
                || name.equals("lava_over_water") || name.equals("cave") || name.equals("run15")) {
            if (name.equals("two_sources")) {
                world.setBlockWithMetadata(4, 5, 8, Blocks.WATER, 0, 3);
                world.setBlockWithMetadata(12, 5, 8, Blocks.WATER, 0, 3);
            } else if (name.equals("lava_over_water")) {
                world.setBlockWithMetadata(8, 5, 8, Blocks.WATER, 0, 3);
                world.setBlockWithMetadata(8, 6, 8, Blocks.LAVA, 0, 3);
            } else if (name.equals("run15")) {
                world.setBlockWithMetadata(1, 5, 8, Blocks.WATER, 0, 3);
            } else {
                int x = 8;
                int y = name.equals("flat") ? 5 : 6;
                world.setBlockWithMetadata(x, y, 8, Blocks.WATER, 0, 3);
            }
        } else if (name.equals("water_lava")) {
            world.setBlockWithMetadata(2, 5, 2, Blocks.LAVA, 0, 3);
            world.setBlockWithMetadata(2, 5, 3, Blocks.WATER, 0, 3);
            world.setBlockWithMetadata(11, 5, 2, Blocks.LAVA, 2, 3);
            world.setBlockWithMetadata(11, 5, 3, Blocks.WATER, 0, 3);
            world.setBlockWithMetadata(2, 4, 11, Blocks.WATER, 0, 3);
            world.setBlockWithMetadata(2, 5, 11, Blocks.LAVA, 0, 3);
        }

        int[] beforeId = new int[SPAN * SPAN * HEIGHT];
        int[] beforeMeta = new int[SPAN * SPAN * HEIGHT];
        for (int y = 0; y < HEIGHT; y++) for (int z = 0; z < SPAN; z++) for (int x = 0; x < SPAN; x++) {
            beforeId[slot(x, y, z)] = Block.getId(world.getBlock(x, y, z));
            beforeMeta[slot(x, y, z)] = world.getBlockMetadata(x, y, z);
        }

        for (int[] kick : kicks) {
            world.getBlock(kick[0], kick[1], kick[2])
                    .neighborChanged(world, kick[0], kick[1], kick[2], Blocks.STONE);
        }

        for (int t = 0; t < ticks; t++) {
            world.data.setTime(world.data.getTime() + 1L);
            world.doScheduledTicks(false);
        }

        for (int y = 0; y < HEIGHT; y++) for (int z = 0; z < SPAN; z++) for (int x = 0; x < SPAN; x++) {
            int afterId = Block.getId(world.getBlock(x, y, z));
            int afterMeta = world.getBlockMetadata(x, y, z);
            int s = slot(x, y, z);
            if (afterId != beforeId[s] || afterMeta != beforeMeta[s] || isLiquid(afterId)) {
                System.out.println(index(x, y, z) + " " + afterId + " " + afterMeta);
            }
        }
        System.out.println("rng " + randomState(world.random));
    }

    static boolean isLiquid(int id) {
        return id == 8 || id == 9 || id == 10 || id == 11;
    }

    static int slot(int x, int y, int z) {
        return (y * SPAN + z) * SPAN + x;
    }

    static int index(int x, int y, int z) {
        return y << 8 | z << 4 | x;
    }

    /** blocks index: x << 12 | z << 8 | y (WorldChunk's 256-high order) */
    static void fill(Block[] blocks, int x0, int y0, int z0, int x1, int y1, int z1, Block block) {
        for (int x = x0; x <= x1; x++) for (int y = y0; y <= y1; y++) for (int z = z0; z <= z1; z++) {
            blocks[x << 12 | z << 8 | y] = block;
        }
    }

    static long randomState(Random random) throws Exception {
        long offset = unsafe.objectFieldOffset(Random.class.getDeclaredField("seed"));
        Object atomic = unsafe.getObject(random, offset);
        long valueOffset = unsafe.objectFieldOffset(AtomicLong.class.getDeclaredField("value"));
        return unsafe.getLong(atomic, valueOffset);
    }
}
