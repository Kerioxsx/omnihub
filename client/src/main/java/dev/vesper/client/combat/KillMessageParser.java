package dev.vesper.client.combat;

import java.util.ArrayList;
import java.util.List;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Best-effort reading of kill messages from chat. Pure Java (no Minecraft
 * classes), so it can be tested on its own: see {@link #selfTest()}.
 *
 * <p>Only English messages where the victim comes first and the killer is
 * named after "by"/"to" are understood. Servers word these differently, so a
 * match is labelled best effort, and the caller additionally requires that
 * the local player hit the victim shortly before.
 */
public final class KillMessageParser {
    /** A Minecraft account name: 1–16 letters, digits or underscores. */
    private static final String NAME = "([A-Za-z0-9_]{1,16})";
    /** Optional lead-in: a few symbols (e.g. a skull) and up to two [Tag]/(Tag) prefixes such as ranks or teams. */
    private static final String LEAD = "^[^A-Za-z0-9_\\[(]{0,4}\\s*(?:[\\[(][^\\])]{1,24}[\\])]\\s*){0,2}";
    /** After the killer: end, or a non-name character (so "Steve123" never matches "Steve"). */
    private static final String TAIL = "(?![A-Za-z0-9_]).*$";

    private static final Pattern[] PATTERNS = {
        // Vanilla melee / projectile / explosion death messages.
        Pattern.compile(LEAD + NAME + " was (?:slain|killed|shot|blown up|fireballed|pummeled|knocked into the void|knocked off a cliff|thrown off a cliff|doomed to fall|struck down|eliminated) by (?:[\\[(][^\\])]{1,24}[\\])]\\s*)?" + NAME + TAIL),
        // Vanilla "while fighting" forms (the killer is the one you fought).
        Pattern.compile(LEAD + NAME + " (?:walked into a fire|was burnt to a crisp|tried to swim in lava|drowned|hit the ground too hard|fell (?:out of the world|from a high place)|was struck by lightning) whilst fighting (?:[\\[(][^\\])]{1,24}[\\])]\\s*)?" + NAME + TAIL),
        // Common minigame forms.
        Pattern.compile(LEAD + NAME + " (?:died to|was finished off by|has been killed by|has been slain by) (?:[\\[(][^\\])]{1,24}[\\])]\\s*)?" + NAME + TAIL),
    };

    private static final Pattern FORMAT = Pattern.compile("(?i)§[0-9A-FK-OR]?");

    private KillMessageParser() {
    }

    /** Removes Minecraft formatting codes (§ plus a code character). */
    public static String strip(String s) {
        return s == null ? "" : FORMAT.matcher(s).replaceAll("");
    }

    /**
     * The victim's name if `message` says `localName` killed someone, else
     * null. Never returns the local player (your own death is not a kill).
     */
    public static String victimKilledBy(String message, String localName) {
        if (message == null || localName == null || localName.isEmpty()) return null;
        String m = strip(message).trim();
        if (m.length() > 200) return null;
        for (Pattern p : PATTERNS) {
            Matcher mt = p.matcher(m);
            if (!mt.matches()) continue;
            String victim = mt.group(1), killer = mt.group(2);
            if (killer.equalsIgnoreCase(localName) && !victim.equalsIgnoreCase(localName)) return victim;
            return null;
        }
        return null;
    }

    /** Runs the built-in cases; returns a description of each failure (empty = all passed). */
    public static List<String> selfTest() {
        List<String> fails = new ArrayList<String>();
        String me = "Steve";
        // True matches.
        expect(fails, "Alex was slain by Steve", me, "Alex");
        expect(fails, "Alex was killed by Steve", me, "Alex");
        expect(fails, "Alex was shot by Steve", me, "Alex");
        expect(fails, "Alex was slain by Steve using [Diamond Sword]", me, "Alex");
        expect(fails, "Alex was killed by Steve. FINAL KILL!", me, "Alex");
        expect(fails, "Alex was knocked into the void by Steve.", me, "Alex");
        expect(fails, "Alex_99 was thrown off a cliff by steve", me, "Alex_99");
        expect(fails, "Alex hit the ground too hard whilst fighting Steve", me, "Alex");
        expect(fails, "Alex died to Steve", me, "Alex");
        expect(fails, "[VIP] Alex was slain by [MVP+] Steve", me, "Alex");
        expect(fails, "☠ Alex was killed by Steve", me, "Alex");
        expect(fails, "[SkyWars] Alex was killed by Steve!", me, "Alex");
        // Formatting codes.
        expect(fails, "§cAlex §7was slain by §aSteve", me, "Alex");
        expect(fails, "§6[§bVIP§6] §bAlex§r was shot by §l§aSteve§r.", me, "Alex");
        // Your own death.
        expect(fails, "Steve was slain by Alex", me, null);
        expect(fails, "Steve was shot by Steve", me, null);
        // Other players' kills.
        expect(fails, "Alex was slain by Notch", me, null);
        expect(fails, "Notch was killed by Alex", me, null);
        // Partial name overlaps.
        expect(fails, "Alex was slain by Steve123", me, null);
        expect(fails, "Alex was slain by xSteve", me, null);
        expect(fails, "Alex was slain by Steve_", me, null);
        expect(fails, "Alex was slain by Ste", me, null);
        // Not a kill message / killer-first wording / plain chat.
        expect(fails, "<Alex> Steve was slain by me lol", me, null);
        expect(fails, "Steve killed Alex", me, null);
        expect(fails, "Alex fell from a high place", me, null);
        expect(fails, "", me, null);
        expect(fails, "Alex was slain by Steve", "", null);
        return fails;
    }

    private static void expect(List<String> fails, String msg, String me, String want) {
        String got = victimKilledBy(msg, me);
        if (want == null ? got != null : !want.equals(got)) fails.add("\"" + msg + "\" as " + me + ": expected " + want + ", got " + got);
    }
}
