package dev.vesper.client.cosmetic;

/**
 * Everything a cosmetic needs to animate, independent of where it is drawn
 * (the in-world player, the title-screen preview or the showroom stage).
 *
 * Coordinate contract for {@link CosmeticRenderer#render}: the current GL
 * matrix has its origin at the player's feet, +y up, 1 unit = 1 block, and
 * is NOT rotated by the body yaw (renderers apply {@link #bodyYaw} where
 * they attach to the body). In body space the player faces +z and its
 * right hand is at -x.
 */
public final class Pose {
    /** Animation clock in seconds (already slowed when reduced motion is on). */
    public float time;
    /** Seconds since the previous frame of this scene, after slowing. */
    public float dt;
    public float partialTicks;
    /** Body yaw in degrees (Minecraft convention: 0 faces +z). */
    public float bodyYaw;
    /** Head yaw relative to the body, degrees. */
    public float headYaw;
    /** Head pitch in degrees (down is positive). */
    public float headPitch;
    public boolean sneaking;
    /** Sprinting, flying or airborne: wings flap faster. */
    public boolean fast;
    public float limbSwing, limbSwingAmount;
    /** Overall alpha (GUI fades); the intensity slider is applied by the renderer. */
    public float alpha = 1f;
    /** Height of the ground relative to the feet (<= 0 when airborne). */
    public float ground;
    /** Scene state (flap phase, blends) carried between frames. */
    public Scene scene;

    /** Smoothed 0..1 sneak amount. */
    public float sneak() {
        return scene == null ? (sneaking ? 1 : 0) : scene.sneakBlend;
    }

    /** Vertical drop of the upper body while sneaking (Minecraft lowers the model ~0.19). */
    public float sneakDrop() {
        return 0.19f * sneak();
    }

    /** Fills the pose for a standing, idle figure (GUI preview, showroom). */
    public Pose idle(float bodyYaw, float alpha) {
        this.bodyYaw = bodyYaw;
        this.headYaw = 0;
        this.headPitch = 0;
        this.sneaking = false;
        this.fast = false;
        this.limbSwing = 0;
        this.limbSwingAmount = 0;
        this.alpha = alpha;
        this.ground = 0;
        this.partialTicks = 0;
        return this;
    }
}
