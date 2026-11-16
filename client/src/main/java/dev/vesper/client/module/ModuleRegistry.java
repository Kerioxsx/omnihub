package dev.vesper.client.module;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** Every module, in registration order. */
public final class ModuleRegistry {
    private static final Map<String, Module> BY_ID = new LinkedHashMap<String, Module>();

    private ModuleRegistry() {
    }

    public static <M extends Module> M register(M m) {
        if (BY_ID.containsKey(m.id)) throw new IllegalStateException("Duplicate module id " + m.id);
        BY_ID.put(m.id, m);
        return m;
    }

    public static Module get(String id) {
        return BY_ID.get(id);
    }

    public static List<Module> all() {
        return Collections.unmodifiableList(new ArrayList<Module>(BY_ID.values()));
    }

    public static List<Module> find(Category c, String query) {
        List<Module> out = new ArrayList<Module>();
        for (Module m : BY_ID.values()) if ((c == null || m.category == c) && m.matches(query)) out.add(m);
        return out;
    }
}
