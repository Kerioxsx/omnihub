package dev.vesper.client.preview;

/**
 * Draws the user's own skin on a player model, with the equipped
 * cosmetics, inside a GUI (owner: cosmetics-animator). No world needed.
 */
public final class PlayerPreview {
    private PlayerPreview() {
    }

    /**
     * @param feetX GUI x of the feet
     * @param feetY GUI y of the feet
     * @param height model height in GUI units
     * @param yaw body yaw in degrees (0 = facing the viewer)
     * @param alpha 0..1 fade
     * @param time seconds, for animation
     */
    public static void draw(float feetX, float feetY, float height, float yaw, float alpha, float time) {
    }
}
