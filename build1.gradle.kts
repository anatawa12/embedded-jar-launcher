import com.anatawa12.gradleRust.CargoBuildTask
import com.anatawa12.gradleRust.CargoToolChain

plugins {
    id("com.github.johnrengelman.shadow") version "6.1.0"
    id("com.anatawa12.gradle-rust")
    java
    application
}

sourceSets.main.get().java.srcDir("src/main/lib")

group = project(":").group
version = project(":").version

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.google.protobuf:protobuf-java:3.6.1")
}

application {
    // use mainClassName for shadow
    //mainClass.set("NativeWrapperRunner")
    @Suppress("DEPRECATION")
    mainClassName = "NativeWrapperRunner"
}

val mainClassName = "com.anatawa12.protobuf.compiler.PluginMain"

// the task to copy jar and write some text for
val copyJar by tasks.creating(Copy::class) {
    from(tasks.shadowJar.flatMap { it.archiveFile })
    into("native/resources")
    dependsOn(tasks.shadowJar)
    rename { "all.jar" }
    doLast {
        file("native/resources").mkdirs()
        file("native/resources/main_class_name.txt")
            .writeText(mainClassName)
    }
}

val crates = mutableMapOf<String, CargoBuildTask>()

val cargoProj = cargo.projects.create("native") {
    projectDir.set(project.projectDir.resolve("native"))
    destinationDir.set(project.projectDir.resolve("native/target"))
    targetName.set("embedded-jar-launcher")
    toolChain.set(CargoToolChain.cross)
    dependencyTasks.add(copyJar)

    tasks.build.get().dependsOn(buildTask)
}

val currentTarget = cargoProj.targets.create("current") {
    target.set("") // current
    dependsOn(copyJar)
    toolChain.set(CargoToolChain.default)
}
crates["current"] = currentTarget

// cross build configuration. this configuration works for my mac environment. may not work for your environment
if (project.hasProperty("cross") && project.property("cross") != "false") {
    crates["linux-aarch_64"] = cargoProj.targets.create("aarch64-unknown-linux-gnu") {
        dependsOn(copyJar)
    }
    crates["linux-x86_64"] = cargoProj.targets.create("x86_64-unknown-linux-gnu") {
        dependsOn(copyJar)
    }

    crates["osx-aarch_64"] = cargoProj.targets.create("aarch64-apple-darwin") {
        // can't build with cross
        toolChain.set(CargoToolChain.default)
        dependsOn(copyJar)
    }
    crates["osx-x86_64"] = cargoProj.targets.create("x86_64-apple-darwin") {
        // can't build with cross
        toolChain.set(CargoToolChain.default)
        dependsOn(copyJar)
    }

    crates["windows-x86_64"] = cargoProj.targets.create("x86_64-pc-windows-gnu") {
        dependsOn(copyJar)
    }
}

// copies native binaries matching to maven native binary classifier
tasks.create("copyRust") {
    crates.values.forEach { dependsOn(it) }
    for ((classifier, task) in crates) {
        inputs.file(task.binaryFile.get())
        outputs.file(buildDir.resolve("libs")
            .resolve("${base.archivesBaseName}-$version-$classifier.exe"))
    }
    doLast {
        for ((classifier, task) in crates) {
            copy {
                from(task.binaryFile.get())
                into(buildDir.resolve("libs"))
                rename { "${base.archivesBaseName}-$version-$classifier.exe" }
            }
        }
        copy {
            from(crates["current"]!!.binaryFile.get())
            into(buildDir.resolve("libs"))
            rename { "protoc-gen-lw-java" }
        }
    }
}
