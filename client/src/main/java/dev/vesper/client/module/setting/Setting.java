package dev.vesper.client.module.setting;

import com.google.gson.JsonElement;

/** One typed, persisted option of a module. */
public abstract class Setting<T> {
    public final String id;
    public final String name;
    public final T defaultValue;
    protected T value;
    /** Shown only when this returns true (e.g. depends on another option). */
    private java.util.function.BooleanSupplier visible = () -> true;

    protected Setting(String id, String name, T defaultValue) {
        this.id = id;
        this.name = name;
        this.defaultValue = defaultValue;
        this.value = defaultValue;
    }

    public T get() {
        return value;
    }

    public void set(T v) {
        value = sanitize(v);
    }

    public void reset() {
        value = defaultValue;
    }

    public boolean isDefault() {
        return value.equals(defaultValue);
    }

    public Setting<T> visibleWhen(java.util.function.BooleanSupplier s) {
        visible = s;
        return this;
    }

    public boolean visible() {
        return visible.getAsBoolean();
    }

    protected T sanitize(T v) {
        return v == null ? defaultValue : v;
    }

    public abstract JsonElement toJson();

    /** Reads a stored value; anything unreadable leaves the default. */
    public abstract void fromJson(JsonElement e);
}
