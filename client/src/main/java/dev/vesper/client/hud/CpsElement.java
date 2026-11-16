package dev.vesper.client.hud;

/** Your own clicks per second (left · right) over the last second. Display only. */
final class CpsElement extends LineElement {
    CpsElement() {
        super("cps", "CPS", "Your clicks per second over the last second (left · right). Display only.", false, 0.01f, 0.20f);
    }

    @Override
    public boolean hasContent() {
        return true;
    }

    @Override
    protected long throttleMs() {
        return 50;
    }

    @Override
    protected void compute(boolean sample) {
        int l = sample ? 7 : Hud.cps(0), r = sample ? 2 : Hud.cps(1);
        label = "CPS";
        value = l + " · " + r;
        valueColor = TEXT;
    }
}
