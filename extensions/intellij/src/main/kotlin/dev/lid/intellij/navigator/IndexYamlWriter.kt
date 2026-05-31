package dev.lid.intellij.navigator

// Pure text-manipulation helpers for docs/arrows/index.yaml.
// No IntelliJ or SnakeYAML imports — all functions are string-in / string-out
// so they can be unit-tested without an IntelliJ platform environment.
object IndexYamlWriter {

    private val YAML_SPECIAL = Regex("""[:#\[\]{},|>&*!?@`]""")

    private val FIELD_ORDER = listOf(
        "status", "detail", "parent", "blocks", "children", "sampled", "audited", "next", "drift",
    )

    /**
     * Serialise a new segment entry as indented YAML text (no trailing newline).
     * Arrays are kept inline ([a, b]) and empty arrays are omitted.
     */
    fun buildSegmentBlock(segmentId: String, entry: Map<String, Any?>): String {
        val lines = mutableListOf("  $segmentId:")
        for (k in FIELD_ORDER) {
            when (val v = entry[k]) {
                null -> continue
                is List<*> -> {
                    @Suppress("UNCHECKED_CAST")
                    val items = (v as List<String?>).filterNotNull()
                    if (items.isEmpty()) continue
                    lines.add("    $k: [${items.joinToString(", ")}]")
                }
                is String -> lines.add("    $k: ${quoteIfNeeded(v)}")
                else -> lines.add("    $k: $v")
            }
        }
        return lines.joinToString("\n")
    }

    /**
     * Insert [block] into the `arrows:` section immediately before the first
     * following top-level key. Appends when `arrows:` is the last section.
     */
    fun insertIntoArrows(content: String, block: String): String {
        val lines = content.split("\n")
        var inArrows = false
        var insertAt = lines.size
        for ((i, line) in lines.withIndex()) {
            if (line.startsWith("arrows:")) { inArrows = true; continue }
            if (inArrows && line.isNotEmpty() && !line.first().isWhitespace()) {
                insertAt = i
                break
            }
        }
        return (lines.subList(0, insertAt) + listOf(block) + lines.subList(insertAt, lines.size))
            .joinToString("\n")
    }

    /**
     * Add or replace a single scalar field on an existing segment without
     * touching any other part of the file. Passing null removes the field.
     * Returns content unchanged when [segmentId] is not found.
     */
    fun patchSegmentField(content: String, segmentId: String, field: String, value: String?): String {
        val lines = content.split("\n").toMutableList()
        val headerRe = Regex("^  ${Regex.escape(segmentId)}:\\s*$")
        val headerIdx = lines.indexOfFirst { headerRe.containsMatchIn(it) }
        if (headerIdx == -1) return content

        var end = headerIdx + 1
        while (end < lines.size) {
            if (lines[end].isNotEmpty() && !lines[end].startsWith("    ")) break
            end++
        }

        val fieldRe = Regex("^    ${Regex.escape(field)}:")
        val existingIdx = (headerIdx + 1 until end).firstOrNull { fieldRe.containsMatchIn(lines[it]) } ?: -1

        if (value == null) {
            if (existingIdx != -1) lines.removeAt(existingIdx)
        } else {
            val newLine = "    $field: ${quoteIfNeeded(value)}"
            if (existingIdx != -1) lines[existingIdx] = newLine else lines.add(end, newLine)
        }
        return lines.joinToString("\n")
    }

    /**
     * Add, replace, or remove an inline-array field on an existing segment.
     * An empty [items] list removes the field.
     * Returns content unchanged when [segmentId] is not found.
     */
    fun patchSegmentInlineArray(content: String, segmentId: String, field: String, items: List<String>): String {
        val lines = content.split("\n").toMutableList()
        val headerRe = Regex("^  ${Regex.escape(segmentId)}:\\s*$")
        val headerIdx = lines.indexOfFirst { headerRe.containsMatchIn(it) }
        if (headerIdx == -1) return content

        var end = headerIdx + 1
        while (end < lines.size) {
            if (lines[end].isNotEmpty() && !lines[end].startsWith("    ")) break
            end++
        }

        val fieldRe = Regex("^    ${Regex.escape(field)}:")
        val existingIdx = (headerIdx + 1 until end).firstOrNull { fieldRe.containsMatchIn(lines[it]) } ?: -1

        if (items.isEmpty()) {
            if (existingIdx != -1) lines.removeAt(existingIdx)
        } else {
            val newLine = "    $field: [${items.joinToString(", ")}]"
            if (existingIdx != -1) lines[existingIdx] = newLine else lines.add(end, newLine)
        }
        return lines.joinToString("\n")
    }

    private fun quoteIfNeeded(value: String): String =
        if (YAML_SPECIAL.containsMatchIn(value) || value != value.trim()) {
            "\"${value.replace("\\", "\\\\").replace("\"", "\\\"")}\""
        } else {
            value
        }
}
