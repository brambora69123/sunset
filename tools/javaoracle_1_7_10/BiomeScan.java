import net.minecraft.world.biome.Biome;
import net.minecraft.world.biome.source.BiomeSource;
import net.minecraft.world.gen.WorldGeneratorType;

/**
 * Biome ids on a chunk grid: the decorator's id (`getBiome(bx + 16, bz + 16)`)
 * and the chunk centre's, per chunk.
 */
public class BiomeScan {
    public static void main(String[] a) throws Exception {
        long seed = Long.parseLong(a[0]);
        int x0 = Integer.parseInt(a[1]);
        int z0 = Integer.parseInt(a[2]);
        int x1 = Integer.parseInt(a[3]);
        int z1 = Integer.parseInt(a[4]);

        BiomeSource source = new BiomeSource(seed, WorldGeneratorType.DEFAULT);
        for (int cx = x0; cx <= x1; cx++) {
            for (int cz = z0; cz <= z1; cz++) {
                Biome decorator = source.getBiome(cx * 16 + 16, cz * 16 + 16);
                Biome centre = source.getBiome(cx * 16 + 8, cz * 16 + 8);
                System.out.println(decorator.id + " " + cx + " " + cz + " " + centre.id);
            }
        }
    }
}
