package dev.lid.intellij.navigator

import org.junit.jupiter.api.Assertions.*
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class LidProjectReaderTest {

    // ── Decision doc discovery ────────────────────────────────────────────────

    @Test
    fun `empty intent dir returns empty map`(@TempDir intentDir: File) {
        val result = buildSpecInfo(intentDir)
        assertTrue(result.isEmpty())
    }

    @Test
    fun `nonexistent intent dir returns empty map`() {
        val result = buildSpecInfo(File("/nonexistent/docs/intent"))
        assertTrue(result.isEmpty())
    }

    @Test
    fun `discovers single decision doc for a segment`(@TempDir intentDir: File) {
        val decisionsDir = intentDir.resolve("auth/decisions").apply { mkdirs() }
        decisionsDir.resolve("token.md").writeText("# Token Storage\n")

        val result = buildSpecInfo(intentDir)
        val docs = result["auth"]?.decisionDocs ?: fail("auth not found")
        assertEquals(1, docs.size)
        assertTrue(docs[0].endsWith("token.md"))
    }

    @Test
    fun `discovers multiple decision docs sorted alphabetically`(@TempDir intentDir: File) {
        val decisionsDir = intentDir.resolve("auth/decisions").apply { mkdirs() }
        decisionsDir.resolve("tokens.md").writeText("# Tokens\n")
        decisionsDir.resolve("sessions.md").writeText("# Sessions\n")

        val result = buildSpecInfo(intentDir)
        val docs = result["auth"]?.decisionDocs ?: fail("auth not found")
        assertEquals(2, docs.size)
        assertTrue(docs[0].endsWith("sessions.md"), "expected sessions.md first, got ${docs[0]}")
        assertTrue(docs[1].endsWith("tokens.md"))
    }

    @Test
    fun `ignores non-md files inside decisions dir`(@TempDir intentDir: File) {
        val decisionsDir = intentDir.resolve("auth/decisions").apply { mkdirs() }
        decisionsDir.resolve("notes.txt").writeText("plain text")
        decisionsDir.resolve("arch.md").writeText("# Arch\n")

        val result = buildSpecInfo(intentDir)
        assertEquals(1, result["auth"]?.decisionDocs?.size)
    }

    @Test
    fun `decision docs for different segments stay separate`(@TempDir intentDir: File) {
        intentDir.resolve("auth/decisions").apply { mkdirs() }
            .resolve("token.md").writeText("# Token\n")
        intentDir.resolve("payment/decisions").apply { mkdirs() }
            .resolve("stripe.md").writeText("# Stripe\n")

        val result = buildSpecInfo(intentDir)
        assertEquals(1, result["auth"]?.decisionDocs?.size)
        assertEquals(1, result["payment"]?.decisionDocs?.size)
        assertFalse(result["auth"]!!.decisionDocs[0].contains("stripe"))
    }

    @Test
    fun `segment with no decisions dir has empty decisionDocs`(@TempDir intentDir: File) {
        intentDir.resolve("auth").mkdirs()
        intentDir.resolve("auth/auth-specs.md").writeText("# auth\n- [ ] **AUTH-001**: open.\n")

        val result = buildSpecInfo(intentDir)
        assertTrue(result["auth"]?.decisionDocs?.isEmpty() ?: true)
    }

    // ── Spec parsing ──────────────────────────────────────────────────────────

    @Test
    fun `counts implemented open and deferred specs`(@TempDir intentDir: File) {
        intentDir.resolve("auth").mkdirs()
        intentDir.resolve("auth/auth-specs.md").writeText(
            "# auth\n- [x] **AUTH-001**: done.\n- [ ] **AUTH-002**: open.\n- [D] **AUTH-003**: deferred.\n"
        )

        val result = buildSpecInfo(intentDir)
        val counts = result["auth"]?.counts ?: fail("auth not found")
        assertEquals(1, counts.implemented)
        assertEquals(1, counts.open)
        assertEquals(1, counts.deferred)
    }

    @Test
    fun `reads spec prefix from YAML frontmatter`(@TempDir intentDir: File) {
        intentDir.resolve("auth").mkdirs()
        intentDir.resolve("auth/auth-specs.md").writeText(
            "---\nprefix: AUTH\n---\n# auth\n- [ ] **AUTH-001**: open.\n"
        )

        val result = buildSpecInfo(intentDir)
        assertEquals("AUTH", result["auth"]?.specPrefix)
    }

    @Test
    fun `records specFile and lldFile paths`(@TempDir intentDir: File) {
        intentDir.resolve("auth").mkdirs()
        val specFile = intentDir.resolve("auth/auth-specs.md").apply { writeText("# auth\n") }
        val lldFile = intentDir.resolve("auth/auth-design.md").apply { writeText("# Design\n") }

        val result = buildSpecInfo(intentDir)
        assertEquals(specFile.absolutePath, result["auth"]?.specFile)
        assertEquals(lldFile.absolutePath, result["auth"]?.lldFile)
    }

    @Test
    fun `merges specs and decision docs for the same segment`(@TempDir intentDir: File) {
        intentDir.resolve("auth/decisions").apply { mkdirs() }
        intentDir.resolve("auth/auth-specs.md").writeText("# auth\n- [ ] **AUTH-001**: open.\n")
        intentDir.resolve("auth/auth-design.md").writeText("# Design\n")
        intentDir.resolve("auth/decisions/token.md").writeText("# Token\n")

        val result = buildSpecInfo(intentDir)
        val info = result["auth"] ?: fail("auth not found")
        assertEquals(1, info.counts.open)
        assertNotNull(info.lldFile)
        assertEquals(1, info.decisionDocs.size)
    }
}
