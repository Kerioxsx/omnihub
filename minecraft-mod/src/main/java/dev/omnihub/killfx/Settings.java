package dev.omnihub.killfx;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.Properties;

/**
 * config/omnihub-killfx.properties. OmniHub on the PC writes it; the mod
 * reads it again whenever the file changes, so changes apply in-game.
 */
public final class Settings {
    public boolean enabled = true;
    /** lightning, blood, flames, firework, souls, hearts, explosion, random */
    public String effect = "blood";
    /** default (the effect's own), none, levelup, orb, anvil, explode, thunder, wither, ding */
    public String sound = "default";
    /** 1 subtle, 2 normal, 3 big */
    public int intensity = 2;
    /** RGB for fireworks and coloured dust. */
    public int color = 0xFF3B3B;
    /** Only players count (not mobs). */
    public boolean playersOnly = false;
    /** A small "Kill" mark under the crosshair and a streak counter. */
    public boolean killMark = true;

    private final File file;
    private long stamp = -1;

    public Settings(File file) {
        this.file = file;
    }

    /** Re-read when the file changed. */
    public void refresh() {
        long m = file.exists() ? file.lastModified() : 0;
        if (m == stamp) return;
        stamp = m;
        if (m == 0) {
            save();
            stamp = file.lastModified();
            return;
        }
        Properties p = new Properties();
        try (InputStream in = new FileInputStream(file)) {
            p.load(in);
        } catch (Exception e) {
            return;
        }
        enabled = bool(p, "enabled", enabled);
        effect = p.getProperty("effect", effect).trim().toLowerCase();
        sound = p.getProperty("sound", sound).trim().toLowerCase();
        intensity = Math.max(1, Math.min(3, num(p, "intensity", intensity)));
        playersOnly = bool(p, "players_only", playersOnly);
        killMark = bool(p, "kill_mark", killMark);
        String c = p.getProperty("color", "").trim().replace("#", "");
        if (c.length() == 6) {
            try {
                color = Integer.parseInt(c, 16);
            } catch (NumberFormatException ignored) {
            }
        }
    }

    private void save() {
        Properties p = new Properties();
        p.setProperty("enabled", String.valueOf(enabled));
        p.setProperty("effect", effect);
        p.setProperty("sound", sound);
        p.setProperty("intensity", String.valueOf(intensity));
        p.setProperty("color", String.format("#%06X", color));
        p.setProperty("players_only", String.valueOf(playersOnly));
        p.setProperty("kill_mark", String.valueOf(killMark));
        try {
            File dir = file.getParentFile();
            if (dir != null) dir.mkdirs();
            try (OutputStream out = new FileOutputStream(file)) {
                p.store(out, "OmniHub Kill Effects (set these in OmniHub: Games > Minecraft client)");
            }
        } catch (Exception ignored) {
        }
    }

    private static boolean bool(Properties p, String k, boolean d) {
        String v = p.getProperty(k);
        return v == null ? d : v.trim().equalsIgnoreCase("true");
    }

    private static int num(Properties p, String k, int d) {
        try {
            return Integer.parseInt(p.getProperty(k, String.valueOf(d)).trim());
        } catch (NumberFormatException e) {
            return d;
        }
    }
}
