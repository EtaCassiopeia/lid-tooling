package dev.lid.intellij.actions

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.progress.ProgressIndicator
import com.intellij.openapi.progress.ProgressManager
import com.intellij.openapi.progress.Task
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VfsUtil
import java.io.File

class InitProjectAction : AnAction() {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val root    = project.basePath ?: return

        ProgressManager.getInstance().run(object : Task.Backgroundable(project, "Initializing LID project", false) {
            override fun run(indicator: ProgressIndicator) {
                ProcessBuilder("lidc", "init")
                    .directory(File(root))
                    .redirectErrorStream(true)
                    .start()
                    .waitFor()

                ApplicationManager.getApplication().invokeLater {
                    val vfsRoot = LocalFileSystem.getInstance().findFileByPath(root)
                    if (vfsRoot != null) {
                        VfsUtil.markDirtyAndRefresh(false, true, true, vfsRoot)
                    }
                }
            }
        })
    }

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabledAndVisible = e.project != null
    }
}
