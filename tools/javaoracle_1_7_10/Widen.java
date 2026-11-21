import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.util.Enumeration;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import java.util.jar.JarOutputStream;
import jdk.internal.org.objectweb.asm.ClassReader;
import jdk.internal.org.objectweb.asm.ClassVisitor;
import jdk.internal.org.objectweb.asm.ClassWriter;
import jdk.internal.org.objectweb.asm.FieldVisitor;
import jdk.internal.org.objectweb.asm.MethodVisitor;
import jdk.internal.org.objectweb.asm.Opcodes;

/**
 * Widens every protected field/method in a mapped 1.7.10 jar to public.
 * The obfuscated jar keeps all classes in one package, so protected access
 * across what calamus splits into different packages is legal there but not
 * after remapping.
 */
public final class Widen {
    public static void main(String[] args) throws IOException {
        try (JarFile in = new JarFile(args[0]);
             JarOutputStream out = new JarOutputStream(new FileOutputStream(args[1]))) {
            Enumeration<JarEntry> entries = in.entries();
            while (entries.hasMoreElements()) {
                JarEntry entry = entries.nextElement();
                byte[] bytes = in.getInputStream(entry).readAllBytes();
                if (entry.getName().endsWith(".class")) {
                    ClassReader reader = new ClassReader(bytes);
                    ClassWriter writer = new ClassWriter(reader, 0);
                    reader.accept(new ClassVisitor(Opcodes.ASM9, writer) {
                        @Override
                        public FieldVisitor visitField(int access, String name, String desc,
                                                       String sig, Object value) {
                            if ((access & Opcodes.ACC_PROTECTED) != 0) {
                                access = (access & ~Opcodes.ACC_PROTECTED) | Opcodes.ACC_PUBLIC;
                            }
                            return super.visitField(access, name, desc, sig, value);
                        }

                        @Override
                        public MethodVisitor visitMethod(int access, String name, String desc,
                                                         String sig, String[] exceptions) {
                            if ((access & Opcodes.ACC_PROTECTED) != 0) {
                                access = (access & ~Opcodes.ACC_PROTECTED) | Opcodes.ACC_PUBLIC;
                            }
                            return super.visitMethod(access, name, desc, sig, exceptions);
                        }
                    }, 0);
                    bytes = writer.toByteArray();
                }
                out.putNextEntry(new JarEntry(entry.getName()));
                out.write(bytes);
                out.closeEntry();
            }
        }
    }
}
