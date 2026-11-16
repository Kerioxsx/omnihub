package dev.vesper.client.module.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;
import org.lwjgl.input.Keyboard;

/** A keyboard key (LWJGL key code; 0 = none). */
public final class KeySetting extends Setting<Integer> {
    public KeySetting(String id, String name, int def) {
        super(id, name, def);
    }

    public int key() {
        return value;
    }

    public String keyName() {
        if (value <= 0) return "None";
        String n = Keyboard.getKeyName(value);
        return n == null ? "Key " + value : n;
    }

    @Override
    public JsonElement toJson() {
        return new JsonPrimitive(value);
    }

    @Override
    public void fromJson(JsonElement e) {
        if (e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber()) value = Math.max(0, e.getAsInt());
    }
}
