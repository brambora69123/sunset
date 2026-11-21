import net.minecraft.block.Block;
import net.minecraft.block.Blocks;

/** `Block.getOpacity()` for every registered block id, the value vanilla's chunk height map tests. */
public class OpacityDump {
    public static void main(String[] a) throws Exception {
        Block.init();
        for (int id = 0; id < 256; id++) {
            Block block = Block.byId(id);
            if (block == null) {
                continue;
            }
            System.out.println(id + " " + block.getName() + " " + block.getOpacity());
        }
    }
}
