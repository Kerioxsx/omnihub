package dev.vesper.client.module;

import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.ColorSetting;
import dev.vesper.client.module.setting.KeySetting;
import dev.vesper.client.module.setting.NumberSetting;
import dev.vesper.client.module.setting.Setting;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

/**
 * A feature the user can switch on and off and tune. Modules are purely
 * visual / informational; none of them change how the game plays.
 */
public abstract class Module {
    public final String id;
    public final String name;
    public final String description;
    public final Category category;
    public final boolean enabledByDefault;
    /** Fine on public servers by default (false: needs the user to opt in). */
    public final boolean publicSafe;
    private boolean enabled;
    private final List<Setting<?>> settings = new ArrayList<Setting<?>>();
    public final KeySetting toggleKey;

    protected Module(String id, String name, String description, Category category, boolean enabledByDefault, boolean publicSafe) {
        this.id = id;
        this.name = name;
        this.description = description;
        this.category = category;
        this.enabledByDefault = enabledByDefault;
        this.publicSafe = publicSafe;
        this.enabled = enabledByDefault;
        this.toggleKey = new KeySetting("key", "Toggle key", 0);
    }

    public boolean enabled() {
        return enabled;
    }

    public void setEnabled(boolean on) {
        if (on == enabled) return;
        enabled = on;
        if (on) onEnable();
        else onDisable();
    }

    public void toggle() {
        setEnabled(!enabled);
    }

    protected void onEnable() {
    }

    protected void onDisable() {
    }

    public List<Setting<?>> settings() {
        return Collections.unmodifiableList(settings);
    }

    public Setting<?> setting(String id) {
        for (Setting<?> s : settings) if (s.id.equals(id)) return s;
        return null;
    }

    public void resetAll() {
        for (Setting<?> s : settings) s.reset();
        toggleKey.reset();
        setEnabled(enabledByDefault);
    }

    protected <S extends Setting<?>> S add(S s) {
        settings.add(s);
        return s;
    }

    protected BoolSetting bool(String id, String name, boolean def) {
        return add(new BoolSetting(id, name, def));
    }

    protected NumberSetting number(String id, String name, double def, double min, double max, double step, String unit) {
        return add(new NumberSetting(id, name, def, min, max, step, unit));
    }

    protected ColorSetting color(String id, String name, int argb) {
        return add(new ColorSetting(id, name, argb));
    }

    protected ChoiceSetting choice(String id, String name, String def, String[] ids, String[] labels) {
        return add(new ChoiceSetting(id, name, def, ids, labels));
    }

    /** Text matched by the settings search. */
    public boolean matches(String q) {
        if (q == null || q.isEmpty()) return true;
        String s = q.toLowerCase(java.util.Locale.ROOT);
        if (name.toLowerCase(java.util.Locale.ROOT).contains(s) || description.toLowerCase(java.util.Locale.ROOT).contains(s)) return true;
        for (Setting<?> st : settings) if (st.name.toLowerCase(java.util.Locale.ROOT).contains(s)) return true;
        return false;
    }
}
