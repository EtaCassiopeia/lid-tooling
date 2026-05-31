package dev.lid.intellij.navigator

import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test

// Realistic fixture that mirrors the urlshort index.yaml:
// dates as bare YAML strings, comments, inline arrays, taxonomy section.
private val REALISTIC_INDEX = """
schema_version: 2
last_updated: 2026-05-29

taxonomy:
  infrastructure: [storage, auth]
  domain: [shortener-core]

arrows:
  # ── Storage layer ────────────────────────────────────────────────────────────
  storage:
    status: MAPPED
    sampled: 2026-04-10
    audited: 2026-04-15
    detail: storage.md
    blocks: [shortener-core]
    children: [in-memory-store]
    next: "provision Redis cluster; promote redis-store to MAPPED after infrastructure is ready"

  in-memory-store:
    status: OK
    parent: storage
    audited: 2026-04-22
    detail: storage/in-memory-store.md

  # ── Domain layer ─────────────────────────────────────────────────────────────
  shortener-core:
    status: MAPPED
    sampled: 2026-04-20
    detail: shortener-core.md

  auth:
    status: UNMAPPED
    detail: auth.md
    blocks: [shortener-core]
    next: "implement HMAC-SHA256 API-key middleware"

unmapped:
  docs:
    intent: []
""".trimIndent() + "\n"

// ── buildSegmentBlock ─────────────────────────────────────────────────────────

class BuildSegmentBlockTest {

    @Test fun headerIsTwoSpaceIndented() {
        val result = IndexYamlWriter.buildSegmentBlock(
            "billing", mapOf("status" to "UNMAPPED", "detail" to "billing/core.md"),
        )
        assertTrue(result.startsWith("  billing:"))
    }

    @Test fun statusAppearsBeforeDetail() {
        val result = IndexYamlWriter.buildSegmentBlock(
            "billing", mapOf("status" to "UNMAPPED", "detail" to "billing/core.md"),
        )
        assertTrue(result.indexOf("    status:") < result.indexOf("    detail:"))
    }

    @Test fun arraysAreKeptInline() {
        val result = IndexYamlWriter.buildSegmentBlock(
            "billing",
            mapOf("status" to "UNMAPPED", "detail" to "billing/core.md", "blocks" to listOf("api", "auth")),
        )
        assertTrue(result.contains("    blocks: [api, auth]"))
        assertFalse(result.contains("    - api"))
    }

    @Test fun emptyArraysAreOmitted() {
        val result = IndexYamlWriter.buildSegmentBlock(
            "billing",
            mapOf(
                "status" to "UNMAPPED",
                "detail" to "billing/core.md",
                "blocks" to emptyList<String>(),
                "children" to emptyList<String>(),
            ),
        )
        assertFalse(result.contains("blocks"))
        assertFalse(result.contains("children"))
    }

    @Test fun yamlSpecialCharsAreQuoted() {
        val result = IndexYamlWriter.buildSegmentBlock(
            "billing",
            mapOf(
                "status" to "UNMAPPED",
                "detail" to "billing/core.md",
                "next" to "implement auth: see issue #42",
            ),
        )
        assertTrue(result.contains("\"implement auth: see issue #42\""))
    }

    @Test fun plainStringsAreNotQuoted() {
        val result = IndexYamlWriter.buildSegmentBlock(
            "billing", mapOf("status" to "UNMAPPED", "detail" to "billing/core.md"),
        )
        assertTrue(result.contains("    status: UNMAPPED"))
        assertTrue(result.contains("    detail: billing/core.md"))
    }
}

// ── insertIntoArrows ──────────────────────────────────────────────────────────

class InsertIntoArrowsTest {

    private val block = IndexYamlWriter.buildSegmentBlock(
        "billing", mapOf("status" to "UNMAPPED", "detail" to "billing/core.md"),
    )

    @Test fun insertsBeforeNextTopLevelKey() {
        val result = IndexYamlWriter.insertIntoArrows(REALISTIC_INDEX, block)
        val billingIdx = result.indexOf("  billing:")
        val unmappedIdx = result.indexOf("unmapped:")
        assertTrue(billingIdx > -1)
        assertTrue(billingIdx < unmappedIdx)
    }

    @Test fun newSegmentAppearsExactlyOnce() {
        val result = IndexYamlWriter.insertIntoArrows(REALISTIC_INDEX, block)
        assertEquals(1, Regex("  billing:").findAll(result).count())
    }

    @Test fun commentsArePreserved() {
        val result = IndexYamlWriter.insertIntoArrows(REALISTIC_INDEX, block)
        assertTrue(result.contains("# ── Storage layer"))
        assertTrue(result.contains("# ── Domain layer"))
    }

    @Test fun datesAreNotCoerced() {
        val result = IndexYamlWriter.insertIntoArrows(REALISTIC_INDEX, block)
        assertTrue(result.contains("sampled: 2026-04-10"))
        assertTrue(result.contains("audited: 2026-04-15"))
        assertFalse(result.contains("T00:00:00"))
    }

