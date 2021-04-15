@file:Suppress("UnstableApiUsage")

package com.anatawa12.gradleRust

import org.gradle.api.DefaultTask
import org.gradle.api.file.RegularFile
import org.gradle.api.provider.Provider
import org.gradle.api.tasks.*
import org.gradle.kotlin.dsl.*
import java.io.File

open class CargoBuildTask : DefaultTask(), EnvironmentProperties {
    private val plugin = project.plugins.getPlugin(GradlePlugin::class)

    /**
     * path to cargo command.
     */
    @Input
    val toolChain = project.objects.property(CargoToolChain::class).convention(plugin.extension.toolChain)

    /**
     * Compile target.
     * empty string or "current" for current machine target.
     */
    @Input
    val target = project.objects.property(String::class).convention("")

    /**
     * path to project
     */
    @InputDirectory
    val projectDir = project.objects.directoryProperty()

    /**
     * is this build release build. defaults true.
     */
    @Input
    val releaseBuild = project.objects.property(Boolean::class).convention(true)

    /**
     * path to output directory
     */
    @get:Internal
    val destinationDir = project.objects.directoryProperty()
        .convention(project.layout.buildDirectory
            .map { it.dir("cargo/") })

    private val targetNormalized: String?
        get() {
            val target = target.get()
            if (target == "") return null
            if (target == "current") return null
            return target
        }

    @OutputDirectory
    val targetDirectory = project.provider { targetNormalized?.let { destinationDir.dir(it) } ?: destinationDir }
        .flatMap { it }

    @get:Internal
    val targetName = project.objects.property(String::class).convention(projectDir.map { it.asFile.name })

    @get:Internal
    val targetFileName by lazy {
        val (prefix, suffix) = toolChain.get().getDestinationFileType(targetNormalized, "bin")
        prefix + targetName.get() + suffix
    }

    /**
     * path to the destination file
     */
    @get:OutputFile
    val binaryFile: Provider<RegularFile> = project.objects.fileProperty()
        .convention(targetDirectory.map { it.file((if (releaseBuild.get()) "release/" else "debug/") + targetFileName) })

    private val container = EnvironmentPropertiesContainer()
    @get:Internal
    override val environment get() = container.environment
    @get:Input
    override val allEnvironment get() = container.allEnvironment
    override fun environment(environmentVariables: Map<String, *>) = container.environment(environmentVariables)
    override fun environment(name: String, value: Any?) = container.environment(name, value)
    override fun extendsFrom(parent: EnvironmentProperties) = container.extendsFrom(parent)

    private fun relativeOrSelf(file: File, workdir: File): File {
        val relativeDst = file.relativeToOrNull(workdir)
        if (relativeDst != null && !relativeDst.startsWith("..")) {
            return relativeDst
        }
        return file
    }

    @TaskAction
    fun runCargo() {
        val workdir: File
        val manifestPath: Any
        val projectDir = projectDir.get()
        workdir = projectDir.asFile
        manifestPath = "Cargo.toml"
        val destinationDir = destinationDir.get().asFile

        project.exec {
            isIgnoreExitValue = false

            workingDir(workdir)

            environment(allEnvironment)

            executable = toolChain.get().cargo
            args("build")
            targetNormalized?.let { target ->
                args("--target", target)
            }
            if (releaseBuild.get())
                args("--release")
            args("--target-dir", destinationDir)

            args("--manifest-path", manifestPath)
        }
    }

}
