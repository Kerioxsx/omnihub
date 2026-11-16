package dev.vesper.client.module.setting;

import com.google.gson.JsonElement;
import com.google.gson.JsonPrimitive;

/** A slider value between min and max, snapped to step. */
public final class NumberSetting extends Setting<Double> {
    public final double min, max, step;
    public final String unit;

    public NumberSetting(String id, String name, double def, double min, double max, double step, String unit) {
        super(id, name, def);
        this.min = min;
        this.max = max;
        this.step = step;
        this.unit = unit;
    }

    public float f() {
        return value.floatValue();
    }

    public int i() {
        return (int) Math.round(value);
    }

    @Override
    protected Double sanitize(Double v) {
        if (v == null || v.isNaN() || v.isInfinite()) return defaultValue;
        double s = step > 0 ? Math.round((v - min) / step) * step + min : v;
        return Math.max(min, Math.min(max, s));
    }

    public String display() {
        String n = step >= 1 ? String.valueOf((long) Math.round(value)) : String.format(java.util.Locale.ROOT, step >= 0.1 ? "%.1f" : "%.2f", value);
        return unit.isEmpty() ? n : n + unit;
    }

    @Override
    public JsonElement toJson() {
        return new JsonPrimitive(value);
    }

    @Override
    public void fromJson(JsonElement e) {
        if (e != null && e.isJsonPrimitive() && e.getAsJsonPrimitive().isNumber()) set(e.getAsDouble());
    }
}
