// Vesper Client: a client-side Forge mod for Minecraft 1.8.9.
// Build: gradle build  ->  build/libs/vesper-client-<version>.jar
// Try it: gradle runClient (needs a display; offline dev account)

plugins {
    java
    id("gg.essential.loom") version "1.15.52"
    id("dev.architectury.architectury-pack200") version "0.1.3"
}

group = "dev.vesper"
version = "0.6.0"

java {
    toolchain.languageVersion.set(JavaLanguageVersion.of(8))
}

loom {
    forge {
        pack200Provider.set(dev.architectury.pack200.java.Pack200Adapter())
    }
    runConfigs {
        getByName("client") {
            property("fml.coreMods.load", "")
            programArgs("--username", "VesperDev", "--width", "1600", "--height", "900")
        }
        remove(getByName("server"))
    }
}

repositories {
    mavenCentral()
}

dependencies {
    minecraft("com.mojang:minecraft:1.8.9")
    mappings("de.oceanlabs.mcp:mcp_stable:22-1.8.9")
    forge("net.minecraftforge:forge:1.8.9-11.15.1.2318-1.8.9")
    // Gson ships with Minecraft 1.8.9 (2.2.4); no extra runtime dependencies.
}

tasks.withType<JavaCompile> {
    options.encoding = "UTF-8"
}

tasks.processResources {
    inputs.property("version", project.version)
    filesMatching("mcmod.info") {
        expand("version" to project.version)
    }
}

val devJar = tasks.named<Jar>("jar") {
    archiveClassifier.set("dev")
    destinationDirectory.set(layout.buildDirectory.dir("intermediates"))
}

// The jar Forge loads: Minecraft names remapped to the ones the game uses.
tasks.named<net.fabricmc.loom.task.RemapJarTask>("remapJar") {
    archiveClassifier.set("")
    input.set(devJar.flatMap { it.archiveFile })
}

// Minecraft 1.8.9's launchwrapper needs Java 8 at run time too.
tasks.withType<JavaExec>().configureEach {
    javaLauncher.set(javaToolchains.launcherFor { languageVersion.set(JavaLanguageVersion.of(8)) })
}
