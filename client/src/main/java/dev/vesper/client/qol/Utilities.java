package dev.vesper.client.qol;

import dev.vesper.client.combat.CombatBus;
import dev.vesper.client.combat.CombatVisualEvent;
import dev.vesper.client.gui.ClientScreen;
import dev.vesper.client.hud.HudEditorScreen;
import dev.vesper.client.module.Category;
import dev.vesper.client.module.Module;
import dev.vesper.client.module.ModuleRegistry;
import dev.vesper.client.module.setting.BoolSetting;
import dev.vesper.client.module.setting.ChoiceSetting;
import dev.vesper.client.module.setting.KeySetting;
import dev.vesper.client.module.setting.NumberSetting;
import java.text.SimpleDateFormat;
import java.util.Date;
import net.minecraft.client.Minecraft;
import net.minecraft.client.entity.EntityPlayerSP;
import net.minecraft.client.gui.GuiGameOver;
import net.minecraft.client.settings.KeyBinding;
import net.minecraft.entity.Entity;
import net.minecraft.item.ItemStack;
import net.minecraft.util.BlockPos;
import net.minecraft.util.ChatComponentText;
import net.minecraft.util.EnumParticleTypes;
import net.minecraft.util.IChatComponent;
import net.minecraftforge.client.event.ClientChatReceivedEvent;
import net.minecraftforge.client.event.EntityViewRenderEvent;
import net.minecraftforge.client.event.RenderGameOverlayEvent;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.InputEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import org.lwjgl.input.Keyboard;

/** Everyday helpers: keys, sprinting, chat, warnings and small screen tweaks. */
public final class Utilities {
    private Utilities() {
    }

    public static void init() {
        Module[] ms = {new MenuKeys(), new ToggleSprint(), new Fullbright(), new NoFog(), new ChatTimestamps(), new ChatCleanup(), new NickHider(), new AutoGG(),
            new DurabilityWarning(), new FoodWarning(), new DeathCoords(), new NoPumpkin(), new HideBossbar(), new MoreCrits(), new AlwaysSharpness()};
        for (Module m : ms) {
            ModuleRegistry.register(m);
            MinecraftForge.EVENT_BUS.register(m);
            FMLCommonHandler.instance().bus().register(m);
        }
    }

    static Minecraft mc() {
        return Minecraft.getMinecraft();
    }

    static void say(String text) {
        if (mc().thePlayer != null) mc().thePlayer.addChatMessage(new ChatComponentText("§b[Vesper]§r " + text));
    }

    static void title(String big, String small) {
        mc().ingameGUI.displayTitle(null, null, 4, 40, 12);
        mc().ingameGUI.displayTitle(big, null, -1, -1, -1);
        mc().ingameGUI.displayTitle(null, small, -1, -1, -1);
    }

    /** Right Shift opens Vesper's menu and Right Ctrl the HUD editor, in game. */
    public static final class MenuKeys extends Module {
        public final KeySetting menu = add(new KeySetting("menu", "Open Vesper's menu", Keyboard.KEY_RSHIFT));
        public final KeySetting hud = add(new KeySetting("hud", "Open the HUD editor", Keyboard.KEY_RCONTROL));

        public MenuKeys() {
            super("menukeys", "Menu keys", "Right Shift opens Vesper's mods menu and Right Ctrl the HUD editor, any time in game.", Category.GENERAL, true, true);
        }

        @SubscribeEvent
        public void onKey(InputEvent.KeyInputEvent e) {
            Minecraft mc = mc();
            if (!enabled() || mc.currentScreen != null || mc.thePlayer == null || !Keyboard.getEventKeyState()) return;
            int k = Keyboard.getEventKey();
            if (k != 0 && k == menu.key()) mc.displayGuiScreen(new ClientScreen(null));
            else if (k != 0 && k == hud.key()) mc.displayGuiScreen(new HudEditorScreen(null));
        }
    }

    /** Sprint (and optionally sneak) stays on after one press of its key. */
    public static final class ToggleSprint extends Module {
        public static ToggleSprint INSTANCE;
        public final BoolSetting sneak = bool("sneak", "Toggle sneak too", false);
        public final BoolSetting startOn = bool("startOn", "Sprint on when I join", true);
        private boolean sprint, sneaking, wasSprintKey, wasSneakKey, joined;

        public ToggleSprint() {
            super("togglesprint", "Toggle sprint", "Press your sprint key once and keep sprinting; optionally the same for sneak.", Category.GENERAL, true, true);
            INSTANCE = this;
        }

