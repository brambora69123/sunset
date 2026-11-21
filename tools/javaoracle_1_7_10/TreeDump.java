import java.util.Random;
import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.world.World;
import net.minecraft.world.gen.feature.AbstractTreeFeature;
import net.minecraft.world.gen.feature.AcaciaTreeFeature;
import net.minecraft.world.gen.feature.BirchTreeFeature;
import net.minecraft.world.gen.feature.BushFeature;
import net.minecraft.world.gen.feature.DarkOakTreeFeature;
import net.minecraft.world.gen.feature.GiantJungleTreeFeature;
import net.minecraft.world.gen.feature.GiantSpruceTreeFeature;
import net.minecraft.world.gen.feature.TreeFeature;

/**
 * One tree on a flat platform, printing every non-air block in save order.
 * The Rust side is `crates/v17/features/src/bin/tree_parity.rs`.
 *
 * Arguments: <tree> <seed> <mode>. Modes: normal, ground (stone
 * under the trunk), canopy (a stone block in the clearance volume).
 */
public class TreeDump {
    public static void main(String[] a) throws Exception {
        String tree = a[0];
        long seed = Long.parseLong(a[1]);
        String mode = a[2];

        // The giant jungle's branches reach eight blocks out.
        int origin = tree.equals("giant_jungle") ? 5 : 8;
        int x = origin;
        int y = 64;
        int z = origin;

        Block.init();
        Block[] blocks = new Block[65536];
        byte[] meta = new byte[65536];
        for (int bx = 0; bx < 16; bx++) {
            for (int bz = 0; bz < 16; bz++) {
                for (int by = 0; by < 64; by++) {
                    int i = (bx * 16 + bz) * 256 + by;
                    blocks[i] = by == 63 ? Blocks.GRASS : Blocks.DIRT;
                }
            }
        }
        if (mode.equals("ground")) {
            blocks[(x * 16 + z) * 256 + (y - 1)] = Blocks.STONE;
        } else if (mode.equals("canopy")) {
            blocks[((x + 1) * 16 + z) * 256 + (y + 3)] = Blocks.STONE;
        } else if (!mode.equals("normal")) {
            throw new IllegalArgumentException(mode);
        }

        World world = MiniWorld.create(seed, blocks, meta);
        Random random = new Random(seed);

        AbstractTreeFeature feature;
        if (tree.equals("acacia")) {
            feature = new AcaciaTreeFeature(false);
        } else if (tree.equals("bush")) {
            feature = new BushFeature(3, 0);
        } else if (tree.equals("dark_oak")) {
            feature = new DarkOakTreeFeature(false);
        } else if (tree.equals("tall_birch")) {
            feature = new BirchTreeFeature(false, true);
        } else if (tree.equals("jungle")) {
            // JungleBiome.pickTree draws the base height before place.
            feature = new TreeFeature(false, 4 + random.nextInt(7), 3, 3, true);
        } else if (tree.equals("giant_jungle")) {
            feature = new GiantJungleTreeFeature(false, 10, 20, 3, 3);
        } else if (tree.equals("giant_pine")) {
            feature = new GiantSpruceTreeFeature(false, false);
        } else if (tree.equals("giant_spruce")) {
            feature = new GiantSpruceTreeFeature(false, true);
        } else {
            throw new IllegalArgumentException(tree);
        }

        feature.prepare(1.0, 1.0, 1.0);
        boolean placed = feature.place(world, random, x, y, z);
        if (placed) {
            feature.placeSoil(world, random, x, y, z);
        }
        System.out.println("placed " + placed);

        StringBuilder out = new StringBuilder();
        for (int by = 0; by < 256; by++) {
            for (int bz = 0; bz < 16; bz++) {
                for (int bx = 0; bx < 16; bx++) {
                    Block block = world.getBlock(bx, by, bz);
                    if (block != Blocks.AIR) {
                        out.append(by * 256 + bz * 16 + bx)
                                .append(':')
                                .append(Block.getId(block))
                                .append(':')
                                .append(world.getBlockMetadata(bx, by, bz))
                                .append('\n');
                    }
                }
            }
        }
        System.out.print(out);
    }
}
