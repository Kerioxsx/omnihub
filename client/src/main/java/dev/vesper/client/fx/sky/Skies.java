package dev.vesper.client.fx.sky;

/**
 * Procedural skies (owner: vfx-engineer). Ids: "nocturne", "bloodmoon",
 * "riftstorm", "inferno", "aurora".
 */
public final class Skies {
    private Skies() {
    }

    public static final String[] IDS = {"nocturne", "bloodmoon", "riftstorm", "inferno", "aurora"};
    public static final String[] NAMES = {"Nocturne", "Bloodmoon", "Riftstorm", "Inferno", "Aurora Veil"};

    /**
     * Fills the given GUI rectangle with the sky as a 2D backdrop (menus,
     * showroom background). Must restore GL state.
     */
    public static void drawBackdrop(String id, float x, float y, float w, float h, float time) {
    }

    /**
     * Draws the sky as a camera-centred dome in the current 3D view (showroom
     * stage, and the world when the user opts in). Must restore GL state.
     */
    public static void drawDome(String id, float time, float alpha) {
    }
}
