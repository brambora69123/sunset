import net.minecraft.block.Block;
import net.minecraft.block.Blocks;
import net.minecraft.block.material.Material;
import net.minecraft.world.World;
import net.minecraft.world.biome.Biome;
import net.minecraft.world.gen.feature.AbstractTreeFeature;
import net.minecraft.world.gen.feature.DeadBushFeature;
import net.minecraft.world.gen.feature.Feature;
import net.minecraft.block.FlowerBlock;
import net.minecraft.world.gen.feature.LiquidFallFeature;
import net.minecraft.world.gen.feature.PumpkinPatchFeature;
import net.minecraft.world.gen.feature.decorator.FeatureDecorator;

/**
 * Replays FeatureDecorator.decorate(biome) stage by stage, printing block
 * hashes so a port can be diffed stage by stage. Mirrors the vanilla body.
 */
public class DecorateStages {
    static FeatureDecorator d;
    static World world;
    static java.util.Random random;
    static Biome biome;

    public static void main(String[] a) throws Exception {
        long seed = Long.parseLong(a[0]);
        int bx = Integer.parseInt(a[1]);
        int bz = Integer.parseInt(a[2]);
        world = WorldPrep.prepare(seed, 0, 0, 1);
        random = new java.util.Random(seed);
        biome = world.getBiome(bx + 16, bz + 16);
        d = biome.decorator;
        d.world = world;
        d.random = random;
        d.x = bx;
        d.z = bz;

        d.placeVeins();
        System.out.println("veins " + hash());
        for (int i = 0; i < d.sandPatchAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.sandPatch.place(world, random, x, world.getSurfaceHeight(x, z), z);
        }
        System.out.println("sand " + hash());
        for (int i = 0; i < d.clayPatchAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.clayPatch.place(world, random, x, world.getSurfaceHeight(x, z), z);
        }
        System.out.println("clay " + hash());
        for (int i = 0; i < d.gravelPatchAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.gravelPatch.place(world, random, x, world.getSurfaceHeight(x, z), z);
        }
        System.out.println("gravel " + hash());

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
        System.out.println("trees " + hash());

        for (int i = 0; i < d.hugeMushroomAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            d.hugeRedMushroom.place(world, random, x, world.getHeight(x, z), z);
        }
        System.out.println("huge " + hash());

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
        System.out.println("flowers " + hash());

        for (int i = 0; i < d.grassAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            Feature plant = biome.pickPlant(random);
            int before = count(31);
            plant.place(world, random, x, y, z);
            if (System.getenv("GRASSY") != null) {
                System.out.println("grass pass " + i + " at " + x + "," + y + "," + z + " placed " + (count(31) - before));
            }
        }
        System.out.println("grass " + hash());

        for (int i = 0; i < d.deadBushAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            new DeadBushFeature(Blocks.DEADBUSH).place(world, random, x, y, z);
        }
        System.out.println("deadbush " + hash());

        for (int i = 0; i < d.lilyPadAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            while (y > 0 && world.isAir(x, y - 1, z)) y--;
            d.lilyPad.place(world, random, x, y, z);
        }
        System.out.println("lily " + hash());

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
        System.out.println("mushrooms " + hash());

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
        System.out.println("sugarcane " + hash());

        if (random.nextInt(32) == 0) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            new PumpkinPatchFeature().place(world, random, x, y, z);
        }
        System.out.println("pumpkin " + hash());

        for (int i = 0; i < d.cactusAttempts; i++) {
            int x = d.x + random.nextInt(16) + 8;
            int z = d.z + random.nextInt(16) + 8;
            int y = random.nextInt(world.getHeight(x, z) * 2);
            d.cactus.place(world, random, x, y, z);
        }
        System.out.println("cactus " + hash());

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
        System.out.println("springs " + hash());
    }

    static int count(int id) {
        int n = 0;
        int bx = d.x;
        int bz = d.z;
        for (int x = -16; x < 32; x++) for (int z = -16; z < 32; z++) for (int y = 0; y < 256; y++) {
            if (Block.REGISTRY.getId(world.getBlock(bx + x, y, bz + z)) == id) n++;
        }
        return n;
    }

    static String hash() {
        long h = 0xcbf29ce484222325L;
        int nonAir = 0;
        int bx = d.x;
        int bz = d.z;
        for (int x = 0; x < 16; x++) for (int z = 0; z < 16; z++) for (int y = 0; y < 256; y++) {
            int id = Block.REGISTRY.getId(world.getBlock(bx + 8 + x, y, bz + 8 + z));
            if (id != 0) nonAir++;
            h ^= id;
            h *= 0x100000001b3L;
        }
        return Long.toHexString(h) + "/" + nonAir;
    }
}
