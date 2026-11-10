package dev.vesper.client.fx;

/**
 * The kill-effect library (owner: vfx-engineer). Ids: "astral_collapse",
 * "violet_rift", "solar_bloom", "frost_constellation", "comet_ascension".
 */
public final class KillEffects {
    private KillEffects() {
    }

    public static final String[] IDS = {"astral_collapse", "violet_rift", "solar_bloom", "frost_constellation", "comet_ascension"};
    public static final String[] NAMES = {"Astral Collapse", "Violet Rift", "Solar Bloom", "Frost Constellation", "Comet Ascension"};

    /**
     * Plays an effect at a world position as a local DEMO (showroom test
     * button). In the showroom the "world" is the stage's own coordinate
     * space; see {@link #renderAt}.
     */
    public static void demo(String id, double x, double y, double z) {
    }

    /**
     * Renders running demo effects whose origin is relative to the current
     * matrix (used by the showroom, which has no world). Must restore GL state.
     */
    public static void renderStage(float partialTicks) {
    }

    /** Placeholder kept for API symmetry; world effects render from Fx's own world-render handler. */
    public static void renderAt(float partialTicks) {
    }
}
