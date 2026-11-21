import java.util.Map;
import java.util.TreeMap;
import net.minecraft.block.Block;
import net.minecraft.world.biome.Biome;
import net.minecraft.world.biome.source.BiomeSource;
import net.minecraft.world.gen.WorldGeneratorType;

/** Counts the biome ids a seed's biome source produces over an area. */
public class BiomeIds {
    public static void main(String[] a) {
        long seed = Long.parseLong(a[0]);
        int radiusChunks = Integer.parseInt(a[1]);
        Block.init();
        BiomeSource source = new BiomeSource(seed, WorldGeneratorType.DEFAULT);
        Map<Integer, Integer> counts = new TreeMap<>();
        for (int cz = -radiusChunks; cz <= radiusChunks; cz++) {
            for (int cx = -radiusChunks; cx <= radiusChunks; cx++) {
                Biome biome = source.getBiome(cx * 16 + 16, cz * 16 + 16);
                int id = biome == null ? -1 : biome.id;
                counts.merge(id, 1, Integer::sum);
            }
        }
        int mutated = 0;
        for (Map.Entry<Integer, Integer> e : counts.entrySet()) {
            System.out.println(e.getKey() + " " + e.getValue());
            if (e.getKey() >= 128) mutated += e.getValue();
        }
        System.out.println("mutated " + mutated);
    }
}
