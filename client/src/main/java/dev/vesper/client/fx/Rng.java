package dev.vesper.client.fx;

/**
 * Small seeded random source (xorshift64*). Effects keep one per instance so
 * the same seed always produces the same shapes; it can be reseeded without
 * allocating, which the skies do every frame.
 */
public final class Rng {
    private long s;

    public Rng(long seed) {
        seed(seed);
    }

    public Rng seed(long seed) {
        s = seed * 0x9E3779B97F4A7C15L + 0x632BE59BD9B4E019L;
        if (s == 0) s = 0x1234567L;
        next();
        next();
        return this;
    }

    public long next() {
        s ^= s >>> 12;
        s ^= s << 25;
        s ^= s >>> 27;
        return s * 0x2545F4914F6CDD1DL;
    }

    /** 0 (inclusive) .. 1 (exclusive). */
    public float f() {
        return (next() >>> 40) / (float) (1 << 24);
    }

    public float range(float a, float b) {
        return a + (b - a) * f();
    }

    /** -1 .. 1. */
    public float signed() {
        return f() * 2 - 1;
    }

    public int i(int n) {
        return (int) ((next() >>> 33) % n);
    }

    public boolean chance(float p) {
        return f() < p;
    }

    /** Mixes two numbers into a well-spread seed. */
    public static long hash(long a, long b) {
        long h = a * 0xBF58476D1CE4E5B9L ^ (b + 0x94D049BB133111EBL);
        h ^= h >>> 31;
        h *= 0xD6E8FEB86659FD93L;
        return h ^ (h >>> 29);
    }
}