    @Test fun inlineArraysArePreserved() {
        val result = IndexYamlWriter.insertIntoArrows(REALISTIC_INDEX, block)
        assertTrue(result.contains("    blocks: [shortener-core]"))
        assertTrue(result.contains("    children: [in-memory-store]"))
        assertTrue(result.contains("  infrastructure: [storage, auth]"))
    }

    @Test fun everyOriginalLineIsPresent() {
        val result = IndexYamlWriter.insertIntoArrows(REALISTIC_INDEX, block)
        for (line in REALISTIC_INDEX.lines()) {
            assertTrue(result.contains(line), "Original line missing: $line")
        }
    }

    @Test fun noBLockedByInjected() {
        val result = IndexYamlWriter.insertIntoArrows(REALISTIC_INDEX, block)
        assertFalse(result.contains("blockedBy"))
    }

    @Test fun appendsWhenArrowsIsLastSection() {
        val minimal = "schema_version: 2\narrows:\n  auth:\n    status: UNMAPPED\n    detail: auth.md\n"
        val result = IndexYamlWriter.insertIntoArrows(minimal, block)
        assertTrue(result.contains("  billing:"))
        assertTrue(result.indexOf("  billing:") > result.indexOf("  auth:"))
    }
}

// ── patchSegmentField ─────────────────────────────────────────────────────────

class PatchSegmentFieldTest {

    @Test fun addsNewFieldToTargetSegment() {
        val result = IndexYamlWriter.patchSegmentField(REALISTIC_INDEX, "auth", "parent", "billing")
        assertTrue(result.contains("    parent: billing"))
    }

    @Test fun replacesExistingFieldValue() {
        val result = IndexYamlWriter.patchSegmentField(REALISTIC_INDEX, "auth", "status", "MAPPED")
        val authBlock = result.substring(result.indexOf("  auth:"))
        assertEquals("    status: MAPPED", authBlock.lines()[1])
        // Status count must not change
        assertEquals(
            Regex("    status: ").findAll(REALISTIC_INDEX).count(),
            Regex("    status: ").findAll(result).count(),
        )
    }

    @Test fun doesNotTouchOtherSegments() {
        val result = IndexYamlWriter.patchSegmentField(REALISTIC_INDEX, "auth", "parent", "billing")
        val storageBlock = result.substring(
            result.indexOf("  storage:"),
            result.indexOf("  in-memory-store:"),
        )
        assertFalse(storageBlock.contains("parent: billing"))
    }

    @Test fun returnsContentUnchangedWhenSegmentNotFound() {
        val result = IndexYamlWriter.patchSegmentField(REALISTIC_INDEX, "nonexistent", "parent", "billing")
        assertEquals(REALISTIC_INDEX, result)
    }

    @Test fun removesFieldWhenValueIsNull() {
        // in-memory-store has `parent: storage`; removing it should leave the segment intact
        val result = IndexYamlWriter.patchSegmentField(REALISTIC_INDEX, "in-memory-store", "parent", null)
        val block = result.substring(result.indexOf("  in-memory-store:"))
            .lines()
            .takeWhile { it.startsWith("    ") || it == "  in-memory-store:" }
        assertFalse(block.any { it.contains("parent:") })
        // Other fields of that segment must still be present
        assertTrue(result.contains("    audited: 2026-04-22"))
    }
}

// ── patchSegmentInlineArray ───────────────────────────────────────────────────

class PatchSegmentInlineArrayTest {

    @Test fun replacesExistingInlineArray() {
        val result = IndexYamlWriter.patchSegmentInlineArray(
            REALISTIC_INDEX, "auth", "blocks", listOf("api"),
        )
        val authLines = result.substring(result.indexOf("  auth:"))
            .lines()
            .takeWhile { it.startsWith("    ") || it == "  auth:" }
        assertTrue(authLines.any { it.contains("blocks: [api]") })
        assertFalse(authLines.any { it.contains("blocks: [shortener-core]") })
    }

    @Test fun removesFieldWhenListIsEmpty() {
        val result = IndexYamlWriter.patchSegmentInlineArray(
            REALISTIC_INDEX, "auth", "blocks", emptyList(),
        )
        val authBlock = result.substring(result.indexOf("  auth:"))
            .lines()
            .takeWhile { it.startsWith("    ") || it == "  auth:" }
        assertFalse(authBlock.any { it.contains("blocks:") })
        // Other content of `auth` must be intact
        assertTrue(result.contains("    status: UNMAPPED"))
    }

    @Test fun addsNewArrayField() {
        val result = IndexYamlWriter.patchSegmentInlineArray(
            REALISTIC_INDEX, "shortener-core", "blocks", listOf("auth"),
        )
        assertTrue(result.contains("    blocks: [auth]"))
    }

    @Test fun doesNotTouchOtherSegments() {
        val result = IndexYamlWriter.patchSegmentInlineArray(
            REALISTIC_INDEX, "auth", "blocks", listOf("api"),
        )
        // storage's blocks must be unchanged
        assertTrue(result.contains("    blocks: [shortener-core]").also {
            // ensure it's still under storage, not auth
            val storageBlock = result.substring(
                result.indexOf("  storage:"),
                result.indexOf("  in-memory-store:"),
            )
            assertTrue(storageBlock.contains("blocks: [shortener-core]"))
        })
    }
}
