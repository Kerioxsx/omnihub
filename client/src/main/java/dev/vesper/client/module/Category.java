package dev.vesper.client.module;

public enum Category {
    COSMETICS("Cosmetics"),
    EFFECTS("Kill & hit effects"),
    HUD("HUD"),
    VISUALS("Visuals"),
    GENERAL("General");

    public final String label;

    Category(String label) {
        this.label = label;
    }
}
