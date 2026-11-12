package dev.vesper.client.hud;

import dev.vesper.client.module.setting.BoolSetting;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityLivingBase;

/**
 * The entity you're looking at (or last looked at, for 3 s): name · health ·
 * distance. Off by default; only in singleplayer unless "Allow on
 * multiplayer" is on, because some servers forbid showing opponents' health.
 */
final class TargetInfoElement extends LineElement {
    private final BoolSetting multiplayer = bool("multiplayer", "Allow on multiplayer (check the server's rules)", false);
    private EntityLivingBase target;
    private long seen;

    TargetInfoElement() {
        super("target", "Target info", "Name, health and distance of what you aim at. Singleplayer only unless you allow it per your server's rules.", false, 0.5f, 0.42f);
    }

    private boolean allowed() {
        return mc.isSingleplayer() || multiplayer.on();
    }

    @Override
    protected long throttleMs() {
        return 100;
    }

    @Override
    public boolean hasContent() {
        return target != null && allowed();
    }

    @Override
    protected void compute(boolean sample) {
        if (sample) {
            label = "Target";
            value = "Zombie · 20 HP · 3 m";
            valueColor = TEXT;
            return;
        }
        Entity p = mc.pointedEntity;
        long now = System.currentTimeMillis();
        if (p instanceof EntityLivingBase && p.isEntityAlive()) {
            target = (EntityLivingBase) p;
            seen = now;
        } else if (target != null && (now - seen > 3000 || !target.isEntityAlive() || mc.theWorld == null || target.worldObj != mc.theWorld)) {
            target = null;
        }
        if (target == null || mc.thePlayer == null) return;
        float hp = target.getHealth();
        double d = mc.thePlayer.getDistanceToEntity(target);
        label = "";
        value = target.getName() + " · " + (hp % 1 == 0 ? String.valueOf((int) hp) : String.format(java.util.Locale.ROOT, "%.1f", hp)) + " HP · " + String.format(java.util.Locale.ROOT, "%.1f m", d);
        float f = hp / Math.max(1, target.getMaxHealth());
        valueColor = f > 0.6f ? GOOD : f > 0.3f ? WARN : BAD;
    }
}
