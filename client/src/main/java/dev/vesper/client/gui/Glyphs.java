package dev.vesper.client.gui;

import dev.vesper.client.render.Draw;

/** Small drawn icons for the menus (one stroke weight, no font glyphs). */
final class Glyphs {
    private Glyphs() {
    }

    /** A magnifying glass centred at (cx, cy), radius r. */
    static void search(float cx, float cy, float r, int c) {
        Draw.ring(cx - r * 0.15f, cy - r * 0.15f, r * 0.55f, r * 0.8f, c, c);
        Draw.line(cx + r * 0.42f, cy + r * 0.42f, cx + r * 0.95f, cy + r * 0.95f, Math.max(1.2f, r * 0.28f), c);
    }
}
