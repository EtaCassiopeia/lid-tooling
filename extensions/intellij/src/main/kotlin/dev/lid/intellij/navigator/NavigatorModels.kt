package dev.lid.intellij.navigator

data class ArrowEntry(
    val status: String? = null,
    val detail: String? = null,
    val blocks: List<String>? = null,
    val children: List<String>? = null,
    val parent: String? = null,
    val next: String? = null,
    val drift: String? = null,
    val sampled: String? = null,
    val audited: String? = null,
)

data class ArrowIndex(
    val arrows: Map<String, ArrowEntry> = emptyMap(),
    val taxonomy: Map<String, List<String>> = emptyMap(),
)

data class SpecCounts(
    var implemented: Int = 0,
    var open: Int = 0,
    var deferred: Int = 0,
)

data class SpecItem(
    val marker: String,
    val id: String,
    val text: String,
    val line: Int,
)

data class SpecInfo(
    val counts: SpecCounts = SpecCounts(),
    val items: MutableList<SpecItem> = mutableListOf(),
    var specFile: String? = null,
    var lldFile: String? = null,
    var specPrefix: String? = null,
)

data class GraphNode(
    val id: String,
    val label: String,
    val status: String,
    val next: String? = null,
    val drift: String? = null,
    val sampled: String? = null,
    val audited: String? = null,
    val specs: SpecCounts? = null,
    val specItems: List<SpecItem>? = null,
    val specFile: String? = null,
    val lldFile: String? = null,
    val specPrefix: String? = null,
    val children: List<String>? = null,
    val parent: String? = null,
)

data class GraphEdge(val source: String, val target: String, val kind: String)

data class GraphPayload(
    val nodes: List<GraphNode>,
    val edges: List<GraphEdge>,
    val clusters: Map<String, List<String>>,
)
