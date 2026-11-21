#!/bin/sh
# Builds the 1.7.10 Java oracle for parity probes.
#
# The mapped 1.7.10 jar keeps `protected` members that were legal in the
# obfuscated default package, so every protected field and method is widened to
# public first (`Widen.java`), then the probes compile against that jar.
#
# Outputs (all under .local/, never committed):
#   .local/tools/1.7.10/server-public.jar, client-public.jar
#   .local/tools/1.7.10/oracle/*.class
#
# Run a probe with tools/javaoracle_1_7_10/run.sh <Class> <args...>.
set -eu
cd "$(dirname "$0")/../.."

NAMED=.local/minecraft/1.7.10/named
OUT=.local/tools/1.7.10
CLASSES="$OUT/oracle"

if [ ! -f "$NAMED/server.jar" ] || [ ! -f "$NAMED/client.jar" ]; then
    echo "missing $NAMED/{server,client}.jar; prepare the 1.7.10 sources first" >&2
    exit 1
fi

mkdir -p "$OUT" "$CLASSES"

# Widen needs the JDK's internal ASM, which lives in java.base.
javac --add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED \
    -d "$OUT" tools/javaoracle_1_7_10/Widen.java
java --add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED \
    -cp "$OUT" Widen "$NAMED/server.jar" "$OUT/server-public.jar"
java --add-exports java.base/jdk.internal.org.objectweb.asm=ALL-UNNAMED \
    -cp "$OUT" Widen "$NAMED/client.jar" "$OUT/client-public.jar"

# The 1.7.10 jars are trimmed of their libraries. Initialising `Minecraft`
# (which the renderer does to read `options.ambientOcclusion`) touches a few of
# them, so compile stand-ins for just those types into the oracle classpath.
# None of them takes part in block tessellation.
STUBS="$OUT/stub-src"
mkdir -p "$STUBS/org/apache/logging/log4j" "$STUBS/com/google/common/collect" "$STUBS/com/google/gson" \
    "$STUBS/org/lwjgl/opengl" "$STUBS/org/lwjgl"
cat > "$STUBS/org/apache/logging/log4j/Logger.java" <<'EOF'
package org.apache.logging.log4j;

public interface Logger {
}
EOF
cat > "$STUBS/org/apache/logging/log4j/LogManager.java" <<'EOF'
package org.apache.logging.log4j;

public class LogManager {
    public static Logger getLogger() {
        return null;
    }
}
EOF
cat > "$STUBS/com/google/common/collect/Lists.java" <<'EOF'
package com.google.common.collect;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Iterator;

public class Lists {
    public static <E> ArrayList<E> newArrayList() {
        return new ArrayList<E>();
    }

    @SafeVarargs
    public static <E> ArrayList<E> newArrayList(E... elements) {
        return new ArrayList<E>(Arrays.asList(elements));
    }

    public static <E> ArrayList<E> newArrayList(Iterable<? extends E> elements) {
        ArrayList<E> list = new ArrayList<E>();
        for (E element : elements) {
            list.add(element);
        }
        return list;
    }

    public static <E> ArrayList<E> newArrayList(Iterator<? extends E> elements) {
        ArrayList<E> list = new ArrayList<E>();
        while (elements.hasNext()) {
            list.add(elements.next());
        }
        return list;
    }
}
EOF
cat > "$STUBS/com/google/common/collect/Maps.java" <<'EOF'
package com.google.common.collect;

import java.util.EnumMap;
import java.util.HashMap;
import java.util.Map;

public class Maps {
    public static <K, V> HashMap<K, V> newHashMap() {
        return new HashMap<K, V>();
    }

    public static <K extends Enum<K>, V> EnumMap<K, V> newEnumMap(Class<K> type) {
        return new EnumMap<K, V>(type);
    }
}
EOF
cat > "$STUBS/com/google/gson/Gson.java" <<'EOF'
package com.google.gson;

public class Gson {
    public Gson() {
    }
}
EOF
cat > "$STUBS/org/lwjgl/LWJGLException.java" <<'EOF'
package org.lwjgl;

public class LWJGLException extends Exception {
}
EOF
cat > "$STUBS/org/lwjgl/opengl/DisplayMode.java" <<'EOF'
package org.lwjgl.opengl;

public class DisplayMode {
    public DisplayMode(int width, int height) {
    }
}
EOF
cat > "$STUBS/org/lwjgl/opengl/OpenGLException.java" <<'EOF'
package org.lwjgl.opengl;

public class OpenGLException extends RuntimeException {
}
EOF
javac -d "$CLASSES" $(find "$STUBS" -name '*.java')

# The world and generator probes resolve server classes first, as before;
# RenderOracle needs the client's Block/BlockRenderer classes first.
javac -cp "$OUT/server-public.jar:$OUT/client-public.jar:$CLASSES" -d "$CLASSES" \
    $(ls tools/javaoracle_1_7_10/*.java | grep -v "/Widen.java$" | grep -v "/RenderOracle.java$")
javac -cp "$OUT/client-public.jar:$OUT/server-public.jar:$CLASSES" -d "$CLASSES" \
    tools/javaoracle_1_7_10/RenderOracle.java
echo "oracle built in $CLASSES"
