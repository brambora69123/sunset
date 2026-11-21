import java.lang.reflect.Method;
import java.util.Random;
import net.minecraft.block.Block;
import net.minecraft.world.World;
import net.minecraft.world.biome.Biome;
import net.minecraft.world.biome.MutatedBiome;

/**
 * The decoration facts of every registered 1.7.10 biome, id by id: the
 * decorator's attempt counts, where its `decorate`/`pickTree`/`pickPlant`/
 * `getRandomFlower` are declared, and the surface blocks.
 */
public class BiomeFacts {
    public static void main(String[] a) throws Exception {
        Block.init();
        for (int id = 0; id < 256; id++) {
            Biome biome = Biome.byId(id);
            if (biome == null || biome.id != id) {
                continue;
            }
            StringBuilder sb = new StringBuilder();
            sb.append(id).append(" class=").append(biome.getClass().getSimpleName());
            if (biome instanceof MutatedBiome) {
                sb.append(" original=").append(((MutatedBiome) biome).original.id);
            }
            sb.append(" tree=").append(biome.decorator.treeAttempts);
            sb.append(" flower=").append(biome.decorator.flowerAttempts);
            sb.append(" grass=").append(biome.decorator.grassAttempts);
            sb.append(" dead=").append(biome.decorator.deadBushAttempts);
            sb.append(" mushroom=").append(biome.decorator.mushroomAttempts);
            sb.append(" huge=").append(biome.decorator.hugeMushroomAttempts);
            sb.append(" cane=").append(biome.decorator.sugarcaneAttempts);
            sb.append(" cactus=").append(biome.decorator.cactusAttempts);
            sb.append(" lily=").append(biome.decorator.lilyPadAttempts);
            sb.append(" sand=").append(biome.decorator.sandPatchAttempts);
            sb.append(" clay=").append(biome.decorator.clayPatchAttempts);
            sb.append(" gravel=").append(biome.decorator.gravelPatchAttempts);
            sb.append(" lakes=").append(biome.decorator.placeLakes);
            sb.append(" temp=").append(biome.temperature);
            sb.append(" surface=").append(Block.REGISTRY.getId(biome.surfaceBlock));
            sb.append(" subsurface=").append(Block.REGISTRY.getId(biome.subsurfaceBlock));
            sb.append(" decorate=").append(declares(biome, "decorate", World.class, Random.class, int.class, int.class));
            sb.append(" pickTree=").append(declares(biome, "pickTree", Random.class));
            sb.append(" pickPlant=").append(declares(biome, "pickPlant", Random.class));
            sb.append(" randomFlower=").append(declares(biome, "getRandomFlower", Random.class, int.class, int.class, int.class));
            System.out.println(sb);
        }
    }

    static String declares(Biome biome, String name, Class<?>... args) {
        Class<?> c = biome.getClass();
        while (c != null) {
            try {
                Method m = c.getDeclaredMethod(name, args);
                return m.getDeclaringClass().getSimpleName();
            } catch (NoSuchMethodException e) {
                c = c.getSuperclass();
            }
        }
        return "none";
    }
}
