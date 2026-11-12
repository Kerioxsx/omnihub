package dev.vesper.client.cosmetic;

/**
 * Preset colour palettes (VISUAL_SPEC section 9) shared by cosmetics, the
 * preview and anything else that wants the user's chosen colours. Values
 * are opaque RGB (alpha FF); callers apply their own alpha.
 */
public final class Palette {
    public final String id, name;
    public final int accent, accentSoft, coreTint, tailHot, tailMid, tailCold;
    public final int wingMembrane, wingShadow, wingRib, token, tokenEdge, glow, ring, spark;
    /** Default sky / kill-effect ids for this palette (see fx.sky.Skies / fx.KillEffects). */
    public final String sky, kill;

    private Palette(String id, String name, int accent, int accentSoft, int coreTint, int tailHot, int tailMid, int tailCold,
                    int wingMembrane, int wingShadow, int wingRib, int token, int tokenEdge, int glow, int ring, int spark,
                    String sky, String kill) {
        this.id = id;
        this.name = name;
        this.accent = 0xFF000000 | accent;
        this.accentSoft = 0xFF000000 | accentSoft;
        this.coreTint = 0xFF000000 | coreTint;
        this.tailHot = 0xFF000000 | tailHot;
        this.tailMid = 0xFF000000 | tailMid;
        this.tailCold = 0xFF000000 | tailCold;
        this.wingMembrane = 0xFF000000 | wingMembrane;
        this.wingShadow = 0xFF000000 | wingShadow;
        this.wingRib = 0xFF000000 | wingRib;
        this.token = 0xFF000000 | token;
        this.tokenEdge = 0xFF000000 | tokenEdge;
        this.glow = 0xFF000000 | glow;
        this.ring = 0xFF000000 | ring;
        this.spark = 0xFF000000 | spark;
        this.sky = sky;
        this.kill = kill;
    }

    public static final Palette TIDAL = new Palette("tidal", "Tidal",
        0x38C6F4, 0x1E6F99, 0x9FE8FF, 0xE6FBFF, 0x2F7BFF, 0x0B2A9E,
        0x1F5C8F, 0x0B1E33, 0x0E1824, 0x2FA8E8, 0x155A9E, 0x7FD4FF, 0xDFF6FF, 0xBFEFFF,
        "nocturne", "astral_collapse");
    public static final Palette EMBER = new Palette("ember", "Ember",
        0xFF5A3C, 0x8C2A1C, 0xFF9AA8, 0xFFE3E0, 0xFF3B4E, 0x7A0E2A,
        0xC21F3A, 0x4A0814, 0x3A0610, 0xFF8A2E, 0xC2410C, 0xFFB040, 0xFFE6CC, 0xFFD08A,
        "bloodmoon", "solar_bloom");
    public static final Palette NEBULA = new Palette("nebula", "Nebula",
        0xB45CFF, 0x5E2A8C, 0xE2C6FF, 0xF6EEFF, 0xA57BFF, 0x4A1D80,
        0x6B2FA6, 0x220C3D, 0x1A0B2B, 0x9A5CF0, 0x5A2AA8, 0xE07BFF, 0xF3E2FF, 0xF0C8FF,
        "aurora", "violet_rift");

    public static final Palette[] ALL = {TIDAL, EMBER, NEBULA};
    public static final String[] IDS = {"tidal", "ember", "nebula"};
    public static final String[] NAMES = {"Tidal", "Ember", "Nebula"};
    /** Per-cosmetic override choices: "shared" follows the global palette. */
    public static final String[] OVERRIDE_IDS = {"shared", "tidal", "ember", "nebula"};
    public static final String[] OVERRIDE_NAMES = {"Shared", "Tidal", "Ember", "Nebula"};

    public static Palette byId(String id) {
        for (Palette p : ALL) if (p.id.equals(id)) return p;
        return TIDAL;
    }

    /** The global palette chosen in the "Cosmetic style" module. */
    public static Palette active() {
        CosmeticStyle s = CosmeticStyle.INSTANCE;
        return s == null ? TIDAL : byId(s.palette.get());
    }

    /** Resolves a per-cosmetic override id ("shared" = the global palette). */
    public static Palette pick(String overrideId) {
        return overrideId == null || overrideId.equals("shared") ? active() : byId(overrideId);
    }

    // ---- wing colourways ----

    /** Wing colours: membrane edge, membrane root, ribs, glow edge. */
    public static final class Wing {
        public final int membrane, shadow, rib, edge;

        Wing(int membrane, int shadow, int rib, int edge) {
            this.membrane = 0xFF000000 | membrane;
            this.shadow = 0xFF000000 | shadow;
            this.rib = 0xFF000000 | rib;
            this.edge = 0xFF000000 | edge;
        }
    }

    public static final String[] WING_IDS = {"palette", "crimson", "azure", "scarlet", "orchid", "onyx"};
    public static final String[] WING_NAMES = {"Palette", "Crimson", "Vivid blue", "Warm red", "Orchid", "Onyx"};
    /** Red A: translucent crimson with dark veins. */
    private static final Wing CRIMSON = new Wing(0xC21F3A, 0x4A0814, 0x2A040C, 0xFF5A5A);
    private static final Wing AZURE = new Wing(0x1E7BFF, 0x07224F, 0x0A1A33, 0x6FD8FF);
    /** Warm red/amber. */
    private static final Wing SCARLET = new Wing(0xE8452C, 0x5A1206, 0x3A0A04, 0xFFB040);
    /** Purple to pink. */
    private static final Wing ORCHID = new Wing(0xC44BD8, 0x3A0F52, 0x24093A, 0xFF7AD9);
    /** Near-black (the reference's dark bat variant) with a palette-free violet rim. */
    private static final Wing ONYX = new Wing(0x1C1C24, 0x08080C, 0x050507, 0x8A7BFF);

    public static Wing wing(String id, Palette p) {
        if ("crimson".equals(id)) return CRIMSON;
        if ("azure".equals(id)) return AZURE;
        if ("scarlet".equals(id)) return SCARLET;
        if ("orchid".equals(id)) return ORCHID;
        if ("onyx".equals(id)) return ONYX;
        return p.wingColours();
    }

    private Wing wingCache;

    public Wing wingColours() {
        if (wingCache == null) wingCache = new Wing(wingMembrane, wingShadow, wingRib, accent);
        return wingCache;
    }
}
