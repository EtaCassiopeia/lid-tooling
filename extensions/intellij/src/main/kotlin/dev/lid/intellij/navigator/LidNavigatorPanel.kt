package dev.lid.intellij.navigator

import com.google.gson.Gson
import com.intellij.openapi.Disposable
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.fileEditor.OpenFileDescriptor
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFileManager
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.VFileEvent
import com.intellij.ui.jcef.JBCefApp
import com.intellij.ui.jcef.JBCefBrowser
import com.intellij.ui.jcef.JBCefBrowserBase
import com.intellij.ui.jcef.JBCefJSQuery
import org.cef.browser.CefBrowser
import org.cef.browser.CefFrame
import org.cef.handler.CefLoadHandlerAdapter
import java.awt.BorderLayout
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit
import javax.swing.JLabel
import javax.swing.JPanel
import javax.swing.SwingConstants

class LidNavigatorPanel(private val project: Project) : JPanel(BorderLayout()), Disposable {

    private val executor = Executors.newSingleThreadScheduledExecutor()
    private var pendingRefresh: ScheduledFuture<*>? = null
    private var browser: JBCefBrowser? = null
    private var query: JBCefJSQuery? = null

    init {
        if (!JBCefApp.isSupported()) {
            add(
                JLabel(
                    "<html><center>JCEF not supported.<br>Use a JetBrains Runtime (JBR) to enable the Intent Navigator.</center></html>",
                    SwingConstants.CENTER,
                ),
            )
        } else {
            val b = JBCefBrowser()
            browser = b
            val q = JBCefJSQuery.create(b as JBCefBrowserBase)
            query = q

            q.addHandler { msgJson ->
                try {
                    handleMessage(msgJson)
                } catch (e: Exception) {
                    postResult(false, e.message ?: "Error")
                }
                null
            }

            project.messageBus.connect(this).subscribe(
                VirtualFileManager.VFS_CHANGES,
                object : BulkFileListener {
                    override fun after(events: List<VFileEvent>) {
                        if (events.any { isRelevant(it.path) }) scheduleRefresh()
                    }
                },
            )

            b.jbCefClient.addLoadHandler(object : CefLoadHandlerAdapter() {
                override fun onLoadEnd(cefBrowser: CefBrowser, frame: CefFrame, httpStatusCode: Int) {
                    if (frame.isMain) postGraph()
                }
            }, b.cefBrowser)

            add(b.component, BorderLayout.CENTER)
            loadHtml()
        }
    }

    private fun isRelevant(path: String) =
        path.contains("docs/arrows/index.yaml") ||
            (path.contains("docs/intent") && path.endsWith(".md"))

    private fun loadHtml() {
        fun res(name: String) = javaClass.getResource("/navigator/$name")?.readText() ?: ""
        val html = res("index.html")
            .replace("__QUERY_INJECT__", query!!.inject("msgJson"))
            .replace("__CYTOSCAPE_JS__", res("cytoscape.min.js"))
            .replace("__DAGRE_JS__", res("dagre.min.js"))
            .replace("__CYTOSCAPE_DAGRE_JS__", res("cytoscape-dagre.min.js"))
        browser!!.loadHTML(html)
    }

    private fun scheduleRefresh(delayMs: Long = 400) {
        pendingRefresh?.cancel(false)
        pendingRefresh = executor.schedule({ postGraph() }, delayMs, TimeUnit.MILLISECONDS)
    }

    private fun postGraph() {
        val payload = try {
            LidProjectReader(project).buildPayload()
        } catch (_: Exception) {
            return
        }
        val json = Gson().toJson(payload)
        dispatchJs("window.dispatchEvent(new MessageEvent('message',{data:{type:'graph',payload:$json}}));")
    }

    private fun handleMessage(msgJson: String) {
        when (val msg = parseMessage(msgJson)) {
            is NavigatorMessage.Open -> openArrowDoc(msg.segmentId)
            is NavigatorMessage.OpenFile -> openFile(msg.path, msg.line)
            is NavigatorMessage.UpdateSpecStatus -> {
                LidProjectWriter(project).applySpecStatus(msg.specFile, msg.specId, msg.line, msg.newStatus)
                postResult(true, "Spec ${msg.specId} marked as ${msg.newStatus}")
            }
            is NavigatorMessage.AddSpec -> {
                LidProjectWriter(project).appendSpec(msg.specFile, msg.segmentId, msg.specId, msg.text)
                postResult(true, "Spec ${msg.specId} added")
            }
            is NavigatorMessage.UpdateSegmentStatus -> {
                LidProjectWriter(project).updateIndexEntry(msg.segmentId, mapOf("status" to msg.newStatus))
                postResult(true, "Segment ${msg.segmentId} → ${msg.newStatus}")
            }
            is NavigatorMessage.UpdateSegmentMeta -> {
                LidProjectWriter(project).updateIndexEntry(
                    msg.segmentId,
                    mapOf(
                        "next" to msg.next.ifEmpty { null },
                        "drift" to msg.drift.ifEmpty { null },
                    ),
                )
                postResult(true, "Segment ${msg.segmentId} metadata saved")
            }
            is NavigatorMessage.AddSegment -> {
                LidProjectWriter(project).addSegment(msg.segmentId, msg.status, msg.detail, msg.blocks, msg.children)
                postResult(true, "Segment ${msg.segmentId} added")
            }
            is NavigatorMessage.RemoveConnection -> {
                LidProjectWriter(project).removeConnection(msg.kind, msg.segmentId, msg.target)
                postResult(true, "Connection removed")
            }
        }
    }

    private fun openArrowDoc(segmentId: String) {
        val base = project.basePath ?: return
        val detail = LidProjectReader(project).loadIndex().arrows[segmentId]?.detail ?: "$segmentId.md"
        openFile("$base/docs/arrows/$detail", null)
    }

    private fun openFile(path: String, line: Int?) {
        ApplicationManager.getApplication().invokeLater {
            val vf = LocalFileSystem.getInstance().refreshAndFindFileByPath(path) ?: return@invokeLater
            val desc = if (line != null && line > 0) OpenFileDescriptor(project, vf, line - 1, 0)
            else OpenFileDescriptor(project, vf)
            desc.navigate(true)
        }
    }

    private fun postResult(ok: Boolean, message: String) {
        val type = if (ok) "mutationOk" else "mutationError"
        val safe = message.replace("\\", "\\\\").replace("\"", "\\\"")
        dispatchJs("window.dispatchEvent(new MessageEvent('message',{data:{type:'$type',message:\"$safe\"}}));")
    }

    private fun dispatchJs(js: String) {
        val b = browser ?: return
        b.cefBrowser.executeJavaScript(js, b.cefBrowser.url ?: "about:blank", 0)
    }

    override fun dispose() {
        executor.shutdownNow()
        browser?.dispose()
    }
}
