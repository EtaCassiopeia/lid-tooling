package dev.lid.intellij.navigator

import com.google.gson.Gson
import com.google.gson.JsonObject

sealed class NavigatorMessage {
    data class Open(val segmentId: String) : NavigatorMessage()
    data class OpenFile(val path: String, val line: Int?) : NavigatorMessage()
    data class UpdateSpecStatus(
        val specFile: String,
        val specId: String,
        val line: Int,
        val newStatus: String,
    ) : NavigatorMessage()
    data class AddSpec(
        val specFile: String?,
        val segmentId: String,
        val specId: String,
        val text: String,
        val specPrefix: String? = null,
    ) : NavigatorMessage()
    data class UpdateSegmentStatus(val segmentId: String, val newStatus: String) : NavigatorMessage()
    data class UpdateSegmentMeta(val segmentId: String, val next: String, val drift: String) : NavigatorMessage()
    data class AddSegment(
        val segmentId: String,
        val status: String,
        val detail: String,
        val blocks: List<String>,
        val children: List<String>,
        val specPrefix: String? = null,
    ) : NavigatorMessage()
    data class RemoveConnection(
        val kind: String,
        val segmentId: String,
        val target: String,
    ) : NavigatorMessage()
}

fun parseMessage(json: String): NavigatorMessage {
    val obj = Gson().fromJson(json, JsonObject::class.java)
    return when (val type = obj.get("type")?.asString) {
        "open" -> NavigatorMessage.Open(obj.get("segmentId").asString)
        "openFile" -> NavigatorMessage.OpenFile(
            obj.get("path").asString,
            obj.get("line")?.takeUnless { it.isJsonNull }?.asInt,
        )
        "updateSpecStatus" -> NavigatorMessage.UpdateSpecStatus(
            obj.get("specFile").asString,
            obj.get("specId").asString,
            obj.get("line").asInt,
            obj.get("newStatus").asString,
        )
        "addSpec" -> NavigatorMessage.AddSpec(
            obj.get("specFile")?.takeUnless { it.isJsonNull }?.asString,
            obj.get("segmentId").asString,
            obj.get("specId").asString,
            obj.get("text").asString,
            obj.get("specPrefix")?.takeUnless { it.isJsonNull }?.asString,
        )
        "updateSegmentStatus" -> NavigatorMessage.UpdateSegmentStatus(
            obj.get("segmentId").asString,
            obj.get("newStatus").asString,
        )
        "updateSegmentMeta" -> NavigatorMessage.UpdateSegmentMeta(
            obj.get("segmentId").asString,
            obj.get("next")?.asString ?: "",
            obj.get("drift")?.asString ?: "",
        )
        "addSegment" -> NavigatorMessage.AddSegment(
            obj.get("segmentId").asString,
            obj.get("status").asString,
            obj.get("detail").asString,
            obj.get("blocks")?.asJsonArray?.map { it.asString } ?: emptyList(),
            obj.get("children")?.asJsonArray?.map { it.asString } ?: emptyList(),
            obj.get("specPrefix")?.takeUnless { it.isJsonNull }?.asString,
        )
        "removeConnection" -> NavigatorMessage.RemoveConnection(
            obj.get("kind").asString,
            obj.get("segmentId").asString,
            obj.get("target").asString,
        )
        else -> error("Unknown navigator message type: $type")
    }
}
