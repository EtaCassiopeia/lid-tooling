package dev.lid.intellij

import com.intellij.openapi.components.Service
import com.intellij.openapi.extensions.PluginDescriptor
import java.nio.file.Path

@Service(Service.Level.APP)
class LidPluginService(private val descriptor: PluginDescriptor) {
    val pluginPath: Path get() = descriptor.pluginPath
}
