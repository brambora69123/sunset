import java.lang.reflect.Field;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.client.Minecraft;
import net.minecraft.client.options.GameOptions;
import net.minecraft.client.render.block.BlockRenderer;
import net.minecraft.client.render.texture.Sprite;
import net.minecraft.client.render.texture.SpriteRegistry;
import net.minecraft.client.render.texture.TextureAtlasSprite;
import net.minecraft.client.render.vertex.Tesselator;
import net.minecraft.client.world.color.GrassColors;
import net.minecraft.world.LightType;
import net.minecraft.world.World;
import net.minecraft.world.chunk.WorldChunk;
import sun.misc.Unsafe;

/**
 * Dumps the 1.7.10 RenderBlocks vertex stream for the blocks the sunset
 * 1.7.10 renderer ports, one case per lighting mode.
 *
 * <p>Every vertex prints the raw 32-byte vertex words: the IEEE-754 bit
 * patterns of x, y, z, u and v (the Tesselator stores floats), the packed
 * colour int and the packed brightness int. That is exactly what the GPU
 * receives, and it diffs bit for bit against the Rust probe.
 *
 * <p>Rendering goes through the real `BlockRenderer.tesselateInWorld`, on a
 * one-chunk {@link MiniWorld}. A constructor-less `Minecraft` carries the two
 * option fields the renderer reads (ambient occlusion and fancy graphics).
 *
 * <p>Run with tools/javaoracle_1_7_10/build.sh then
 * tools/javaoracle_1_7_10/run.sh RenderOracle, and diff the output against
 * {@code cargo run -p sunset-1-7-render --bin render_probe}; the two streams
 * must stay bit-exact.
 */
public class RenderOracle {
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

    /** Atlas: 256x256, 16x16 icons. The Rust fixture uses the same table. */
    static final Map<String, int[]> SPRITES = new HashMap<>();

    static {
        grid("stone", 0, 0);
        grid("dirt", 1, 0);
        grid("dirt_podzol_top", 8, 0);
        grid("dirt_podzol_side", 9, 0);
        grid("grass_top", 2, 0);
        grid("grass_side", 3, 0);
        grid("grass_side_snowed", 4, 0);
        grid("grass_side_overlay", 5, 0);
        grid("log_oak", 0, 1);
        grid("log_oak_top", 1, 1);
        grid("log_spruce", 2, 1);
        grid("log_spruce_top", 3, 1);
        grid("log_birch", 4, 1);
        grid("log_birch_top", 5, 1);
        grid("log_jungle", 6, 1);
        grid("log_jungle_top", 7, 1);
        grid("glass", 0, 2);
        grid("torch_on", 1, 2);
        grid("planks_oak", 2, 2);
        grid("planks_spruce", 3, 2);
        grid("planks_birch", 4, 2);
        grid("planks_jungle", 5, 2);
        grid("planks_acacia", 6, 2);
        grid("planks_big_oak", 7, 2);
        grid("stone_slab_top", 0, 3);
        grid("stone_slab_side", 1, 3);
        grid("tallgrass", 2, 3);
        grid("fern", 3, 3);
        grid("deadbush", 4, 3);
        grid("water_still", 5, 3);
        grid("water_flow", 6, 3);
        grid("snow", 7, 3);
    }

    static void grid(String name, int column, int row) {
        SPRITES.put(name, new int[]{column * 16, row * 16});
    }

    static class TestSprite extends TextureAtlasSprite {
        TestSprite(String name, int x, int y) {
            super(name);
            this.setWidth(16);
            this.setHeight(16);
            this.init(256, 256, x, y, false);
        }    }

    static class TestRegistry implements SpriteRegistry {
        final Map<String, Sprite> built = new HashMap<>();
        int nextColumn = 8;
        int nextRow = 3;

        @Override
        public Sprite registerSprite(String name) {
            int[] at = SPRITES.get(name);
            if (at == null) {
                // Unused by the rendered faces; park it outside the shared table.
                System.err.println("note: unlisted test sprite " + name);
                at = new int[]{this.nextColumn * 16, this.nextRow * 16};
                this.nextColumn++;
                if (this.nextColumn >= 16) {
                    this.nextColumn = 8;
                    this.nextRow++;
                }
            }
            int[] position = at;
            return this.built.computeIfAbsent(name, n -> new TestSprite(n, position[0], position[1]));
        }
    }

    interface Setup {
        void build(Placer placer);
    }

    static class Placer {
        final Block[] blocks = new Block[16 * 16 * 16];
        final byte[] metadata = new byte[16 * 16 * 16];

        void put(int x, int y, int z, Block block, int meta) {
            int index = x * 16 * 16 + z * 16 + y;
            this.blocks[index] = block;
            this.metadata[index] = (byte) meta;
        }
    }

    static final int[][] LIGHT = new int[16 * 16 * 16][2];

    static {
        for (int y = 0; y < 16; y++) {
            for (int z = 0; z < 16; z++) {
                for (int x = 0; x < 16; x++) {
                    LIGHT[x * 16 * 16 + z * 16 + y] = new int[]{(x * 5 + y * 3 + z * 7) & 15, (x * 11 + y * 2 + z * 13) & 15};
                }
            }
        }
    }

