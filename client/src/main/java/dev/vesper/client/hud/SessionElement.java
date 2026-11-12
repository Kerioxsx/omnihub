package dev.vesper.client.hud;

import java.util.Locale;

/** Time since you joined the current world or server. */
final class SessionElement extends LineElement {
    SessionElement() {
        super("session", "Session timer", "How long you've been in this world or on this server.", false, 0.01f, 0.25f);
    }

    @Override
    public boolean hasContent() {
        return Hud.worldJoinMs() != 0;
    }

    @Override
    protected void compute(boolean sample) {
        long join = Hud.worldJoinMs();
        long secs = sample ? 754 : join == 0 ? 0 : (System.currentTimeMillis() - join) / 1000;
        long hrs = secs / 3600, min = (secs / 60) % 60, s = secs % 60;
        label = "Session";
        value = hrs > 0 ? String.format(Locale.ROOT, "%d:%02d:%02d", hrs, min, s) : String.format(Locale.ROOT, "%d:%02d", min, s);
        valueColor = TEXT;
    }
}
