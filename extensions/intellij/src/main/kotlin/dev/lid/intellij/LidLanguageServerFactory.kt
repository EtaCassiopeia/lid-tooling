package dev.lid.intellij

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider
import java.io.File

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
        try {
            // Installed layout: <plugin-root>/lib/<name>.jar
            // Navigate up two levels to reach the plugin root, then into server/.
            val jar = LidLanguageServerFactory::class.java
                .protectionDomain?.codeSource?.location?.toURI()
                ?.let { File(it) }
            if (jar?.isFile == true) {
                val binary = jar.parentFile?.parentFile
                    ?.resolve("server/${platformBinaryName()}")
                if (binary?.canExecute() == true) return binary.absolutePath
            }
        } catch (_: Exception) {}
        return "lid-lsp"
    }

    private fun platformBinaryName(): String {
        val os   = System.getProperty("os.name").lowercase()
        val arch = System.getProperty("os.arch").lowercase()
        return when {
            os.contains("win")                                         -> "lid-lsp-x86_64-pc-windows-msvc.exe"
            os.contains("mac") && arch in listOf("aarch64", "arm64")  -> "lid-lsp-aarch64-apple-darwin"
            os.contains("mac")                                         -> "lid-lsp-x86_64-apple-darwin"
            else                                                       -> "lid-lsp-x86_64-unknown-linux-gnu"
        }
    }
}
