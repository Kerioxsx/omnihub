package dev.vesper.client.config;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import dev.vesper.client.VesperClient;
import dev.vesper.client.hud.HudLayout;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.module.setting.Setting;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * config/vesper/config.json: named profiles, each holding every module's
 * state and the HUD layout. Versioned; a file that can't be read is set
 * aside (config.json.broken-<time>) and defaults are used, so a bad file
 * never stops the game from starting.
 */
public final class ConfigStore {
    public static final int SCHEMA = 1;
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();

    private final File dir;
    private final File file;
    private JsonObject root = new JsonObject();
    private String active = "Default";
    private boolean dirty;
    private long dirtySince;
    private String lastError;

    public ConfigStore(File dir) {
        this.dir = dir;
        this.file = new File(dir, "config.json");
    }

    public File file() {
        return file;
    }

    public File dir() {
        return dir;
    }

    public String activeProfile() {
        return active;
    }

    public String lastError() {
        return lastError;
    }

    public List<String> profiles() {
        List<String> out = new ArrayList<String>();
        JsonObject p = profilesObj();
        for (Map.Entry<String, JsonElement> e : p.entrySet()) out.add(e.getKey());
        if (!out.contains(active)) out.add(0, active);
        return out;
    }

    /** Reads the file (call after modules are registered to apply them). */
    public void load() {
        root = new JsonObject();
        if (file.isFile()) {
            try {
                String text = new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8);
                JsonElement e = new JsonParser().parse(text);
                if (!e.isJsonObject()) throw new IOException("not a JSON object");
                root = migrate(e.getAsJsonObject());
            } catch (Exception ex) {
                lastError = "Settings file couldn't be read (" + ex.getMessage() + "); defaults are in use.";
                VesperClient.LOG.warn(lastError);
                setAside();
                root = new JsonObject();
            }
        }
        if (root.has("activeProfile") && root.get("activeProfile").isJsonPrimitive()) active = root.get("activeProfile").getAsString();
    }

    private JsonObject migrate(JsonObject o) {
        int v = o.has("schemaVersion") && o.get("schemaVersion").isJsonPrimitive() ? o.get("schemaVersion").getAsInt() : 0;
        if (v > SCHEMA) VesperClient.LOG.warn("Settings come from a newer Vesper ({}); unknown parts are kept as they are.", v);
        // Version 0 (none yet) and 1 share the same shape.
        o.addProperty("schemaVersion", SCHEMA);
        return o;
    }

    private void setAside() {
        try {
            Files.move(file.toPath(), new File(dir, "config.json.broken-" + System.currentTimeMillis()).toPath(), StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException ignored) {
        }
    }

    private JsonObject profilesObj() {
        if (!root.has("profiles") || !root.get("profiles").isJsonObject()) root.add("profiles", new JsonObject());
        return root.getAsJsonObject("profiles");
    }

    /** Puts the active profile's stored values into the modules and HUD. */
    public void apply() {
        JsonObject p = profilesObj().has(active) && profilesObj().get(active).isJsonObject() ? profilesObj().getAsJsonObject(active) : new JsonObject();
        JsonObject mods = p.has("modules") && p.get("modules").isJsonObject() ? p.getAsJsonObject("modules") : new JsonObject();
        for (Module m : ModuleRegistry.all()) {
            m.resetAll();
            if (!mods.has(m.id) || !mods.get(m.id).isJsonObject()) continue;
            JsonObject mo = mods.getAsJsonObject(m.id);
            try {
                if (mo.has("enabled")) m.setEnabled(mo.get("enabled").getAsBoolean());
                if (mo.has("key")) m.toggleKey.fromJson(mo.get("key"));
                JsonObject s = mo.has("settings") && mo.get("settings").isJsonObject() ? mo.getAsJsonObject("settings") : new JsonObject();
                for (Setting<?> st : m.settings()) if (s.has(st.id)) st.fromJson(s.get(st.id));
            } catch (Exception ex) {
                VesperClient.LOG.warn("Settings for {} couldn't be read; using defaults", m.id);
            }
        }
        HudLayout.fromJson(p.has("hud") && p.get("hud").isJsonObject() ? p.getAsJsonObject("hud") : null);
    }

    private JsonObject snapshot() {
        JsonObject p = new JsonObject();
        JsonObject mods = new JsonObject();
        for (Module m : ModuleRegistry.all()) {
            JsonObject mo = new JsonObject();
            mo.addProperty("enabled", m.enabled());
            mo.add("key", m.toggleKey.toJson());
            JsonObject s = new JsonObject();
            for (Setting<?> st : m.settings()) s.add(st.id, st.toJson());
            mo.add("settings", s);
            mods.add(m.id, mo);
        }
        p.add("modules", mods);
        p.add("hud", HudLayout.toJson());
        return p;
    }

    public void markDirty() {
        if (!dirty) dirtySince = System.currentTimeMillis();
        dirty = true;
    }

    /** Saves a second after the last change (called every tick). */
    public void tick() {
        if (dirty && System.currentTimeMillis() - dirtySince > 1000) save();
    }

    public void save() {
        dirty = false;
        profilesObj().add(active, snapshot());
        root.addProperty("schemaVersion", SCHEMA);
        root.addProperty("activeProfile", active);
        try {
            dir.mkdirs();
            File tmp = new File(dir, "config.json.tmp");
            Files.write(tmp.toPath(), GSON.toJson(root).getBytes(StandardCharsets.UTF_8));
            try {
                Files.move(tmp.toPath(), file.toPath(), StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
            } catch (IOException atomicFailed) {
                Files.move(tmp.toPath(), file.toPath(), StandardCopyOption.REPLACE_EXISTING);
            }
            lastError = null;
        } catch (IOException ex) {
            lastError = "Settings couldn't be saved: " + ex.getMessage();
            VesperClient.LOG.warn(lastError);
        }
    }

    /** Saves the current profile, then switches (a new name starts as a copy). */
    public void switchProfile(String name) {
        if (name == null || name.trim().isEmpty()) return;
        save();
        if (!profilesObj().has(name)) profilesObj().add(name, snapshot());
        active = name;
        apply();
        save();
    }

    public void deleteProfile(String name) {
        if (name.equals(active)) return;
        profilesObj().remove(name);
        save();
    }

    /** Every module and the HUD back to defaults (this profile only). */
    public void resetProfile() {
        for (Module m : ModuleRegistry.all()) m.resetAll();
        HudLayout.fromJson(null);
        save();
    }
}
