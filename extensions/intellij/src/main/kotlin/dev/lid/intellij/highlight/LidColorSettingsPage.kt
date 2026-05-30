package dev.lid.intellij.highlight

import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.PlainSyntaxHighlighter
import com.intellij.openapi.options.colors.AttributesDescriptor
import com.intellij.openapi.options.colors.ColorDescriptor
import com.intellij.openapi.options.colors.ColorSettingsPage

class LidColorSettingsPage : ColorSettingsPage {

    override fun getDisplayName(): String = "LID"
    override fun getIcon() = null
    override fun getHighlighter() = PlainSyntaxHighlighter()

    override fun getDemoText(): String = """
        // @spec AUTH-001 validate password on login
        fun authenticate(user: String, pass: String): Boolean {
            // @spec AUTH-002 sessions expire after 30 minutes
            return checkCredentials(user, pass)
        }

        // In auth-specs.md:
        - [ ] **AUTH-001**: the system shall validate password strength
        - [x] **AUTH-002**: sessions shall expire after 30 minutes of inactivity
        - [D] **AUTH-003**: biometric login deferred to v2
    """.trimIndent()

    override fun getAdditionalHighlightingTagToDescriptorMap(): Map<String, TextAttributesKey>? = null

    override fun getAttributeDescriptors(): Array<AttributesDescriptor> = arrayOf(
        AttributesDescriptor("@spec keyword", LidTextAttributeKeys.SPEC_KEYWORD),
        AttributesDescriptor("Spec ID (e.g. AUTH-001)", LidTextAttributeKeys.SPEC_ID),
        AttributesDescriptor("Spec status marker ([ ], [x], [D])", LidTextAttributeKeys.SPEC_STATUS),
    )

    override fun getColorDescriptors(): Array<ColorDescriptor> = ColorDescriptor.EMPTY_ARRAY
}
