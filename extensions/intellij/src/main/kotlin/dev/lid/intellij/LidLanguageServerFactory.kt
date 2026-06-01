package dev.lid.intellij

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.openapi.extensions.PluginId
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider

class LidLanguageServerFactory : LanguageServerFactory {
    override fun createConnectionProvider(project: Project): StreamConnectionProvider =
        LidLanguageServer(project)
}

private class LidLanguageServer(project: Project) : OSProcessStreamConnectionProvider() {
    init {
        setCommandLine(
            GeneralCommandLine(resolveServerPath())
                .withWorkDirectory(project.basePath)
        )
    }

    private fun resolveServerPath(): String {
        val pluginDir = PluginManagerCore.getPlugin(PluginId.getId("dev.lid"))?.pluginPath
        if (pluginDir != null) {
            val binary = pluginDir.resolve("server/${platformBinaryName()}").toFile()
            if (binary.canExecute()) return binary.absolutePath
        }
        return "lid-lsp"
    }

    private fun platformBinaryName(): String {
        val os   = System.getProperty("os.name").lowercase()
        val arch = System.getProperty("os.arch").lowercase()
        return when {
            os.contains("win")                              -> "lid-lsp-x86_64-pc-windows-msvc.exe"
            os.contains("mac") && arch in listOf("aarch64", "arm64") -> "lid-lsp-aarch64-apple-darwin"
            os.contains("mac")                             -> "lid-lsp-x86_64-apple-darwin"
            else                                           -> "lid-lsp-x86_64-unknown-linux-gnu"
        }
    }
}
