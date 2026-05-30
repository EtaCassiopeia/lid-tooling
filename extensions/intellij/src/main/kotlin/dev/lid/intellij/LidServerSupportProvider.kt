package dev.lid.intellij

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.redhat.devtools.lsp4ij.ServerSupportProvider
import com.redhat.devtools.lsp4ij.server.ProcessStreamConnectionProvider
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider
import java.nio.file.Paths

class LidServerSupportProvider : ServerSupportProvider {

    override fun createConnectionProvider(project: Project): StreamConnectionProvider =
        ProcessStreamConnectionProvider(listOf(resolveServerPath()), project.basePath ?: ".")

    override fun isFileSupported(file: VirtualFile, project: Project): Boolean {
        val path = file.path
        return SUPPORTED_EXTENSIONS.any { path.endsWith(it) }
            || LID_PATH_SEGMENTS.any { path.contains(it) }
    }

    private fun resolveServerPath(): String {
        // Bundled binary placed next to the plugin jar by the release pipeline.
        val bundled = Paths.get(
            LidServerSupportProvider::class.java.protectionDomain.codeSource.location.toURI()
        ).parent.resolve("server/lid-lsp").toFile()
        if (bundled.canExecute()) return bundled.absolutePath
        // Fall back to whatever is on PATH (dev / manual install).
        return "lid-lsp"
    }

    companion object {
        private val SUPPORTED_EXTENSIONS = setOf(
            ".rs", ".ts", ".tsx", ".js", ".jsx",
            ".py", ".go", ".java", ".scala", ".cs",
            ".rb", ".cpp", ".c", ".md", ".yaml",
        )
        private val LID_PATH_SEGMENTS = listOf("docs/intent/", "docs/arrows/")
    }
}