        public boolean sprintToggled() {
            return enabled() && sprint;
        }

        public boolean sneakToggled() {
            return enabled() && sneak.on() && sneaking;
        }

        @Override
        protected void onDisable() {
            release();
        }

        private void release() {
            Minecraft mc = mc();
            KeyBinding s = mc.gameSettings.keyBindSprint, n = mc.gameSettings.keyBindSneak;
            KeyBinding.setKeyBindState(s.getKeyCode(), s.getKeyCode() > 0 && Keyboard.isKeyDown(s.getKeyCode()));
            KeyBinding.setKeyBindState(n.getKeyCode(), n.getKeyCode() > 0 && Keyboard.isKeyDown(n.getKeyCode()));
        }

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            Minecraft mc = mc();
            if (e.phase != TickEvent.Phase.START || !enabled()) return;
            if (mc.thePlayer == null) {
                joined = false;
                return;
            }
            if (!joined) {
                joined = true;
                sprint = startOn.on();
            }
            KeyBinding s = mc.gameSettings.keyBindSprint, n = mc.gameSettings.keyBindSneak;
            boolean sk = s.getKeyCode() > 0 && Keyboard.isKeyDown(s.getKeyCode()) && mc.currentScreen == null;
            boolean nk = n.getKeyCode() > 0 && Keyboard.isKeyDown(n.getKeyCode()) && mc.currentScreen == null;
            if (sk && !wasSprintKey) sprint = !sprint;
            if (nk && !wasSneakKey && sneak.on()) sneaking = !sneaking;
            wasSprintKey = sk;
            wasSneakKey = nk;
            if (mc.currentScreen != null) return;
            if (s.getKeyCode() > 0) KeyBinding.setKeyBindState(s.getKeyCode(), sprint || sk);
            if (sneak.on() && n.getKeyCode() > 0) KeyBinding.setKeyBindState(n.getKeyCode(), sneaking || nk);
        }
    }

    /** See in the dark: brightness to the maximum while on; your setting comes back when off. */
    public static final class Fullbright extends Module {
        private float saved = -1;

        public Fullbright() {
            super("fullbright", "Fullbright", "See in caves and at night: brightness to the maximum while on, yours back when off.", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            if (e.phase != TickEvent.Phase.END) return;
            Minecraft mc = mc();
            if (enabled()) {
                if (saved < 0) saved = mc.gameSettings.gammaSetting;
                mc.gameSettings.gammaSetting = 100f;
            } else if (saved >= 0) {
                mc.gameSettings.gammaSetting = saved;
                saved = -1;
            }
        }
    }

    /** No distance fog, or just clear water and lava. */
    public static final class NoFog extends Module {
        public final ChoiceSetting where = choice("where", "Where", "all", new String[] {"all", "liquid"}, new String[] {"Everywhere", "Only in water and lava"});

        public NoFog() {
            super("nofog", "No fog", "Removes the fog, everywhere or only under water and in lava, so you see further.", Category.VISUALS, false, true);
        }

        @SubscribeEvent
        public void onFog(EntityViewRenderEvent.FogDensity e) {
            if (!enabled()) return;
            boolean liquid = e.block.getMaterial().isLiquid();
            if (where.is("liquid") && !liquid) return;
            e.density = liquid ? 0.01f : 0.0005f;
            e.setCanceled(true);
        }
    }

    /** The time in front of each chat message. */
    public static final class ChatTimestamps extends Module {
        public final BoolSetting seconds = bool("seconds", "Show seconds", false);

        public ChatTimestamps() {
            super("timestamps", "Chat timestamps", "The time in front of every chat message.", Category.GENERAL, false, true);
        }

        @SubscribeEvent
        public void onChat(ClientChatReceivedEvent e) {
            if (!enabled() || e.type == 2 || e.message == null) return;
            String t = new SimpleDateFormat(seconds.on() ? "HH:mm:ss" : "HH:mm").format(new Date());
            IChatComponent c = new ChatComponentText("§8[" + t + "] §r");
            c.appendSibling(e.message);
            e.message = c;
        }
    }

    /** Folds repeated messages into one with a counter, and hides lobby join lines. */
    public static final class ChatCleanup extends Module {
        public final BoolSetting stack = bool("stack", "Stack repeated messages (x2, x3…)", true);
        public final BoolSetting lobby = bool("lobby", "Hide \"joined the lobby\" lines", true);
        private String last = "";
        private int repeat;

        public ChatCleanup() {
            super("chatcleanup", "Chat cleanup", "Stacks repeated messages into one with a counter and hides lobby join spam.", Category.GENERAL, false, true);
        }

        @SubscribeEvent
        public void onChat(ClientChatReceivedEvent e) {
            if (!enabled() || e.type == 2 || e.message == null) return;
            String plain = e.message.getUnformattedText();
            if (lobby.on() && plain.contains("joined the lobby")) {
                e.setCanceled(true);
                return;
            }
            if (!stack.on()) return;
            if (plain.equals(last) && !plain.trim().isEmpty()) {
                repeat++;
                mc().ingameGUI.getChatGUI().deleteChatLine(0x7E5E7);
                IChatComponent c = e.message.createCopy();
                c.appendSibling(new ChatComponentText(" §7(x" + repeat + ")"));
                mc().ingameGUI.getChatGUI().printChatMessageWithOptionalDeletion(c, 0x7E5E7);
                e.setCanceled(true);
            } else {
                last = plain;
                repeat = 1;
            }
        }
    }

    /** Your name shows as something else in chat on your screen (for videos and streams). */
    public static final class NickHider extends Module {
        public final ChoiceSetting alias = choice("alias", "Show my name as", "You", new String[] {"You", "Player", "Vesper", "Hidden"}, new String[] {"You", "Player", "Vesper", "Hidden"});

        public NickHider() {
            super("nickhider", "Nick hider", "Your name shows as something else in chat on your screen, for videos and streams.", Category.GENERAL, false, true);
        }

        @SubscribeEvent
        public void onChat(ClientChatReceivedEvent e) {
            if (!enabled() || e.message == null) return;
            String name = mc().getSession().getUsername();
            String text = e.message.getFormattedText();
            if (name.isEmpty() || !text.contains(name)) return;
            e.message = new ChatComponentText(text.replace(name, alias.get()));
        }
    }

    /** Says gg once when a game ends (off by default). */
    public static final class AutoGG extends Module {
        public final ChoiceSetting phrase = choice("phrase", "Message", "gg", new String[] {"gg", "GG", "gg wp", "good game"}, new String[] {"gg", "GG", "gg wp", "good game"});
        private static final String[] ENDS = {"1st Killer", "Winner:", "WINNER!", "Reward Summary", "won the game", "VICTORY", "1st Place"};
        private long lastSent;

        public AutoGG() {
            super("autogg", "Auto GG", "Says gg in chat once when a game ends (spots common end-of-game messages).", Category.GENERAL, false, true);
        }

        @SubscribeEvent
        public void onChat(ClientChatReceivedEvent e) {
            if (!enabled() || e.message == null || mc().thePlayer == null || mc().isSingleplayer()) return;
            String t = e.message.getUnformattedText();
            for (String s : ENDS) {
                if (t.contains(s) && System.currentTimeMillis() - lastSent > 15000) {
                    lastSent = System.currentTimeMillis();
                    mc().thePlayer.sendChatMessage(phrase.get());
                    return;
                }
            }
        }
    }

    /** A warning when your sword, tool or armor is about to break. */
    public static final class DurabilityWarning extends Module {
        public final NumberSetting below = number("below", "Warn below", 10, 2, 40, 1, "%");
        private final java.util.Set<String> warned = new java.util.HashSet<String>();

        public DurabilityWarning() {
            super("durabilitywarn", "Durability warning", "A title and a sound when what you hold or wear is about to break.", Category.GENERAL, true, true);
        }

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            EntityPlayerSP p = mc().thePlayer;
            if (e.phase != TickEvent.Phase.END || !enabled() || p == null || p.ticksExisted % 10 != 0) return;
            check(p.getHeldItem());
            for (ItemStack s : p.inventory.armorInventory) check(s);
        }

        private void check(ItemStack s) {
            if (s == null || !s.isItemStackDamageable()) return;
            float left = 1 - s.getItemDamage() / (float) s.getMaxDamage();
            String key = s.getUnlocalizedName() + s.getMaxDamage();
            if (left * 100 < below.f()) {
                if (warned.add(key)) {
                    title("§c" + s.getDisplayName(), "is about to break (" + (s.getMaxDamage() - s.getItemDamage()) + " uses left)");
                    mc().thePlayer.playSound("note.pling", 0.8f, 0.6f);
                }
            } else {
                warned.remove(key);
            }
        }
    }

    /** A reminder to eat when your hunger gets low. */
    public static final class FoodWarning extends Module {
        public final NumberSetting below = number("below", "Warn at", 6, 1, 9, 1, " food");
        private boolean warned;

        public FoodWarning() {
            super("foodwarn", "Hunger warning", "A short reminder to eat when your hunger gets low (you stop sprinting at 3).", Category.GENERAL, false, true);
        }

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            EntityPlayerSP p = mc().thePlayer;
            if (e.phase != TickEvent.Phase.END || !enabled() || p == null || p.capabilities.isCreativeMode) return;
            float food = p.getFoodStats().getFoodLevel() / 2f;
            if (food <= below.f() && !warned) {
                warned = true;
                title("§6Eat something", "Hunger: " + food + " / 10");
            } else if (food > below.f()) {
                warned = false;
            }
        }
    }

    /** Tells you where you died, so you can go back. */
    public static final class DeathCoords extends Module {
        public static BlockPos last;
        public static int lastDim;
        private boolean dead;

        public DeathCoords() {
            super("deathcoords", "Death coordinates", "When you die, chat tells you where (and the death marker in Visuals can point to it).", Category.GENERAL, true, true);
        }

        @SubscribeEvent
        public void onTick(TickEvent.ClientTickEvent e) {
            Minecraft mc = mc();
            if (e.phase != TickEvent.Phase.END || mc.thePlayer == null) return;
            boolean now = mc.currentScreen instanceof GuiGameOver || mc.thePlayer.getHealth() <= 0;
            if (now && !dead) {
                last = new BlockPos(mc.thePlayer);
                lastDim = mc.thePlayer.dimension;
                if (enabled()) say("You died at X " + last.getX() + ", Y " + last.getY() + ", Z " + last.getZ() + ".");
            }
            dead = now;
        }
    }

    /** No pumpkin blur on your screen when you wear a carved pumpkin. */
    public static final class NoPumpkin extends Module {
        public NoPumpkin() {
            super("nopumpkin", "No pumpkin overlay", "Wearing a carved pumpkin no longer covers your screen.", Category.HUD, false, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Pre e) {
            if (enabled() && e.type == RenderGameOverlayEvent.ElementType.HELMET) e.setCanceled(true);
        }
    }

    /** Hides the boss health bar at the top of the screen. */
    public static final class HideBossbar extends Module {
        public HideBossbar() {
            super("hidebossbar", "Hide boss bar", "Hides the boss bar at the top (servers often use it for adverts).", Category.HUD, false, true);
        }

        @SubscribeEvent
        public void onHud(RenderGameOverlayEvent.Pre e) {
            if (enabled() && e.type == RenderGameOverlayEvent.ElementType.BOSSHEALTH) e.setCanceled(true);
        }
    }

    static Entity target(CombatVisualEvent e) {
        return mc().theWorld == null ? null : mc().theWorld.getEntityByID(e.targetEntityId);
    }

    /** Extra critical-hit particles on every hit you land (only on your screen). */
    public static final class MoreCrits extends Module {
        public final NumberSetting amount = number("amount", "Amount", 2, 1, 5, 1, "×");

        public MoreCrits() {
            super("morecrits", "More crit particles", "More critical-hit particles on every hit you land (only on your screen).", Category.EFFECTS, false, true);
            CombatBus.listen(new CombatBus.Listener() {
                public void on(CombatVisualEvent e) {
                    Entity t = target(e);
                    if (!enabled() || e.type != CombatVisualEvent.Type.HIT_CONFIRMED || t == null) return;
                    for (int i = 0; i < amount.f(); i++) mc().effectRenderer.emitParticleAtEntity(t, EnumParticleTypes.CRIT);
                }
            });
        }
    }

    /** Blue sharpness sparkles on every hit, enchanted sword or not. */
    public static final class AlwaysSharpness extends Module {
        public AlwaysSharpness() {
            super("alwayssharp", "Always sharpness particles", "The blue enchanted-hit sparkles on every hit you land.", Category.EFFECTS, false, true);
            CombatBus.listen(new CombatBus.Listener() {
                public void on(CombatVisualEvent e) {
                    Entity t = target(e);
                    if (enabled() && e.type == CombatVisualEvent.Type.HIT_CONFIRMED && t != null) mc().effectRenderer.emitParticleAtEntity(t, EnumParticleTypes.CRIT_MAGIC);
                }
            });
        }
    }
}
