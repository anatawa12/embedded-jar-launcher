package com.anatawa12.gradleRust

import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.kotlin.dsl.create

class GradlePlugin : Plugin<Project> {
    lateinit var extension: CargoExtension
        private set

    override fun apply(project: Project) {
        extension = project.extensions
            .create<CargoExtension>("cargo", project)
    }
}