    static Field bufferField;
    static Field indexField;
    static Field tesselatingField;

    public static void main(String[] args) throws Exception {
        Block.init();

        // A client with just the two option fields RenderBlocks reads.
        GameOptions options = (GameOptions) unsafe.allocateInstance(GameOptions.class);
        options.ambientOcclusion = 0;
        options.fancyGraphics = true;
        Minecraft minecraft = (Minecraft) unsafe.allocateInstance(Minecraft.class);
        Field optionsField = Minecraft.class.getDeclaredField("options");
        optionsField.setAccessible(true);
        optionsField.set(minecraft, options);
        Field instanceField = Minecraft.class.getDeclaredField("INSTANCE");
        instanceField.setAccessible(true);
        instanceField.set(null, minecraft);

        // One flat grass tint for every biome so the test is deterministic.
        int[] grass = new int[65536];
        Arrays.fill(grass, 0x91BD59);
        GrassColors.set(grass);

        TestRegistry registry = new TestRegistry();
        Blocks.STONE.registerSprites(registry);
        Blocks.DIRT.registerSprites(registry);
        Blocks.GRASS.registerSprites(registry);
        Blocks.LOG.registerSprites(registry);
        Blocks.GLASS.registerSprites(registry);
        Blocks.TORCH.registerSprites(registry);
        Blocks.PLANKS.registerSprites(registry);
        Blocks.OAK_STAIRS.registerSprites(registry);
        Blocks.STONE_SLAB.registerSprites(registry);
        Blocks.FENCE.registerSprites(registry);
        Blocks.TALLGRASS.registerSprites(registry);
        Blocks.WATER.registerSprites(registry);
        Blocks.FLOWING_WATER.registerSprites(registry);
        Blocks.SNOW_LAYER.registerSprites(registry);

        bufferField = Tesselator.class.getDeclaredField("vertexBuffer");
        bufferField.setAccessible(true);
        indexField = Tesselator.class.getDeclaredField("index");
        indexField.setAccessible(true);
        tesselatingField = Tesselator.class.getDeclaredField("tesselating");
        tesselatingField.setAccessible(true);

        // Standard cube, all six faces, the three lighting modes.
        for (int ao = 0; ao <= 2; ao++) {
            final int mode = ao;
            run("stone_ao" + mode, mode, false, Blocks.STONE, 0, 4, 4, 4, p -> p.put(4, 3, 4, Blocks.STONE, 0));
        }

        // Cube with two faces culled by solid neighbours.
        for (int ao = 0; ao <= 1; ao++) {
            final int mode = ao;
            run("stone_culled_ao" + mode, mode, false, Blocks.STONE, 0, 4, 4, 4, p -> {
                p.put(4, 3, 4, Blocks.STONE, 0);
                p.put(5, 4, 4, Blocks.STONE, 0);
                p.put(4, 4, 5, Blocks.STONE, 0);
            });
        }

        // Grass: tinted top, untinted sides, fancy side overlay pass.
        for (int ao = 0; ao <= 1; ao++) {
            final int mode = ao;
            run("grass_ao" + mode + "_fancy", mode, true, Blocks.GRASS, 0, 5, 4, 4, p -> p.put(5, 3, 4, Blocks.DIRT, 0));
            run("grass_ao" + mode + "_plain", mode, false, Blocks.GRASS, 0, 5, 4, 4, p -> p.put(5, 3, 4, Blocks.DIRT, 0));
        }

        // Log for each axis: metadata bit 4 and 8 pick the top sprite and rotate UVs.
        for (int meta : new int[]{0, 4, 8}) {
            for (int ao = 0; ao <= 1; ao++) {
                final int mode = ao;
                final int logMeta = meta;
                run("log_" + axisName(meta) + "_ao" + mode, mode, false, Blocks.LOG, logMeta, 6, 4, 4, p -> p.put(6, 3, 4, Blocks.STONE, 0));
            }
        }

        // Glass: culled against same-metadata glass, opaque neighbour culls too.
        for (int ao = 0; ao <= 1; ao++) {
            final int mode = ao;
            run("glass_ao" + mode, mode, false, Blocks.GLASS, 0, 7, 4, 4, p -> {
                p.put(8, 4, 4, Blocks.GLASS, 0);
                p.put(7, 4, 5, Blocks.GLASS, 0);
                p.put(7, 3, 4, Blocks.STONE, 0);
            });
        }

        // A floor torch and a wall torch.
        for (int ao = 0; ao <= 1; ao++) {
            final int mode = ao;
            run("torch_floor_ao" + mode, mode, false, Blocks.TORCH, 0, 9, 4, 4, p -> p.put(9, 3, 4, Blocks.STONE, 0));
        }
        run("torch_wall_ao1", 1, false, Blocks.TORCH, 1, 9, 4, 6, p -> p.put(10, 4, 6, Blocks.STONE, 0));

        // Oak stairs, bottom and upside-down.
        for (int ao = 0; ao <= 2; ao++) {
            final int mode = ao;
            run("stairs_east_ao" + mode, mode, false, Blocks.OAK_STAIRS, 0, 2, 4, 2, p -> p.put(2, 3, 2, Blocks.STONE, 0));
            run("stairs_upside_ao" + mode, mode, false, Blocks.OAK_STAIRS, 4, 2, 4, 2, p -> p.put(2, 3, 2, Blocks.STONE, 0));
        }

        // Stone slab bottom and top.
        for (int ao = 0; ao <= 2; ao++) {
            final int mode = ao;
            run("slab_bottom_ao" + mode, mode, false, Blocks.STONE_SLAB, 0, 3, 4, 3, p -> p.put(3, 3, 3, Blocks.STONE, 0));
            run("slab_top_ao" + mode, mode, false, Blocks.STONE_SLAB, 8, 3, 4, 3, p -> p.put(3, 3, 3, Blocks.STONE, 0));
        }

        // A fence connected east and west, with air north and south.
        for (int ao = 0; ao <= 2; ao++) {
            final int mode = ao;
            run("fence_ao" + mode, mode, false, Blocks.FENCE, 0, 6, 6, 4, p -> {
                p.put(5, 6, 4, Blocks.STONE, 0);
                p.put(7, 6, 4, Blocks.STONE, 0);
            });
        }

        // Tall grass and fern: the two variants of the tall plant block.
        for (int meta = 1; meta <= 2; meta++) {
            for (int ao = 0; ao <= 1; ao++) {
                final int mode = ao;
                final int plantMeta = meta;
                String name = meta == 1 ? "tallgrass" : "fern";
                run(name + "_ao" + mode, mode, false, Blocks.TALLGRASS, plantMeta, 2, 6, 4, p -> p.put(2, 5, 4, Blocks.DIRT, 0));
            }
        }

        // Still water and flowing water in a channel.
        for (int ao = 0; ao <= 1; ao++) {
            final int mode = ao;
            run("water_still_ao" + mode, mode, false, Blocks.WATER, 0, 8, 4, 8, p -> p.put(8, 3, 8, Blocks.WATER, 0));
        }
        run("water_flow_ao1", 1, false, Blocks.FLOWING_WATER, 1, 8, 4, 10, p -> {
            p.put(7, 4, 10, Blocks.WATER, 0);
            p.put(8, 3, 10, Blocks.STONE, 0);
        });

        // A non-full-bounds standard block: three snow layers.
        for (int ao = 0; ao <= 2; ao++) {
            final int mode = ao;
            run("snow_ao" + mode, mode, false, Blocks.SNOW_LAYER, 3, 5, 6, 4, p -> p.put(5, 5, 4, Blocks.STONE, 0));
        }
    }

