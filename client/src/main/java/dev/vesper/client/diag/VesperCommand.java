package dev.vesper.client.diag;

import dev.vesper.client.VesperClient;
import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.fx.KillEffects;
import dev.vesper.client.module.General;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.command.CommandBase;
import net.minecraft.command.ICommandSender;
import net.minecraft.entity.Entity;
import net.minecraft.util.BlockPos;
import net.minecraft.util.ChatComponentText;
import net.minecraft.util.EnumChatFormatting;
import net.minecraft.util.MovingObjectPosition;
import net.minecraft.util.Vec3;

/**
 * Client-only `/vesper` command (never sent to the server):
 * <pre>
 * /vesper demo [effect]   play a kill effect as a local DEMO where you look
 * /vesper effects on|off  master switch for all effects
 * /vesper toggle <module> turn a module on or off
 * /vesper profile <name>  switch (or create) a settings profile
 * /vesper diag            version, modules, particle budget, frame time
 * </pre>
 */
public final class VesperCommand extends CommandBase {
    @Override
    public String getCommandName() {
        return "vesper";
    }

    @Override
    public String getCommandUsage(ICommandSender sender) {
        return "/vesper demo [effect] | effects on|off | toggle <module> | profile <name> | diag";
    }

    @Override
    public int getRequiredPermissionLevel() {
        return 0;
    }

    @Override
    public boolean canCommandSenderUseCommand(ICommandSender sender) {
        return true;
    }

    private static void say(String msg) {
        Minecraft.getMinecraft().thePlayer.addChatMessage(new ChatComponentText(EnumChatFormatting.AQUA + "[Vesper] " + EnumChatFormatting.RESET + msg));
    }

    @Override
    public void processCommand(ICommandSender sender, String[] args) {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.thePlayer == null) return;
        String sub = args.length == 0 ? "diag" : args[0].toLowerCase(java.util.Locale.ROOT);
        if (sub.equals("demo")) {
            String id = args.length > 1 ? args[1] : null;
            if (id != null && !Arrays.asList(KillEffects.IDS).contains(id)) {
                say("Unknown effect. One of: " + String.join(", ", KillEffects.IDS));
                return;
            }
            Vec3 at = target(mc);
            Entity e = mc.pointedEntity;
            CombatVisualEvent ev = new CombatVisualEvent(CombatVisualEvent.Type.DEMO, CombatVisualEvent.Confidence.DEMO, CombatVisualEvent.Source.MANUAL_PREVIEW, mc.thePlayer.getUniqueID(), e == null ? null : e.getUniqueID(), e == null ? -1 : e.getEntityId(), at.xCoord, at.yCoord, at.zCoord, e == null ? 1.8 : e.height, e instanceof net.minecraft.entity.player.EntityPlayer);
            if (id != null) KillEffects.demo(id, at.xCoord, at.yCoord, at.zCoord);
            else CombatBus.post(ev);
            say("Demo " + (id == null ? "(selected effect)" : id) + " — local preview only.");
        } else if (sub.equals("effects") && args.length > 1) {
            General.INSTANCE.effects.set(args[1].equalsIgnoreCase("on"));
            VesperClient.config().markDirty();
            say("All effects " + (General.fx() ? "on" : "off") + ".");
        } else if (sub.equals("toggle") && args.length > 1) {
            Module m = ModuleRegistry.get(args[1]);
            if (m == null) {
                say("No module " + args[1] + ".");
                return;
            }
            m.toggle();
            VesperClient.config().markDirty();
            say(m.name + (m.enabled() ? " on." : " off."));
        } else if (sub.equals("profile") && args.length > 1) {
            VesperClient.config().switchProfile(args[1]);
            say("Profile: " + VesperClient.config().activeProfile());
        } else {
            int on = 0;
            for (Module m : ModuleRegistry.all()) if (m.enabled()) on++;
            say("Vesper " + VesperClient.VERSION + " · " + on + "/" + ModuleRegistry.all().size() + " modules on · profile " + VesperClient.config().activeProfile() + " · quality " + General.INSTANCE.quality.label() + " (particles ≤ " + General.particleBudget() + ") · " + Minecraft.getDebugFPS() + " FPS · effects " + (General.fx() ? "on" : "off"));
            String err = VesperClient.config().lastError();
            if (err != null) say(err);
        }
    }

    /** Where the crosshair points (an entity's feet, a block's top), else 4 blocks ahead. */
    private static Vec3 target(Minecraft mc) {
        if (mc.pointedEntity != null) return new Vec3(mc.pointedEntity.posX, mc.pointedEntity.posY, mc.pointedEntity.posZ);
        MovingObjectPosition o = mc.objectMouseOver;
        if (o != null && o.typeOfHit == MovingObjectPosition.MovingObjectType.BLOCK) {
            BlockPos p = o.getBlockPos();
            return new Vec3(p.getX() + 0.5, p.getY() + 1, p.getZ() + 0.5);
        }
        Vec3 look = mc.thePlayer.getLookVec();
        return new Vec3(mc.thePlayer.posX + look.xCoord * 4, mc.thePlayer.posY, mc.thePlayer.posZ + look.zCoord * 4);
    }

    @Override
    public List<String> addTabCompletionOptions(ICommandSender sender, String[] args, BlockPos pos) {
        if (args.length == 1) return getListOfStringsMatchingLastWord(args, "demo", "effects", "toggle", "profile", "diag");
        if (args.length == 2 && args[0].equals("demo")) return getListOfStringsMatchingLastWord(args, KillEffects.IDS);
        if (args.length == 2 && args[0].equals("effects")) return getListOfStringsMatchingLastWord(args, "on", "off");
        if (args.length == 2 && args[0].equals("toggle")) {
            List<String> ids = new ArrayList<String>();
            for (Module m : ModuleRegistry.all()) ids.add(m.id);
            return getListOfStringsMatchingLastWord(args, ids.toArray(new String[0]));
        }
        return null;
    }
}
