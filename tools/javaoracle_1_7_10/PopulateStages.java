import net.minecraft.block.material.Material;
import net.minecraft.block.Blocks;
import net.minecraft.world.World;
import net.minecraft.world.biome.Biome;
import net.minecraft.world.gen.feature.AbstractTreeFeature;
import net.minecraft.world.gen.feature.DeadBushFeature;
import net.minecraft.world.gen.feature.Feature;
import net.minecraft.block.FlowerBlock;
import net.minecraft.world.gen.feature.LakeFeature;
import net.minecraft.world.gen.feature.LiquidFallFeature;
import net.minecraft.world.gen.feature.PumpkinPatchFeature;
import net.minecraft.world.gen.feature.decorator.FeatureDecorator;

/**
 * Population stage by stage: prints the next two RNG longs after the requested
 * stage, so a port can compare the draw stream pass by pass.
 * Stages 0..14 are the decorator passes in vanilla order, 15 the ice/snow pass.
 */
public class PopulateStages {
    static int stop;
    static World world;
    static java.util.Random random;
    static FeatureDecorator d;

    public static void main(String[] a) throws Exception {
        long seed = Long.parseLong(a[0]);
        int cx = Integer.parseInt(a[1]);
        int cz = Integer.parseInt(a[2]);
        stop = Integer.parseInt(a[3]);

        world = WorldPrep.prepare(seed, cx, cz, 1);
        int bx = cx * 16;
        int bz = cz * 16;
        Biome biome = world.getBiome(bx + 16, bz + 16);
        random = new java.util.Random(seed);
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
            new net.minecraft.world.gen.feature.DungeonFeature().place(world, random, x, y, z);
        }

        d = biome.decorator;
        d.world = world;
        d.random = random;
        d.x = bx;
        d.z = bz;

        d.placeVeins();
        mark(0);

        for (int i = 0; i < d.sandPatchAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.sandPatch.place(world, random, x, world.getSurfaceHeight(x, z), z);
        }
        mark(1);
        for (int i = 0; i < d.clayPatchAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.clayPatch.place(world, random, x, world.getSurfaceHeight(x, z), z);
        }
        mark(2);
        for (int i = 0; i < d.gravelPatchAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.gravelPatch.place(world, random, x, world.getSurfaceHeight(x, z), z);
        }
        mark(3);

        int trees = d.treeAttempts;
        if (random.nextInt(10) == 0) trees++;
        for (int i = 0; i < trees; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = world.getHeight(x, z);
            AbstractTreeFeature tree = biome.pickTree(random);
            tree.prepare(1.0, 1.0, 1.0);
            if (tree.place(world, random, x, y, z)) {
                tree.placeSoil(world, random, x, y, z);
            }
        }
        mark(4);

        for (int i = 0; i < d.hugeMushroomAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.hugeRedMushroom.place(world, random, x, world.getHeight(x, z), z);
        }
        mark(5);

        for (int i = 0; i < d.flowerAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) + 32);
            String name = biome.getRandomFlower(random, x, y, z);
            FlowerBlock flower = FlowerBlock.getFlower(name);
            if (flower.getMaterial() != Material.AIR) {
                d.flower.set(flower, FlowerBlock.getFlowerVariant(name));
                d.flower.place(world, random, x, y, z);
            }
        }
        mark(6);

        for (int i = 0; i < d.grassAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            Feature plant = biome.pickPlant(random);
            plant.place(world, random, x, y, z);
        }
        mark(7);

        for (int i = 0; i < d.deadBushAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            new DeadBushFeature(Blocks.DEADBUSH).place(world, random, x, y, z);
        }
        mark(8);

        for (int i = 0; i < d.lilyPadAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            while (y > 0 && world.isAir(x, y - 1, z)) y--;
            d.lilyPad.place(world, random, x, y, z);
        }
        mark(9);

        for (int i = 0; i < d.mushroomAttempts; i++) {
            if (random.nextInt(4) == 0) {
                int x = d.x + random.nextInt(16) + 8;
                int z = d.z + random.nextInt(16) + 8;
                d.brownMushroom.place(world, random, x, world.getHeight(x, z), z);
            }
            if (random.nextInt(8) == 0) {
                int x = d.x + random.nextInt(16) + 8;
                int z = d.z + random.nextInt(16) + 8;
                int y = random.nextInt(world.getHeight(x, z) * 2);
                d.redMushroom.place(world, random, x, y, z);
            }
        }
        if (random.nextInt(4) == 0) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            d.brownMushroom.place(world, random, x, y, z);
        }
        if (random.nextInt(8) == 0) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            d.redMushroom.place(world, random, x, y, z);
        }
        mark(10);

        for (int i = 0; i < d.sugarcaneAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            d.sugarcane.place(world, random, x, y, z);
        }
        for (int i = 0; i < 10; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            d.sugarcane.place(world, random, x, y, z);
        }
        mark(11);

        if (random.nextInt(32) == 0) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            new PumpkinPatchFeature().place(world, random, x, y, z);
        }
        mark(12);

        for (int i = 0; i < d.cactusAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            d.cactus.place(world, random, x, y, z);
        }
        mark(13);

        if (d.placeLakes) {
            for (int i = 0; i < 50; i++) {
                int x = d.x + random.nextInt(16) + 8;
                int y = random.nextInt(random.nextInt(248) + 8);
                int z = d.z + random.nextInt(16) + 8;
                new LiquidFallFeature(Blocks.FLOWING_WATER).place(world, random, x, y, z);
            }
            for (int i = 0; i < 20; i++) {
                int x = d.x + random.nextInt(16) + 8;
                int y = random.nextInt(random.nextInt(random.nextInt(240) + 8) + 8);
                int z = d.z + random.nextInt(16) + 8;
                new LiquidFallFeature(Blocks.FLOWING_LAVA).place(world, random, x, y, z);
            }
        }
        mark(14);

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
        mark(15);
        System.out.println("end");
    }

    static void mark(int stage) {
        if (stage == stop) {
            System.out.println(stage + " " + random.nextLong() + " " + random.nextLong());
            System.exit(0);
        }
    }
}
