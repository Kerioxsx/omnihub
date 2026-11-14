package dev.vesper.client.music;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.Proxy;
import java.net.URL;
import java.net.URLEncoder;

/**
 * What's playing on this PC and its synced lyrics, read from OmniHub
 * (`http://127.0.0.1:47830/vesper/lyrics`, this PC only) twice a second on
 * a background thread while the lyrics are shown. Without OmniHub nothing
 * happens.
 */
public final class LyricsFeed {
    public static final String URL_BASE = "http://127.0.0.1:47830/vesper/lyrics";

    /** One answer from OmniHub; lines are kept across answers for the same song. */
    public static final class Snapshot {
        public final String key, title, artist, lyricsState;
        public final boolean playing;
        public final long positionMs, durationMs, at;
        public final long[] ms;
        public final String[] text;

        Snapshot(String key, String title, String artist, String lyricsState, boolean playing, long positionMs, long durationMs, long[] ms, String[] text) {
            this.key = key;
            this.title = title;
            this.artist = artist;
            this.lyricsState = lyricsState;
            this.playing = playing;
            this.positionMs = positionMs;
            this.durationMs = durationMs;
            this.at = System.nanoTime();
            this.ms = ms;
            this.text = text;
        }

        /** The song's position now (ms), plus the user's offset. */
        public long now(long offsetMs) {
            long p = positionMs + (playing ? (System.nanoTime() - at) / 1_000_000L : 0);
            return p + offsetMs;
        }

        /** Index of the line playing at `pos`, -1 before the first. */
        public int lineAt(long pos) {
            int i = -1;
            for (int k = 0; k < ms.length; k++) {
                if (ms[k] <= pos) i = k;
                else break;
            }
            return i;
        }
    }

    private static volatile Snapshot snap;
    private static volatile long lastOk;
    private static volatile boolean wanted;
    private static Thread thread;

    private LyricsFeed() {
    }

    public static Snapshot get() {
        return System.currentTimeMillis() - lastOk < 4000 ? snap : null;
    }

    public static boolean connected() {
        return System.currentTimeMillis() - lastOk < 4000;
    }

    /** Ask for updates (call every tick the lyrics are shown); the thread sleeps otherwise. */
    public static synchronized void want(boolean on) {
        wanted = on;
        if (on && thread == null) {
            thread = new Thread(LyricsFeed::loop, "Vesper lyrics");
            thread.setDaemon(true);
            thread.start();
        }
    }

    private static void loop() {
        while (true) {
            try {
                if (wanted) poll();
                Thread.sleep(wanted ? (connected() ? 500 : 3000) : 1000);
            } catch (InterruptedException e) {
                return;
            } catch (Throwable t) {
                try {
                    Thread.sleep(3000);
                } catch (InterruptedException e) {
                    return;
                }
            }
        }
    }

    private static void poll() throws Exception {
        Snapshot old = snap;
        String have = old == null || old.key == null ? "" : old.key;
        HttpURLConnection c = (HttpURLConnection) new URL(URL_BASE + "?have=" + URLEncoder.encode(have, "UTF-8")).openConnection(Proxy.NO_PROXY);
        c.setConnectTimeout(400);
        c.setReadTimeout(800);
        c.setUseCaches(false);
        try {
            if (c.getResponseCode() != 200) return;
            InputStream in = c.getInputStream();
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buf = new byte[8192];
            int n;
            while ((n = in.read(buf)) > 0 && out.size() < (2 << 20)) out.write(buf, 0, n);
            JsonObject o = new JsonParser().parse(out.toString("UTF-8")).getAsJsonObject();
            lastOk = System.currentTimeMillis();
            JsonElement k = o.get("key");
            if (k == null || k.isJsonNull()) {
                snap = new Snapshot(null, "", "", "none", false, 0, 0, new long[0], new String[0]);
                return;
            }
            String key = k.getAsString();
            long[] ms;
            String[] text;
            JsonElement ls = o.get("lines");
            if (ls != null && ls.isJsonArray()) {
                JsonArray a = ls.getAsJsonArray();
                ms = new long[a.size()];
                text = new String[a.size()];
                for (int i = 0; i < a.size(); i++) {
                    JsonObject l = a.get(i).getAsJsonObject();
                    ms[i] = l.get("ms").getAsLong();
                    text[i] = l.get("text").getAsString();
                }
            } else if (old != null && key.equals(old.key)) {
                ms = old.ms;
                text = old.text;
            } else {
                ms = new long[0];
                text = new String[0];
            }
            snap = new Snapshot(key, str(o, "title"), str(o, "artist"), str(o, "lyrics"), o.has("playing") && o.get("playing").getAsBoolean(), o.get("positionMs").getAsLong(), o.has("durationMs") ? o.get("durationMs").getAsLong() : 0, ms, text);
        } finally {
            c.disconnect();
        }
    }

    private static String str(JsonObject o, String k) {
        JsonElement e = o.get(k);
        return e == null || e.isJsonNull() ? "" : e.getAsString();
    }
}
