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
        val pluginDir = PluginManagerCore.getPlugin(PluginId.getId("dev.lid.intellij"))?.pluginPath
        if (pluginDir != null) {
            val binary = pluginDir.resolve("server/lid-lsp").toFile()
            if (binary.canExecute()) return binary.absolutePath
        }
        return "lid-lsp"
    }
}
