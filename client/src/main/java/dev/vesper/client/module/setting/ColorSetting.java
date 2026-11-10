package dev.vesper.client.module.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;

/** An ARGB colour, stored as "#AARRGGBB". */
public final class ColorSetting extends Setting<Integer> {
    public ColorSetting(String id, String name, int argb) {
        super(id, name, argb);
    }

    public int argb() {
        return value;
    }

    public int rgb() {
        return value & 0xFFFFFF;
    }

    @Override
    public JsonElement toJson() {
        return new JsonPrimitive(String.format("#%08X", value));
    }

    @Override
    public void fromJson(JsonElement e) {
        if (e == null || !e.isJsonPrimitive()) return;
        String s = e.getAsString().trim().replace("#", "");
        try {
            if (s.length() == 6) value = 0xFF000000 | Integer.parseInt(s, 16);
            else if (s.length() == 8) value = (int) Long.parseLong(s, 16);
        } catch (NumberFormatException ignored) {
        }
    }
}
