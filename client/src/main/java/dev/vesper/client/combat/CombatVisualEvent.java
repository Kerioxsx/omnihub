package dev.vesper.client.combat;

import java.util.UUID;

/**
 * Something combat-related that a visual may react to. Purely
 * informational: nothing here changes the game.
 */
public final class CombatVisualEvent {
    public enum Type {
        /** You attacked this entity (your swing; the server may still reject the hit). */
        HIT_CONFIRMED,
        /** An entity died (its death was observed, not necessarily by you). */
        ENTITY_DEATH,
        /** A kill credited to you by a trustworthy or labelled source. */
        PLAYER_KILL_CONFIRMED,
        /** A local test from the showroom or a test button. */
        DEMO
    }

    public enum Confidence {
        /** From the integrated (singleplayer/LAN host) server: the game itself says so. */
        CONFIRMED,
        /** From a server-specific chat pattern the user turned on. */
        BEST_EFFORT,
        DEMO
    }

    public enum Source {
        SINGLEPLAYER,
        COOPERATING_SERVER,
        SERVER_MESSAGE,
        LOCAL_ATTACK,
        MANUAL_PREVIEW
    }

    public final Type type;
    public final Confidence confidence;
    public final Source source;
    public final UUID actor, target;
    public final int targetEntityId;
    public final double x, y, z;
    /** Height of the target (for effects that rise from the body). */
    public final double height;
    public final boolean targetIsPlayer;
    public final long time;

    public CombatVisualEvent(Type type, Confidence confidence, Source source, UUID actor, UUID target, int targetEntityId, double x, double y, double z, double height, boolean targetIsPlayer) {
        this.type = type;
        this.confidence = confidence;
        this.source = source;
        this.actor = actor;
        this.target = target;
        this.targetEntityId = targetEntityId;
        this.x = x;
        this.y = y;
        this.z = z;
        this.height = height;
        this.targetIsPlayer = targetIsPlayer;
        this.time = System.currentTimeMillis();
    }

    @Override
    public String toString() {
        return type + "/" + confidence + "/" + source + " #" + targetEntityId;
    }
}
