package dev.lid.intellij.navigator

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.LocalFileSystem
import org.yaml.snakeyaml.DumperOptions
import org.yaml.snakeyaml.Yaml
import java.io.File

class LidProjectWriter(private val project: Project) {

    fun applySpecStatus(specFile: String, specId: String, line: Int, newStatus: String) {
        val file = File(specFile)
        val lines = file.readLines().toMutableList()
        val idx = line - 1
        check(idx in lines.indices) { "Line $line out of range in $specFile" }
        val newMarker = when (newStatus) {
            "implemented" -> "x"
            "deferred" -> "D"
            else -> " "
        }
        val newText = lines[idx].replace(Regex("""^(\s*-\s+\[)[xX D](\].*)$""")) { m ->
            "${m.groupValues[1]}$newMarker${m.groupValues[2]}"
        }
        check(newText != lines[idx]) { "No spec marker found for $specId on line $line" }
        lines[idx] = newText
        file.writeText(lines.joinToString("\n") + "\n")
        refreshVfs(specFile)
    }

    fun appendSpec(specFile: String?, segmentId: String, specId: String, text: String, specPrefix: String? = null) {
        val resolved = specFile ?: run {
            val base = project.basePath ?: error("No project base")
            "$base/docs/intent/$segmentId/$segmentId-specs.md"
        }
        val newLine = "- [ ] **$specId**: $text"
        val file = File(resolved)
        if (file.exists()) {
            val existing = file.readText()
            val sep = if (existing.endsWith("\n")) "" else "\n"
            file.writeText(existing + sep + newLine + "\n")
        } else {
            file.parentFile.mkdirs()
            val frontmatter = if (specPrefix != null) "---\nprefix: $specPrefix\n---\n\n" else ""
            file.writeText("${frontmatter}# $segmentId specs\n\n$newLine\n")
        }
        refreshVfs(resolved)
    }

    @Suppress("UNCHECKED_CAST")
    fun updateIndexEntry(segmentId: String, changes: Map<String, Any?>, create: Boolean = false) {
        val indexFile = File(project.basePath ?: error("No project base"), "docs/arrows/index.yaml")
        val yaml = buildYaml()
        val raw: MutableMap<String, Any> = if (indexFile.exists()) {
            yaml.load<Map<String, Any>>(indexFile.readText())?.toMutableMap() ?: mutableMapOf()
        } else {
            mutableMapOf()
        }

        val arrows = raw.getOrPut("arrows") { mutableMapOf<String, Any>() }
            .let { it as? MutableMap<String, Any> ?: mutableMapOf<String, Any>().also { m -> raw["arrows"] = m } }

        if (!create) check(arrows.containsKey(segmentId)) { "Segment '$segmentId' not found in index.yaml" }

        val entry = (arrows[segmentId] as? MutableMap<String, Any>)
            ?: mutableMapOf<String, Any>().also { arrows[segmentId] = it }

        for ((k, v) in changes) {
            if (v == null) entry.remove(k) else entry[k] = v
        }

        indexFile.writeText(yaml.dump(raw))
        refreshVfs(indexFile.absolutePath)
    }

    fun addSegment(segmentId: String, status: String, detail: String, blocks: List<String>, children: List<String>) {
        val entry = mutableMapOf<String, Any?>("status" to status, "detail" to detail)
        if (blocks.isNotEmpty()) entry["blocks"] = blocks
        if (children.isNotEmpty()) entry["children"] = children
        updateIndexEntry(segmentId, entry, create = true)
        for (child in children) {
            updateIndexEntry(child, mapOf("parent" to segmentId))
        }
    }

    fun scaffoldSegment(segmentId: String, detail: String, specPrefix: String?) {
        val base = project.basePath ?: return
        val root = File(base)

        val arrowPath = root.resolve("docs/arrows/$detail")
        if (!arrowPath.exists()) {
            arrowPath.parentFile?.mkdirs()
            val title = segmentId.replace('-', ' ')
            arrowPath.writeText(
                "# $title\n\n## Overview\n\n<!-- Describe the $segmentId segment here. -->\n\n" +
                "## References\n\n### LLD\n- `docs/intent/$segmentId/$segmentId-design.md`\n\n" +
                "### EARS\n- `docs/intent/$segmentId/$segmentId-specs.md`\n",
            )
            refreshVfs(arrowPath.absolutePath)
        }

        val intentDir = root.resolve("docs/intent/$segmentId")
        intentDir.mkdirs()

        val prefix = specPrefix ?: segmentId.uppercase()
        val specsPath = intentDir.resolve("$segmentId-specs.md")
        if (!specsPath.exists()) {
            specsPath.writeText("---\nprefix: $prefix\n---\n\n# $segmentId specs\n")
            refreshVfs(specsPath.absolutePath)
        }

        val designPath = intentDir.resolve("$segmentId-design.md")
        if (!designPath.exists()) {
            designPath.writeText(
                "# $segmentId design\n\n## Overview\n\n<!-- Describe the design for $segmentId here. -->\n\n## Decisions\n",
            )
            refreshVfs(designPath.absolutePath)
        }
    }

    fun removeConnection(kind: String, segmentId: String, target: String) {
        val reader = LidProjectReader(project)
        when (kind) {
            "blocks" -> {
                val next = (reader.loadIndex().arrows[segmentId]?.blocks ?: emptyList()).filter { it != target }
                updateIndexEntry(segmentId, mapOf("blocks" to next.ifEmpty { null }))
            }
            "blockedBy" -> {
                val next = (reader.loadIndex().arrows[target]?.blocks ?: emptyList()).filter { it != segmentId }
                updateIndexEntry(target, mapOf("blocks" to next.ifEmpty { null }))
            }
            "children" -> {
                val index = reader.loadIndex()
                val next = (index.arrows[segmentId]?.children ?: emptyList()).filter { it != target }
                updateIndexEntry(segmentId, mapOf("children" to next.ifEmpty { null }))
                if (index.arrows[target]?.parent == segmentId) {
                    updateIndexEntry(target, mapOf("parent" to null))
                }
            }
            "parent" -> {
                val index = reader.loadIndex()
                val next = (index.arrows[target]?.children ?: emptyList()).filter { it != segmentId }
                updateIndexEntry(target, mapOf("children" to next.ifEmpty { null }))
                if (index.arrows[segmentId]?.parent == target) {
                    updateIndexEntry(segmentId, mapOf("parent" to null))
                }
            }
        }
    }

    private fun refreshVfs(path: String) {
        LocalFileSystem.getInstance().refreshAndFindFileByPath(path)?.refresh(false, false)
    }

    private fun buildYaml(): Yaml {
        val opts = DumperOptions().apply {
            defaultFlowStyle = DumperOptions.FlowStyle.BLOCK
            indent = 2
            isPrettyFlow = true
        }
        return Yaml(opts)
    }
}
