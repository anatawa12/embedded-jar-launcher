@file:Suppress("UnstableApiUsage")

package com.anatawa12.gradleRust

import groovy.lang.Closure
import org.gradle.api.Action
import org.gradle.api.NamedDomainObjectContainer
import org.gradle.api.Project
import org.gradle.kotlin.dsl.*

open class CargoExtension(private val project: Project) {
    val toolChain = project.objects.property(CargoToolChain::class)
        .convention(project.provider {
            CargoToolChain.default
                ?: error("cargo tool chain not found so it's required to set toolChain manually")
        })
    val projects = project.objects.domainObjectContainer(CargoProject::class) { CargoProject(it, project, this) }

    fun projects(block: Action<NamedDomainObjectContainer<CargoProject>>) {
        block.execute(projects)
    }

    fun projects(block: Closure<*>) {
        projects.configure(block)
    }
}
