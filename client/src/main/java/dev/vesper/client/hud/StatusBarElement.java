package dev.vesper.client.hud;

import dev.vesper.client.module.General;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.render.Ease;
import java.text.SimpleDateFormat;
import java.util.Calendar;
import java.util.Date;
import java.util.Locale;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ServerData;

/**
 * Top status bar (VISUAL_SPEC §6.1): sigil tile, then a pill with the server,
 * FPS, ping (multiplayer only) and the local clock. All values are real.
 */
final class StatusBarElement extends HudElement {
    private static final int ICON_GLOBE = 0, ICON_GAUGE = 1, ICON_SIGNAL = 2, ICON_CLOCK = 3;
    private static final float TILE = 14, GAP = 3, BAR_H = 14, PAD = 6, ICON = 7, ICON_GAP = 3, DIV = 11, TEXT_SIZE = 7f;
    private static final int SIGIL = 0xFFF2F3F5;

    // Sigil unit points (VISUAL_SPEC §3), y up.
    private static final double[] RX = {0.53, 0.58, 0.86, 0.58, 0.53}, RY = {0.95, 0.56, 0.50, 0.44, 0.09};
    private static final double[] LX = {0.47, 0.42, 0.14, 0.42, 0.47}, LY = {0.91, 0.52, 0.46, 0.40, 0.05};
    private static final double[] DX = {0.80, 0.85, 0.80, 0.75, 0.80}, DY = {0.88, 0.82, 0.76, 0.82, 0.88};

    final BoolSetting showServer = bool("server", "Server address", true);
    final BoolSetting showFps = bool("fps", "FPS", true);
    final BoolSetting showPing = bool("ping", "Ping (multiplayer)", true);
    final BoolSetting showClock = bool("clock", "Clock", true);
    final BoolSetting clock24 = bool("clock24", "24-hour clock", true);

    private final SimpleDateFormat fmt24 = new SimpleDateFormat("HH:mm", Locale.ROOT);
    private final SimpleDateFormat fmt12 = new SimpleDateFormat("h:mm a", Locale.ENGLISH);
    private final Date date = new Date();

    private int n;
    private final int[] icon = new int[4], valueColor = new int[4];
    private final String[] value = new String[4], tail = new String[4];
    private final float[] slotW = new float[4];
    private float barW;
    private int fps, ping = -1, hour, minute;
    private final double[] bx = new double[40], by = new double[40];

    StatusBarElement() {
        super("statusBar", "Status bar", "Top bar with the Vesper sigil, server, FPS, ping and clock.", true, true, 6 / 480f, 6 / 270f, 1f);
    }

    @Override
    public boolean hasContent() {
        return mc.theWorld != null;
    }

    @Override
    public void refresh(boolean sample) {
        if (!due(250, sample)) return;
        n = 0;
        if (showServer.on()) {
            String s = sample ? "play.example.net" : serverText();
            if (s != null) slot(ICON_GLOBE, s, "", TEXT);
        }
        if (showFps.on()) {
            fps = sample ? 144 : Minecraft.getDebugFPS();
            slot(ICON_GAUGE, String.valueOf(fps), " fps", fps >= 120 ? GOOD : fps >= 60 ? TEXT : BAD);
        }
        if (showPing.on()) {
            ping = sample ? 23 : PingElement.ping();
            if (ping >= 0) slot(ICON_SIGNAL, String.valueOf(ping), " ms", TEXT);
        }
        if (showClock.on()) {
            date.setTime(System.currentTimeMillis());
            Calendar c = Calendar.getInstance();
            hour = c.get(Calendar.HOUR_OF_DAY);
            minute = c.get(Calendar.MINUTE);
            slot(ICON_CLOCK, (clock24.on() ? fmt24 : fmt12).format(date), "", TEXT);
        }
        barW = PAD * 2;
        for (int i = 0; i < n; i++) {
            slotW[i] = ICON + ICON_GAP + textW(value[i], TEXT_SIZE, false) + (tail[i].isEmpty() ? 0 : textW(tail[i], TEXT_SIZE, false));
            barW += slotW[i] + (i > 0 ? DIV : 0);
        }
        w = TILE + (n > 0 ? GAP + barW : 0);
        h = TILE;
    }

    private void slot(int ic, String v, String t, int col) {
        icon[n] = ic;
        value[n] = v;
        tail[n] = t;
        valueColor[n] = col;
        n++;
    }

    private static String serverText() {
        if (mc.isSingleplayer()) return "Singleplayer";
        ServerData d = mc.getCurrentServerData();
        if (d == null || d.serverIP == null) return null;
        String ip = d.serverIP;
        return ip.length() > 28 ? ip.substring(0, 27) + "…" : ip;
    }

    /** Join animation: slots slide down from -20 with alpha, staggered 60 ms, 350 ms easeOutBack. */
    private float slide(int i, boolean sample) {
        long join = Hud.worldJoinMs();
        if (sample || join == 0 || General.calm()) return 1f;
        return Ease.clamp01((System.currentTimeMillis() - join - 300 - i * 60) / 350f);
    }

