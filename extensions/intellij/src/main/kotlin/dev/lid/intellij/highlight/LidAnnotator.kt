package dev.lid.intellij.highlight

import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.Annotator
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile

class LidAnnotator : Annotator {

    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        if (element !is PsiFile) return
        val text = element.text

        // @spec AUTH-001 in any source-code comment
        for (match in CITATION_PATTERN.findAll(text)) {
            highlight(holder, match.groups[1]!!.range, LidTextAttributeKeys.SPEC_KEYWORD)
            highlight(holder, match.groups[2]!!.range, LidTextAttributeKeys.SPEC_ID)
        }

        // - [x] **AUTH-001**: text  in *-specs.md files
        if (element.name.endsWith("-specs.md")) {
            for (match in DEFINITION_PATTERN.findAll(text)) {
                highlight(holder, match.groups[1]!!.range, LidTextAttributeKeys.SPEC_STATUS)
                highlight(holder, match.groups[2]!!.range, LidTextAttributeKeys.SPEC_ID)
            }
        }
    }

    // IntRange from Kotlin regex is inclusive on both ends; TextRange end is exclusive.
    private fun highlight(holder: AnnotationHolder, range: IntRange, key: TextAttributesKey) {
        holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
            .range(TextRange(range.first, range.last + 1))
            .textAttributes(key)
            .create()
    }

    companion object {
        // Matches: @spec AUTH-001  /  @spec AUTH-UI-042
        private val CITATION_PATTERN =
            Regex("""(@spec)\s+([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*-\d+)""")

        // Matches: - [ ] **AUTH-001**:  or  - [x] **AUTH-001**:  or  - [D] **AUTH-001**:
        private val DEFINITION_PATTERN =
            Regex("""-\s+\[([xD ])\]\s+\*\*([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*-\d+)\*\*""")
    }
}
