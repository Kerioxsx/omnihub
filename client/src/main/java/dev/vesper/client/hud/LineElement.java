package dev.vesper.client.hud;

/** A small panel with one line: a muted label and a value (FPS, ping, CPS, session...). */
abstract class LineElement extends HudElement {
    static final float SIZE = 7.5f, PAD = 5, HEIGHT = 13;

    protected String label = "", value = "";
    protected int valueColor = TEXT;
    private float labelW;

    LineElement(String id, String name, String description, boolean enabledByDefault, float x, float y) {
        super(id, name, description, enabledByDefault, true, x, y, 1f);
    }

    /** Fills label/value/valueColor. Called at most every {@link #throttleMs()}. */
    protected abstract void compute(boolean sample);

    protected long throttleMs() {
        return 250;
    }

    @Override
    public void refresh(boolean sample) {
        if (!due(throttleMs(), sample)) return;
        compute(sample);
        labelW = label.isEmpty() ? 0 : textW(label, SIZE, false) + 3;
        w = PAD * 2 + labelW + textW(value, SIZE, true);
        h = HEIGHT;
    }

    @Override
    protected void paint(boolean sample) {
        panel(0, 0, w, h);
        if (!label.isEmpty()) textMid(label, PAD, h / 2f, SIZE, false, TEXT_MUTED);
        textMid(value, PAD + labelW, h / 2f, SIZE, true, valueColor);
    }
}