    @Override
    protected void paint(boolean sample) {
        float t0 = slide(0, sample);
        float off = -20 * (1 - Ease.outBack(t0));
        // Logo tile.
        round(0, off, TILE, TILE, 3, alphaOf(0xD10E0F13, t0));
        outline(0, off, TILE, TILE, 3, 0.5f, alphaOf(0x0FFFFFFF, t0));
        sigil(2.5f, off + 2.5f, 9, alphaOf(SIGIL, t0));
        if (n == 0) return;
        float x0 = TILE + GAP;
        round(x0, off, barW, BAR_H, 3, alphaOf(0xC70E0F13, t0));
        float x = x0 + PAD, cy = BAR_H / 2f;
        for (int i = 0; i < n; i++) {
            float t = slide(i + 1, sample);
            float o = -20 * (1 - Ease.outBack(t));
            if (i > 0) {
                rect(x + 5, o + cy - 3.5f, 1, 7, alphaOf(0x1AFFFFFF, t));
                x += DIV;
            }
            drawIcon(icon[i], x, o + cy - ICON / 2f, t);
            float tx = x + ICON + ICON_GAP;
            tx += textMid(value[i], tx, o + cy, TEXT_SIZE, false, alphaOf(valueColor[i], t));
            if (!tail[i].isEmpty()) textMid(tail[i], tx, o + cy, TEXT_SIZE, false, alphaOf(TEXT_MUTED, t));
            x += slotW[i];
        }
    }

    private static int alphaOf(int c, float t) {
        float a = Ease.clamp01(t * 1.6f);
        return a >= 1 ? c : ((Math.round((c >>> 24) * a) & 255) << 24) | (c & 0xFFFFFF);
    }

    /** The original Vesper sigil in a size×size box (filled when small, monoline when large). */
    private void sigil(float x, float y, float size, int c) {
        if (size * scale() <= 12) {
            fan(RX, RY, 0.555, 0.50, x, y, size, c);
            fan(LX, LY, 0.445, 0.46, x, y, size, c);
            tri(px(0.80, x, size), py(0.88, y, size), px(0.85, x, size), py(0.82, y, size), px(0.80, x, size), py(0.76, y, size), c);
            tri(px(0.80, x, size), py(0.88, y, size), px(0.80, x, size), py(0.76, y, size), px(0.75, x, size), py(0.82, y, size), c);
        } else {
            float t = Math.max(0.75f, size / 28f);
            strip(RX, RY, 5, x, y, size, t, c);
            strip(LX, LY, 5, x, y, size, t, c);
            strip(DX, DY, 5, x, y, size, t, c);
        }
    }

    private void fan(double[] xs, double[] ys, double cxu, double cyu, float x, float y, float size, int c) {
        float cx = px(cxu, x, size), cy = py(cyu, y, size);
        for (int i = 0; i < 5; i++) {
            int j = (i + 1) % 5;
            tri(cx, cy, px(xs[i], x, size), py(ys[i], y, size), px(xs[j], x, size), py(ys[j], y, size), c);
        }
    }

    private void strip(double[] xs, double[] ys, int cnt, float x, float y, float size, float t, int c) {
        for (int i = 0; i < cnt; i++) {
            bx[i] = px(xs[i], x, size);
            by[i] = py(ys[i], y, size);
        }
        polyline(bx, by, cnt, t, c);
    }

    private static float px(double u, float x, float size) {
        return (float) (x + u * size);
    }

    private static float py(double v, float y, float size) {
        return (float) (y + (1 - v) * size);
    }

    private void drawIcon(int ic, float x, float y, float t) {
        float cx = x + 3.5f, cy = y + 3.5f;
        switch (ic) {
            case ICON_GLOBE: {
                int c = alphaOf(ACCENT, t);
                ring(cx, cy, 3.1f, 0.8f, c);
                ellipse(cx, cy, 1.3f, 3.1f, 0, Math.PI * 2, 24, c);
                line(cx - 3.1f, cy, cx + 3.1f, cy, 0.6f, c);
                break;
            }
            case ICON_GAUGE: {
                int c = alphaOf(TEXT, t);
                ellipse(cx, cy + 1.2f, 3.2f, 3.2f, -Math.PI / 2, Math.PI / 2, 14, c);
                double f = Math.min(1, fps / 240.0);
                double a = Math.PI * (1 - f); // left (0 fps) to right (240+)
                line(cx, cy + 1.2f, (float) (cx + Math.cos(a) * 2.6), (float) (cy + 1.2f - Math.sin(a) * 2.6), 0.8f, c);
                break;
            }
            case ICON_SIGNAL: {
                int c = alphaOf(ping < 100 ? GOOD : ping < 200 ? WARN : BAD, t);
                for (int b = 0; b < 3; b++) {
                    float bh = 2.5f + b * 2f;
                    rect(x + 0.5f + b * 2.3f, y + 7 - bh, 1.5f, bh, c);
                }
                break;
            }
            default: {
                int c = alphaOf(TEXT, t);
                ring(cx, cy, 3.1f, 0.8f, c);
                double ma = minute / 60.0 * Math.PI * 2, ha = ((hour % 12) + minute / 60.0) / 12.0 * Math.PI * 2;
                line(cx, cy, (float) (cx + Math.sin(ma) * 2.2), (float) (cy - Math.cos(ma) * 2.2), 0.6f, c);
                line(cx, cy, (float) (cx + Math.sin(ha) * 1.5), (float) (cy - Math.cos(ha) * 1.5), 0.7f, c);
            }
        }
    }

    /** Arc of an ellipse; angles measured from straight up, clockwise. */
    private void ellipse(float cx, float cy, float rx, float ry, double a0, double a1, int segs, int c) {
        int cnt = Math.min(segs + 1, bx.length);
        for (int i = 0; i < cnt; i++) {
            double a = a0 + (a1 - a0) * i / (cnt - 1);
            bx[i] = cx + Math.sin(a) * rx;
            by[i] = cy - Math.cos(a) * ry;
        }
        polyline(bx, by, cnt, 0.6f, c);
    }
}
