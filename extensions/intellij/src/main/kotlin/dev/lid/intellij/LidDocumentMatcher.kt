package dev.lid.intellij

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.redhat.devtools.lsp4ij.DocumentMatcher
import java.io.File

class LidDocumentMatcher : DocumentMatcher {
    override fun match(file: VirtualFile, project: Project): Boolean =
        project.basePath?.let { File(it, "docs/arrows/index.yaml").exists() } ?: false
}
