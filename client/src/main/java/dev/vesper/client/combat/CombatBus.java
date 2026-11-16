package dev.vesper.client.combat;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ConcurrentLinkedQueue;

/**
 * Hands combat events to listeners on the client thread. Events may be
 * posted from the integrated server's thread; they are queued and delivered
 * on the next client tick.
 */
public final class CombatBus {
    public interface Listener {
        void on(CombatVisualEvent e);
    }

    private static final List<Listener> LISTENERS = new ArrayList<Listener>();
    private static final ConcurrentLinkedQueue<CombatVisualEvent> QUEUE = new ConcurrentLinkedQueue<CombatVisualEvent>();

    private CombatBus() {
    }

    public static void listen(Listener l) {
        LISTENERS.add(l);
    }

    /** Safe from any thread. */
    public static void post(CombatVisualEvent e) {
        if (QUEUE.size() < 256) QUEUE.add(e);
    }

    /** Client thread, once per tick. */
    public static void deliver() {
        CombatVisualEvent e;
        int n = 0;
        while ((e = QUEUE.poll()) != null && n++ < 64) for (Listener l : LISTENERS) l.on(e);
    }

    /** World unload / server switch: drop anything pending. */
    public static void clear() {
        QUEUE.clear();
    }
}
