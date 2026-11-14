package dev.vesper.client.fx;

/** The user's look settings for a kill effect, captured when it starts. */
public final class KillSettings {
    public float intensity = 1f;
    public float duration = 1f;
    public boolean useTint;
    public int tint = 0xFFFFFFFF;
    public boolean calm;
    public boolean reducedFlashes = true;

    public KillSettings copy() {
        KillSettings k = new KillSettings();
        k.intensity = intensity;
        k.duration = duration;
        k.useTint = useTint;
        k.tint = tint;
        k.calm = calm;
        k.reducedFlashes = reducedFlashes;
        return k;
    }
}
