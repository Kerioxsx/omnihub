package dev.vesper.client.module.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;
import java.util.Arrays;
import java.util.List;

/** One of a fixed list of ids, each with a label. */
public final class ChoiceSetting extends Setting<String> {
    public final List<String> ids;
    public final List<String> labels;

    public ChoiceSetting(String id, String name, String def, String[] ids, String[] labels) {
        super(id, name, def);
        this.ids = Arrays.asList(ids);
        this.labels = Arrays.asList(labels);
    }

    public int index() {
        return Math.max(0, ids.indexOf(value));
    }

    public String label() {
        return labels.get(index());
    }

    public void cycle(int dir) {
        int n = ids.size();
        value = ids.get(((index() + dir) % n + n) % n);
    }

    public boolean is(String id) {
        return value.equals(id);
    }

    @Override
    protected String sanitize(String v) {
        return v != null && ids.contains(v) ? v : defaultValue;
    }

    @Override
    public JsonElement toJson() {
        return new JsonPrimitive(value);
    }

    @Override
    public void fromJson(JsonElement e) {
        if (e != null && e.isJsonPrimitive()) set(e.getAsString());
    }
}
