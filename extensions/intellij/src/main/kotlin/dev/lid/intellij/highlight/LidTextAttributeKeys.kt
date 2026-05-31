package dev.lid.intellij.highlight

import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey

object LidTextAttributeKeys {
    // "@spec" keyword in source citations
    val SPEC_KEYWORD: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "LID_SPEC_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD
    )
    // The spec ID itself (e.g. "AUTH-001")
    val SPEC_ID: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "LID_SPEC_ID", DefaultLanguageHighlighterColors.CONSTANT
    )
    // Status marker in spec files: [ ], [x], [D]
    val SPEC_STATUS: TextAttributesKey = TextAttributesKey.createTextAttributesKey(
        "LID_SPEC_STATUS", DefaultLanguageHighlighterColors.STRING
    )
}
