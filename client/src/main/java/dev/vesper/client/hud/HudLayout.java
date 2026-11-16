package dev.vesper.client.hud;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Where each HUD element sits: position as a fraction of the screen (so it
 * survives resolution and GUI-scale changes), scale and visibility.
 */
public final class HudLayout {
    public static final class Place {
        public float x, y, scale;
        public boolean shown;

        Place(float x, float y, float scale, boolean shown) {
            this.x = x;
            this.y = y;
            this.scale = scale;
            this.shown = shown;
        }
    }

    private static final Map<String, Place> DEFAULTS = new LinkedHashMap<String, Place>();
    private static final Map<String, Place> PLACES = new LinkedHashMap<String, Place>();

    private HudLayout() {
    }

    /** Called by each element when it registers. */
    public static void define(String id, float x, float y, float scale, boolean shown) {
        DEFAULTS.put(id, new Place(x, y, scale, shown));
        if (!PLACES.containsKey(id)) PLACES.put(id, new Place(x, y, scale, shown));
    }

    public static Place get(String id) {
        Place p = PLACES.get(id);
        if (p == null) {
            Place d = DEFAULTS.get(id);
            p = d == null ? new Place(0.02f, 0.02f, 1f, true) : new Place(d.x, d.y, d.scale, d.shown);
            PLACES.put(id, p);
        }
        return p;
    }

    public static void reset(String id) {
        Place d = DEFAULTS.get(id);
        if (d != null) PLACES.put(id, new Place(d.x, d.y, d.scale, d.shown));
    }

    public static JsonObject toJson() {
        JsonObject o = new JsonObject();
        for (Map.Entry<String, Place> e : PLACES.entrySet()) {
            JsonObject p = new JsonObject();
            p.addProperty("x", e.getValue().x);
            p.addProperty("y", e.getValue().y);
            p.addProperty("scale", e.getValue().scale);
            p.addProperty("shown", e.getValue().shown);
            o.add(e.getKey(), p);
        }
        return o;
    }

    public static void fromJson(JsonObject o) {
        PLACES.clear();
        for (Map.Entry<String, Place> d : DEFAULTS.entrySet()) PLACES.put(d.getKey(), new Place(d.getValue().x, d.getValue().y, d.getValue().scale, d.getValue().shown));
        if (o == null) return;
        for (Map.Entry<String, JsonElement> e : o.entrySet()) {
            if (!e.getValue().isJsonObject()) continue;
            JsonObject p = e.getValue().getAsJsonObject();
            try {
                Place pl = get(e.getKey());
                pl.x = clamp(p.get("x").getAsFloat(), 0, 1);
                pl.y = clamp(p.get("y").getAsFloat(), 0, 1);
                pl.scale = clamp(p.get("scale").getAsFloat(), 0.5f, 3f);
                pl.shown = p.get("shown").getAsBoolean();
            } catch (Exception ignored) {
                reset(e.getKey());
            }
        }
    }

    private static float clamp(float v, float lo, float hi) {
        return Float.isNaN(v) ? lo : Math.max(lo, Math.min(hi, v));
    }
}