    static String axisName(int metadata) {
        if ((metadata & 12) == 4) {
            return "x";
        }
        if ((metadata & 12) == 8) {
            return "z";
        }
        return "y";
    }

    static void run(String name, int ao, boolean fancy, Block block, int meta, int x, int y, int z, Setup setup) throws Exception {
        Placer placer = new Placer();
        Arrays.fill(placer.blocks, Blocks.AIR);
        setup.build(placer);
        placer.put(x, y, z, block, meta);
        World world = MiniWorld.create(0, placer.blocks, placer.metadata);
        WorldChunk chunk = world.getChunkAt(0, 0);
        byte[] biomes = new byte[256];
        Arrays.fill(biomes, (byte) 1);
        chunk.setBiomes(biomes);
        for (int yy = 0; yy < 16; yy++) {
            for (int zz = 0; zz < 16; zz++) {
                for (int xx = 0; xx < 16; xx++) {
                    int[] light = LIGHT[xx * 16 * 16 + zz * 16 + yy];
                    chunk.setLightAt(LightType.SKY, xx, yy, zz, light[0]);
                    chunk.setLightAt(LightType.BLOCK, xx, yy, zz, light[1]);
                }
            }
        }

        Minecraft.getInstance().options.ambientOcclusion = ao;
        BlockRenderer.fancyGraphics = fancy;
        BlockRenderer renderer = new BlockRenderer(world);

        Tesselator tesselator = Tesselator.INSTANCE;
        // `end()` would upload to OpenGL; reset the flag directly instead.
        tesselatingField.setBoolean(tesselator, false);
        tesselator.begin(7);
        boolean rendered = renderer.tesselateInWorld(block, x, y, z);

        int[] buffer = (int[]) bufferField.get(tesselator);
        int index = indexField.getInt(tesselator);
        System.out.println("== " + name + " rendered=" + rendered + " vertices=" + (index / 8));
        for (int i = 0; i < index / 8; i++) {
            int o = i * 8;
            System.out.println(
                String.format(
                    java.util.Locale.ROOT,
                    "v%d x=%08X y=%08X z=%08X u=%08X v=%08X color=%08X light=%d",
                    i,
                    buffer[o],
                    buffer[o + 1],
                    buffer[o + 2],
                    buffer[o + 3],
                    buffer[o + 4],
                    buffer[o + 5],
                    buffer[o + 7]
                )
            );
        }
    }
}
