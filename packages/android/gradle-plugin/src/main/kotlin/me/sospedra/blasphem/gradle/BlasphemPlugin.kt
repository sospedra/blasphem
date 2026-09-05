package me.sospedra.blasphem.gradle

import com.android.build.api.variant.AndroidComponentsExtension
import groovy.json.JsonOutput
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import javax.inject.Inject

class LocaleSelection {
    internal var value: List<String>? = null
    fun set(value: String) {
        require(value.trim().lowercase() == "all") { "String locales must be all" }
        this.value = listOf("all")
    }
    fun set(value: List<String>) { this.value = value.toList() }
}

open class BlasphemExtension @Inject constructor(objects: ObjectFactory) {
    val locales = LocaleSelection()
    val assets: Property<String> = objects.property(String::class.java).convention("bundled")
    val detectLanguage: Property<Boolean> = objects.property(Boolean::class.java).convention(true)
}

abstract class GenerateBlasphemConfig : DefaultTask() {
    @get:Input abstract val configuration: Property<String>
    @get:OutputDirectory abstract val outputDirectory: DirectoryProperty
    @TaskAction fun generate() {
        val target = outputDirectory.file("blasphem/bundle.json").get().asFile
        target.parentFile.mkdirs()
        target.writeText(configuration.get())
        javaClass.getResourceAsStream("/blasphem-NOTICE")!!.use { input ->
            target.resolveSibling("NOTICE").outputStream().use { input.copyTo(it) }
        }
    }
}

class BlasphemPlugin : Plugin<Project> {
    override fun apply(project: Project) {
        val extension = project.extensions.create("blasphem", BlasphemExtension::class.java)
        val task = project.tasks.register("generateBlasphemConfig", GenerateBlasphemConfig::class.java) {
            it.outputDirectory.set(project.layout.buildDirectory.dir("generated/blasphem/assets"))
        }
        var android = false
        listOf("com.android.application", "com.android.library").forEach { plugin ->
            project.pluginManager.withPlugin(plugin) {
                android = true
                val components = project.extensions.getByType(AndroidComponentsExtension::class.java)
                components.onVariants { variant ->
                    variant.sources.assets?.addGeneratedSourceDirectory(task, GenerateBlasphemConfig::outputDirectory)
                }
            }
        }
        project.afterEvaluate {
            if (!android) throw GradleException("Blasphem requires an Android application or library")
            val requested = extension.locales.value ?: throw GradleException("blasphem.locales is required")
            if (requested.isEmpty()) throw GradleException("blasphem.locales cannot be empty")
            val aliases = LOCALES.flatMap { (code, aliases) -> (aliases + code).map { it to code } }.toMap()
            val codes = if (requested == listOf("all")) LOCALES.map { it.first } else requested.map {
                aliases[it.trim().lowercase()] ?: throw GradleException("Unsupported Blasphem locale: $it")
            }.distinct().sortedBy { code -> LOCALES.indexOfFirst { it.first == code } }
            val mode = when (val input = extension.assets.get()) {
                "bundled" -> "bundled"
                "remote", "jsdelivr" -> "remote"
                else -> throw GradleException("Unsupported Blasphem assets: $input")
            }
            val files = codes.flatMap { code ->
                if (extension.detectLanguage.get()) listOf("$code.pack", "$code.detect") else listOf("$code.pack")
            }.distinct()
            task.configure {
                it.configuration.set(JsonOutput.toJson(linkedMapOf(
                    "formatVersion" to 1, "engineVersion" to RELEASE_VERSION, "dataVersion" to RELEASE_VERSION,
                    "locales" to codes, "assets" to mode, "detectLanguage" to extension.detectLanguage.get(),
                    "files" to files, "manifest" to mapOf("bytes" to MANIFEST_BYTES, "sha256" to MANIFEST_SHA256),
                )))
            }
            project.dependencies.add("implementation", "me.sospedra.blasphem:blasphem:$RELEASE_VERSION")
            if (mode == "bundled") files.forEach { file ->
                val (code, kind) = file.split('.')
                project.dependencies.add("implementation", "me.sospedra.blasphem:blasphem-$kind-$code:$RELEASE_VERSION")
            }
        }
    }
}
