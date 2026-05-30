plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "2.1.0"
    id("org.jetbrains.intellij.platform") version "2.3.0"
}

group = "dev.lid"
version = "0.2.3"

repositories {
    mavenCentral()
    intellijPlatform { defaultRepositories() }
}

dependencies {
    intellijPlatform {
        intellijIdeaCommunity("2024.3")
        plugin("com.redhat.devtools.lsp4ij:0.9.0")
    }
}

intellijPlatform {
    pluginConfiguration {
        name = "LID — Linked-Intent Development"
        version = project.version.toString()
        ideaVersion { sinceBuild = "243" }
    }
    publishing {
        token = providers.environmentVariable("JETBRAINS_MARKETPLACE_TOKEN")
    }
    signing {
        certificateChain = providers.environmentVariable("CERTIFICATE_CHAIN")
        privateKey        = providers.environmentVariable("PRIVATE_KEY")
        password          = providers.environmentVariable("PRIVATE_KEY_PASSWORD")
    }
}

kotlin {
    jvmToolchain(21)
}

// Bundle the lid-lsp binary (populated by CI) into the plugin zip
// so end-users need no separate install.  The server/ dir is .gitignored
// locally; the release pipeline copies the platform binary before buildPlugin.
tasks.prepareSandbox {
    val serverDir = layout.projectDirectory.dir("server")
    if (serverDir.asFile.exists()) {
        from(serverDir) {
            into("${intellijPlatform.projectName.get()}/server")
        }
    }
}
